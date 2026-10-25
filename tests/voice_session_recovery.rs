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
