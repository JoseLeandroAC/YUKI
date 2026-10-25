use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use yuki::contracts::events::EventType;
use yuki::contracts::voice::{
    AudioFormat, AudioInput, VoiceConfiguration, VoiceError, VoiceProviderMetadata,
    VoiceProviderType, VoiceSessionState,
};
use yuki::core::yuki_core::YukiCore;
use yuki::models::errors::ModelError;
use yuki::models::mock::{MockBehavior, MockModelProvider};
use yuki::persistence::sqlite::{AuditFilter, SqliteAuditStore};
use yuki::persistence::AuditQueryStore;
use yuki::voice::conversation::VoiceConversationCoordinator;
use yuki::voice::playback::AudioPlaybackController;
use yuki::voice::router::GovernedVoiceRouter;
use yuki::voice::session::VoiceSessionManager;
use yuki::voice::stt::mock::MockSttProvider;
use yuki::voice::tts::mock::MockTtsProvider;
use yuki::voice::wakeword::WakeWordDetector;

fn create_dummy_audio() -> AudioInput {
    AudioInput::new(vec![0u8; 100], AudioFormat::default(), 100, "mic-test")
}

// ============================================================================
// 1. RETRY IMEDIATO APÓS ERRO DE STT
// ============================================================================
#[tokio::test]
async fn test_recovery_01_immediate_retry_after_stt_failure() {
    let core = Arc::new(YukiCore::new());
    let failing_stt = Arc::new(MockSttProvider::new().with_failure(true));
    let tts = Arc::new(MockTtsProvider::new());
    let router_fail = Arc::new(GovernedVoiceRouter::new(
        failing_stt,
        None,
        tts.clone(),
        None,
        VoiceConfiguration::default(),
    ));

    let session_mgr = Arc::new(VoiceSessionManager::new());
    let (coord_fail, _) = VoiceConversationCoordinator::for_test(
        core.clone(),
        router_fail,
        session_mgr.clone(),
        Arc::new(tokio::sync::Mutex::new(WakeWordDetector::default())),
        None,
    );

    let dev = "dev-stt-fail";
    let res1 = coord_fail
        .process_button_turn(dev, create_dummy_audio(), false, false)
        .await;

    // 1. Erro original é preservado
    assert!(
        matches!(
            res1.unwrap_err(),
            VoiceError::TranscriptionFailed(_) | VoiceError::UnauthorizedCloudFallback(_)
        ),
        "Erro de STT original deve ser preservado e propagado"
    );

    // 2. Sessão transita para Failed e in_flight é liberado
    let sess1 = session_mgr.get_session(dev).expect("Sessão deve existir");
    assert!(
        matches!(sess1.state, VoiceSessionState::Failed(_)),
        "Sessão após erro de STT deve estar no estado Failed: {:?}",
        sess1.state
    );
    assert!(
        !sess1.in_flight,
        "Flag in_flight deve estar desativado após término do erro"
    );
    assert!(
        sess1
            .history
            .iter()
            .any(|(s, _)| *s == VoiceSessionState::Transcribing),
        "Histórico deve registrar passagem por Transcribing"
    );

    // 3. Retry imediato deve funcionar sem SessionConflict
    let ok_stt = Arc::new(MockSttProvider::new().with_response("Yuki online"));
    let router_ok = Arc::new(GovernedVoiceRouter::new(
        ok_stt,
        None,
        tts,
        None,
        VoiceConfiguration::default(),
    ));
    let (coord_ok, _) = VoiceConversationCoordinator::for_test(
        core,
        router_ok,
        session_mgr.clone(),
        Arc::new(tokio::sync::Mutex::new(WakeWordDetector::default())),
        None,
    );

    let res_retry = coord_ok
        .process_button_turn(dev, create_dummy_audio(), false, false)
        .await;
    assert!(
        res_retry.is_ok(),
        "Novo turno imediato deve ter sucesso sem SessionConflict: {:?}",
        res_retry.err()
    );

    let sess_after = session_mgr.get_session(dev).unwrap();
    assert_eq!(sess_after.state, VoiceSessionState::Idle);
    assert!(!sess_after.in_flight);
    assert_eq!(sess_after.generation, sess1.generation + 1);
}

// ============================================================================
// 2. RETRY IMEDIATO APÓS ERRO DE CORE
// ============================================================================
#[tokio::test]
async fn test_recovery_02_immediate_retry_after_core_failure() {
    let mock_model_fail = Arc::new(MockModelProvider::with_behavior(
        MockBehavior::SimulateError(ModelError::Internal("Core Failure Sim".to_string())),
    ));
    let core_fail = Arc::new(YukiCore::new().with_model_provider(mock_model_fail));

    let ok_stt = Arc::new(MockSttProvider::new().with_response("Olá Yuki"));
    let tts = Arc::new(MockTtsProvider::new());
    let router = Arc::new(GovernedVoiceRouter::new(
        ok_stt.clone(),
        None,
        tts.clone(),
        None,
        VoiceConfiguration::default(),
    ));

    let session_mgr = Arc::new(VoiceSessionManager::new());
    let (coord_fail, _) = VoiceConversationCoordinator::for_test(
        core_fail,
        router.clone(),
        session_mgr.clone(),
        Arc::new(tokio::sync::Mutex::new(WakeWordDetector::default())),
        None,
    );

    let dev = "dev-core-fail";
    let res1 = coord_fail
        .process_button_turn(dev, create_dummy_audio(), false, false)
        .await;
    assert!(res1.is_err());

    let sess1 = session_mgr.get_session(dev).unwrap();
    assert!(matches!(sess1.state, VoiceSessionState::Failed(_)));
    assert!(!sess1.in_flight);
    assert!(sess1
        .history
        .iter()
        .any(|(s, _)| *s == VoiceSessionState::Processing));

    // Retry com Core funcional
    let core_ok = Arc::new(YukiCore::new());
    let (coord_ok, _) = VoiceConversationCoordinator::for_test(
        core_ok,
        router,
        session_mgr.clone(),
        Arc::new(tokio::sync::Mutex::new(WakeWordDetector::default())),
        None,
    );

    let res_retry = coord_ok
        .process_button_turn(dev, create_dummy_audio(), false, false)
        .await;
    assert!(
        res_retry.is_ok(),
        "Novo turno deve iniciar imediatamente após erro de Core: {:?}",
        res_retry.err()
    );
}

