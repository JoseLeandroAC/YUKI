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

#[test]
fn test_whisper_config_validation_and_builders() {
    use yuki::voice::stt::{
        validate_beam_size, validate_best_of, validate_threads, LocalWhisperConfig,
    };

    // 1. Validação de threads
    assert!(validate_threads(1).is_ok());
    assert!(validate_threads(4).is_ok());
    assert!(validate_threads(64).is_ok());
    assert!(validate_threads(0).is_err());
    assert!(validate_threads(65).is_err());

    // 2. Validação de beam size (1 a 10)
    assert!(validate_beam_size(1).is_ok());
    assert!(validate_beam_size(5).is_ok());
    assert!(validate_beam_size(10).is_ok());
    assert!(validate_beam_size(0).is_err());
    assert!(validate_beam_size(11).is_err());

    // 3. Validação de best of (1 a 10)
    assert!(validate_best_of(1).is_ok());
    assert!(validate_best_of(5).is_ok());
    assert!(validate_best_of(10).is_ok());
    assert!(validate_best_of(0).is_err());
    assert!(validate_best_of(11).is_err());

    // 4. Builders e mutadores
    let mut cfg = LocalWhisperConfig::default();
    assert_eq!(cfg.threads, None);
    assert_eq!(cfg.beam_size, None);
    assert!(!cfg.no_fallback);
    assert!(!cfg.is_fast_mode());

    cfg = cfg.with_threads(8).expect("valid threads");
    assert_eq!(cfg.threads, Some(8));

    cfg = cfg.with_beam_size(3).expect("valid beam size");
    assert_eq!(cfg.beam_size, Some(3));

    cfg = cfg.with_best_of(3).expect("valid best of");
    assert_eq!(cfg.best_of, Some(3));

    cfg = cfg.with_no_fallback(true);
    assert!(cfg.no_fallback);

    // Fast decoding ativa modo greedy (beam=1, bo=1, no-fallback=true)
    let fast_cfg = LocalWhisperConfig::default().with_fast_decoding();
    assert_eq!(fast_cfg.beam_size, Some(1));
    assert_eq!(fast_cfg.best_of, Some(1));
    assert!(fast_cfg.no_fallback);
    assert!(fast_cfg.is_fast_mode());

    // Prompt lexical com sanitização de espaço
    let prompt_cfg = LocalWhisperConfig::default()
        .with_initial_prompt("  Yuki, FUVEST, Engenharia de Computação.  ");
    assert_eq!(
        prompt_cfg.initial_prompt.as_deref(),
        Some("Yuki, FUVEST, Engenharia de Computação.")
    );

    let empty_prompt = LocalWhisperConfig::default().with_initial_prompt("   ");
    assert_eq!(empty_prompt.initial_prompt, None);
}

#[test]
fn test_whisper_prompt_language_presets() {
    use yuki::voice::stt::{
        default_lexical_prompt_for_language, LocalWhisperConfig, YUKI_DEFAULT_VOCABULARY_PROMPT_PT,
    };

    assert_eq!(
        default_lexical_prompt_for_language("pt"),
        Some(YUKI_DEFAULT_VOCABULARY_PROMPT_PT)
    );
    assert_eq!(
        default_lexical_prompt_for_language("pt-BR"),
        Some(YUKI_DEFAULT_VOCABULARY_PROMPT_PT)
    );
    assert!(default_lexical_prompt_for_language("en").is_some());
    assert!(default_lexical_prompt_for_language("es").is_some());
    assert!(default_lexical_prompt_for_language("ja").is_some());

    // Modo auto ou idiomas não-mapeados não devem receber prompt preset para evitar viés
    assert_eq!(default_lexical_prompt_for_language("auto"), None);
    assert_eq!(default_lexical_prompt_for_language("xyz"), None);

    let cfg_pt = LocalWhisperConfig::default()
        .with_language("pt")
        .with_default_prompt_for_language();
    assert_eq!(
        cfg_pt.initial_prompt.as_deref(),
        Some(YUKI_DEFAULT_VOCABULARY_PROMPT_PT)
    );

    let cfg_auto = LocalWhisperConfig::default()
        .with_language("auto")
        .with_default_prompt_for_language();
    assert_eq!(cfg_auto.initial_prompt, None);
}

