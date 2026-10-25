use crate::contracts::events::{AuditEvent, EventType};
use crate::contracts::identifiers::{CausationId, CorrelationId};
use crate::contracts::input::UserInput;
use crate::contracts::output::YukiResult;
use crate::contracts::voice::{AudioInput, AudioOutput, VoiceError, VoiceSessionState};
use crate::core::yuki_core::YukiCore;
use crate::persistence::sqlite::SqliteAuditStore;
use crate::voice::playback::AudioPlaybackController;
use crate::voice::router::GovernedVoiceRouter;
use crate::voice::session::{VoiceSession, VoiceSessionManager};
use crate::voice::wakeword::WakeWordDetector;
use std::sync::Arc;

/// Coordenador governado da conversação por voz da Yuki.
///
/// INTEGRAÇÃO ARQUITETURAL:
/// Conecta o subsistema de voz ao pipeline auditado do `YukiCore`, garantindo que:
/// 1. Áudio transcrito é tratado estritamente como dado não-confiável (`Data != Instruction`).
/// 2. Entrada de voz não é autorização (`Audio Input != Command != Authorization != Execution`).
/// 3. Emite eventos formais `VoiceInputReceived` e `VoiceOutputProduced` no armazenamento persistente.
/// 4. Preserva o mesmo perfil de personalidade e diretrizes ontológicas da interação textual.
pub struct VoiceConversationCoordinator {
    core: Arc<YukiCore>,
    router: Arc<GovernedVoiceRouter>,
    session_manager: Arc<VoiceSessionManager>,
    playback: Arc<AudioPlaybackController>,
    wake_detector: Arc<tokio::sync::Mutex<WakeWordDetector>>,
    persistent_audit: Option<Arc<SqliteAuditStore>>,
}

impl VoiceConversationCoordinator {
    pub fn new(
        core: Arc<YukiCore>,
        router: Arc<GovernedVoiceRouter>,
        session_manager: Arc<VoiceSessionManager>,
        playback: Arc<AudioPlaybackController>,
        wake_detector: Arc<tokio::sync::Mutex<WakeWordDetector>>,
        persistent_audit: Option<Arc<SqliteAuditStore>>,
    ) -> Self {
        Self {
            core,
            router,
            session_manager,
            playback,
            wake_detector,
            persistent_audit,
        }
    }

    /// Construtor de produção: inicializa o coordenador de voz utilizando o controlador
    /// de reprodução real do sistema operacional (`AudioPlaybackController::production()`).
    pub fn production(
        core: Arc<YukiCore>,
        router: Arc<GovernedVoiceRouter>,
        session_manager: Arc<VoiceSessionManager>,
        wake_detector: Arc<tokio::sync::Mutex<WakeWordDetector>>,
        persistent_audit: Option<Arc<SqliteAuditStore>>,
    ) -> Self {
        Self::new(
            core,
            router,
            session_manager,
            Arc::new(AudioPlaybackController::production()),
            wake_detector,
            persistent_audit,
        )
    }

    /// Construtor de conveniência para ambiente de produção padrão utilizando provedores locais soberanos.
    pub fn production_default(
        core: Arc<YukiCore>,
        persistent_audit: Option<Arc<SqliteAuditStore>>,
    ) -> Self {
        let config = crate::contracts::voice::VoiceConfiguration::default();
        let router = Arc::new(GovernedVoiceRouter::default_local(config));
        let session_manager = Arc::new(VoiceSessionManager::new());
        let wake_detector = Arc::new(tokio::sync::Mutex::new(WakeWordDetector::default()));
        Self::production(
            core,
            router,
            session_manager,
            wake_detector,
            persistent_audit,
        )
    }

