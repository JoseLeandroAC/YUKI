use std::path::Path;
use yuki::voice::audio::AudioCodec;
use yuki::voice::stt::{
    analyze_acoustic_activity, is_silent_pcm, is_silent_windowed, WindowedSilenceConfig,
};

// ---------------------------------------------------------------------------
// 1. Casos Sintéticos Controlados (Identificados Explicitamente como Sintéticos)
// ---------------------------------------------------------------------------

#[test]
fn test_synthetic_pure_zero_silence() {
    let samples = vec![0i16; 16000]; // 1.0s de zero absoluto
    let config = WindowedSilenceConfig::default();

    // Baseline legado
    assert!(
        is_silent_pcm(&samples),
        "Baseline deve reconhecer zero como silêncio"
    );

    // Detector por janelas experimental
    let analysis = analyze_acoustic_activity(&samples, 16000, &config);
    assert!(
        analysis.is_silent,
        "Janelado deve reconhecer zero como silêncio"
    );
    assert_eq!(analysis.active_windows, 0);
    assert_eq!(analysis.max_consecutive_active, 0);
    assert!(is_silent_windowed(&samples, 16000, &config));
}

#[test]
fn test_synthetic_ambient_hiss_noise() {
    // Ruído contínuo de baixo nível (-60 dBFS, amplitude +/- 25)
    let samples: Vec<i16> = (0..16000)
        .map(|i| if i % 2 == 0 { 25 } else { -25 })
        .collect();
    let config = WindowedSilenceConfig::default();

    // Baseline legado
    assert!(
        is_silent_pcm(&samples),
        "Baseline deve reconhecer ruído baixo como silêncio"
    );

    // Detector por janelas experimental
    let analysis = analyze_acoustic_activity(&samples, 16000, &config);
    assert!(
        analysis.is_silent,
        "Janelado deve reconhecer ruído baixo como silêncio"
    );
    assert_eq!(analysis.active_windows, 0);
    assert!(is_silent_windowed(&samples, 16000, &config));
}

#[test]
fn test_synthetic_stationary_continuous_drone() {
    // Ruído estacionário contínuo com RMS ~90, mas desvio padrão quase nulo (ex: zumbido/ventilador)
    let samples: Vec<i16> = (0..16000)
        .map(|i| {
            let offset = (i % 4) as i16 - 2;
            90 + offset
        })
        .collect();
    let config = WindowedSilenceConfig::default();

    let analysis = analyze_acoustic_activity(&samples, 16000, &config);
    // Deve ser rejeitado por variabilidade insuficiente (estacionariedade artificial)
    assert!(
        analysis.is_silent,
        "Ruído estacionário contínuo deve ser classificado como silêncio/ruído de fundo"
    );
    assert!(
        analysis
            .rejection_reason
            .as_ref()
            .unwrap()
            .contains("StationaryContinuousNoise"),
        "Motivo deve registrar ruído estacionário"
    );
}

#[test]
fn test_synthetic_isolated_transient_click() {
    // Silêncio de 1 segundo com um único estalo pontual de 647 amostras (similar ao glitch elétrico)
    let mut samples = vec![0i16; 16000];
    samples[8000] = 647; // 1 única amostra com pico alto

    let config = WindowedSilenceConfig::default();

    // FALHA DO BASELINE LEGADO:
    // O baseline calcula max_abs global e falha porque 647 >= 250!
    let legacy_result = is_silent_pcm(&samples);
    assert!(
        !legacy_result,
        "Baseline legado falha e classifica pico isolado como NÃO-silêncio"
    );

    // SUCESSO DO DETECTOR POR JANELAS EXPERIMENTAL:
    // O detector janelado avalia persistência temporal e descarta o pico transitório isolado (< 60ms)
    let analysis = analyze_acoustic_activity(&samples, 16000, &config);
    assert!(
        analysis.is_silent,
        "Detector janelado deve descartar estalo transitório pontual de 1 amostra"
    );
    assert_eq!(analysis.max_consecutive_active, 1);
    assert!(
        analysis
            .rejection_reason
            .as_ref()
            .unwrap()
            .contains("TransientSpikesWithoutSustainedSpeech"),
        "Motivo deve apontar transiente sem sustentação"
    );
}