#[test]
fn test_whisper_stderr_parsing_extended_metrics() {
    use yuki::voice::stt::parse_whisper_stderr;

    let mock_stderr = r#"
whisper_init_from_file_with_params_no_state: loading model from 'models/whisper/ggml-small.bin'
whisper_model_load: n_vocab       = 51865
whisper_model_load: n_audio_ctx   = 1500
whisper_full_with_state: auto-detected language: pt (p = 0.998412)
whisper_print_timings:     load time =   142.35 ms
whisper_print_timings:      mel time =    35.12 ms
whisper_print_timings:   sample time =     8.45 ms
whisper_print_timings:   encode time =   410.60 ms
whisper_print_timings:   decode time =   895.20 ms
whisper_print_timings:   prompt time =    18.70 ms
whisper_print_timings:    total time =  1510.42 ms
whisper_print_timings:     fallbacks = 1
"#;

    let meta = parse_whisper_stderr(mock_stderr);

    assert_eq!(meta.detected_language.as_deref(), Some("pt"));
    assert!((meta.probability.unwrap() - 0.998412).abs() < 1e-4);
    assert_eq!(meta.load_time_ms, Some(142.35));
    assert_eq!(meta.mel_time_ms, Some(35.12));
    assert_eq!(meta.sample_time_ms, Some(8.45));
    assert_eq!(meta.encode_time_ms, Some(410.60));
    assert_eq!(meta.decode_time_ms, Some(895.20));
    assert_eq!(meta.prompt_time_ms, Some(18.70));
    assert_eq!(meta.total_time_ms, Some(1510.42));
    assert_eq!(meta.fallbacks, Some(1));
}

#[tokio::test]
async fn test_whisper_stt_silence_short_circuit_with_timings() {
    let config = LocalWhisperConfig {
        binary_path: Some(std::path::PathBuf::from("mock_whisper")),
        model_path: Some(std::path::PathBuf::from("mock_model.bin")),
        ..Default::default()
    };
    let provider = LocalWhisperSttProvider::new(config);

    // Gera áudio 16kHz de 1 segundo de puro silêncio
    let silent_samples = vec![0i16; 16000];
    let format = AudioFormat {
        sample_rate: 16000,
        channels: 1,
        bits_per_sample: 16,
    };
    let payload = AudioCodec::encode_wav(&silent_samples, &format).expect("encode wav ok");
    let audio_input = AudioInput::new(payload, format, 1000, "silence_test");

    let result = provider
        .transcribe_with_metadata(&audio_input)
        .await
        .expect("transcribe silence ok");

    assert_eq!(result.text, "");
    assert!(result.timings.total_stt_wall_ms >= 0.0);
    assert_eq!(result.timings.subprocess_wall_ms, 0.0);
    assert_eq!(result.decoding_config.language_requested, "pt");
}

#[test]
fn test_diagnostic_report_stt_integration_and_serialization() {
    use yuki::contracts::voice::{
        AudioDiagnosticsReport, BufferMetrics, CaptureTimingMetrics, NativeAudioFormatInfo,
        SttDecodingConfigSummary, SttTimingMetrics,
    };
    use yuki::voice::diag::update_diagnostic_with_stt;

    let mut report = AudioDiagnosticsReport {
        session_id: "test-stt-diag-01".to_string(),
        timestamp: "2026-10-13T12:00:00Z".to_string(),
        device_id: None,
        device_name: None,
        native_format: NativeAudioFormatInfo {
            format_tag: 1,
            format_tag_hex: "0x0001".to_string(),
            format_tag_name: "PCM".to_string(),
            is_extensible: false,
            is_float: false,
            channels: 1,
            sample_rate: 16000,
            bits_per_sample: 16,
            block_align: 2,
            avg_bytes_per_sec: 32000,
            sub_format_guid: None,
        },
        timings: CaptureTimingMetrics {
            requested_duration_ms: 1000,
            init_duration_ms: 10,
            effective_capture_ms: 1000,
            samples_duration_ms: 1000,
        },
        buffer_metrics: BufferMetrics {
            total_packets: 10,
            total_raw_frames: 16000,
            total_raw_bytes: 32000,
            silent_buffer_events: 0,
            discontinuity_glitches: 0,
        },
        acoustic_canonical: yuki::voice::diag::compute_acoustic_metrics(&[100i16; 16000], 16000),
        acoustic_normalized: None,
        stt_timings: None,
        stt_config: None,
        resampler_kind: None,
        raw_wav_path: None,
        canonical_wav_path: None,
        normalized_wav_path: None,
        report_path: None,
    };

    let timings = SttTimingMetrics {
        audio_prep_ms: 1.5,
        audio_norm_ms: 2.3,
        temp_write_ms: 3.1,
        subprocess_wall_ms: 450.0,
        whisper_load_ms: Some(120.0),
        whisper_mel_ms: Some(30.0),
        whisper_sample_ms: Some(5.0),
        whisper_encode_ms: Some(150.0),
        whisper_decode_ms: Some(140.0),
        whisper_prompt_ms: Some(10.0),
        whisper_total_ms: Some(440.0),
        whisper_fallbacks: Some(0),
        post_process_ms: 0.8,
        total_stt_wall_ms: 457.7,
    };

    let config = SttDecodingConfigSummary {
        language_requested: "pt".to_string(),
        prompt_configured: true,
        prompt_preview: Some("Yuki, FUVEST...".to_string()),
        threads: Some(4),
        beam_size: Some(1),
        best_of: Some(1),
        no_fallback: true,
    };

    update_diagnostic_with_stt(&mut report, &timings, &config).expect("update stt ok");

    assert!(report.stt_timings.is_some());
    assert!(report.stt_config.is_some());

    // Testa roundtrip de serialização JSON
    let json = serde_json::to_string_pretty(&report).expect("serialize ok");
    let deserialized: AudioDiagnosticsReport = serde_json::from_str(&json).expect("deserialize ok");
    assert_eq!(report, deserialized);
}

