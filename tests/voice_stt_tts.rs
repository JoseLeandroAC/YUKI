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
    assert!(controller.stop().is_ok());
    assert_eq!(controller.status(), PlaybackStatus::Stopped);

    // Reset
    assert!(controller.reset().is_ok());
    assert_eq!(controller.status(), PlaybackStatus::Idle);

    // Device unavailable failure
    controller.set_device_available(false);
    let play_res = controller.play(&dummy_output);
    assert!(matches!(play_res, Err(VoiceError::DeviceUnavailable(_))));
}

#[test]
fn test_stt_silence_detection_and_sanitization() {
    use yuki::voice::stt::{is_silent_pcm, sanitize_whisper_transcript};

    // Test silence detection
    let all_zeroes = vec![0i16; 16000];
    assert!(is_silent_pcm(&all_zeroes));

    let ambient_noise = vec![15i16, -20, 10, -5, 30, -15];
    assert!(is_silent_pcm(&ambient_noise));

    // High amplitude active audio (not silent)
    let active_speech: Vec<i16> = (0..16000)
        .map(|i| ((i as f64 * 0.1).sin() * 8000.0) as i16)
        .collect();
    assert!(!is_silent_pcm(&active_speech));

    // Test transcript sanitization
    assert_eq!(sanitize_whisper_transcript(""), "");
    assert_eq!(sanitize_whisper_transcript("   "), "");
    assert_eq!(sanitize_whisper_transcript("[BLANK_AUDIO]"), "");
    assert_eq!(
        sanitize_whisper_transcript("[00:00.000 --> 00:02.000] Olá Yuki!"),
        "Olá Yuki!"
    );
    assert_eq!(
        sanitize_whisper_transcript("  [00:01.500 --> 00:03.200] Comando executado com sucesso.  "),
        "Comando executado com sucesso."
    );
}

#[test]
fn test_piper_model_catalog_and_configuration() {
    use yuki::voice::tts::{get_piper_voice_catalog, LocalPiperConfig, LocalPiperTtsProvider};

    let catalog = get_piper_voice_catalog();
    assert!(catalog.len() >= 4);

    let ids: Vec<&str> = catalog.iter().map(|m| m.id.as_str()).collect();
    assert!(ids.contains(&"pt_BR-faber-medium"));
    assert!(ids.contains(&"pt_BR-cadu-medium"));
    assert!(ids.contains(&"pt_BR-edresson-low"));
    assert!(ids.contains(&"pt_BR-jeff-medium"));

    // Verify download instructions contain required references and transparency notes
    let instructions = LocalPiperTtsProvider::download_instructions();
    assert!(instructions.contains("rhasspy/piper-voices"));
    assert!(instructions.contains("Invoke-WebRequest"));
    assert!(instructions.contains("masculinos"));

    // Verify config parameters
    let cfg = LocalPiperConfig::default();
    assert_eq!(cfg.length_scale, 1.0);
    assert_eq!(cfg.noise_scale, 0.55);
    assert_eq!(cfg.noise_w, 0.70);
}

#[tokio::test]
async fn test_voice_lab_standardized_phrases_and_profile() {
    use yuki::voice::lab::{
        find_phrase, get_standardized_phrases, YukiVocalProfile, STANDARDIZED_PHRASES,
    };

    let phrases = get_standardized_phrases();
    assert_eq!(phrases.len(), 5);

    let phrase_ids: Vec<&str> = STANDARDIZED_PHRASES.iter().map(|p| p.id).collect();
    assert!(phrase_ids.contains(&"saudacao"));
    assert!(phrase_ids.contains(&"explicacao_tecnica"));
    assert!(phrase_ids.contains(&"conversa_natural"));
    assert!(phrase_ids.contains(&"aviso"));
    assert!(phrase_ids.contains(&"pronuncia"));

    assert!(find_phrase("saudacao").is_some());
    assert!(find_phrase("inexistente").is_none());

    let profile = YukiVocalProfile::default();
    assert_eq!(profile.version, "v0.1");
    assert_eq!(profile.min_target_naturalness, 80);
    assert_eq!(profile.min_target_tonality, 75);
    assert_eq!(profile.min_target_smoothness, 80);
    assert_eq!(profile.min_target_expressiveness, 70);
    assert_eq!(profile.min_target_rhythm, 75);
    assert_eq!(profile.recommended_length_scale, 1.14);
}