    /// Construtor de teste: inicializa o coordenador de voz utilizando o controlador
    /// mock por construção (`AudioPlaybackController::mock()`), garantindo silêncio absoluto.
    pub fn for_test(
        core: Arc<YukiCore>,
        router: Arc<GovernedVoiceRouter>,
        session_manager: Arc<VoiceSessionManager>,
        wake_detector: Arc<tokio::sync::Mutex<WakeWordDetector>>,
        persistent_audit: Option<Arc<SqliteAuditStore>>,
    ) -> (Self, Arc<crate::voice::playback::MockPlaybackSink>) {
        let (playback, mock) = AudioPlaybackController::mock();
        let coordinator = Self::new(
            core,
            router,
            session_manager,
            Arc::new(playback),
            wake_detector,
            persistent_audit,
        );
        (coordinator, mock)
    }

    /// Construtor para testes ao vivo (opt-in explícito via `YUKI_LIVE_AUDIO_TESTS=1`).
    pub fn live_test(
        core: Arc<YukiCore>,
        router: Arc<GovernedVoiceRouter>,
        session_manager: Arc<VoiceSessionManager>,
        wake_detector: Arc<tokio::sync::Mutex<WakeWordDetector>>,
        persistent_audit: Option<Arc<SqliteAuditStore>>,
    ) -> Self {
        Self::new(
            core,
            router,
            session_manager,
            Arc::new(AudioPlaybackController::live_test()),
            wake_detector,
            persistent_audit,
        )
    }

    /// Construtor parametrizado pelo ambiente de execução (Produção, Teste, Desabilitado).
    pub fn for_environment(
        env: crate::voice::playback::PlaybackEnvironment,
        core: Arc<YukiCore>,
        router: Arc<GovernedVoiceRouter>,
        session_manager: Arc<VoiceSessionManager>,
        wake_detector: Arc<tokio::sync::Mutex<WakeWordDetector>>,
        persistent_audit: Option<Arc<SqliteAuditStore>>,
    ) -> Self {
        let playback = Arc::new(AudioPlaybackController::for_environment(env));
        Self::new(
            core,
            router,
            session_manager,
            playback,
            wake_detector,
            persistent_audit,
        )
    }

    pub fn playback(&self) -> &AudioPlaybackController {
        &self.playback
    }

    pub fn session_manager(&self) -> &VoiceSessionManager {
        &self.session_manager
    }

    pub fn wake_detector(&self) -> &tokio::sync::Mutex<WakeWordDetector> {
        &self.wake_detector
    }