#[test]
fn test_stt_eval_normalization() {
    use yuki::voice::stt::eval::normalize_for_evaluation;

    // 1. Maiúsculas e pontuações
    assert_eq!(
        normalize_for_evaluation("Olá, Yuki! Tudo bem?"),
        "olá yuki tudo bem"
    );

    // 2. Preservação de acentos e diacríticos em português
    assert_eq!(
        normalize_for_evaluation("Atenção, não esqueça o coração e a bênção."),
        "atenção não esqueça o coração e a bênção"
    );
    assert_eq!(
        normalize_for_evaluation("é, à, ó, ú, í, â, ê, ô, ç"),
        "é à ó ú í â ê ô ç"
    );

    // 3. Colapso de múltiplos espaços e traços
    assert_eq!(
        normalize_for_evaluation("   Yuki    ---   abrir   calculadora...   "),
        "yuki abrir calculadora"
    );

    // 4. Caracteres japoneses (Kanji e Kana)
    assert_eq!(
        normalize_for_evaluation("Yuki、こんにちは！人工知能。"),
        "yuki こんにちは 人工知能"
    );
}

#[test]
fn test_stt_eval_wer_exact_calculations() {
    use yuki::voice::stt::eval::calculate_wer;

    // 1. Frases idênticas -> WER = 0.0, Hits = 4, S=0, D=0, I=0
    let res_ident = calculate_wer("Yuki abrir a calculadora", "Yuki abrir a calculadora");
    assert_eq!(res_ident.wer_rate, 0.0);
    assert_eq!(res_ident.hits, 4);
    assert_eq!(res_ident.substitutions, 0);
    assert_eq!(res_ident.deletions, 0);
    assert_eq!(res_ident.insertions, 0);
    assert_eq!(res_ident.reference_words, 4);

    // 2. Substituição pura: 1 de 4 palavras trocada -> WER = 0.25 (25%)
    let res_sub = calculate_wer("o gato bebe leite", "o cachorro bebe leite");
    assert_eq!(res_sub.wer_rate, 0.25);
    assert_eq!(res_sub.substitutions, 1);
    assert_eq!(res_sub.deletions, 0);
    assert_eq!(res_sub.insertions, 0);
    assert_eq!(res_sub.hits, 3);
    assert_eq!(res_sub.reference_words, 4);

    // 3. Deleção pura: 1 palavra omitida de 4 -> WER = 0.25 (25%)
    let res_del = calculate_wer("o gato bebe leite", "o gato bebe");
    assert_eq!(res_del.wer_rate, 0.25);
    assert_eq!(res_del.substitutions, 0);
    assert_eq!(res_del.deletions, 1);
    assert_eq!(res_del.insertions, 0);
    assert_eq!(res_del.hits, 3);
    assert_eq!(res_del.reference_words, 4);

    // 4. Inserção pura: 2 palavras a mais em 3 -> WER = 2/3 (~0.6667)
    let res_ins = calculate_wer("o gato bebe", "o gato bebe muito leite");
    assert!((res_ins.wer_rate - 2.0 / 3.0).abs() < 1e-6);
    assert_eq!(res_ins.substitutions, 0);
    assert_eq!(res_ins.deletions, 0);
    assert_eq!(res_ins.insertions, 2);
    assert_eq!(res_ins.hits, 3);
    assert_eq!(res_ins.reference_words, 3);

    // 5. Casos vazios e limites
    let res_empty_both = calculate_wer("", "");
    assert_eq!(res_empty_both.wer_rate, 0.0);
    assert_eq!(res_empty_both.reference_words, 0);

    let res_empty_ref = calculate_wer("", "palavra inserida");
    assert_eq!(res_empty_ref.wer_rate, 1.0);
    assert_eq!(res_empty_ref.insertions, 2);

    let res_empty_hyp = calculate_wer("palavra na referencia", "");
    assert_eq!(res_empty_hyp.wer_rate, 1.0);
    assert_eq!(res_empty_hyp.deletions, 3);
}

#[test]
fn test_stt_eval_cer_latin_and_japanese() {
    use yuki::voice::stt::eval::calculate_cer;

    // 1. Textos latinos idênticos com pontuação ignorada
    let res_ident = calculate_cer("Olá, Yuki!", "olá yuki");
    assert_eq!(res_ident.cer_rate, 0.0);
    assert_eq!(res_ident.hits, 7); // 'o', 'l', 'á', 'y', 'u', 'k', 'i'

    // 2. Erro de um caractere no nome próprio ("Yuki" vs "Iuki")
    let res_name = calculate_cer("Yuki", "Iuki");
    assert_eq!(res_name.substitutions, 1);
    assert_eq!(res_name.hits, 3);
    assert_eq!(res_name.reference_chars, 4);
    assert_eq!(res_name.cer_rate, 0.25);

    // 3. Japonês: "こんにちは" (5 chars) vs "こんばんは" (5 chars)
    // 'に' e 'ち' substituídos por 'ば' -> 2 substituições
    let res_ja = calculate_cer("こんにちは", "こんばんは");
    assert_eq!(res_ja.reference_chars, 5);
    assert_eq!(res_ja.substitutions, 2);
    assert_eq!(res_ja.cer_rate, 0.4);
}