#[tokio::test]
async fn test_voice_lab_sample_generation_acoustic_and_uninstalled_piper() {
    use yuki::voice::lab::generate_voice_sample;

    // 1. Acoustic sample generation succeeds deterministically
    let (audio, meta) = generate_voice_sample("acoustic", "saudacao", None, None, None)
        .await
        .expect("Acoustic sample generation ok");

    assert!(!audio.payload.is_empty());
    assert!(!meta.is_neural);
    assert_eq!(meta.voice_id, "acoustic");
    assert_eq!(meta.phrase_id, "saudacao");
    assert!(meta.duration_ms > 500);
    assert!(std::path::Path::new(&meta.file_path).exists());

    // 2. Uninstalled Piper model fails closed with transparent download instructions
    let piper_res = generate_voice_sample("pt_BR-faber-medium", "saudacao", None, None, None).await;
    // Se o modelo não estiver baixado no ambiente de teste, deve retornar erro limpo com URLs
    if let Err(VoiceError::ProviderUnavailable(err_msg)) = piper_res {
        assert!(err_msg.contains("huggingface.co") || err_msg.contains("não está baixado"));
    }
}

#[test]
fn test_voice_lab_evaluations_persistence_and_summary() {
    use yuki::voice::lab::{
        compute_evaluations_summary, load_evaluations, save_evaluation, VoiceEvaluation,
    };

    let temp_dir = std::env::temp_dir();
    let temp_eval_path = temp_dir.join(format!(
        "yuki_eval_test_{}.json",
        yuki::contracts::identifiers::generate_unique_id("test")
    ));
    std::env::set_var(
        "YUKI_VOICE_EVALS_FILE",
        temp_eval_path.to_string_lossy().to_string(),
    );

    // Initial load on non-existent file
    let initial = load_evaluations().expect("initial load ok");
    assert!(initial.is_empty());

    // Save evaluation 1
    let eval1 = VoiceEvaluation {
        evaluation_id: "eval-1".to_string(),
        voice_id: "test-voice-a".to_string(),
        phrase_id: Some("saudacao".to_string()),
        naturalness: 85,
        tonality: 80,
        smoothness: 85,
        expressiveness: 75,
        rhythm: 80,
        average_score: 81.0,
        approved: true,
        operator_notes: "Excelente voz para introdução".to_string(),
        evaluated_at: "2026-10-12T12:00:00Z".to_string(),
    };
    save_evaluation(eval1).expect("save 1 ok");

    // Save evaluation 2
    let eval2 = VoiceEvaluation {
        evaluation_id: "eval-2".to_string(),
        voice_id: "test-voice-a".to_string(),
        phrase_id: Some("aviso".to_string()),
        naturalness: 75,
        tonality: 70,
        smoothness: 75,
        expressiveness: 65,
        rhythm: 70,
        average_score: 71.0,
        approved: false,
        operator_notes: "Tom ligeiramente mecânico em alertas".to_string(),
        evaluated_at: "2026-10-12T12:10:00Z".to_string(),
    };
    save_evaluation(eval2).expect("save 2 ok");

    // Load and verify
    let loaded = load_evaluations().expect("loaded ok");
    assert_eq!(loaded.len(), 2);

    // Summary calculation
    let summary = compute_evaluations_summary();
    let stats_a = summary.get("test-voice-a").expect("stats found");
    assert_eq!(stats_a.total_evaluations, 2);
    assert_eq!(stats_a.approvals_count, 1);
    assert_eq!(stats_a.avg_naturalness, 80.0);
    assert_eq!(stats_a.avg_tonality, 75.0);

    // Cleanup
    let _ = std::fs::remove_file(&temp_eval_path);
    std::env::remove_var("YUKI_VOICE_EVALS_FILE");
}