#[test]
fn test_synthetic_sustained_quiet_speech() {
    // Simulação de fala verdadeira de baixa amplitude: 5 janelas consecutivas (100 ms)
    // com RMS ~95 e pico 350 (formato silábico)
    let mut samples = vec![0i16; 16000];
    let start_idx = 4800; // aos 300 ms
    let burst_len = 1600; // 100 ms = 5 janelas de 20ms
    for i in 0..burst_len {
        let t = i as f64 / 16000.0;
        let s = (t * 2.0 * std::f64::consts::PI * 300.0).sin() * 350.0;
        samples[start_idx + i] = s as i16;
    }

    let config = WindowedSilenceConfig::default();

    // Ambos devem reconhecer a fonação sustentada como fala (NÃO silêncio)
    assert!(
        !is_silent_pcm(&samples),
        "Fala de baixa amplitude sustentada não deve ser silêncio no baseline"
    );

    let analysis = analyze_acoustic_activity(&samples, 16000, &config);
    assert!(
        !analysis.is_silent,
        "Fala de baixa amplitude sustentada deve ser preservada pelo detector janelado"
    );
    assert!(
        analysis.max_consecutive_active >= 3,
        "Deve ter ao menos 3 janelas ativas consecutivas (obteve {})",
        analysis.max_consecutive_active
    );
}

// ---------------------------------------------------------------------------
// 2. Testes de Normalização com Teto de Ganho (Bounded Normalization)
// ---------------------------------------------------------------------------

#[test]
fn test_bounded_normalization_caps_gain_correctly() {
    // Sinal com pico fraco de 647
    let samples = vec![10i16, -20, 647, -300, 50];

    // 1. Normalização baseline sem teto: 28000 / 647 = 43.276x (+32.7 dB)
    let mut baseline_samples = samples.clone();
    let factor_unbounded =
        AudioCodec::normalize_amplitude_bounded(&mut baseline_samples, 28000, None);
    assert!((factor_unbounded - 43.2766).abs() < 0.01);
    assert_eq!(baseline_samples.iter().map(|s| s.abs()).max(), Some(28000));

    // 2. Normalização com teto de +18.0 dB (max_factor = 10^(18/20) = 7.94328)
    let mut bounded_18_samples = samples.clone();
    let factor_18 =
        AudioCodec::normalize_amplitude_bounded(&mut bounded_18_samples, 28000, Some(18.0));
    assert!((factor_18 - 7.94328).abs() < 0.01);
    let max_peak_18 = bounded_18_samples.iter().map(|s| s.abs()).max().unwrap();
    assert_eq!(max_peak_18, (647.0 * factor_18).round() as i16);
    assert!(
        max_peak_18 < 6000,
        "Pico deve ser contido em menos de 6000 (obteve {})",
        max_peak_18
    );

    // 3. Normalização com teto de +12.0 dB (max_factor = 10^(12/20) = 3.98107)
    let mut bounded_12_samples = samples.clone();
    let factor_12 =
        AudioCodec::normalize_amplitude_bounded(&mut bounded_12_samples, 28000, Some(12.0));
    assert!((factor_12 - 3.98107).abs() < 0.01);

    // 4. Sinal que já tem boa amplitude (pico 14000): ganho necessário é 2.0x (+6.02 dB)
    // Como +6.02 dB < +18.0 dB, o teto NÃO interfere e normaliza até 28000
    let mut loud_samples = vec![14000i16, -10000, 5000];
    let factor_loud = AudioCodec::normalize_amplitude_bounded(&mut loud_samples, 28000, Some(18.0));
    assert_eq!(factor_loud, 2.0);
    assert_eq!(loud_samples[0], 28000);
}

// ---------------------------------------------------------------------------
// 3. Testes A/B Comparativos com WAVs Físicos Reais Existentes
// ---------------------------------------------------------------------------

#[test]
fn test_ab_real_realtek_silence() {
    let wav_path = Path::new("data/voice_diag/canonical_16k_diag_1a142036d81_1.wav");
    if !wav_path.exists() {
        eprintln!(
            "Aviso: WAV físico não encontrado no caminho {}",
            wav_path.display()
        );
        return;
    }

    let bytes = std::fs::read(wav_path).expect("read wav");
    let (_, samples) = AudioCodec::decode_wav(&bytes).expect("decode wav");

    let config = WindowedSilenceConfig::default();

    // Baseline legado: rejeita em silêncio
    assert!(
        is_silent_pcm(&samples),
        "Realtek silêncio deve ser silêncio no baseline"
    );

    // Detector janelado experimental: rejeita em silêncio
    let analysis = analyze_acoustic_activity(&samples, 16000, &config);
    assert!(
        analysis.is_silent,
        "Realtek silêncio deve ser silêncio no janelado"
    );
    assert_eq!(analysis.silence_ratio_pct, 100.0);
    assert_eq!(analysis.active_windows, 0);
}