    /// Processa ativação direta por botão ou atalho de terminal.
    ///
    /// Fluxo:
    /// 1. Inicia sessão no estado `Capturing`.
    /// 2. Transcreve o áudio (`Transcribing`).
    /// 3. Encaminha o texto para o `YukiCore` (`Processing`).
    /// 4. Sintetiza a resposta falada (`Synthesizing`).
    /// 5. Envia áudio para reprodução (`Speaking`).
    /// 6. Registra auditoria durável sem vazar payload binário bruto.
    pub async fn process_button_turn(
        &self,
        device_id: &str,
        audio_input: AudioInput,
        allow_cloud_stt: bool,
        allow_cloud_tts: bool,
    ) -> Result<(YukiResult, AudioOutput), VoiceError> {
        // 1. Inicia sessão governada (geração isolada e in_flight = true).
        // A validação de conflito de sessão ocorre ANTES de tocar no playback.
        // Se a tentativa for rejeitada por SessionConflict (ex: turno já em andamento),
        // o áudio da sessão ativa NÃO É INTERROMPIDO.
        let config = self.router.config().clone();
        let session_id = format!("voice_{}_{}", device_id, uuid_ms());
        let mut session = self
            .session_manager
            .start_session(&session_id, device_id, config)?;

        // Sessão iniciada legitimamente para este dispositivo:
        // Interrompe qualquer áudio remanescente no sink de turnos anteriores já concluídos.
        let _ = self.playback.interrupt();

        // Guardião RAII que assegura liberação de in_flight caso a future seja abortada/dropada
        let mut guard = SessionTurnGuard::new(
            self.session_manager.clone(),
            device_id.to_string(),
            session_id.clone(),
            session.generation,
        );

        let turn_result = self
            .execute_turn_steps(
                &mut session,
                device_id,
                audio_input,
                allow_cloud_stt,
                allow_cloud_tts,
            )
            .await;

        match turn_result {
            Ok(output) => {
                // Conclusão atômica do turno:
                // Valida session_id, generation, in_flight e ausência de Interrupted sob lock exclusivo no manager.
                // Se uma interrupção tardia tiver ocorrido após o último passo de execute_turn_steps,
                // complete_turn libera in_flight = false, preserva Interrupted e retorna Err(VoiceError::Interrupted).
                match self.session_manager.complete_turn(
                    device_id,
                    &session.session_id,
                    session.generation,
                ) {
                    Ok(()) => {
                        guard.mark_completed();
                        Ok(output)
                    }
                    Err(e) => {
                        let _ = self.playback.interrupt();
                        guard.mark_completed();

                        let phase = session.state.to_string();
                        let error_kind = match &e {
                            VoiceError::Interrupted => "Interrupted",
                            VoiceError::SessionConflict(_) => "SessionConflict",
                            _ => "Other",
                        };
                        let err_str = e.to_string();
                        let sanitized_err = sanitize_error_for_audit(&err_str);

                        self.record_audit_event(
                            EventType::VoiceProcessingFailed,
                            serde_json::json!({
                                "device_id": device_id,
                                "session_id": session_id,
                                "generation": session.generation,
                                "phase": phase,
                                "error_kind": error_kind,
                                "error": sanitized_err,
                            }),
                        )
                        .await;

                        Err(e)
                    }
                }
            }
            Err(e) => {
                let err_str = e.to_string();
                let phase = session.state.to_string();
                let is_interrupted = matches!(e, VoiceError::Interrupted)
                    || self.session_manager.is_interrupted(
                        device_id,
                        &session.session_id,
                        session.generation,
                    );

                let error_state = if is_interrupted {
                    VoiceSessionState::Interrupted
                } else if session.state == VoiceSessionState::Idle {
                    VoiceSessionState::Idle
                } else {
                    VoiceSessionState::Failed(err_str.clone())
                };

                // Encerramento atômico com liberação garantida de in_flight
                self.session_manager.fail_turn(
                    device_id,
                    &session_id,
                    session.generation,
                    error_state,
                );
                guard.mark_completed();

                let error_kind = match &e {
                    VoiceError::TranscriptionFailed(_) => "TranscriptionFailed",
                    VoiceError::SynthesisFailed(_) => "SynthesisFailed",
                    VoiceError::UnauthorizedCloudFallback(_) => "UnauthorizedCloudFallback",
                    VoiceError::ProviderUnavailable(_) => "ProviderUnavailable",
                    VoiceError::Timeout(_) => "Timeout",
                    VoiceError::Interrupted => "Interrupted",
                    VoiceError::SessionConflict(_) => "SessionConflict",
                    VoiceError::InvalidStateTransition { .. } => "InvalidStateTransition",
                    VoiceError::PlaybackFailed(_) => "PlaybackFailed",
                    VoiceError::AudioTooLarge { .. } => "AudioTooLarge",
                    VoiceError::AudioTooLong { .. } => "AudioTooLong",
                    VoiceError::DeviceUnavailable(_) => "DeviceUnavailable",
                    VoiceError::Io(_) => "Io",
                    _ => "Other",
                };

                let sanitized_err = sanitize_error_for_audit(&err_str);

                self.record_audit_event(
                    EventType::VoiceProcessingFailed,
                    serde_json::json!({
                        "device_id": device_id,
                        "session_id": session_id,
                        "generation": session.generation,
                        "phase": phase,
                        "error_kind": error_kind,
                        "error": sanitized_err,
                    }),
                )
                .await;

                Err(e)
            }
        }
    }