#[test]
fn test_stt_language_configuration_and_env() {
    use yuki::voice::stt::LocalWhisperConfig;

    // 1. Sem variável de ambiente definida, default deve ser "pt"
    std::env::remove_var("YUKI_WHISPER_LANGUAGE");
    let cfg_default = LocalWhisperConfig::default();
    assert_eq!(cfg_default.language, "pt");

    // 2. Com YUKI_WHISPER_LANGUAGE="en", default deve ler "en"
    std::env::set_var("YUKI_WHISPER_LANGUAGE", "en");
    let cfg_en = LocalWhisperConfig::default();
    assert_eq!(cfg_en.language, "en");

    // 3. Com YUKI_WHISPER_LANGUAGE="auto", default deve ler "auto"
    std::env::set_var("YUKI_WHISPER_LANGUAGE", "auto");
    let cfg_auto = LocalWhisperConfig::default();
    assert_eq!(cfg_auto.language, "auto");

    // 4. with_language deve sobrescrever e normalizar
    let cfg_custom = cfg_default.with_language("pt-BR");
    assert_eq!(cfg_custom.language, "pt");

    let cfg_ja = cfg_custom.with_language("ja");
    assert_eq!(cfg_ja.language, "ja");

    // Limpeza
    std::env::remove_var("YUKI_WHISPER_LANGUAGE");
}

#[test]
fn test_stt_language_normalization_and_validation() {
    use yuki::voice::stt::{is_supported_language, normalize_language_code};

    // Normalização canônica
    assert_eq!(normalize_language_code("pt"), "pt");
    assert_eq!(normalize_language_code("pt-BR"), "pt");
    assert_eq!(normalize_language_code("pt_br"), "pt");
    assert_eq!(normalize_language_code("en"), "en");
    assert_eq!(normalize_language_code("en-US"), "en");
    assert_eq!(normalize_language_code("EN_GB"), "en");
    assert_eq!(normalize_language_code("es"), "es");
    assert_eq!(normalize_language_code("es-ES"), "es");
    assert_eq!(normalize_language_code("ja"), "ja");
    assert_eq!(normalize_language_code("ja-JP"), "ja");
    assert_eq!(normalize_language_code("auto"), "auto");
    assert_eq!(normalize_language_code("AUTO"), "auto");
    assert_eq!(normalize_language_code(""), "auto");

    // Validação de idiomas conhecidos
    assert!(is_supported_language("pt"));
    assert!(is_supported_language("pt-BR"));
    assert!(is_supported_language("en"));
    assert!(is_supported_language("es"));
    assert!(is_supported_language("ja"));
    assert!(is_supported_language("fr"));
    assert!(is_supported_language("de"));
    assert!(is_supported_language("it"));
    assert!(is_supported_language("auto"));
    assert!(!is_supported_language("invalido_longo_123"));
}