#[test]
fn test_stt_eval_key_terms_spotting() {
    use yuki::voice::stt::eval::{evaluate_key_terms, KeyTermStatus};

    let reference = "Yuki, hoje eu preciso revisar matemática para a FUVEST e a OBMEP.";
    let hypothesis = "Hoje eu preciso revisar matemática para a FUVEST e o ENEM.";
    let terms = vec![
        "Yuki".to_string(),
        "FUVEST".to_string(),
        "OBMEP".to_string(),
        "matemática".to_string(),
        "ENEM".to_string(),
    ];

    let result = evaluate_key_terms(reference, hypothesis, &terms);
    assert_eq!(result.total_terms, 5);

    // FUVEST e matemática: presentes na ref e na hyp -> TruePositive
    assert_eq!(result.true_positives, 2);
    // Yuki e OBMEP: presentes na ref mas ausentes na hyp -> FalseNegative
    assert_eq!(result.false_negatives, 2);
    // ENEM: ausente na ref mas inserido na hyp -> FalsePositive
    assert_eq!(result.false_positives, 1);
    assert_eq!(result.true_negatives, 0);

    // Acurácia dos termos esperados: 2 / (2 + 2) = 50%
    assert_eq!(result.accuracy, 0.5);

    let fuvest = result.details.iter().find(|d| d.term == "FUVEST").unwrap();
    assert_eq!(fuvest.status, KeyTermStatus::TruePositive);

    let yuki = result.details.iter().find(|d| d.term == "Yuki").unwrap();
    assert_eq!(yuki.status, KeyTermStatus::FalseNegative);

    let enem = result.details.iter().find(|d| d.term == "ENEM").unwrap();
    assert_eq!(enem.status, KeyTermStatus::FalsePositive);
}

#[test]
fn test_stt_eval_latency_robust_stats() {
    use yuki::voice::stt::eval::compute_latency_stats;

    // 1. Array vazio
    let empty = compute_latency_stats(&[]);
    assert_eq!(empty.runs_count, 0);
    assert_eq!(empty.median_ms, 0.0);

    // 2. Execução única
    let single = compute_latency_stats(&[1500.0]);
    assert_eq!(single.runs_count, 1);
    assert_eq!(single.median_ms, 1500.0);
    assert_eq!(single.mean_ms, 1500.0);
    assert_eq!(single.std_dev_ms, 0.0);
    assert_eq!(single.min_ms, 1500.0);
    assert_eq!(single.max_ms, 1500.0);

    // 3. 3 execuções (ímpar)
    let runs_odd = compute_latency_stats(&[1200.0, 1000.0, 1100.0]);
    assert_eq!(runs_odd.runs_count, 3);
    assert_eq!(runs_odd.median_ms, 1100.0);
    assert_eq!(runs_odd.mean_ms, 1100.0);
    assert_eq!(runs_odd.min_ms, 1000.0);
    assert_eq!(runs_odd.max_ms, 1200.0);
    assert_eq!(runs_odd.std_dev_ms, 100.0);

    // 4. 2 execuções (par)
    let runs_even = compute_latency_stats(&[100.0, 200.0]);
    assert_eq!(runs_even.runs_count, 2);
    assert_eq!(runs_even.median_ms, 150.0);
    assert_eq!(runs_even.mean_ms, 150.0);
}

#[test]
fn test_stt_eval_corpus_manifest_serialization_and_filters() {
    use yuki::voice::stt::eval::{CorpusCategory, CorpusTestCase, VoiceCorpusManifest};

    let manifest = VoiceCorpusManifest {
        version: "1.0.0".to_string(),
        description: "Test manifest".to_string(),
        test_cases: vec![
            CorpusTestCase {
                id: "test_cmd".to_string(),
                audio_path: "data/test.wav".to_string(),
                ground_truth: Some("abrir calculadora".to_string()),
                review_pending: false,
                language: "pt".to_string(),
                device_name: Some("Realtek".to_string()),
                category: CorpusCategory::Command,
                important_terms: vec!["calculadora".to_string()],
                duration_ms: Some(3000),
                notes: None,
            },
            CorpusTestCase {
                id: "test_mult".to_string(),
                audio_path: "data/test_en.wav".to_string(),
                ground_truth: None,
                review_pending: true,
                language: "en".to_string(),
                device_name: None,
                category: CorpusCategory::Multilingual,
                important_terms: vec![],
                duration_ms: None,
                notes: None,
            },
        ],
    };

    // Filtros
    let cmds = manifest.filter_by_category(CorpusCategory::Command);
    assert_eq!(cmds.len(), 1);
    assert_eq!(cmds[0].id, "test_cmd");

    let pt_cases = manifest.filter_by_language("pt");
    assert_eq!(pt_cases.len(), 1);
    assert_eq!(pt_cases[0].id, "test_cmd");

    // Serialização JSON
    let temp_file = std::env::temp_dir().join(format!(
        "yuki_test_manifest_{}.json",
        yuki::contracts::identifiers::generate_unique_id("manifest")
    ));
    manifest.save_to_file(&temp_file).expect("save manifest ok");

    let loaded = VoiceCorpusManifest::load_from_file(&temp_file).expect("load manifest ok");
    assert_eq!(loaded.version, "1.0.0");
    assert_eq!(loaded.test_cases.len(), 2);
    assert_eq!(loaded.test_cases[0].id, "test_cmd");
    assert_eq!(loaded.test_cases[1].category, CorpusCategory::Multilingual);

    let _ = std::fs::remove_file(temp_file);
}

