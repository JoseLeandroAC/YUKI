use std::sync::Arc;
use yuki::contracts::voice::{AudioFormat, AudioFrame, AudioInput, VoiceConfiguration, VoiceError};
use yuki::core::yuki_core::YukiCore;
use yuki::models::mock::MockModelProvider;
use yuki::voice::audio::AudioCodec;
use yuki::voice::conversation::VoiceConversationCoordinator;
use yuki::voice::playback::AudioPlaybackController;
use yuki::voice::router::GovernedVoiceRouter;
use yuki::voice::session::VoiceSessionManager;
use yuki::voice::stt::{CloudSttProvider, MockSttProvider};
use yuki::voice::tts::{CloudTtsProvider, LocalAcousticTtsProvider, MockTtsProvider};
use yuki::voice::wakeword::{WakeWordDetection, WakeWordDetector};

#[tokio::test]
async fn test_governed_voice_router_local_first_cloud_only_when_authorized() {
    let failing_local_stt = Arc::new(MockSttProvider::new().with_failure(true));
    let cloud_stt = Arc::new(CloudSttProvider::new(
        "Gemini-Voice-STT",
        "https://api.gemini.mock/speech",
        "gemini_voice_key",
        true,
    ));

    let local_tts = Arc::new(LocalAcousticTtsProvider::new(16000));
    let cloud_tts = Arc::new(CloudTtsProvider::new(
        "ElevenLabs-TTS",
        "https://api.eleven.mock/tts",
        "eleven_key",
        true,
    ));

    let config = VoiceConfiguration {
        cloud_fallback_allowed: false,
        ..Default::default()
    };

    let router = GovernedVoiceRouter::new(
        failing_local_stt.clone(),
        Some(cloud_stt.clone()),
        local_tts.clone(),
        Some(cloud_tts.clone()),
        config,
    );

    let dummy_audio = AudioInput::new(vec![0u8; 100], AudioFormat::default(), 200, "test");

    // 1. Local STT falha e Cloud Fallback está desabilitado na configuração -> Rejeita
    let res = router.transcribe(&dummy_audio, false).await;
    assert!(matches!(res, Err(VoiceError::UnauthorizedCloudFallback(_))));

    // 2. Mesmo que operador autorize, se config desabilita fallback -> Rejeita
    let res = router.transcribe(&dummy_audio, true).await;
    assert!(matches!(res, Err(VoiceError::UnauthorizedCloudFallback(_))));

    // 3. Habilita fallback na config, mas operador NÃO autorizou a capability específica egress:voice_cloud_stt -> Rejeita
    let mut permissive_config = router.config().clone();
    permissive_config.cloud_fallback_allowed = true;
    let permissive_router = GovernedVoiceRouter::new(
        failing_local_stt,
        Some(cloud_stt),
        local_tts,
        Some(cloud_tts),
        permissive_config,
    );

    let res = permissive_router.transcribe(&dummy_audio, false).await;
    assert!(matches!(res, Err(VoiceError::UnauthorizedCloudFallback(_))));

    // 4. Somente com config permitindo E autorização explícita do operador -> Sucesso
    let res = permissive_router.transcribe(&dummy_audio, true).await;
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), "Transcrição via Provedor Cloud Governado");
}

#[tokio::test]
async fn test_governed_voice_router_stt_and_tts_permissions_isolated() {
    let failing_local_stt = Arc::new(MockSttProvider::new().with_failure(true));
    let cloud_stt = Arc::new(CloudSttProvider::new(
        "Cloud-STT",
        "https://api.cloud/stt",
        "key",
        true,
    ));

    let failing_local_tts = Arc::new(MockTtsProvider::new().with_failure(true));
    let cloud_tts = Arc::new(CloudTtsProvider::new(
        "Cloud-TTS",
        "https://api.cloud/tts",
        "key",
        true,
    ));

    let config = VoiceConfiguration {
        cloud_fallback_allowed: true,
        ..Default::default()
    };

    let router = GovernedVoiceRouter::new(
        failing_local_stt,
        Some(cloud_stt),
        failing_local_tts,
        Some(cloud_tts),
        config,
    );

    // Autorização de TTS NÃO autoriza STT
    let dummy_audio = AudioInput::new(vec![0u8; 100], AudioFormat::default(), 200, "test");
    let stt_res = router.transcribe(&dummy_audio, false).await;
    assert!(matches!(
        stt_res,
        Err(VoiceError::UnauthorizedCloudFallback(_))
    ));

    // Autorização de STT NÃO autoriza TTS
    let tts_res = router.synthesize("Texto de teste", false).await;
    assert!(matches!(
        tts_res,
        Err(VoiceError::UnauthorizedCloudFallback(_))
    ));

    // Autorização explícita de TTS
    let tts_ok = router.synthesize("Texto de teste", true).await;
    assert!(tts_ok.is_ok());
}

