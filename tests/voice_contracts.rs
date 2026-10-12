use std::future::Future;
use std::pin::Pin;
use yuki::contracts::identifiers::now_utc;
use yuki::contracts::voice::{
    AudioFormat, AudioFrame, AudioInput, AudioOutput, SpeechToTextProvider, TextToSpeechProvider,
    VoiceConfiguration, VoiceError, VoiceProviderMetadata, VoiceProviderType, VoiceSessionState,
};
use yuki::voice::audio::AudioCodec;
use yuki::voice::session::{VoiceSession, VoiceSessionManager};

struct DummyFailingStt;
impl SpeechToTextProvider for DummyFailingStt {
    fn metadata(&self) -> &VoiceProviderMetadata {
        static META: std::sync::OnceLock<VoiceProviderMetadata> = std::sync::OnceLock::new();
        META.get_or_init(|| {
            VoiceProviderMetadata::new(
                "DummyFailingStt",
                VoiceProviderType::LocalEdge,
                vec!["pt-BR".to_string()],
                16000,
                false,
                "0.1.0",
            )
        })
    }

    fn health<'a>(&'a self) -> Pin<Box<dyn Future<Output = Result<(), VoiceError>> + Send + 'a>> {
        Box::pin(async { Err(VoiceError::ProviderUnavailable("Offline test".into())) })
    }

    fn transcribe<'a>(
        &'a self,
        _audio: &'a AudioInput,
    ) -> Pin<Box<dyn Future<Output = Result<String, VoiceError>> + Send + 'a>> {
        Box::pin(async { Err(VoiceError::TranscriptionFailed("Mock error".into())) })
    }
}

struct DummyFailingTts;
impl TextToSpeechProvider for DummyFailingTts {
    fn metadata(&self) -> &VoiceProviderMetadata {
        static META: std::sync::OnceLock<VoiceProviderMetadata> = std::sync::OnceLock::new();
        META.get_or_init(|| {
            VoiceProviderMetadata::new(
                "DummyFailingTts",
                VoiceProviderType::LocalEdge,
                vec!["pt-BR".to_string()],
                16000,
                false,
                "0.1.0",
            )
        })
    }

    fn health<'a>(&'a self) -> Pin<Box<dyn Future<Output = Result<(), VoiceError>> + Send + 'a>> {
        Box::pin(async { Err(VoiceError::ProviderUnavailable("TTS offline".into())) })
    }

    fn synthesize<'a>(
        &'a self,
        _text: &'a str,
        _voice_params: Option<&'a VoiceConfiguration>,
    ) -> Pin<Box<dyn Future<Output = Result<AudioOutput, VoiceError>> + Send + 'a>> {
        Box::pin(async { Err(VoiceError::SynthesisFailed("TTS mock error".into())) })
    }
}

#[test]
fn test_voice_session_valid_transitions() {
    let mut session = VoiceSession::new("s1", "mic-0", VoiceConfiguration::default());
    assert_eq!(session.state, VoiceSessionState::Idle);

    // Idle -> WakeListening
    assert!(session
        .transition_to(VoiceSessionState::WakeListening)
        .is_ok());

    // WakeListening -> Capturing
    assert!(session.transition_to(VoiceSessionState::Capturing).is_ok());

    // Capturing -> Transcribing
    assert!(session
        .transition_to(VoiceSessionState::Transcribing)
        .is_ok());

    // Transcribing -> Processing
    assert!(session.transition_to(VoiceSessionState::Processing).is_ok());

    // Processing -> Synthesizing
    assert!(session
        .transition_to(VoiceSessionState::Synthesizing)
        .is_ok());

    // Synthesizing -> Speaking
    assert!(session.transition_to(VoiceSessionState::Speaking).is_ok());

    // Speaking -> WakeListening
    assert!(session
        .transition_to(VoiceSessionState::WakeListening)
        .is_ok());

    // Interrupted from WakeListening
    assert!(session.interrupt().is_ok());
    assert_eq!(session.state, VoiceSessionState::Interrupted);

    // Reset to Idle
    assert!(session.reset().is_ok());
    assert_eq!(session.state, VoiceSessionState::Idle);
}

#[test]
fn test_voice_session_invalid_transitions() {
    let mut session = VoiceSession::new("s2", "mic-0", VoiceConfiguration::default());
    assert_eq!(session.state, VoiceSessionState::Idle);

    // Idle cannot transition directly to Speaking
    let res = session.transition_to(VoiceSessionState::Speaking);
    assert!(matches!(
        res,
        Err(VoiceError::InvalidStateTransition { .. })
    ));

    // Idle cannot transition directly to Synthesizing
    let res = session.transition_to(VoiceSessionState::Synthesizing);
    assert!(matches!(
        res,
        Err(VoiceError::InvalidStateTransition { .. })
    ));

    // Set to Capturing (valid from Idle)
    assert!(session.transition_to(VoiceSessionState::Capturing).is_ok());

    // Capturing cannot jump directly to Speaking
    let res = session.transition_to(VoiceSessionState::Speaking);
    assert!(matches!(
        res,
        Err(VoiceError::InvalidStateTransition { .. })
    ));
}