#[test]
fn test_stt_parse_whisper_stderr_detection_and_timings() {
    use yuki::voice::stt::parse_whisper_stderr;

    // 1. Simulação de saída real do whisper.cpp com detecção de idioma 'pt' e confiança 99.8%
    let stderr_sample = r#"
system_info: n_threads = 4 / 8 | AVX2 = 1 | OPENMP = 1
main: processing 'audio.wav' (88480 samples, 5.5 sec), 4 threads, lang = auto, timestamps = 0 ...
whisper_full_with_state: auto-detected language: pt (p = 0.998412)
whisper_print_timings:     load time =  3085.32 ms
whisper_print_timings:     fallbacks =   0 p /   0 h
whisper_print_timings:      mel time =    36.30 ms
whisper_print_timings:   sample time =    99.32 ms /    30 runs
whisper_print_timings:   encode time = 21604.45 ms /     2 runs
whisper_print_timings:   decode time =   297.57 ms /     3 runs
whisper_print_timings:    total time = 26121.40 ms
"#;

    let meta = parse_whisper_stderr(stderr_sample);
    assert_eq!(meta.detected_language.as_deref(), Some("pt"));
    assert!((meta.probability.unwrap() - 0.998412).abs() < 1e-4);
    assert_eq!(meta.load_time_ms, Some(3085.32));
    assert_eq!(meta.encode_time_ms, Some(21604.45));
    assert_eq!(meta.total_time_ms, Some(26_121.4));

    // 2. Simulação com idioma 'ja' detectado
    let stderr_ja = "whisper_full_with_state: auto-detected language: ja (p = 0.884100)\nwhisper_print_timings:    total time = 15400.00 ms";
    let meta_ja = parse_whisper_stderr(stderr_ja);
    assert_eq!(meta_ja.detected_language.as_deref(), Some("ja"));
    assert!((meta_ja.probability.unwrap() - 0.8841).abs() < 1e-4);
    assert_eq!(meta_ja.total_time_ms, Some(15400.0));

    // 3. Simulação sem auto-detect (idioma fixo especificado pelo usuário)
    let stderr_fixed = "main: processing 'audio.wav', lang = pt\nwhisper_print_timings:    total time = 5000.00 ms";
    let meta_fixed = parse_whisper_stderr(stderr_fixed);
    assert_eq!(meta_fixed.detected_language, None);
    assert_eq!(meta_fixed.probability, None);
    assert_eq!(meta_fixed.total_time_ms, Some(5000.00));
}

#[tokio::test]
async fn test_stt_silence_checked_before_normalization_resilience() {
    use yuki::contracts::voice::{AudioFormat, AudioInput};
    use yuki::voice::audio::AudioCodec;
    use yuki::voice::stt::{is_silent_pcm, LocalWhisperConfig, LocalWhisperSttProvider};

    // Gera um sinal de silêncio com ruído térmico mínimo (não nulo)
    let ambient_noise: Vec<i16> = (0..16000)
        .map(|i| if i % 2 == 0 { 15 } else { -15 })
        .collect();

    // A função is_silent_pcm deve considerar silêncio
    assert!(is_silent_pcm(&ambient_noise));

    // Se normalizássemos antes, a amplitude iria para 28000:
    let mut modified = ambient_noise.clone();
    AudioCodec::normalize_amplitude(&mut modified, 28000);
    assert_eq!(modified.iter().map(|s| s.abs()).max(), Some(28000));
    // E então deixaria de ser considerado silêncio!
    assert!(!is_silent_pcm(&modified));

    // Mas no provedor Whisper, o teste de silêncio ocorre ANTES da normalização.
    // Assim, mesmo sem binários configurados, áudio silencioso retorna transcrição vazia sem erro:
    let format = AudioFormat::default();
    let wav_bytes = AudioCodec::encode_wav(&ambient_noise, &format).expect("encode ok");
    let silent_input = AudioInput::new(wav_bytes, format, 1000, "ambient-mic");

    // Com caminhos configurados, o curto-circuito de silêncio acústico deve
    // retornar imediatamente Ok(vazio) sem tentar executar o binário externo (que falharia):
    let provider = LocalWhisperSttProvider::new(LocalWhisperConfig {
        binary_path: Some(std::path::PathBuf::from("dummy_whisper_bin_does_not_exist")),
        model_path: Some(std::path::PathBuf::from("dummy_model_bin_does_not_exist")),
        ..Default::default()
    });

    let res = provider.transcribe_with_metadata(&silent_input).await;
    assert!(res.is_ok());
    let output = res.unwrap();
    assert_eq!(output.text, "");
}