    async fn execute_turn_steps(
        &self,
        session: &mut VoiceSession,
        device_id: &str,
        audio_input: AudioInput,
        allow_cloud_stt: bool,
        allow_cloud_tts: bool,
    ) -> Result<(YukiResult, AudioOutput), VoiceError> {
        // 1. Captura
        session.transition_to(VoiceSessionState::Capturing)?;
        if !self.session_manager.update_session(session)
            || self.session_manager.is_interrupted(
                device_id,
                &session.session_id,
                session.generation,
            )
        {
            return Err(VoiceError::Interrupted);
        }

        // 2. Transcrição (STT)
        session.transition_to(VoiceSessionState::Transcribing)?;
        if !self.session_manager.update_session(session)
            || self.session_manager.is_interrupted(
                device_id,
                &session.session_id,
                session.generation,
            )
        {
            return Err(VoiceError::Interrupted);
        }

        let transcribed_text = self
            .router
            .transcribe(&audio_input, allow_cloud_stt)
            .await?;

        if !self.session_manager.update_session(session)
            || self.session_manager.is_interrupted(
                device_id,
                &session.session_id,
                session.generation,
            )
        {
            return Err(VoiceError::Interrupted);
        }

        // Registra evento de auditoria VoiceInputReceived (minimização de dados: apenas metadados técnicos)
        self.record_audit_event(
            EventType::VoiceInputReceived,
            serde_json::json!({
                "device_id": device_id,
                "session_id": session.session_id,
                "generation": session.generation,
                "duration_ms": audio_input.duration_ms,
                "sample_rate": audio_input.format.sample_rate,
                "text_char_count": transcribed_text.chars().count(),
                "source": audio_input.source,
            }),
        )
        .await;

        if transcribed_text.trim().is_empty() {
            session.transition_to(VoiceSessionState::Idle)?;
            self.session_manager.update_session(session);
            return Err(VoiceError::TranscriptionFailed(
                "Nenhuma fala reconhecida no áudio capturado".to_string(),
            ));
        }

        // 3. Processamento no YukiCore
        session.transition_to(VoiceSessionState::Processing)?;
        if !self.session_manager.update_session(session)
            || self.session_manager.is_interrupted(
                device_id,
                &session.session_id,
                session.generation,
            )
        {
            return Err(VoiceError::Interrupted);
        }

        let user_input = UserInput::new(&transcribed_text);
        let user_output = self
            .core
            .process_input_async(user_input)
            .await
            .map_err(|e| VoiceError::TranscriptionFailed(format!("Erro no Core: {}", e)))?;

        if !self.session_manager.update_session(session)
            || self.session_manager.is_interrupted(
                device_id,
                &session.session_id,
                session.generation,
            )
        {
            return Err(VoiceError::Interrupted);
        }

        // 4. Síntese de Voz (TTS)
        session.transition_to(VoiceSessionState::Synthesizing)?;
        if !self.session_manager.update_session(session)
            || self.session_manager.is_interrupted(
                device_id,
                &session.session_id,
                session.generation,
            )
        {
            return Err(VoiceError::Interrupted);
        }

        let audio_output = self
            .router
            .synthesize(&user_output.content, allow_cloud_tts)
            .await?;

        if !self.session_manager.update_session(session)
            || self.session_manager.is_interrupted(
                device_id,
                &session.session_id,
                session.generation,
            )
        {
            return Err(VoiceError::Interrupted);
        }

        // Registra evento de auditoria VoiceOutputProduced
        self.record_audit_event(
            EventType::VoiceOutputProduced,
            serde_json::json!({
                "device_id": device_id,
                "session_id": session.session_id,
                "generation": session.generation,
                "duration_ms": audio_output.duration_ms,
                "voice_name": audio_output.voice_name,
                "response_char_count": user_output.content.chars().count(),
            }),
        )
        .await;

        // 5. Inicia reprodução segura
        session.transition_to(VoiceSessionState::Speaking)?;
        if !self.session_manager.update_session(session)
            || self.session_manager.is_interrupted(
                device_id,
                &session.session_id,
                session.generation,
            )
        {
            return Err(VoiceError::Interrupted);
        }

        self.playback.play(&audio_output)?;

        // Verificação pós-disparo para mitigar corrida entre check e início do playback:
        if self
            .session_manager
            .is_interrupted(device_id, &session.session_id, session.generation)
        {
            let _ = self.playback.interrupt();
            return Err(VoiceError::Interrupted);
        }

        Ok((user_output, audio_output))
    }