#[test]
fn test_stt_eval_report_export_json_and_roundtrip() {
    use yuki::contracts::voice::{SttDecodingConfigSummary, SttTimingMetrics};
    use yuki::voice::stt::eval::{
        compute_latency_stats, CorpusEvaluationReport, SingleEvaluationReport,
    };

    let rep = SingleEvaluationReport {
        test_case_id: "T14_unit".to_string(),
        audio_path: "test.wav".to_string(),
        ground_truth: Some("Yuki, abrir calculadora.".to_string()),
        hypothesis: "Yuki abrir calculadora".to_string(),
        review_pending: false,
        wer: Some(yuki::voice::stt::eval::calculate_wer(
            "Yuki, abrir calculadora.",
            "Yuki abrir calculadora",
        )),
        cer: Some(yuki::voice::stt::eval::calculate_cer(
            "Yuki, abrir calculadora.",
            "Yuki abrir calculadora",
        )),
        key_terms: None,
        latency: compute_latency_stats(&[500.0, 520.0]),
        timings: SttTimingMetrics::default(),
        decoding_config: SttDecodingConfigSummary {
            language_requested: "pt".to_string(),
            prompt_configured: false,
            prompt_preview: None,
            threads: None,
            beam_size: None,
            best_of: None,
            no_fallback: false,
        },
    };

    let corpus_report = CorpusEvaluationReport {
        generated_at: "2026-10-13T16:00:00Z".to_string(),
        manifest_path: Some("manifest.json".to_string()),
        total_cases: 1,
        evaluated_cases: 1,
        review_pending_cases: 0,
        average_wer: Some(0.0),
        average_cer: Some(0.0),
        overall_key_terms_accuracy: None,
        average_latency_ms: 510.0,
        reports: vec![rep],
    };

    let temp_export = std::env::temp_dir().join(format!(
        "yuki_test_report_{}.json",
        yuki::contracts::identifiers::generate_unique_id("eval_rep")
    ));

    corpus_report
        .export_to_json(&temp_export)
        .expect("export ok");

    let content = std::fs::read_to_string(&temp_export).expect("read export ok");
    let loaded: CorpusEvaluationReport = serde_json::from_str(&content).expect("parse export ok");
    assert_eq!(loaded.total_cases, 1);
    assert_eq!(loaded.reports[0].test_case_id, "T14_unit");
    assert_eq!(loaded.reports[0].wer.as_ref().unwrap().wer_rate, 0.0);

    let _ = std::fs::remove_file(temp_export);
}