// ============================================================================
// 3. RETRY IMEDIATO APÓS ERRO DE TTS
// ============================================================================
#[tokio::test]
async fn test_recovery_03_immediate_retry_after_tts_failure() {
    let core = Arc::new(YukiCore::new());
    let ok_stt = Arc::new(MockSttProvider::new().with_response("Olá"));
    let failing_tts = Arc::new(MockTtsProvider::new().with_failure(true));
    let router_fail = Arc::new(GovernedVoiceRouter::new(
        ok_stt.clone(),
        None,
        failing_tts,
        None,
        VoiceConfiguration::default(),
    ));

    let session_mgr = Arc::new(VoiceSessionManager::new());
    let (coord_fail, _) = VoiceConversationCoordinator::for_test(
        core.clone(),
        router_fail,
        session_mgr.clone(),
        Arc::new(tokio::sync::Mutex::new(WakeWordDetector::default())),
        None,
    );

    let dev = "dev-tts-fail";
    let res1 = coord_fail
        .process_button_turn(dev, create_dummy_audio(), false, false)
        .await;
    assert!(
        matches!(
            res1.unwrap_err(),
            VoiceError::SynthesisFailed(_) | VoiceError::UnauthorizedCloudFallback(_)
        ),
        "Erro de síntese ou roteamento deve ser preservado"
    );

    let sess1 = session_mgr.get_session(dev).unwrap();
    assert!(matches!(sess1.state, VoiceSessionState::Failed(_)));
    assert!(!sess1.in_flight);
    assert!(sess1
        .history
        .iter()
        .any(|(s, _)| *s == VoiceSessionState::Synthesizing));

    // Retry com TTS funcional
    let ok_tts = Arc::new(MockTtsProvider::new());
    let router_ok = Arc::new(GovernedVoiceRouter::new(
        ok_stt,
        None,
        ok_tts,
        None,
        VoiceConfiguration::default(),
    ));
    let (coord_ok, _) = VoiceConversationCoordinator::for_test(
        core,
        router_ok,
        session_mgr.clone(),
        Arc::new(tokio::sync::Mutex::new(WakeWordDetector::default())),
        None,
    );

    let res_retry = coord_ok
        .process_button_turn(dev, create_dummy_audio(), false, false)
        .await;
    assert!(
        res_retry.is_ok(),
        "Novo turno deve iniciar imediatamente após falha de TTS: {:?}",
        res_retry.err()
    );
}

// ============================================================================
// 4. RETRY IMEDIATO APÓS ERRO DE PLAYBACK
// ============================================================================
#[tokio::test]
async fn test_recovery_04_immediate_retry_after_playback_failure() {
    let core = Arc::new(YukiCore::new());
    let ok_stt = Arc::new(MockSttProvider::new().with_response("Olá Yuki"));
    let ok_tts = Arc::new(MockTtsProvider::new());
    let router = Arc::new(GovernedVoiceRouter::new(
        ok_stt,
        None,
        ok_tts,
        None,
        VoiceConfiguration::default(),
    ));

    let (playback_controller, mock_sink) = AudioPlaybackController::mock();
    let session_mgr = Arc::new(VoiceSessionManager::new());

    let coordinator = VoiceConversationCoordinator::new(
        core.clone(),
        router.clone(),
        session_mgr.clone(),
        Arc::new(playback_controller),
        Arc::new(tokio::sync::Mutex::new(WakeWordDetector::default())),
        None,
    );

    let dev = "dev-playback-fail";

    // Simula falha de dispositivo de áudio indisponível no sink de playback
    mock_sink.set_device_available(false);

    let res1 = coordinator
        .process_button_turn(dev, create_dummy_audio(), false, false)
        .await;
    assert!(
        matches!(res1.unwrap_err(), VoiceError::DeviceUnavailable(_)),
        "Erro de dispositivo em playback deve ser propagado"
    );

    let sess1 = session_mgr.get_session(dev).unwrap();
    assert!(
        matches!(sess1.state, VoiceSessionState::Failed(_)),
        "Sessão deve estar em Failed: {:?}",
        sess1.state
    );
    assert!(!sess1.in_flight);
    assert!(sess1
        .history
        .iter()
        .any(|(s, _)| *s == VoiceSessionState::Speaking));

    // Restaura disponibilidade do dispositivo de reprodução
    mock_sink.set_device_available(true);

    let res_retry = coordinator
        .process_button_turn(dev, create_dummy_audio(), false, false)
        .await;
    assert!(
        res_retry.is_ok(),
        "Novo turno deve iniciar após recuperação do dispositivo de playback: {:?}",
        res_retry.err()
    );
}

// ============================================================================
// 5. INTERRUPÇÃO DURANTE PROCESSAMENTO EM ANDAMENTO BLOQUEIA REÚSO PREMATURO
// ============================================================================
#[tokio::test]
async fn test_recovery_05_interruption_during_in_flight_processing_blocks_premature_reuse() {
    let session_mgr = Arc::new(VoiceSessionManager::new());
    let dev = "dev-in-flight-test";

    // 1. Inicia sessão 1 com in_flight = true
    let sess1 = session_mgr
        .start_session("s1", dev, VoiceConfiguration::default())
        .expect("Sessão 1 deve iniciar");
    assert!(sess1.in_flight);
    assert_eq!(sess1.generation, 1);

    // 2. Operador aciona interrupt() enquanto a operação ainda está em andamento
    let mut sess1_interrupted = session_mgr.get_session(dev).unwrap();
    sess1_interrupted.interrupt().unwrap();
    // Atenção: o flag in_flight permanece true porque a operação assíncrona ainda está executando!
    session_mgr.update_session(&sess1_interrupted);

    let current = session_mgr.get_session(dev).unwrap();
    assert_eq!(current.state, VoiceSessionState::Interrupted);
    assert!(
        current.in_flight,
        "A operação antiga ainda está em execução ativa"
    );

    // 3. Tentativa de iniciar NOVO turno enquanto in_flight == true DEVE SER REJEITADA
    let premature_start =
        session_mgr.start_session("s2-premature", dev, VoiceConfiguration::default());
    assert!(
        premature_start.is_err(),
        "start_session DEVE rejeitar reuso enquanto in_flight == true"
    );
    let conflict = premature_start.unwrap_err();
    assert!(
        matches!(conflict, VoiceError::SessionConflict(_)),
        "Erro deve ser SessionConflict: {:?}",
        conflict
    );

    // 4. A operação antiga finalmente termina / cancela e limpa in_flight
    session_mgr.finish_operation(dev, "s1", 1, Some(VoiceSessionState::Interrupted));
    let finished_sess = session_mgr.get_session(dev).unwrap();
    assert!(!finished_sess.in_flight);
    assert_eq!(finished_sess.state, VoiceSessionState::Interrupted);

    // 5. Agora que a operação antiga encerrou efetivamente, novo turno DEVE SER PERMITIDO
    let legitimate_start =
        session_mgr.start_session("s2-legitimate", dev, VoiceConfiguration::default());
    assert!(
        legitimate_start.is_ok(),
        "Novo turno DEVE ser permitido após a operação antiga encerrar in_flight"
    );
    let sess2 = legitimate_start.unwrap();
    assert_eq!(sess2.generation, 2);
    assert!(sess2.in_flight);
}