#[test]
fn test_audio_codec_encode_raw_wav_formats() {
    use yuki::voice::audio::AudioCodec;

    // 1. Testa PCM 16-bit
    let raw_pcm16 = vec![0u8; 32000]; // 1s a 16kHz mono (16-bit = 2 bytes)
    let wav_pcm16 =
        AudioCodec::encode_raw_wav(&raw_pcm16, 1, 16000, 16, false).expect("encode pcm16");
    assert_eq!(&wav_pcm16[0..4], b"RIFF");
    assert_eq!(&wav_pcm16[8..12], b"WAVE");
    assert_eq!(&wav_pcm16[12..16], b"fmt ");
    let fmt_size = u32::from_le_bytes([wav_pcm16[16], wav_pcm16[17], wav_pcm16[18], wav_pcm16[19]]);
    assert_eq!(fmt_size, 16);
    let tag = u16::from_le_bytes([wav_pcm16[20], wav_pcm16[21]]);
    assert_eq!(tag, 1); // PCM

    // 2. Testa IEEE Float 32-bit (formato Realtek/Windows moderno)
    let raw_float32 = vec![0u8; 48000 * 2 * 4]; // 1s a 48kHz stereo 32-bit float
    let wav_float =
        AudioCodec::encode_raw_wav(&raw_float32, 2, 48000, 32, true).expect("encode float32");
    assert_eq!(&wav_float[0..4], b"RIFF");
    assert_eq!(&wav_float[8..12], b"WAVE");
    assert_eq!(&wav_float[12..16], b"fmt ");
    let fmt_size_float =
        u32::from_le_bytes([wav_float[16], wav_float[17], wav_float[18], wav_float[19]]);
    assert_eq!(fmt_size_float, 18);
    let tag_float = u16::from_le_bytes([wav_float[20], wav_float[21]]);
    assert_eq!(tag_float, 3); // IEEE Float
    let cb_size = u16::from_le_bytes([wav_float[36], wav_float[37]]);
    assert_eq!(cb_size, 0);

    // 3. Testa PCM 24-bit
    let raw_pcm24 = vec![0u8; 44100 * 3]; // 1s a 44.1kHz mono 24-bit
    let wav_pcm24 =
        AudioCodec::encode_raw_wav(&raw_pcm24, 1, 44100, 24, false).expect("encode pcm24");
    assert_eq!(&wav_pcm24[0..4], b"RIFF");
    let tag_pcm24 = u16::from_le_bytes([wav_pcm24[20], wav_pcm24[21]]);
    assert_eq!(tag_pcm24, 1); // PCM
}

#[test]
fn test_acoustic_metrics_computation() {
    use yuki::voice::diag::compute_acoustic_metrics;

    // 1. Silêncio absoluto
    let silence = vec![0i16; 16000];
    let metrics_silence = compute_acoustic_metrics(&silence, 16000);
    assert_eq!(metrics_silence.sample_count, 16000);
    assert_eq!(metrics_silence.peak, 0);
    assert_eq!(metrics_silence.rms, 0.0);
    assert_eq!(metrics_silence.clipping_samples, 0);
    assert_eq!(metrics_silence.silence_ratio_pct, 100.0);

    // 2. Sinal com clipping (saturação digital intencional)
    let mut clipped_signal = vec![1000i16; 16000];
    clipped_signal[100] = 32767;
    clipped_signal[101] = -32768;
    clipped_signal[102] = 32765;
    let metrics_clipped = compute_acoustic_metrics(&clipped_signal, 16000);
    assert_eq!(metrics_clipped.peak, 32767);
    assert_eq!(metrics_clipped.clipping_samples, 3);
    assert!(metrics_clipped.clipping_ratio_pct > 0.0);
    assert_eq!(metrics_clipped.silence_ratio_pct, 0.0);

    // 3. Sinal de fala simulada (amplitude moderada sem clipping)
    let speech_sim: Vec<i16> = (0..16000)
        .map(|i| ((i as f64 * 0.1).sin() * 8000.0) as i16)
        .collect();
    let metrics_speech = compute_acoustic_metrics(&speech_sim, 16000);
    assert!(metrics_speech.peak > 7000 && metrics_speech.peak <= 8000);
    assert!(metrics_speech.rms > 5000.0 && metrics_speech.rms < 6000.0);
    assert_eq!(metrics_clipped.clipping_samples, 3);
    assert_eq!(metrics_speech.clipping_samples, 0);
}