#[test]
fn test_stt_eval_corpus_manifest_structural_integrity() {
    use std::collections::HashSet;
    use std::path::Path;
    use yuki::voice::stt::eval::VoiceCorpusManifest;

    let manifest_path = Path::new("data/voice_corpus/corpus_manifest.json");
    if !manifest_path.exists() {
        // Se o teste rodar fora do workspace raiz, pula
        return;
    }

    let manifest = VoiceCorpusManifest::load_from_file(manifest_path)
        .expect("Manifesto de corpus deve carregar sem erros");

    assert!(
        manifest.test_cases.len() >= 16,
        "Manifesto deve conter ao menos os 16 testes físicos (T7 a T20), encontrados {}",
        manifest.test_cases.len()
    );

    let mut seen_ids = HashSet::new();
    let valid_languages = ["pt", "en", "es", "ja"];

    for tc in &manifest.test_cases {
        // 1. Unicidade estrita de IDs
        assert!(
            seen_ids.insert(tc.id.clone()),
            "ID duplicado detectado no manifesto: '{}'",
            tc.id
        );

        // 2. Coerência de review_pending e ground_truth
        if !tc.review_pending {
            assert!(
                tc.ground_truth
                    .as_ref()
                    .is_some_and(|gt| !gt.trim().is_empty()),
                "Caso '{}' com review_pending=false deve conter ground_truth não-vazio",
                tc.id
            );
        }

        // 3. Formatação do caminho de áudio
        assert!(
            tc.audio_path.ends_with(".wav"),
            "Caso '{}' deve apontar para arquivo .wav: '{}'",
            tc.id,
            tc.audio_path
        );

        let audio_file = Path::new(&tc.audio_path);
        assert!(
            audio_file.exists(),
            "Arquivo de áudio físico referenciado em '{}' deve existir no disco: '{}'",
            tc.id,
            tc.audio_path
        );

        // 4. Idioma válido
        assert!(
            valid_languages.contains(&tc.language.as_str()),
            "Caso '{}' possui idioma inválido ou não suportado: '{}'",
            tc.id,
            tc.language
        );

        // 5. Duração positiva se presente
        if let Some(dur) = tc.duration_ms {
            assert!(dur > 0, "Caso '{}' tem duração não positiva", tc.id);
        }
    }

    // Validação específica da correção comprovada de T11 (japonês) e multilíngues
    let t11 = manifest
        .test_cases
        .iter()
        .find(|c| c.id == "T11_realtek_saudacao_ja")
        .expect("T11_realtek_saudacao_ja deve existir no manifesto");
    assert_eq!(t11.language, "ja");
    assert!(t11.review_pending);

    let t9 = manifest
        .test_cases
        .iter()
        .find(|c| c.id == "T09_realtek_saudacao_en")
        .expect("T09_realtek_saudacao_en deve existir");
    assert_eq!(t9.language, "en");

    let t10 = manifest
        .test_cases
        .iter()
        .find(|c| c.id == "T10_realtek_saudacao_es")
        .expect("T10_realtek_saudacao_es deve existir");
    assert_eq!(t10.language, "es");
}

#[test]
fn test_dsp_resampler_fir_frequency_response_analytical() {
    use yuki::voice::resample::{
        design_kaiser_sinc_lpf, evaluate_frequency_response_db, AudioResampler,
    };

    let taps = design_kaiser_sinc_lpf(
        AudioResampler::DEFAULT_FIR_TAPS,
        AudioResampler::DEFAULT_CUTOFF_HZ,
        48000.0,
        AudioResampler::DEFAULT_KAISER_BETA,
    );
    assert_eq!(taps.len(), 97);

    // 1. Ganho DC (0 Hz): 0 dB exato (+- 0.01 dB)
    let resp_dc = evaluate_frequency_response_db(&taps, 0.0, 48000.0);
    assert!(
        resp_dc.abs() < 0.01,
        "Ganho DC deve ser 0 dB, obtido: {:.3} dB",
        resp_dc
    );

    // 2. Banda passante de voz humana (1 kHz, 3 kHz, 5 kHz): atenuação mínima (< 0.3 dB)
    let resp_1k = evaluate_frequency_response_db(&taps, 1000.0, 48000.0);
    assert!(
        resp_1k.abs() < 0.1,
        "Atenuação em 1 kHz deve ser < 0.1 dB, obtido: {:.3} dB",
        resp_1k
    );

    let resp_3k = evaluate_frequency_response_db(&taps, 3000.0, 48000.0);
    assert!(
        resp_3k.abs() < 0.15,
        "Atenuação em 3 kHz deve ser < 0.15 dB, obtido: {:.3} dB",
        resp_3k
    );

    let resp_5k = evaluate_frequency_response_db(&taps, 5000.0, 48000.0);
    assert!(
        resp_5k.abs() < 0.3,
        "Atenuação em 5 kHz deve ser < 0.3 dB, obtido: {:.3} dB",
        resp_5k
    );

    // 3. Frequência de corte (7.200 Hz): transição controlada
    let resp_cutoff = evaluate_frequency_response_db(&taps, 7200.0, 48000.0);
    assert!(
        resp_cutoff < -3.0 && resp_cutoff > -12.0,
        "Corte em 7.200 Hz deve estar entre -3 e -12 dB, obtido: {:.3} dB",
        resp_cutoff
    );

    // 4. Nyquist de saída (8.000 Hz): atenuação relevante
    let resp_8k = evaluate_frequency_response_db(&taps, 8000.0, 48000.0);
    assert!(
        resp_8k <= -20.0,
        "Nyquist em 8.000 Hz deve ter atenuação >= 20 dB, obtido: {:.3} dB",
        resp_8k
    );

    // 5. Banda de rejeição (aliasing) >= 9.000 Hz: atenuação >= 38 dB
    let resp_9k = evaluate_frequency_response_db(&taps, 9000.0, 48000.0);
    assert!(
        resp_9k <= -38.0,
        "Frequência de 9.000 Hz deve ser atenuada em >= 38 dB, obtido: {:.3} dB",
        resp_9k
    );

    let resp_11k = evaluate_frequency_response_db(&taps, 11000.0, 48000.0);
    assert!(
        resp_11k <= -45.0,
        "Frequência de 11.000 Hz deve ser atenuada em >= 45 dB, obtido: {:.3} dB",
        resp_11k
    );

    let resp_15k = evaluate_frequency_response_db(&taps, 15000.0, 48000.0);
    assert!(
        resp_15k <= -50.0,
        "Frequência de 15.000 Hz deve ser atenuada em >= 50 dB, obtido: {:.3} dB",
        resp_15k
    );
}

