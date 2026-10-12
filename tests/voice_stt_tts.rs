use yuki::contracts::voice::{
    AudioFormat, AudioInput, SpeechToTextProvider, TextToSpeechProvider, VoiceConfiguration,
    VoiceError,
};
use yuki::personality::adapter::ContextUrgency;
use yuki::personality::profile::PersonalityProfile;
use yuki::voice::audio::AudioCodec;
use yuki::voice::playback::{AudioPlaybackController, PlaybackStatus};
use yuki::voice::stt::{LocalWhisperConfig, LocalWhisperSttProvider, MockSttProvider};
use yuki::voice::tts::{
    LocalAcousticTtsProvider, LocalPiperConfig, LocalPiperTtsProvider, MockTtsProvider, VocalTone,
    VoiceIdentity,
};

#[tokio::test]
async fn test_stt_local_whisper_unconfigured_instructions_and_safety() {
    let config = LocalWhisperConfig {
        binary_path: None,
        model_path: None,
        ..Default::default()
    };

    let provider = LocalWhisperSttProvider::new(config);
    assert!(!provider.is_environment_ready());

    let health = provider.health().await;
    assert!(health.is_err());
    let err_msg = health.unwrap_err().to_string();
    assert!(err_msg.contains("whisper.cpp") || err_msg.contains("YUKI_WHISPER_BIN"));

    // Exceed max duration
    let fake_audio = AudioInput::new(vec![0u8; 100], AudioFormat::default(), 35_000, "test");
    let res = provider.transcribe(&fake_audio).await;
    assert!(matches!(res, Err(VoiceError::AudioTooLong { .. })));
}

#[tokio::test]
async fn test_stt_mock_deterministic_flow() {
    let mock = MockSttProvider::new().with_response("Yuki, iniciar missão");
    assert!(mock.health().await.is_ok());

    let audio = AudioInput::new(vec![1, 2, 3], AudioFormat::default(), 500, "mic");
    let text = mock.transcribe(&audio).await.expect("transcription ok");
    assert_eq!(text, "Yuki, iniciar missão");

    // Failure simulation
    let failing_mock = MockSttProvider::new().with_failure(true);
    assert!(failing_mock.health().await.is_err());
    assert!(matches!(
        failing_mock.transcribe(&audio).await,
        Err(VoiceError::TranscriptionFailed(_))
    ));
}

#[tokio::test]
async fn test_tts_local_acoustic_real_synthesis() {
    let tts = LocalAcousticTtsProvider::new(16000);
    assert!(tts.health().await.is_ok());

    let config = VoiceConfiguration::default();
    let text = "Olá José, eu sou a Yuki. Estou aqui com você.";
    let output = tts
        .synthesize(text, Some(&config))
        .await
        .expect("Synthesis succeeded");

    assert_eq!(output.voice_name, "yuki-acoustic-ptbr");
    assert!(output.duration_ms > 500);
    assert!(output.payload.len() > 1000);

    // Decode generated WAV to verify compliance
    let (fmt, samples) = AudioCodec::decode_wav(&output.payload).expect("WAV decode succeeded");
    assert_eq!(fmt.sample_rate, 16000);
    assert_eq!(fmt.channels, 1);
    assert_eq!(fmt.bits_per_sample, 16);
    assert!(!samples.is_empty());

    // Rejection of empty text
    let empty_res = tts.synthesize("   ", Some(&config)).await;
    assert!(matches!(empty_res, Err(VoiceError::SynthesisFailed(_))));
}

#[tokio::test]
async fn test_tts_local_piper_unconfigured_instructions() {
    let config = LocalPiperConfig {
        binary_path: None,
        model_path: None,
        ..Default::default()
    };

    let piper = LocalPiperTtsProvider::new(config);
    assert!(!piper.is_environment_ready());

    let health = piper.health().await;
    assert!(health.is_err());
    let err_msg = health.unwrap_err().to_string();
    assert!(err_msg.contains("YUKI_PIPER_BIN") || err_msg.contains("Piper"));
}

#[tokio::test]
async fn test_tts_mock_deterministic_flow() {
    let mock = MockTtsProvider::new();
    assert!(mock.health().await.is_ok());

    let output = mock
        .synthesize("Teste mock de síntese", None)
        .await
        .expect("mock synthesis ok");
    assert_eq!(output.voice_name, "mock-voice");
    assert_eq!(mock.get_synthesized_texts().len(), 1);

    // Failure injection
    let failing = MockTtsProvider::new().with_failure(true);
    assert!(failing.health().await.is_err());
    assert!(matches!(
        failing.synthesize("Olá", None).await,
        Err(VoiceError::SynthesisFailed(_))
    ));
}

#[test]
fn test_voice_identity_contextual_adaptation() {
    let identity = VoiceIdentity::default();
    assert_eq!(identity.gender, "feminine");
    assert_eq!(identity.tone, VocalTone::WarmAndCompetent);
    assert!(identity.avoid_biological_feigning);

    let base_cfg = VoiceConfiguration::default();

    // Standard urgency
    let adapted_std =
        identity.adapt_configuration(&base_cfg, None, Some(&ContextUrgency::Standard));
    assert_eq!(adapted_std.speaking_rate, 1.0);
    assert_eq!(adapted_std.pitch, 1.0);

    // SecurityCritical urgency: faster, firmer
    let adapted_crit =
        identity.adapt_configuration(&base_cfg, None, Some(&ContextUrgency::SecurityCritical));
    assert!(adapted_crit.speaking_rate > 1.0);
    assert!(adapted_crit.pitch < 1.0);

    // Personality modulation
    let profile = PersonalityProfile {
        warmth: 0.9,
        expressiveness: 0.8,
        ..Default::default()
    };
    let adapted_prof =
        identity.adapt_configuration(&base_cfg, Some(&profile), Some(&ContextUrgency::Standard));
    assert_ne!(adapted_prof.pitch, base_cfg.pitch);
}

#[test]
fn test_audio_playback_controller_lifecycle_and_interruption() {
    let controller = AudioPlaybackController::new();
    assert_eq!(controller.status(), PlaybackStatus::Idle);

    let dummy_output = yuki::contracts::voice::AudioOutput::new(
        vec![0u8; 100],
        AudioFormat::default(),
        1000,
        "test-voice",
    );

    // Play
    assert!(controller.play(&dummy_output).is_ok());
    assert_eq!(controller.status(), PlaybackStatus::Playing);

    // Interruption
    assert!(controller.interrupt().is_ok());
    assert_eq!(controller.status(), PlaybackStatus::Interrupted);

    // Stop
    controller.stop();
    assert_eq!(controller.status(), PlaybackStatus::Stopped);

    // Reset
    controller.reset();
    assert_eq!(controller.status(), PlaybackStatus::Idle);

    // Device unavailable failure
    controller.set_device_available(false);
    let play_res = controller.play(&dummy_output);
    assert!(matches!(play_res, Err(VoiceError::DeviceUnavailable(_))));
}