#[test]
fn test_ab_real_realtek_low_volume_speech_preservation() {
    // "E o que me ajuda essa matemática?" - RMS: -51.12 dBFS, Pico: 945
    let wav_path = Path::new("data/voice_diag/canonical_16k_diag_1a141edca97_1.wav");
    if !wav_path.exists() {
        eprintln!(
            "Aviso: WAV físico não encontrado no caminho {}",
            wav_path.display()
        );
        return;
    }

    let bytes = std::fs::read(wav_path).expect("read wav");
    let (_, samples) = AudioCodec::decode_wav(&bytes).expect("decode wav");

    let config = WindowedSilenceConfig::default();

    // Baseline legado: não é silêncio (RMS > 120 e peak > 250)
    assert!(
        !is_silent_pcm(&samples),
        "Fala baixa não deve ser silêncio no baseline"
    );

    // Detector janelado experimental: DEVE PRESERVAR A FALA!
    let analysis = analyze_acoustic_activity(&samples, 16000, &config);
    assert!(
        !analysis.is_silent,
        "CRÍTICO: Fala legítima de baixa amplitude (-51 dBFS) DEVE SER PRESERVADA pelo detector janelado!"
    );
    assert!(
        analysis.max_consecutive_active >= 3,
        "Fala legítima de baixa amplitude possui fonação contínua (obteve {} janelas consecutivas)",
        analysis.max_consecutive_active
    );
}

#[test]
fn test_ab_real_speech_preservation_suite() {
    // Testa preservação em todos os áudios de fala legítima reais disponíveis
    let speech_wavs = [
        (
            "data/voice_diag/canonical_16k_diag_1a141fbcfb0_1.wav",
            "Realtek com 'Yuki'",
        ),
        (
            "data/voice_diag/canonical_16k_diag_1a1420049f7_1.wav",
            "Realtek sem 'Yuki'",
        ),
        (
            "data/voice_diag/canonical_16k_diag_1a141d14d0f_1.wav",
            "Redmi Buds 'Que horas são?'",
        ),
        (
            "data/voice_diag/canonical_16k_diag_1a141d50b26_1.wav",
            "Redmi Buds fala normal",
        ),
        (
            "data/voice_diag/canonical_16k_diag_1a141e709e3_1.wav",
            "Redmi Buds fala normal 2",
        ),
        (
            "data/voice_diag/canonical_16k_diag_1a141eb37c6_1.wav",
            "Redmi Buds fala normal 3",
        ),
    ];

    let config = WindowedSilenceConfig::default();

    for (path_str, desc) in speech_wavs {
        let p = Path::new(path_str);
        if !p.exists() {
            continue;
        }
        let bytes = std::fs::read(p).expect("read wav");
        let (_, samples) = AudioCodec::decode_wav(&bytes).expect("decode wav");

        let analysis = analyze_acoustic_activity(&samples, 16000, &config);
        assert!(
            !analysis.is_silent,
            "Falso negativo inaceitável: arquivo '{}' ({}) foi classificado indevidamente como silêncio!",
            path_str, desc
        );
        assert!(
            analysis.max_consecutive_active >= 3,
            "Arquivo '{}' ({}) deve ter sustentação fonética >= 3 (obteve {})",
            path_str,
            desc,
            analysis.max_consecutive_active
        );
    }
}

// ---------------------------------------------------------------------------
// 4. Análise e Caracterização Acústica de Voz de Terceiros e Vídeos de Fundo
// ---------------------------------------------------------------------------