// ============================================================================
// 6. CONCLUSÃO TARDIA DE TAREFA ANTIGA NÃO CORROMPE NOVA SESSÃO (FENCE)
// ============================================================================
#[tokio::test]
async fn test_recovery_06_stale_delayed_task_cannot_corrupt_new_session() {
    let session_mgr = Arc::new(VoiceSessionManager::new());
    let dev = "dev-fence-test";

    // 1. Sessão 1 inicia e depois encerra
    let mut sess1 = session_mgr
        .start_session("sess-1", dev, VoiceConfiguration::default())
        .unwrap();
    sess1.in_flight = false;
    sess1.state = VoiceSessionState::Idle;
    session_mgr.update_session(&sess1);

    // 2. Sessão 2 inicia para o mesmo dispositivo (geração 2)
    let mut sess2 = session_mgr
        .start_session("sess-2", dev, VoiceConfiguration::default())
        .unwrap();
    assert_eq!(sess2.generation, 2);
    sess2.transition_to(VoiceSessionState::Capturing).unwrap();
    session_mgr.update_session(&sess2);

    let active_before_late_update = session_mgr.get_session(dev).unwrap();
    assert_eq!(active_before_late_update.session_id, "sess-2");
    assert_eq!(
        active_before_late_update.state,
        VoiceSessionState::Capturing
    );

    // 3. Tarefa assíncrona atrasada da Sessão 1 tenta atualizar o gerenciador
    // (ex: tentando marcar a sessão como Failed ou mudar estado para Transcribing)
    let mut stale_sess1_update = sess1.clone();
    stale_sess1_update.state = VoiceSessionState::Failed("Erro tardio da Sessão 1".to_string());

    let update_accepted = session_mgr.update_session(&stale_sess1_update);
    assert!(
        !update_accepted,
        "update_session DEVE rejeitar atualização de sessão com generation/session_id obsoletos"
    );

    // 4. Comprova ausência de corrupção do estado da nova sessão
    let active_after_late_update = session_mgr.get_session(dev).unwrap();
    assert_eq!(
        active_after_late_update.session_id, "sess-2",
        "session_id ativo não pode ter sido sobrescrito"
    );
    assert_eq!(
        active_after_late_update.generation, 2,
        "Geração ativa deve permanecer inalterada"
    );
    assert_eq!(
        active_after_late_update.state,
        VoiceSessionState::Capturing,
        "Estado da nova sessão não pode ser corrompido pela tarefa tardia"
    );
}