#[test]
fn test_voice_session_cancellation_and_interruption() {
    let mut session = VoiceSession::new("s3", "mic-0", VoiceConfiguration::default());
    session.transition_to(VoiceSessionState::Capturing).unwrap();

    // Interrupting during capturing
    assert!(session.interrupt().is_ok());
    assert_eq!(session.state, VoiceSessionState::Interrupted);

    // Resetting
    assert!(session.reset().is_ok());
    assert_eq!(session.state, VoiceSessionState::Idle);
}

#[test]
fn test_voice_session_timeout() {
    let config = VoiceConfiguration {
        session_timeout_secs: 0, // immediate timeout
        ..Default::default()
    };

    let mut session = VoiceSession::new("s4", "mic-0", config);
    // Give a tiny delay or check elapsed
    session.last_activity = now_utc() - chrono::Duration::seconds(5);
    assert!(session.is_timed_out());
}

#[test]
fn test_voice_session_manager_concurrency_conflict() {
    let manager = VoiceSessionManager::new();
    let config = VoiceConfiguration::default();

    // Start first session for device-1
    let mut s1 = manager
        .start_session("sess-1", "device-1", config.clone())
        .unwrap();
    s1.transition_to(VoiceSessionState::Capturing).unwrap();
    manager.update_session(&s1);

    // Attempt second session for same device-1 while s1 is active
    let s2_res = manager.start_session("sess-2", "device-1", config.clone());
    assert!(matches!(s2_res, Err(VoiceError::SessionConflict(_))));

    // Different device (device-2) can start concurrently without conflict
    let s3_res = manager.start_session("sess-3", "device-2", config.clone());
    assert!(s3_res.is_ok());

    // When s1 goes Idle or closes, device-1 can start a new session
    s1.reset().unwrap();
    manager.update_session(&s1);
    let s4_res = manager.start_session("sess-4", "device-1", config);
    assert!(s4_res.is_ok());
}

#[tokio::test]
async fn test_provider_failures_and_unavailable() {
    let stt = DummyFailingStt;
    assert!(stt.health().await.is_err());
    let dummy_audio = AudioInput::new(vec![], AudioFormat::default(), 100, "test");
    let transcribe_res = stt.transcribe(&dummy_audio).await;
    assert!(matches!(
        transcribe_res,
        Err(VoiceError::TranscriptionFailed(_))
    ));

    let tts = DummyFailingTts;
    assert!(tts.health().await.is_err());
    let synth_res = tts.synthesize("ola", None).await;
    assert!(matches!(synth_res, Err(VoiceError::SynthesisFailed(_))));
}

#[test]
fn test_audio_codec_wav_encode_decode_roundtrip() {
    let format = AudioFormat {
        sample_rate: 16000,
        channels: 1,
        bits_per_sample: 16,
    };
    let samples: Vec<i16> = (0..1600).map(|i| ((i % 100) * 100) as i16).collect();

    let wav_bytes = AudioCodec::encode_wav(&samples, &format).expect("WAV encode failed");
    assert!(wav_bytes.len() >= 44 + (1600 * 2));

    let (decoded_format, decoded_samples) =
        AudioCodec::decode_wav(&wav_bytes).expect("WAV decode failed");
    assert_eq!(decoded_format, format);
    assert_eq!(decoded_samples, samples);
}

#[test]
fn test_audio_codec_corrupted_wav_fails_closed() {
    // Truncated header
    assert!(matches!(
        AudioCodec::decode_wav(&[0u8; 20]),
        Err(VoiceError::InvalidAudioFormat(_))
    ));

    // Wrong RIFF signature
    let mut bad_riff = vec![0u8; 44];
    bad_riff[0..4].copy_from_slice(b"NOPE");
    assert!(matches!(
        AudioCodec::decode_wav(&bad_riff),
        Err(VoiceError::InvalidAudioFormat(_))
    ));
}

#[test]
fn test_audio_constraints_and_duration_validation() {
    let format = AudioFormat::default();
    // 32000 samples at 16000Hz = 2000ms
    let duration = AudioCodec::validate_constraints(64000, 32000, &format, 1024 * 1024, 5000)
        .expect("Valid constraints");
    assert_eq!(duration, 2000);

    // Exceed max duration
    let dur_err = AudioCodec::validate_constraints(
        64000,
        32000,
        &format,
        1024 * 1024,
        1000, // max 1000ms
    );
    assert!(matches!(dur_err, Err(VoiceError::AudioTooLong { .. })));

    // Exceed max bytes
    let size_err = AudioCodec::validate_constraints(
        2000, 100, &format, 1000, // max 1000 bytes
        5000,
    );
    assert!(matches!(size_err, Err(VoiceError::AudioTooLarge { .. })));
}

#[test]
fn test_audio_frame_and_silent_detection() {
    let silent_samples = vec![0i16; 160];
    let frame = AudioFrame::new(silent_samples, 16000, 1, 0);
    assert!(frame.is_silent(10.0));
    assert_eq!(frame.duration_ms(), 10);

    let loud_samples = vec![25000i16; 160];
    let loud_frame = AudioFrame::new(loud_samples, 16000, 1, 0);
    assert!(!loud_frame.is_silent(10.0));
}