    /// Interrompe qualquer fala em andamento e assinala a sessão ativa como interrompida (barge-in intencional).
    pub fn interrupt(&self, device_id: &str) -> Result<(), VoiceError> {
        self.playback.interrupt()?;
        self.session_manager.interrupt_session(device_id);
        Ok(())
    }

    async fn record_audit_event(&self, event_type: EventType, payload: serde_json::Value) {
        if let Some(audit) = &self.persistent_audit {
            let correlation = CorrelationId::new();
            let causation = CausationId::new("voice_conversation");
            let event = AuditEvent::new(
                event_type,
                correlation,
                causation,
                payload,
                "yuki_voice_coordinator",
            );
            if let Ok(clean) = event.assert_no_secrets().map(|_| event) {
                let _ = audit.record_event(&clean);
            }
        }
    }
}

fn uuid_ms() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("{:x}", ms)
}

/// Guardião RAII de ciclo de vida do turno conversacional.
/// Assegura que `in_flight` seja liberado e que a sessão seja transicionada
/// para `Interrupted` caso a `Future` do turno seja cancelada, abortada ou caia prematuramente.
struct SessionTurnGuard {
    session_manager: Arc<VoiceSessionManager>,
    device_id: String,
    session_id: String,
    generation: u64,
    completed: bool,
}

impl SessionTurnGuard {
    fn new(
        session_manager: Arc<VoiceSessionManager>,
        device_id: String,
        session_id: String,
        generation: u64,
    ) -> Self {
        Self {
            session_manager,
            device_id,
            session_id,
            generation,
            completed: false,
        }
    }

    fn mark_completed(&mut self) {
        self.completed = true;
    }
}

impl Drop for SessionTurnGuard {
    fn drop(&mut self) {
        if !self.completed {
            self.session_manager.fail_turn(
                &self.device_id,
                &self.session_id,
                self.generation,
                VoiceSessionState::Interrupted,
            );
        }
    }
}

/// Sanitiza mensagens de erro técnicas antes da persistência no registro de auditoria,
/// prevenindo a exposição acidental de credenciais, chaves, senhas ou payloads excessivos.
/// Garante truncamento respeitando estritamente os limites de caracteres válidos em UTF-8.
pub fn sanitize_error_for_audit(err: &str) -> String {
    let mut sanitized = err.to_string();
    let sensitive_keys = [
        "bearer ",
        "password",
        "api_key",
        "apikey",
        "private_key",
        "secret_key",
        "access_token",
        "refresh_token",
    ];
    for key in &sensitive_keys {
        if sanitized.to_lowercase().contains(key) {
            sanitized = format!("[REDACTED_SENSITIVE: error message contained '{}']", key);
            break;
        }
    }
    // Trunca mensagens excessivamente longas respeitando fronteiras de caracteres válidas em UTF-8
    // para evitar exaustão de payload no SQLite e pânico ao fatiar no meio de caractere multibyte.
    const MAX_AUDIT_ERROR_BYTES: usize = 500;
    if sanitized.len() > MAX_AUDIT_ERROR_BYTES {
        let mut boundary = MAX_AUDIT_ERROR_BYTES;
        while boundary > 0 && !sanitized.is_char_boundary(boundary) {
            boundary -= 1;
        }
        sanitized.truncate(boundary);
        sanitized.push_str("... [truncated]");
    }
    sanitized
}