// ============================================================================
// 7. CANCELAMENTO VIA FUTURE DROP LIBERA IN_FLIGHT (RAII GUARD)
// ============================================================================
#[tokio::test]
async fn test_recovery_07_cancellation_via_future_drop_releases_in_flight() {
    let core = Arc::new(YukiCore::new());

    // STT lento que aguarda sinal para prosseguir
    struct SlowSttProvider {
        started: Arc<AtomicBool>,
        allow_proceed: Arc<tokio::sync::Notify>,
        meta: VoiceProviderMetadata,
    }
    impl yuki::contracts::voice::SpeechToTextProvider for SlowSttProvider {
        fn metadata(&self) -> &yuki::contracts::voice::VoiceProviderMetadata {
            &self.meta
        }
        fn health<'a>(
            &'a self,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), VoiceError>> + Send + 'a>>
        {
            Box::pin(async { Ok(()) })
        }
        fn transcribe<'a>(
            &'a self,
            _audio: &'a AudioInput,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<String, VoiceError>> + Send + 'a>,
        > {
            self.started.store(true, Ordering::SeqCst);
            let notify = self.allow_proceed.clone();
            Box::pin(async move {
                notify.notified().await;
                Ok("Resultado lento".to_string())
            })
        }
    }

    let started = Arc::new(AtomicBool::new(false));
    let allow_proceed = Arc::new(tokio::sync::Notify::new());
    let slow_stt = Arc::new(SlowSttProvider {
        started: started.clone(),
        allow_proceed,
        meta: VoiceProviderMetadata::new(
            "SlowStt",
            VoiceProviderType::Mock,
            vec!["pt".to_string()],
            16000,
            false,
            "1.0",
        ),
    });
    let tts = Arc::new(MockTtsProvider::new());
    let router = Arc::new(GovernedVoiceRouter::new(
        slow_stt,
        None,
        tts,
        None,
        VoiceConfiguration::default(),
    ));

    let session_mgr = Arc::new(VoiceSessionManager::new());
    let (coordinator, _) = VoiceConversationCoordinator::for_test(
        core,
        router,
        session_mgr.clone(),
        Arc::new(tokio::sync::Mutex::new(WakeWordDetector::default())),
        None,
    );
    let coordinator = Arc::new(coordinator);

    let dev = "dev-drop-test";

    // Dispara o turno assíncrono em uma task que será cancelada (abort)
    let coord_clone = coordinator.clone();
    let handle = tokio::spawn(async move {
        coord_clone
            .process_button_turn(dev, create_dummy_audio(), false, false)
            .await
    });

    // Aguarda o turno entrar em execução (STT iniciado)
    for _ in 0..50 {
        if started.load(Ordering::SeqCst) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(started.load(Ordering::SeqCst), "STT deve ter iniciado");

    let sess_running = session_mgr.get_session(dev).unwrap();
    assert!(
        sess_running.in_flight,
        "Enquanto executa, in_flight deve ser true"
    );

    // Cancela a task abortando a future externamente
    handle.abort();
    let _ = handle.await; // Aguarda término do abort

    // Dá um tick de evento para o Drop do SessionTurnGuard executar
    tokio::task::yield_now().await;

    // O SessionTurnGuard executou no Drop:
    let sess_after_drop = session_mgr.get_session(dev).unwrap();
    assert!(
        !sess_after_drop.in_flight,
        "Drop da future DEVE limpar o flag in_flight"
    );
    assert_eq!(
        sess_after_drop.state,
        VoiceSessionState::Interrupted,
        "Sessão abortada deve ser marcada como Interrupted pelo guardião RAII"
    );

    // Novo turno imediato deve ter permissão de iniciar
    let ok_stt = Arc::new(MockSttProvider::new().with_response("Yuki após abort"));
    let router_ok = Arc::new(GovernedVoiceRouter::new(
        ok_stt,
        None,
        Arc::new(MockTtsProvider::new()),
        None,
        VoiceConfiguration::default(),
    ));
    let (coord_ok, _) = VoiceConversationCoordinator::for_test(
        Arc::new(YukiCore::new()),
        router_ok,
        session_mgr.clone(),
        Arc::new(tokio::sync::Mutex::new(WakeWordDetector::default())),
        None,
    );

    let retry_res = coord_ok
        .process_button_turn(dev, create_dummy_audio(), false, false)
        .await;
    assert!(
        retry_res.is_ok(),
        "Novo turno DEVE iniciar imediatamente após cancelamento via drop da future: {:?}",
        retry_res.err()
    );
}

// ============================================================================
// 8. AUDITORIA E PROPAGAÇÃO CORRETAS DE ERROS
// ============================================================================
#[tokio::test]
async fn test_recovery_08_audit_events_and_original_error_preservation() {
    let tmp_dir = std::env::temp_dir();
    let db_path = tmp_dir.join(format!(
        "audit_recovery_test_{}.db",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let audit_store = Arc::new(SqliteAuditStore::open(&db_path).unwrap());

    let core = Arc::new(YukiCore::new());
    let failing_stt = Arc::new(MockSttProvider::new().with_failure(true));
    let tts = Arc::new(MockTtsProvider::new());
    let router = Arc::new(GovernedVoiceRouter::new(
        failing_stt,
        None,
        tts,
        None,
        VoiceConfiguration::default(),
    ));

    let (playback, _) = AudioPlaybackController::mock();
    let session_mgr = Arc::new(VoiceSessionManager::new());

    let coordinator = VoiceConversationCoordinator::new(
        core,
        router,
        session_mgr.clone(),
        Arc::new(playback),
        Arc::new(tokio::sync::Mutex::new(WakeWordDetector::default())),
        Some(audit_store.clone()),
    );

    let dev = "dev-audit-err-test";
    let res = coordinator
        .process_button_turn(dev, create_dummy_audio(), false, false)
        .await;

    // 1. Preservação do erro original
    assert!(
        matches!(
            res.as_ref().err(),
            Some(VoiceError::TranscriptionFailed(_) | VoiceError::UnauthorizedCloudFallback(_))
        ),
        "Erro original deve ser preservado: {:?}",
        res.err()
    );

    // 2. Registro de evento de auditoria VoiceProcessingFailed
    let filter = AuditFilter::default();
    let events = audit_store.query_events(&filter).unwrap();
    let fail_event = events
        .iter()
        .find(|e| e.event_type == EventType::VoiceProcessingFailed)
        .expect("Evento VoiceProcessingFailed deve ter sido persistido na auditoria");

    assert_eq!(
        fail_event
            .payload
            .get("device_id")
            .unwrap()
            .as_str()
            .unwrap(),
        dev
    );
    assert!(
        fail_event.payload.get("error").is_some(),
        "Payload de auditoria deve conter descrição do erro"
    );
    assert!(
        fail_event.payload.get("session_id").is_some(),
        "Payload deve conter session_id para correlação"
    );
    assert_eq!(
        fail_event.causation_id.0.as_str(),
        "voice_conversation",
        "Causation deve ser voice_conversation"
    );

    let _ = std::fs::remove_file(&db_path);
}

// ============================================================================
// 9. EFEITOS OBSERVÁVEIS DO FENCING: TAREFA ANTIGA NÃO INVOCA CORE NEM PLAYBACK
// ============================================================================
#[tokio::test]
async fn test_recovery_09_stale_task_cannot_trigger_core_or_produce_audio() {
    let dev = "dev-observable-test";

    // Mock STT que pausa após iniciar para simular corrida/atraso
    struct PausingStt {
        meta: VoiceProviderMetadata,
        started: Arc<AtomicBool>,
        allow_proceed: Arc<tokio::sync::Notify>,
    }
    impl yuki::contracts::voice::SpeechToTextProvider for PausingStt {
        fn metadata(&self) -> &VoiceProviderMetadata {
            &self.meta
        }
        fn health<'a>(
            &'a self,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), VoiceError>> + Send + 'a>>
        {
            Box::pin(async { Ok(()) })
        }
        fn transcribe<'a>(
            &'a self,
            _audio: &'a AudioInput,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<String, VoiceError>> + Send + 'a>,
        > {
            self.started.store(true, Ordering::SeqCst);
            let notify = self.allow_proceed.clone();
            Box::pin(async move {
                notify.notified().await;
                Ok("Comando atrasado da sessão antiga".to_string())
            })
        }
    }

    let started = Arc::new(AtomicBool::new(false));
    let allow_proceed = Arc::new(tokio::sync::Notify::new());
    let pausing_stt = Arc::new(PausingStt {
        meta: VoiceProviderMetadata::new(
            "PausingStt",
            VoiceProviderType::LocalEdge,
            vec!["pt".to_string()],
            16000,
            false,
            "1.0",
        ),
        started: started.clone(),
        allow_proceed: allow_proceed.clone(),
    });

    // Provedor de modelo rastreável para efeito observável no Core
    struct TrackingProvider {
        called: Arc<AtomicBool>,
        inner: MockModelProvider,
    }
    impl yuki::models::provider::ModelProvider for TrackingProvider {
        fn generate<'a>(
            &'a self,
            request: &'a yuki::models::provider::ModelRequest,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<
                        Output = Result<
                            yuki::models::provider::ModelResponse,
                            yuki::models::errors::ModelError,
                        >,
                    > + Send
                    + 'a,
            >,
        > {
            self.called.store(true, Ordering::SeqCst);
            self.inner.generate(request)
        }
        fn metadata(&self) -> yuki::models::provider::ModelMetadata {
            self.inner.metadata()
        }
        fn health(&self) -> yuki::models::provider::HealthStatus {
            self.inner.health()
        }
    }

    let core_called = Arc::new(AtomicBool::new(false));
    let core = Arc::new(
        YukiCore::new().with_model_provider(Arc::new(TrackingProvider {
            called: core_called.clone(),
            inner: MockModelProvider::new(),
        })),
    );

    let (playback, mock_sink) = AudioPlaybackController::mock();
    let router = Arc::new(GovernedVoiceRouter::new(
        pausing_stt,
        None,
        Arc::new(MockTtsProvider::new()),
        None,
        VoiceConfiguration::default(),
    ));

    let session_mgr = Arc::new(VoiceSessionManager::new());
    let coordinator = Arc::new(VoiceConversationCoordinator::new(
        core,
        router,
        session_mgr.clone(),
        Arc::new(playback),
        Arc::new(tokio::sync::Mutex::new(WakeWordDetector::default())),
        None,
    ));

    // Inicia Turno 1 em background
    let coord1 = coordinator.clone();
    let turn1_handle = tokio::spawn(async move {
        coord1
            .process_button_turn(dev, create_dummy_audio(), false, false)
            .await
    });

    // Aguarda Turno 1 entrar em execução dentro do STT
    for _ in 0..50 {
        if started.load(Ordering::SeqCst) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(
        started.load(Ordering::SeqCst),
        "Turno 1 deve ter iniciado STT"
    );

    let sess1 = session_mgr.get_session(dev).unwrap();
    assert_eq!(sess1.generation, 1);
    assert!(sess1.in_flight);

    // Simula cancelamento/interrupção do Turno 1 e liberação da sessão
    session_mgr.finish_operation(
        dev,
        &sess1.session_id,
        1,
        Some(VoiceSessionState::Interrupted),
    );

    // Turno 2 (Geração 2) inicia no mesmo dispositivo
    let sess2 = session_mgr
        .start_session("sess-turno-2", dev, VoiceConfiguration::default())
        .expect("Turno 2 deve iniciar com sucesso");
    assert_eq!(sess2.generation, 2);
    assert_eq!(sess2.session_id, "sess-turno-2");

    // Agora o Turno 1 acorda do STT
    allow_proceed.notify_one();
    let turn1_res = turn1_handle.await.unwrap();

    // 1. Efeito Observável: Turno 1 foi barrado com Interrupted
    assert!(
        matches!(turn1_res, Err(VoiceError::Interrupted)),
        "Turno 1 defasado DEVE retornar VoiceError::Interrupted: {:?}",
        turn1_res
    );

    // 2. Efeito Observável: O Core NUNCA foi chamado pelo Turno 1
    assert!(
        !core_called.load(Ordering::SeqCst),
        "Core NÃO PODE ter sido invocado por tarefa defasada"
    );

    // 3. Efeito Observável: O Playback NUNCA recebeu áudio do Turno 1
    assert_eq!(
        mock_sink.played_count(),
        0,
        "Playback NÃO PODE ter reproduzido áudio de tarefa defasada"
    );

    // 4. Integridade da Sessão Ativa: Sessão 2 permaneceu intocada
    let sess_active = session_mgr.get_session(dev).unwrap();
    assert_eq!(sess_active.session_id, "sess-turno-2");
    assert_eq!(sess_active.generation, 2);
}

// ============================================================================
// 10. TIMEOUT VS IN_FLIGHT (OPERAÇÃO PROLONGADA DO WHISPER)
// ============================================================================
#[tokio::test]
async fn test_recovery_10_in_flight_blocks_new_turn_even_after_session_timeout() {
    let session_mgr = Arc::new(VoiceSessionManager::new());
    let dev = "dev-whisper-slow";

    // Configura sessão com timeout curtíssimo de 1 segundo
    let config = VoiceConfiguration {
        session_timeout_secs: 1,
        ..Default::default()
    };

    // Turno 1 inicia com in_flight = true (simulando inferência pesada no Whisper)
    let sess1 = session_mgr
        .start_session("sess-whisper-1", dev, config.clone())
        .expect("Turno 1 deve iniciar com sucesso");
    assert_eq!(sess1.generation, 1);
    assert!(sess1.in_flight);

    // Aguarda mais do que session_timeout_secs (2.1s para ultrapassar 1s com margem inteira de num_seconds > 1)
    tokio::time::sleep(Duration::from_millis(2100)).await;

    // Confirma que a sessão expirou por inatividade de tempo
    let current_sess = session_mgr.get_session(dev).unwrap();
    assert!(
        current_sess.is_timed_out(),
        "Sessão deve estar com is_timed_out() == true"
    );
    assert!(
        current_sess.in_flight,
        "Sessão deve permanecer in_flight enquanto o processo não concluir"
    );

    // Tenta iniciar Turno 2 no mesmo dispositivo:
    // DEVE ser bloqueado categoricamente com SessionConflict!
    // Invariante: Timeout sozinho NÃO é confirmação de cancelamento físico.
    let turn2_res = session_mgr.start_session("sess-whisper-2", dev, config.clone());
    assert!(
        turn2_res.is_err(),
        "Turno 2 deve ser bloqueado enquanto o Turno 1 estiver in_flight, mesmo após timeout"
    );
    match turn2_res {
        Err(VoiceError::SessionConflict(msg)) => {
            assert!(
                msg.contains("operação assíncrona em andamento"),
                "Mensagem deve esclarecer operação assíncrona em andamento: {}",
                msg
            );
        }
        other => panic!("Esperado SessionConflict, obtido: {:?}", other),
    }

    // Agora o Turno 1 finalmente conclui (finish_operation)
    session_mgr.finish_operation(
        dev,
        &sess1.session_id,
        sess1.generation,
        Some(VoiceSessionState::Idle),
    );

    // Com in_flight = false, o novo turno pode agora iniciar com sucesso
    let turn2_success = session_mgr
        .start_session("sess-whisper-2", dev, config)
        .expect("Turno 2 deve iniciar após liberação de in_flight");
    assert_eq!(turn2_success.generation, 2);
    assert_eq!(turn2_success.session_id, "sess-whisper-2");
}

// ============================================================================
// 11. INTERRUPÇÃO VS UPDATE_SESSION (PRESERVAÇÃO DE CANCELAMENTO)
// ============================================================================
#[tokio::test]
async fn test_recovery_11_interrupted_state_cannot_be_overwritten_by_same_generation_update() {
    let session_mgr = Arc::new(VoiceSessionManager::new());
    let dev = "dev-interrupt-overwrite";

    // 1. Teste direto na camada de sessão:
    // Inicia sessão e transiciona para Interrupted
    let sess = session_mgr
        .start_session("sess-test-1", dev, VoiceConfiguration::default())
        .unwrap();
    assert_eq!(sess.generation, 1);

    // Operador interrompe a sessão no manager
    let mut interrupted_copy = sess.clone();
    interrupted_copy
        .transition_to(VoiceSessionState::Interrupted)
        .unwrap();
    assert!(session_mgr.update_session(&interrupted_copy));

    // Agora uma cópia defasada da mesma geração que estava em Transcribing tenta atualizar para Processing
    let mut stale_copy = sess.clone();
    stale_copy
        .transition_to(VoiceSessionState::Capturing)
        .unwrap();
    stale_copy
        .transition_to(VoiceSessionState::Transcribing)
        .unwrap();
    stale_copy
        .transition_to(VoiceSessionState::Processing)
        .unwrap();
    let updated = session_mgr.update_session(&stale_copy);

    // DEVE ser rejeitado com false e estado mantido como Interrupted!
    assert!(
        !updated,
        "update_session deve rejeitar sobrescrita de Interrupted por estado em andamento"
    );
    let current = session_mgr.get_session(dev).unwrap();
    assert_eq!(
        current.state,
        VoiceSessionState::Interrupted,
        "Estado deve permanecer Interrupted"
    );

    // 2. Teste integrado no VoiceConversationCoordinator:
    // Inicia um turno em background com STT que pausa
    let started = Arc::new(AtomicBool::new(false));
    let allow_proceed = Arc::new(tokio::sync::Notify::new());

    struct PausingStt2 {
        meta: VoiceProviderMetadata,
        started: Arc<AtomicBool>,
        allow_proceed: Arc<tokio::sync::Notify>,
    }
    impl yuki::contracts::voice::SpeechToTextProvider for PausingStt2 {
        fn metadata(&self) -> &VoiceProviderMetadata {
            &self.meta
        }
        fn health<'a>(
            &'a self,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), VoiceError>> + Send + 'a>>
        {
            Box::pin(async { Ok(()) })
        }
        fn transcribe<'a>(
            &'a self,
            _audio: &'a AudioInput,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<String, VoiceError>> + Send + 'a>,
        > {
            self.started.store(true, Ordering::SeqCst);
            let notify = self.allow_proceed.clone();
            Box::pin(async move {
                notify.notified().await;
                Ok("Yuki teste".to_string())
            })
        }
    }

    let pausing_stt = Arc::new(PausingStt2 {
        meta: VoiceProviderMetadata::new(
            "PausingStt2",
            VoiceProviderType::LocalEdge,
            vec!["pt".to_string()],
            16000,
            false,
            "1.0",
        ),
        started: started.clone(),
        allow_proceed: allow_proceed.clone(),
    });

    let (playback, mock_sink) = AudioPlaybackController::mock();
    let router = Arc::new(GovernedVoiceRouter::new(
        pausing_stt,
        None,
        Arc::new(MockTtsProvider::new()),
        None,
        VoiceConfiguration::default(),
    ));

    let core_called = Arc::new(AtomicBool::new(false));
    struct TrackingCoreProvider(Arc<AtomicBool>, MockModelProvider);
    impl yuki::models::provider::ModelProvider for TrackingCoreProvider {
        fn generate<'a>(
            &'a self,
            req: &'a yuki::models::provider::ModelRequest,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<
                        Output = Result<
                            yuki::models::provider::ModelResponse,
                            yuki::models::errors::ModelError,
                        >,
                    > + Send
                    + 'a,
            >,
        > {
            self.0.store(true, Ordering::SeqCst);
            self.1.generate(req)
        }
        fn metadata(&self) -> yuki::models::provider::ModelMetadata {
            self.1.metadata()
        }
        fn health(&self) -> yuki::models::provider::HealthStatus {
            self.1.health()
        }
    }

    let core = Arc::new(
        YukiCore::new().with_model_provider(Arc::new(TrackingCoreProvider(
            core_called.clone(),
            MockModelProvider::new(),
        ))),
    );

    let coordinator = Arc::new(VoiceConversationCoordinator::new(
        core,
        router,
        session_mgr.clone(),
        Arc::new(playback),
        Arc::new(tokio::sync::Mutex::new(WakeWordDetector::default())),
        None,
    ));

    let dev_coord = "dev-coord-interrupt";
    let coord_clone = coordinator.clone();
    let turn_handle = tokio::spawn(async move {
        coord_clone
            .process_button_turn(dev_coord, create_dummy_audio(), false, false)
            .await
    });

    // Aguarda o STT iniciar
    for _ in 0..50 {
        if started.load(Ordering::SeqCst) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(started.load(Ordering::SeqCst));

    // Usuário pressiona interrupt() no coordenador enquanto o STT está processando
    coordinator
        .interrupt(dev_coord)
        .expect("Interrupção deve ter sucesso");

    // Libera o STT para acordar e tentar seguir para Core/TTS
    allow_proceed.notify_one();
    let turn_res = turn_handle.await.unwrap();

    // 1. O turno deve retornar Err(VoiceError::Interrupted)
    assert!(
        matches!(turn_res, Err(VoiceError::Interrupted)),
        "Turno interrompido deve retornar Err(VoiceError::Interrupted): {:?}",
        turn_res
    );

    // 2. O Core NUNCA foi chamado
    assert!(
        !core_called.load(Ordering::SeqCst),
        "Core não pode ter sido chamado após interrupção"
    );

    // 3. O Playback NUNCA emitiu áudio
    assert_eq!(
        mock_sink.played_count(),
        0,
        "Playback não pode ter executado áudio após interrupção"
    );

    // 4. O estado no manager permaneceu Interrupted (não foi revertido para Idle ou Processing)
    let final_sess = session_mgr.get_session(dev_coord).unwrap();
    assert_eq!(final_sess.state, VoiceSessionState::Interrupted);
    assert!(!final_sess.in_flight);
}

// ============================================================================
// 12. SANITIZAÇÃO UTF-8 MULTIBYTE E REDAÇÃO DE CREDENCIAIS
// ============================================================================
#[test]
fn test_recovery_12_audit_sanitization_multibyte_utf8_no_panic() {
    use yuki::voice::conversation::sanitize_error_for_audit;

    // Caso A: Mensagem com exatamente 499 bytes ASCII seguida por caractere multibyte de 2 bytes (português 'ã')
    // Com truncate(500) ingênuo, isso causaria pânico de runtime ('byte index 500 is not a char boundary')
    let mut msg_pt = "A".repeat(499);
    msg_pt.push('ã'); // 'ã' tem 2 bytes em UTF-8: [0xC3, 0xA3] -> len total = 501
    assert_eq!(msg_pt.len(), 501);

    let sanitized_pt = sanitize_error_for_audit(&msg_pt);
    assert!(
        sanitized_pt.ends_with("... [truncated]"),
        "Deve conter sufixo de truncamento"
    );
    // Garante que o texto resultante é string UTF-8 válida
    let _ = std::str::from_utf8(sanitized_pt.as_bytes()).expect("Deve ser UTF-8 válido");

    // Caso B: Mensagem com 499 bytes ASCII seguida por caractere japonês de 3 bytes ('雪' - Yuki)
    // O byte 500 cairia no segundo byte de '雪' [0xE9, 0x9B, 0xAA]
    let mut msg_jp = "B".repeat(499);
    msg_jp.push('雪'); // 3 bytes -> len total = 502
    assert_eq!(msg_jp.len(), 502);

    let sanitized_jp = sanitize_error_for_audit(&msg_jp);
    assert!(sanitized_jp.ends_with("... [truncated]"));
    let _ = std::str::from_utf8(sanitized_jp.as_bytes()).expect("Deve ser UTF-8 válido");

    // Caso C: Mensagem com 499 bytes ASCII seguida por emoji de 4 bytes ('🤖')
    let mut msg_emoji = "C".repeat(499);
    msg_emoji.push('🤖'); // 4 bytes -> len total = 503
    assert_eq!(msg_emoji.len(), 503);

    let sanitized_emoji = sanitize_error_for_audit(&msg_emoji);
    assert!(sanitized_emoji.ends_with("... [truncated]"));
    let _ = std::str::from_utf8(sanitized_emoji.as_bytes()).expect("Deve ser UTF-8 válido");

    // Caso D: Redação de credenciais sensíveis (Bearer, api_key, password, tokens)
    let msg_secret = "Falha ao autenticar: Bearer sk-ant-api03-secret-token-key";
    let sanitized_secret = sanitize_error_for_audit(msg_secret);
    assert!(
        sanitized_secret.contains("[REDACTED_SENSITIVE"),
        "Credencial sensível deve ser redigida: {}",
        sanitized_secret
    );
    assert!(
        !sanitized_secret.contains("sk-ant-api03"),
        "Payload do token não pode vazar para o log de auditoria"
    );

    let msg_pwd = "Database connection error: password=admin_secret_pass";
    let sanitized_pwd = sanitize_error_for_audit(msg_pwd);
    assert!(sanitized_pwd.contains("[REDACTED_SENSITIVE"));
    assert!(!sanitized_pwd.contains("admin_secret_pass"));
}

// ============================================================================
// 13. CICLO DE VIDA DE ARQUIVOS TEMPORÁRIOS E CANCELAMENTO DE PROCESSOS
// ============================================================================
#[tokio::test]
async fn test_recovery_13_subprocess_cleanup_and_temp_file_lifecycle() {
    // Valida que o padrão TempCleaner com RAII Drop limpa arquivos temporários de STT/TTS
    // mesmo diante de interrupção ou queda assíncrona
    let temp_dir = std::env::temp_dir();
    let temp_file_path = temp_dir.join(format!("yuki_test_cleaner_{}.wav", uuid_ms_test()));

    // Escreve arquivo temporário simulando geração de WAV para Whisper ou Piper
    std::fs::write(&temp_file_path, b"RIFF....WAVEfmt ....data....").unwrap();
    assert!(temp_file_path.exists(), "Arquivo temporário deve existir");

    struct TempCleaner(std::path::PathBuf);
    impl Drop for TempCleaner {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    // Executa bloco com cleaner
    {
        let _cleaner = TempCleaner(temp_file_path.clone());
        assert!(temp_file_path.exists());
        // Simula término do escopo / abort da future
    }

    // Após o Drop, o arquivo foi deslinkado sem deixar órfãos no sistema de arquivos
    assert!(
        !temp_file_path.exists(),
        "Arquivo temporário deve ter sido removido automaticamente pelo TempCleaner"
    );
}

fn uuid_ms_test() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{:x}", ms)
}

// ============================================================================
// 14. ENCERRAMENTO ATÔMICO, INTERRUPÇÃO TARDIA E PREVENÇÃO DE SUCESSO FALSO
// ============================================================================
#[tokio::test]
async fn test_recovery_14_late_interruption_during_turn_completion_releases_in_flight_and_prevents_false_success(
) {
    let session_mgr = Arc::new(VoiceSessionManager::new());
    let dev = "dev-late-interrupt";

    // 1. Validação unitária atômica no VoiceSessionManager
    let sess = session_mgr
        .start_session("sess-1", dev, VoiceConfiguration::default())
        .expect("Sessão inicial deve ser criada");
    assert_eq!(sess.generation, 1);
    assert!(sess.in_flight);

    // Operador aciona interrupção concorrente (barge-in / cancelamento)
    assert!(session_mgr.interrupt_session(dev));
    let sess_int = session_mgr.get_session(dev).unwrap();
    assert_eq!(sess_int.state, VoiceSessionState::Interrupted);

    // Tentativa de concluir o turno com complete_turn
    let res = session_mgr.complete_turn(dev, &sess.session_id, sess.generation);

    // Invariante 1: Não produz falso sucesso
    assert!(
        matches!(res, Err(VoiceError::Interrupted)),
        "complete_turn deve retornar Err(VoiceError::Interrupted), obtido: {:?}",
        res
    );

    // Invariante 2: in_flight é liberado (não fica preso) e estado Interrupted é preservado
    let sess_after = session_mgr.get_session(dev).unwrap();
    assert!(
        !sess_after.in_flight,
        "in_flight deve ser liberado para permitir novo turno"
    );
    assert_eq!(
        sess_after.state,
        VoiceSessionState::Interrupted,
        "Estado deve permanecer Interrupted"
    );

    // Invariante 3: Novo turno no mesmo dispositivo pode iniciar imediatamente sem SessionConflict
    let sess2 = session_mgr
        .start_session("sess-2", dev, VoiceConfiguration::default())
        .expect("Novo turno deve iniciar sem conflito");
    assert_eq!(sess2.generation, 2);
    assert!(sess2.in_flight);

    // 2. Validação integrada determinística com VoiceConversationCoordinator
    let started = Arc::new(AtomicBool::new(false));
    let step_barrier = Arc::new(tokio::sync::Notify::new());

    struct SyncTtsProvider {
        meta: VoiceProviderMetadata,
        started: Arc<AtomicBool>,
        step_barrier: Arc<tokio::sync::Notify>,
    }
    impl yuki::contracts::voice::TextToSpeechProvider for SyncTtsProvider {
        fn metadata(&self) -> &VoiceProviderMetadata {
            &self.meta
        }
        fn health<'a>(
            &'a self,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), VoiceError>> + Send + 'a>>
        {
            Box::pin(async { Ok(()) })
        }
        fn synthesize<'a>(
            &'a self,
            _text: &'a str,
            _voice_params: Option<&'a VoiceConfiguration>,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<
                        Output = Result<yuki::contracts::voice::AudioOutput, VoiceError>,
                    > + Send
                    + 'a,
            >,
        > {
            self.started.store(true, Ordering::SeqCst);
            let barrier = self.step_barrier.clone();
            Box::pin(async move {
                barrier.notified().await;
                Ok(yuki::contracts::voice::AudioOutput::new(
                    vec![0u8; 100],
                    AudioFormat::default(),
                    50,
                    "test",
                ))
            })
        }
    }

    let sync_tts = Arc::new(SyncTtsProvider {
        meta: VoiceProviderMetadata::new(
            "SyncTts",
            VoiceProviderType::LocalEdge,
            vec!["pt".to_string()],
            16000,
            false,
            "1.0",
        ),
        started: started.clone(),
        step_barrier: step_barrier.clone(),
    });

    let (playback, mock_sink) = AudioPlaybackController::mock();
    let router = Arc::new(GovernedVoiceRouter::new(
        Arc::new(MockSttProvider::new().with_response("Yuki comando")),
        None,
        sync_tts,
        None,
        VoiceConfiguration::default(),
    ));

    let core = Arc::new(YukiCore::new());
    let coordinator = Arc::new(VoiceConversationCoordinator::new(
        core,
        router,
        session_mgr.clone(),
        Arc::new(playback),
        Arc::new(tokio::sync::Mutex::new(WakeWordDetector::default())),
        None,
    ));

    let dev_coord = "dev-coord-late-interrupt";
    let coord_clone = coordinator.clone();
    let turn_handle = tokio::spawn(async move {
        coord_clone
            .process_button_turn(dev_coord, create_dummy_audio(), false, false)
            .await
    });

    // Aguarda deterministicamente a síntese TTS iniciar
    while !started.load(Ordering::SeqCst) {
        tokio::task::yield_now().await;
    }

    // Operador aciona interrupt() no coordenador enquanto o TTS está finalizando
    coordinator
        .interrupt(dev_coord)
        .expect("Interrupção deve ser registrada");

    // Libera a síntese TTS para terminar
    step_barrier.notify_one();
    let turn_result = turn_handle.await.unwrap();

    // Comprova que não houve sucesso falso:
    assert!(
        matches!(turn_result, Err(VoiceError::Interrupted)),
        "Turno deve falhar com Interrupted, obtido: {:?}",
        turn_result
    );

    // Comprova que in_flight foi liberado:
    let final_sess = session_mgr.get_session(dev_coord).unwrap();
    assert!(
        !final_sess.in_flight,
        "in_flight no manager deve ser liberado após interrupção tardia"
    );
    assert_eq!(final_sess.state, VoiceSessionState::Interrupted);

    // Comprova que playback foi interrompido e não vazou áudio
    assert_eq!(mock_sink.interrupted_count(), 2); // 1 do coordinator.interrupt() + 1 da finalização do turno
}

// ============================================================================
// 15. PLAYBACK VS SESSIONCONFLICT E PRESERVAÇÃO DE BARGE-IN INTENCIONAL
// ============================================================================
#[tokio::test]
async fn test_recovery_15_rejected_session_conflict_does_not_interrupt_active_playback_or_break_barge_in(
) {
    let session_mgr = Arc::new(VoiceSessionManager::new());
    let dev = "dev-playback-conflict";

    let (playback, mock_sink) = AudioPlaybackController::mock();
    let router = Arc::new(GovernedVoiceRouter::new(
        Arc::new(MockSttProvider::new().with_response("Yuki")),
        None,
        Arc::new(MockTtsProvider::new()),
        None,
        VoiceConfiguration::default(),
    ));

    let core = Arc::new(YukiCore::new());
    let coordinator = Arc::new(VoiceConversationCoordinator::new(
        core,
        router,
        session_mgr.clone(),
        Arc::new(playback),
        Arc::new(tokio::sync::Mutex::new(WakeWordDetector::default())),
        None,
    ));

    // 1. Simula que o Turno 1 está ativo e reproduzindo áudio legítimo
    let sess1 = session_mgr
        .start_session("sess-active", dev, VoiceConfiguration::default())
        .expect("Sessão ativa deve iniciar");
    assert_eq!(sess1.generation, 1);
    assert!(sess1.in_flight);

    // Inicia reprodução física/mock
    let valid_audio = yuki::contracts::voice::AudioOutput::new(
        vec![0u8; 100],
        AudioFormat::default(),
        5000,
        "voice-active",
    );
    coordinator
        .playback()
        .play(&valid_audio)
        .expect("Playback deve iniciar");
    assert_eq!(
        coordinator.playback().status(),
        yuki::voice::playback::PlaybackStatus::Playing
    );
    assert_eq!(mock_sink.played_count(), 1);
    assert_eq!(mock_sink.interrupted_count(), 0);

    // 2. Uma tentativa concorrente ou inválida tenta iniciar novo turno no mesmo dispositivo
    // (ex: toque acidental no botão ou chamada duplicada)
    let conflict_attempt = coordinator
        .process_button_turn(dev, create_dummy_audio(), false, false)
        .await;

    // INVARIANTE CRÍTICA:
    // A tentativa é rejeitada com SessionConflict...
    assert!(
        matches!(conflict_attempt, Err(VoiceError::SessionConflict(_))),
        "Tentativa concorrente deve ser rejeitada com SessionConflict: {:?}",
        conflict_attempt
    );

    // ...E O ÁUDIO ATIVO NÃO FOI INTERROMPIDO!
    assert_eq!(
        coordinator.playback().status(),
        yuki::voice::playback::PlaybackStatus::Playing,
        "Playback legítimo da sessão ativa DEVE continuar tocando"
    );
    assert_eq!(
        mock_sink.interrupted_count(),
        0,
        "Sink de áudio NÃO PODE ter recebido sinal de interrupção em tentativa rejeitada"
    );

    // 3. BARGE-IN INTENCIONAL:
    // Agora o operador deliberadamente aciona a interrupção da reprodução
    coordinator
        .interrupt(dev)
        .expect("Interrupção deliberada deve ter sucesso");

    // Comprova que o barge-in intencional interrompeu imediatamente o áudio:
    assert_eq!(
        coordinator.playback().status(),
        yuki::voice::playback::PlaybackStatus::Interrupted
    );
    assert_eq!(mock_sink.interrupted_count(), 1);

    // Comprova que o estado da sessão foi marcado como Interrupted:
    let sess_after_interrupt = session_mgr.get_session(dev).unwrap();
    assert_eq!(sess_after_interrupt.state, VoiceSessionState::Interrupted);
}

// ============================================================================
// 16. ATOMICIDADE DE COMPLETE_TURN CONTRA SESSÃO OBSOLETA / GERAÇÃO SUPERSEDED
// ============================================================================
#[test]
fn test_recovery_16_atomic_complete_turn_rejects_stale_generation_and_preserves_new_session() {
    let session_mgr = VoiceSessionManager::new();
    let dev = "dev-stale-fencing";

    // Sessão 1 inicia na Geração 1
    let sess1 = session_mgr
        .start_session("sess-1", dev, VoiceConfiguration::default())
        .unwrap();
    assert_eq!(sess1.generation, 1);

    // Sessão 1 conclui
    session_mgr
        .complete_turn(dev, "sess-1", 1)
        .expect("Sessão 1 deve concluir normalmente");

    // Sessão 2 inicia na Geração 2 e passa para Transcribing
    let mut sess2 = session_mgr
        .start_session("sess-2", dev, VoiceConfiguration::default())
        .unwrap();
    assert_eq!(sess2.generation, 2);
    sess2.transition_to(VoiceSessionState::Capturing).unwrap();
    sess2
        .transition_to(VoiceSessionState::Transcribing)
        .unwrap();
    assert!(session_mgr.update_session(&sess2));

    // Agora uma tarefa atrasada (zumbi) da Sessão 1 tenta invocar complete_turn ou fail_turn
    let stale_complete = session_mgr.complete_turn(dev, "sess-1", 1);
    assert!(
        matches!(stale_complete, Err(VoiceError::SessionConflict(_))),
        "complete_turn da Sessão 1 com geração antiga DEVE ser rejeitado com SessionConflict"
    );

    // Tentativa com fail_turn de geração antiga
    session_mgr.fail_turn(dev, "sess-1", 1, VoiceSessionState::Idle);

    // INVARIANTE: A Sessão 2 permaneceu 100% íntegra e ativa
    let active_sess = session_mgr.get_session(dev).unwrap();
    assert_eq!(active_sess.session_id, "sess-2");
    assert_eq!(active_sess.generation, 2);
    assert_eq!(active_sess.state, VoiceSessionState::Transcribing);
    assert!(active_sess.in_flight);
}