#[test]
fn test_voice_diagnostics_session_artifacts_and_cleanup() {
    use std::path::Path;
    use yuki::contracts::voice::{BufferMetrics, CaptureTimingMetrics, NativeAudioFormatInfo};
    use yuki::voice::audio::AudioCodec;
    use yuki::voice::diag::{
        save_diagnostic_session, update_diagnostic_with_normalized, DIAGNOSTICS_DIR,
    };

    let session_id = format!(
        "test_diag_{}",
        yuki::contracts::identifiers::generate_unique_id("unit")
    );
    let native_format = NativeAudioFormatInfo {
        format_tag: 0xFFFE,
        format_tag_hex: "0xFFFE".to_string(),
        format_tag_name: "WAVE_FORMAT_EXTENSIBLE (IEEE Float)".to_string(),
        is_extensible: true,
        is_float: true,
        channels: 2,
        sample_rate: 48000,
        bits_per_sample: 32,
        block_align: 8,
        avg_bytes_per_sec: 384000,
        sub_format_guid: Some("00000003-0000-0010-8000-00aa00389b71".to_string()),
    };

    let timings = CaptureTimingMetrics {
        requested_duration_ms: 1000,
        init_duration_ms: 120,
        effective_capture_ms: 1005,
        samples_duration_ms: 1000,
    };

    let buffer_metrics = BufferMetrics {
        total_packets: 60,
        total_raw_frames: 48000,
        total_raw_bytes: 384000,
        silent_buffer_events: 0,
        discontinuity_glitches: 0,
    };

    let raw_bytes = vec![0u8; 384000];
    let canonical_samples = vec![150i16; 16000];

    // Salva sessão de diagnóstico
    let mut report = save_diagnostic_session(
        &session_id,
        Some("test-mic-id".to_string()),
        Some("Microfone Realtek Teste".to_string()),
        &native_format,
        timings,
        buffer_metrics,
        &raw_bytes,
        &canonical_samples,
    )
    .expect("save_diagnostic_session deve ter sucesso");

    let raw_path = Path::new(DIAGNOSTICS_DIR).join(format!("raw_captured_{}.wav", session_id));
    let canon_path = Path::new(DIAGNOSTICS_DIR).join(format!("canonical_16k_{}.wav", session_id));
    let json_path = Path::new(DIAGNOSTICS_DIR).join(format!("diag_{}.json", session_id));

    assert!(
        raw_path.exists(),
        "raw_captured_{}.wav deve existir",
        session_id
    );
    assert!(
        canon_path.exists(),
        "canonical_16k_{}.wav deve existir",
        session_id
    );
    assert!(json_path.exists(), "diag_{}.json deve existir", session_id);

    // Valida que o áudio canônico gravado é um WAV válido decodificável
    let canon_bytes = std::fs::read(&canon_path).expect("read canon ok");
    let (fmt, samples) = AudioCodec::decode_wav(&canon_bytes).expect("decode canon ok");
    assert_eq!(fmt.sample_rate, 16000);
    assert_eq!(fmt.channels, 1);
    assert_eq!(samples.len(), 16000);

    // Atualiza com versão normalizada
    let normalized_samples = vec![300i16; 16000];
    let normalized_wav = AudioCodec::encode_wav(&normalized_samples, &fmt).expect("encode norm ok");
    update_diagnostic_with_normalized(&mut report, &normalized_samples, &normalized_wav)
        .expect("update_diagnostic_with_normalized ok");

    let norm_path = Path::new(DIAGNOSTICS_DIR).join(format!("normalized_16k_{}.wav", session_id));
    assert!(
        norm_path.exists(),
        "normalized_16k_{}.wav deve existir",
        session_id
    );
    assert!(report.acoustic_normalized.is_some());

    // Limpeza higiênica dos arquivos temporários de teste
    let _ = std::fs::remove_file(&raw_path);
    let _ = std::fs::remove_file(&canon_path);
    let _ = std::fs::remove_file(&norm_path);
    let _ = std::fs::remove_file(&json_path);
}