#[test]
fn test_dsp_resampler_sinusoidal_end_to_end_aliasing_suppression() {
    use yuki::contracts::voice::ResamplerKind;
    use yuki::voice::audio::AudioCodec;
    use yuki::voice::resample::AudioResampler;

    let sample_rate = 48000;
    let duration_secs = 1.0;
    let n_samples = (sample_rate as f64 * duration_secs) as usize;

    // Gerador de tom senoidal mono de 48 kHz
    let generate_sine = |freq_hz: f64, amplitude: f64| -> Vec<i16> {
        let mut samples = Vec::with_capacity(n_samples);
        for i in 0..n_samples {
            let t = (i as f64) / (sample_rate as f64);
            let val = amplitude * (2.0 * std::f64::consts::PI * freq_hz * t).sin();
            samples.push(val.round().clamp(-32768.0, 32767.0) as i16);
        }
        samples
    };

    // 1. Tom de 1.000 Hz (dentro da banda vocal passante)
    let sine_1k = generate_sine(1000.0, 10000.0);
    let out_1k_linear =
        AudioResampler::resample_oneshot(&sine_1k, 48000, 16000, ResamplerKind::LinearBaseline);
    let out_1k_fir =
        AudioResampler::resample_oneshot(&sine_1k, 48000, 16000, ResamplerKind::FirAntialiasing);

    let rms_in_1k = AudioCodec::calculate_rms(&sine_1k);
    let rms_linear_1k = AudioCodec::calculate_rms(&out_1k_linear);
    let rms_fir_1k = AudioCodec::calculate_rms(&out_1k_fir);

    // Ambas preservam energia em 1 kHz com desvio < 0.5 dB
    let db_diff = 20.0 * (rms_fir_1k / rms_linear_1k).log10();
    assert!(
        db_diff.abs() < 0.5,
        "Em 1 kHz, FIR e Linear devem ter energia equivalente. Diff: {:.3} dB",
        db_diff
    );
    assert!((rms_fir_1k - rms_in_1k).abs() / rms_in_1k < 0.05);

    // 2. Tom de 11.000 Hz (frequência acima do Nyquist de 16 kHz: causa aliasing dobrando para 5.000 Hz)
    let sine_11k = generate_sine(11000.0, 10000.0);
    let out_11k_linear =
        AudioResampler::resample_oneshot(&sine_11k, 48000, 16000, ResamplerKind::LinearBaseline);
    let out_11k_fir =
        AudioResampler::resample_oneshot(&sine_11k, 48000, 16000, ResamplerKind::FirAntialiasing);

    let rms_linear_11k = AudioCodec::calculate_rms(&out_11k_linear);
    let rms_fir_11k = AudioCodec::calculate_rms(&out_11k_fir);

    // No resampler linear, o tom de 11 kHz sobrevive quase integralmente por subamostragem direta!
    assert!(
        rms_linear_11k > 6000.0,
        "No resampler linear, 11 kHz sofre aliasing para 5 kHz mantendo amplitude alta: {:.1}",
        rms_linear_11k
    );

    // No resampler FIR Antialiasing, o tom de 11 kHz é suprimido em mais de 40 dB!
    let suppression_db = 20.0 * (rms_fir_11k / rms_linear_11k).log10();
    assert!(
        suppression_db <= -40.0,
        "FIR deve atenuar aliasing de 11 kHz em >= 40 dB. Obtido: {:.2} dB (RMS fir: {:.1}, linear: {:.1})",
        suppression_db,
        rms_fir_11k,
        rms_linear_11k
    );
}

#[test]
fn test_dsp_resampler_stability_and_edge_cases() {
    use yuki::contracts::voice::ResamplerKind;
    use yuki::voice::resample::AudioResampler;

    // 1. Buffer vazio
    let empty: Vec<i16> = Vec::new();
    let res_empty =
        AudioResampler::resample_oneshot(&empty, 48000, 16000, ResamplerKind::FirAntialiasing);
    assert!(res_empty.is_empty());

    // 2. Passthrough 1:1 (16 kHz -> 16 kHz)
    let samples = vec![100, 200, -300, 400, -500];
    let passthrough =
        AudioResampler::resample_oneshot(&samples, 16000, 16000, ResamplerKind::FirAntialiasing);
    assert_eq!(samples, passthrough);

    // 3. Bloco muito curto (menor que o número de taps, ex: 10 amostras)
    let short_block = vec![1000i16; 10];
    let res_short = AudioResampler::resample_oneshot(
        &short_block,
        48000,
        16000,
        ResamplerKind::FirAntialiasing,
    );
    assert_eq!(res_short.len(), 3);
    for &s in &res_short {
        assert!(s > 0);
    }

    // 4. Amplitude máxima e mínima sem clipping induzido ou overflow
    let extreme_samples = vec![i16::MAX, i16::MIN, i16::MAX, i16::MIN];
    let res_extreme = AudioResampler::resample_oneshot(
        &extreme_samples,
        48000,
        16000,
        ResamplerKind::FirAntialiasing,
    );
    assert_eq!(res_extreme.len(), 1);
    assert!(res_extreme[0] != 0);
}