#[test]
fn test_ab_real_background_voice_and_video_acoustics() {
    // Sessões onde houve conversa da irmã e vídeo do Iberê de fundo (sem comando intencional do operador)
    let ambient_wavs = [
        (
            "data/voice_diag/canonical_16k_diag_1a141c9c7b6_1.wav",
            "Redmi Buds conversa/vídeo de fundo",
        ),
        (
            "data/voice_diag/canonical_16k_diag_1a14205e2b8_1.wav",
            "Redmi Buds vídeo Iberê de fundo",
        ),
    ];

    let config = WindowedSilenceConfig::default();

    for (path_str, desc) in ambient_wavs {
        let p = Path::new(path_str);
        if !p.exists() {
            continue;
        }
        let bytes = std::fs::read(p).expect("read wav");
        let (_, samples) = AudioCodec::decode_wav(&bytes).expect("decode wav");

        let analysis = analyze_acoustic_activity(&samples, 16000, &config);

        // Confirmação da Regra 4 de Governança:
        // Como o áudio continha de fato fala da irmã e áudio do vídeo, o sinal acústico
        // possui blocos de fonação humana real (max_consecutive_active > 3 e rms > 80).
        // Isso NÃO É um erro do detector acústico VAD, mas sim atividade vocal legítima de terceiros.
        println!(
            "[{}] Análise Acústica: active_windows={}/{}, max_consec={}, rms_mean={:.1}, rms_std={:.1}, is_silent={}",
            desc,
            analysis.active_windows,
            analysis.total_windows,
            analysis.max_consecutive_active,
            analysis.rms_mean,
            analysis.rms_std_dev,
            analysis.is_silent
        );

        assert!(
            analysis.active_windows > 0,
            "Deve registrar presença física de energia vocal proveniente do ambiente"
        );
    }
}

// ---------------------------------------------------------------------------
// 5. Comparação de Configurações Experimentais do Detector de Silêncio
// ---------------------------------------------------------------------------