#[tokio::test]
async fn test_voice_conversation_button_turn_end_to_end() {
    let model = Arc::new(MockModelProvider::with_behavior(
        yuki::models::mock::MockBehavior::DirectText(
            "Olá, sou a Yuki! Em que posso ajudar?".to_string(),
        ),
    ));
    let core = Arc::new(YukiCore::new().with_model_provider(model));

    let local_stt = Arc::new(MockSttProvider::new().with_response("Yuki, bom dia"));
    let local_tts = Arc::new(LocalAcousticTtsProvider::new(16000));
    let config = VoiceConfiguration::default();

    let router = Arc::new(GovernedVoiceRouter::new(
        local_stt, None, local_tts, None, config,
    ));

    let session_mgr = Arc::new(VoiceSessionManager::new());
    let playback = Arc::new(AudioPlaybackController::new());
    let wake_detector = Arc::new(tokio::sync::Mutex::new(WakeWordDetector::default()));

    let coordinator = VoiceConversationCoordinator::new(
        core,
        router,
        session_mgr,
        playback.clone(),
        wake_detector,
        None,
    );

    let format = AudioFormat::default();
    let samples = vec![100i16; 1600]; // 100ms
    let wav_payload = AudioCodec::encode_wav(&samples, &format).unwrap();
    let input = AudioInput::new(wav_payload, format, 100, "mic-0");

    let (result, audio_output) = coordinator
        .process_button_turn("device-test", input, false, false)
        .await
        .expect("Button turn succeeded");

    assert!(result.content.contains("Yuki"));
    assert!(audio_output.duration_ms > 0);
    assert_eq!(
        playback.status(),
        yuki::voice::playback::PlaybackStatus::Playing
    );
}

#[tokio::test]
async fn test_wake_word_detector_local_and_invariants() {
    let mut detector = WakeWordDetector::new("yuki", 0.5, true);
    assert_eq!(detector.keyword(), "yuki");
    assert!(detector.is_enabled());

    // 1. Frame silencioso -> sem detecção
    let silent_frame = AudioFrame::new(vec![0i16; 160], 16000, 1, 0);
    let res = detector.process_frame(&silent_frame).unwrap();
    assert_eq!(res, WakeWordDetection::None);

    // 2. Desativação completa
    detector.set_enabled(false);
    assert!(!detector.is_enabled());
    let loud_frame = AudioFrame::new(vec![15000i16; 160], 16000, 1, 10);
    let res = detector.process_frame(&loud_frame).unwrap();
    assert_eq!(res, WakeWordDetection::None);

    // 3. Reativação e teste de sensibilidade
    detector.set_enabled(true);
    detector.set_sensitivity(0.8);
    assert_eq!(detector.sensitivity(), 0.8);

    // 4. Invariante: detecção de wake word NUNCA é token de autorização
    // (O detector apenas retorna enum WakeWordDetection::Detected com confidence e timestamp,
    // sem interagir com security controller ou conceder capabilities).
}

#[test]
fn test_voice_interruption_barge_in() {
    let playback = AudioPlaybackController::new();
    let dummy_output = yuki::contracts::voice::AudioOutput::new(
        vec![0u8; 100],
        AudioFormat::default(),
        5000,
        "test-voice",
    );

    // Inicia reprodução
    playback.play(&dummy_output).unwrap();
    assert_eq!(
        playback.status(),
        yuki::voice::playback::PlaybackStatus::Playing
    );

    // Usuário interrompe
    playback.interrupt().unwrap();
    assert_eq!(
        playback.status(),
        yuki::voice::playback::PlaybackStatus::Interrupted
    );

    // Reset limpa
    playback.reset();
    assert_eq!(
        playback.status(),
        yuki::voice::playback::PlaybackStatus::Idle
    );
}