#[test]
fn test_dsp_resampler_streaming_continuity_and_reset() {
    use yuki::voice::resample::AudioResampler;

    let mut resampler = AudioResampler::fir_48k_to_16k();

    // Simula 10 pacotes WASAPI consecutivos de 480 amostras cada (10ms a 48 kHz)
    let chunk_size = 480;
    let mut total_output = Vec::new();

    for p in 0..10 {
        let chunk: Vec<i16> = (0..chunk_size)
            .map(|i| {
                let t = (p * chunk_size + i) as f64 / 48000.0;
                (8000.0 * (2.0 * std::f64::consts::PI * 1000.0 * t).sin()) as i16
            })
            .collect();
        let out = resampler.process_streaming(&chunk);
        total_output.extend(out);
    }

    // Deve produzir aproximadamente 160 amostras por pacote de 480 (3:1)
    assert!(total_output.len() > 1500 && total_output.len() <= 1600);

    // Testa reset do estado
    resampler.reset();
    let after_reset = resampler.process_streaming(&vec![0i16; 480]);
    assert!(!after_reset.is_empty());
}

#[test]
fn test_dsp_resampler_cpu_performance_benchmark() {
    use std::time::Instant;
    use yuki::contracts::voice::ResamplerKind;
    use yuki::voice::resample::AudioResampler;

    // 10 segundos de áudio a 48 kHz = 480.000 amostras
    let samples_10s = vec![500i16; 480_000];

    let t0 = Instant::now();
    let out = AudioResampler::resample_oneshot(
        &samples_10s,
        48000,
        16000,
        ResamplerKind::FirAntialiasing,
    );
    let elapsed = t0.elapsed();

    assert_eq!(out.len(), 160_000);
    // 10 segundos de áudio devem ser processados em bem menos de 1000ms mesmo no perfil de teste debug não-otimizado (aceleração > 10x em tempo real)
    assert!(
        elapsed.as_millis() < 1000,
        "Reamostragem de 10s deve levar < 1000ms em debug, levou: {:?}",
        elapsed
    );
}

#[tokio::test]
async fn test_ab_resampler_real_audio_dsp_comparison() {
    use std::path::Path;
    use yuki::contracts::voice::ResamplerKind;
    use yuki::voice::audio::{AudioCodec, AudioResampler};
    use yuki::voice::stt::eval::run_resampler_ab_comparison;
    use yuki::voice::stt::LocalWhisperConfig;

    let raw_t14 = Path::new("data/voice_diag/raw_captured_diag_1a13645e57e_1.wav");
    if !raw_t14.exists() {
        return;
    }

    // 1. Verificação DSP de Reamostragem sobre o áudio físico T14 (Realtek 48 kHz float stereo)
    let raw_bytes = std::fs::read(raw_t14).expect("read raw t14 ok");
    let (rate, mono) = AudioCodec::decode_wav_to_mono_i16(&raw_bytes).expect("decode raw ok");
    assert_eq!(rate, 48000);
    assert!(!mono.is_empty());

    let out_linear =
        AudioResampler::resample_oneshot(&mono, rate, 16000, ResamplerKind::LinearBaseline);
    let out_fir =
        AudioResampler::resample_oneshot(&mono, rate, 16000, ResamplerKind::FirAntialiasing);

    assert_eq!(out_linear.len(), mono.len() / 3);
    assert_eq!(out_fir.len(), mono.len() / 3);

    let rms_linear = AudioCodec::calculate_rms(&out_linear);
    let rms_fir = AudioCodec::calculate_rms(&out_fir);

    // RMS entre Linear e FIR no sinal vocal real deve diferir por menos de 1 dB
    let db_diff = 20.0 * (rms_fir / rms_linear).log10();
    assert!(
        db_diff.abs() < 1.0,
        "Diferença RMS entre resamplers no sinal real deve ser < 1 dB: {:.3} dB",
        db_diff
    );

    // 2. Execução A/B com o Whisper se o ambiente local possuir executável e modelo
    let config = LocalWhisperConfig::default().with_language("pt");
    let whisper_ready = config.binary_path.as_ref().is_some_and(|p| p.exists())
        && config.model_path.as_ref().is_some_and(|p| p.exists());

    if whisper_ready {
        let ab_rep = run_resampler_ab_comparison(
            "T14_curto_calculadora",
            raw_t14,
            Some("Yuki, abrir calculadora."),
            &["Yuki".to_string(), "calculadora".to_string()],
            &config,
        )
        .await
        .expect("run ab comparison ok");

        assert!(!ab_rep.hypothesis_linear.is_empty());
        assert!(!ab_rep.hypothesis_fir.is_empty());
        assert!(ab_rep.wer_linear.is_some());
        assert!(ab_rep.wer_fir.is_some());
    }
}