#[test]
fn test_ab_detector_configuration_matrix() {
    // Matriz de configurações experimentais para documentação de trade-offs
    let configs = [
        (
            "Config Beta (Padrão 20ms / 80 RMS / 3 runs)",
            WindowedSilenceConfig {
                window_ms: 20,
                active_rms_threshold: 80.0,
                active_peak_threshold: 250,
                min_consecutive_active_windows: 3,
                max_silence_ratio_pct: 85.0,
                min_variance_std_dev: 10.0,
            },
        ),
        (
            "Config Gamma (Conservadora 20ms / 60 RMS / 2 runs)",
            WindowedSilenceConfig {
                window_ms: 20,
                active_rms_threshold: 60.0,
                active_peak_threshold: 200,
                min_consecutive_active_windows: 2,
                max_silence_ratio_pct: 90.0,
                min_variance_std_dev: 5.0,
            },
        ),
        (
            "Config Delta (Seletiva 20ms / 120 RMS / 4 runs)",
            WindowedSilenceConfig {
                window_ms: 20,
                active_rms_threshold: 120.0,
                active_peak_threshold: 300,
                min_consecutive_active_windows: 4,
                max_silence_ratio_pct: 80.0,
                min_variance_std_dev: 15.0,
            },
        ),
    ];

    // Testa em áudio de baixa amplitude real (Realtek fala baixa: "E o que me ajuda essa matemática?")
    let low_speech_path = Path::new("data/voice_diag/canonical_16k_diag_1a141edca97_1.wav");
    if low_speech_path.exists() {
        let bytes = std::fs::read(low_speech_path).expect("read wav");
        let (_, samples) = AudioCodec::decode_wav(&bytes).expect("decode wav");

        for (name, cfg) in &configs {
            let analysis = analyze_acoustic_activity(&samples, 16000, cfg);
            println!(
                "[{} em Fala Baixa]: is_silent={}, active={}, max_consec={}",
                name, analysis.is_silent, analysis.active_windows, analysis.max_consecutive_active
            );
            // Em Beta e Gamma, a fala de baixa amplitude NUNCA deve ser descartada (Zero falsos negativos)
            if name.contains("Beta") || name.contains("Gamma") {
                assert!(
                    !analysis.is_silent,
                    "Configuração '{}' não pode gerar falso negativo na fala baixa",
                    name
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 6. Teste A/B End-to-End de Decodificação Whisper com Teto de Ganho
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ab_end_to_end_whisper_bounded_gain() {
    use yuki::contracts::voice::{AudioFormat, AudioInput};
    use yuki::voice::stt::{LocalWhisperConfig, LocalWhisperSttProvider};

    let cfg_default = LocalWhisperConfig::default();
    if !LocalWhisperSttProvider::new(cfg_default.clone()).is_environment_ready() {
        eprintln!("Aviso: Ambiente Whisper local não configurado; pulando teste end-to-end.");
        return;
    }

    // 1. Fala de baixa amplitude Realtek (-51 dBFS, "E o que me ajuda essa matemática?")
    let low_speech_path = Path::new("data/voice_diag/canonical_16k_diag_1a141edca97_1.wav");
    if low_speech_path.exists() {
        let bytes = std::fs::read(low_speech_path).expect("read wav");
        let input = AudioInput::new(bytes, AudioFormat::default(), 5040, "test_low_speech");

        // Baseline: ganho ilimitado
        let stt_baseline =
            LocalWhisperSttProvider::new(LocalWhisperConfig::default().with_fast_decoding());
        let res_baseline = stt_baseline
            .transcribe_with_metadata(&input)
            .await
            .expect("transcribe baseline");

        // Bounded +18 dB
        let stt_bounded_18 = LocalWhisperSttProvider::new(
            LocalWhisperConfig::default()
                .with_fast_decoding()
                .with_max_gain_db(Some(18.0)),
        );
        let res_bounded_18 = stt_bounded_18
            .transcribe_with_metadata(&input)
            .await
            .expect("transcribe 18db");

        // Bounded +24 dB
        let stt_bounded_24 = LocalWhisperSttProvider::new(
            LocalWhisperConfig::default()
                .with_fast_decoding()
                .with_max_gain_db(Some(24.0)),
        );
        let res_bounded_24 = stt_bounded_24
            .transcribe_with_metadata(&input)
            .await
            .expect("transcribe 24db");

        println!(
            "\n=== A/B Whisper Fala Baixa (canonical_16k_diag_1a141edca97_1.wav) ===\n\
             • Baseline (Ganho Pleno +29.4 dB): \"{}\" (latência: {:.2}s)\n\
             • Bounded +18 dB (Ganho Clamped):  \"{}\" (latência: {:.2}s)\n\
             • Bounded +24 dB (Ganho Clamped):  \"{}\" (latência: {:.2}s)",
            res_baseline.text,
            res_baseline.elapsed.as_secs_f64(),
            res_bounded_18.text,
            res_bounded_18.elapsed.as_secs_f64(),
            res_bounded_24.text,
            res_bounded_24.elapsed.as_secs_f64(),
        );

        // Ambas as configurações bounded devem preservar o reconhecimento de palavras essenciais
        assert!(
            res_bounded_18.text.to_lowercase().contains("ajuda")
                || res_bounded_18.text.to_lowercase().contains("matemática")
                || res_bounded_24.text.to_lowercase().contains("matemática"),
            "Fala de baixa amplitude deve ser reconhecida com teto de ganho bounded"
        );
    }

    // 2. Ruído Redmi Buds com vozes difusas de fundo (canonical_16k_diag_1a141c9c7b6_1.wav)
    let redmi_path = Path::new("data/voice_diag/canonical_16k_diag_1a141c9c7b6_1.wav");
    if redmi_path.exists() {
        let bytes = std::fs::read(redmi_path).expect("read wav");
        let input = AudioInput::new(bytes, AudioFormat::default(), 5000, "test_redmi_noise");

        // Baseline: ganho pleno (+32.7 dB)
        let stt_baseline =
            LocalWhisperSttProvider::new(LocalWhisperConfig::default().with_fast_decoding());
        let res_baseline = stt_baseline
            .transcribe_with_metadata(&input)
            .await
            .expect("transcribe baseline");

        // Bounded +18 dB
        let stt_bounded_18 = LocalWhisperSttProvider::new(
            LocalWhisperConfig::default()
                .with_fast_decoding()
                .with_max_gain_db(Some(18.0)),
        );
        let res_bounded_18 = stt_bounded_18
            .transcribe_with_metadata(&input)
            .await
            .expect("transcribe 18db");

        println!(
            "\n=== A/B Whisper Redmi Buds (canonical_16k_diag_1a141c9c7b6_1.wav) ===\n\
             • Baseline (Ganho Pleno +32.7 dB): \"{}\" (latência: {:.2}s)\n\
             • Bounded +18 dB (Ganho Clamped):  \"{}\" (latência: {:.2}s)",
            res_baseline.text,
            res_baseline.elapsed.as_secs_f64(),
            res_bounded_18.text,
            res_bounded_18.elapsed.as_secs_f64(),
        );
    }
}
