use crate::contracts::events::{AuditEvent, EventType};
use crate::contracts::identifiers::{CausationId, CorrelationId};
use crate::contracts::input::UserInput;
use crate::contracts::output::YukiResult;
use crate::contracts::voice::{AudioInput, AudioOutput, VoiceError, VoiceSessionState};
use crate::core::yuki_core::YukiCore;
use crate::persistence::sqlite::SqliteAuditStore;
use crate::voice::playback::AudioPlaybackController;
use crate::voice::router::GovernedVoiceRouter;
use crate::voice::session::VoiceSessionManager;
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
        // Interrompe reprodução prévia em curso (barge-in do usuário)
        self.playback.interrupt()?;

        // 1. Inicia ou recupera sessão
        let config = self.router.config().clone();
        let session_id = format!("voice_{}_{}", device_id, uuid_ms());
        let mut session = self
            .session_manager
            .start_session(&session_id, device_id, config)?;

        session.transition_to(VoiceSessionState::Capturing)?;
        self.session_manager.update_session(&session);

        // 2. Transcrição (STT)
        session.transition_to(VoiceSessionState::Transcribing)?;
        self.session_manager.update_session(&session);

        let transcribed_text = self
            .router
            .transcribe(&audio_input, allow_cloud_stt)
            .await?;

        // Registra evento de auditoria VoiceInputReceived (minimização de dados: apenas metadados técnicos)
        self.record_audit_event(
            EventType::VoiceInputReceived,
            serde_json::json!({
                "device_id": device_id,
                "duration_ms": audio_input.duration_ms,
                "sample_rate": audio_input.format.sample_rate,
                "text_char_count": transcribed_text.chars().count(),
                "source": audio_input.source,
            }),
        )
        .await;

        if transcribed_text.trim().is_empty() {
            session.transition_to(VoiceSessionState::Idle)?;
            self.session_manager.update_session(&session);
            return Err(VoiceError::TranscriptionFailed(
                "Nenhuma fala reconhecida no áudio capturado".to_string(),
            ));
        }

        // 3. Processamento no YukiCore
        session.transition_to(VoiceSessionState::Processing)?;
        self.session_manager.update_session(&session);

        let user_input = UserInput::new(&transcribed_text);
        let user_output = self
            .core
            .process_input_async(user_input)
            .await
            .map_err(|e| VoiceError::TranscriptionFailed(format!("Erro no Core: {}", e)))?;

        // 4. Síntese de Voz (TTS)
        session.transition_to(VoiceSessionState::Synthesizing)?;
        self.session_manager.update_session(&session);

        let audio_output = self
            .router
            .synthesize(&user_output.content, allow_cloud_tts)
            .await?;

        // Registra evento de auditoria VoiceOutputProduced
        self.record_audit_event(
            EventType::VoiceOutputProduced,
            serde_json::json!({
                "device_id": device_id,
                "duration_ms": audio_output.duration_ms,
                "voice_name": audio_output.voice_name,
                "response_char_count": user_output.content.chars().count(),
            }),
        )
        .await;

        // 5. Inicia reprodução segura
        session.transition_to(VoiceSessionState::Speaking)?;
        self.session_manager.update_session(&session);

        self.playback.play(&audio_output)?;

        // Retorna a sessão para Idle (ou pronta para o próximo turno)
        session.transition_to(VoiceSessionState::Idle)?;
        self.session_manager.update_session(&session);

        Ok((user_output, audio_output))
    }

    /// Interrompe qualquer fala em andamento.
    pub fn interrupt(&self, device_id: &str) -> Result<(), VoiceError> {
        self.playback.interrupt()?;
        if let Some(mut session) = self.session_manager.get_session(device_id) {
            let _ = session.interrupt();
            self.session_manager.update_session(&session);
        }
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
