use crate::contracts::identifiers::now_utc;
use crate::contracts::voice::{
    AcousticMetrics, AudioDiagnosticsReport, BufferMetrics, CaptureTimingMetrics,
    NativeAudioFormatInfo, SttDecodingConfigSummary, SttTimingMetrics, VoiceError,
};
use crate::voice::audio::AudioCodec;
use std::path::Path;

/// Diretório padrão não-versionado para persistência de artefatos diagnósticos de áudio físico.
pub const DIAGNOSTICS_DIR: &str = "data/voice_diag";

/// Verifica se o diagnóstico de áudio foi solicitado via variável de ambiente.
pub fn is_diagnostics_enabled() -> bool {
    std::env::var("YUKI_VOICE_DIAGNOSTICS")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("yes"))
        .unwrap_or(false)
}

/// Calcula métricas acústicas analíticas a partir de um vetor de amostras `i16`.
///
/// Medições incluídas:
/// - RMS (Root Mean Square) e equivalente dBFS (relativo a 32767).
/// - Pico máximo absoluto e equivalente dBFS.
/// - Contagem e percentual de amostras em clipping (saturação digital >= 32760).
/// - Estimativa de silêncio acústico analisada em janelas contíguas de 20ms.
pub fn compute_acoustic_metrics(samples: &[i16], sample_rate: u32) -> AcousticMetrics {
    if samples.is_empty() {
        return AcousticMetrics {
            sample_count: 0,
            rms: 0.0,
            rms_dbfs: -96.0,
            peak: 0,
            peak_dbfs: -96.0,
            clipping_samples: 0,
            clipping_ratio_pct: 0.0,
            silence_ratio_pct: 100.0,
        };
    }

    let mut sum_sq: f64 = 0.0;
    let mut peak: i16 = 0;
    let mut clipping: usize = 0;

    for &s in samples {
        let abs = s.saturating_abs();
        if abs > peak {
            peak = abs;
        }
        if abs >= 32760 {
            clipping += 1;
        }
        sum_sq += (s as f64) * (s as f64);
    }

    let n = samples.len() as f64;
    let rms = (sum_sq / n).sqrt() as f32;

    let rms_dbfs = if rms > 0.001 {
        20.0 * (rms / 32767.0).log10()
    } else {
        -96.0
    };

    let peak_dbfs = if peak > 0 {
        20.0 * (peak as f32 / 32767.0).log10()
    } else {
        -96.0
    };

    let clipping_ratio_pct = (clipping as f32 / samples.len() as f32) * 100.0;

    // Estimativa de silêncio analisando blocos temporais de 20ms
    let effective_rate = if sample_rate > 0 { sample_rate } else { 16000 };
    let frame_size = ((effective_rate as f64 * 0.02).round() as usize).max(1);
    let mut total_frames = 0;
    let mut silent_frames = 0;

    for chunk in samples.chunks(frame_size) {
        total_frames += 1;
        let mut f_sum_sq = 0.0f64;
        let mut f_peak = 0i16;
        for &s in chunk {
            let a = s.saturating_abs();
            if a > f_peak {
                f_peak = a;
            }
            f_sum_sq += (s as f64) * (s as f64);
        }
        let f_rms = (f_sum_sq / chunk.len() as f64).sqrt() as f32;
        if f_rms < 120.0 && f_peak < 250 {
            silent_frames += 1;
        }
    }

    let silence_ratio_pct = if total_frames > 0 {
        (silent_frames as f32 / total_frames as f32) * 100.0
    } else {
        0.0
    };

    AcousticMetrics {
        sample_count: samples.len(),
        rms,
        rms_dbfs,
        peak,
        peak_dbfs,
        clipping_samples: clipping,
        clipping_ratio_pct,
        silence_ratio_pct,
    }
}

/// Salva todos os artefatos de diagnóstico de áudio em `data/voice_diag/` de forma atômica e estruturada.
///
/// Salva:
/// 1. `raw_captured_{session_id}.wav`: áudio bruto nas características nativas do hardware.
/// 2. `canonical_16k_{session_id}.wav`: áudio mono 16kHz resampleado antes de qualquer normalização.
/// 3. `diag_{session_id}.json`: metadados completos de driver, buffers, timings e acústica.
#[allow(clippy::too_many_arguments)]
pub fn save_diagnostic_session(
    session_id: &str,
    device_id: Option<String>,
    device_name: Option<String>,
    native_format: &NativeAudioFormatInfo,
    timings: CaptureTimingMetrics,
    buffer_metrics: BufferMetrics,
    raw_bytes: &[u8],
    canonical_samples: &[i16],
) -> Result<AudioDiagnosticsReport, VoiceError> {
    let diag_dir = Path::new(DIAGNOSTICS_DIR);
    if let Err(e) = std::fs::create_dir_all(diag_dir) {
        return Err(VoiceError::Io(format!(
            "Falha ao criar diretório de diagnósticos de voz '{}': {}",
            diag_dir.display(),
            e
        )));
    }

    let timestamp = now_utc().to_rfc3339();

    // 1. Salva arquivo WAV bruto nativo
    let raw_wav_filename = format!("raw_captured_{}.wav", session_id);
    let raw_wav_path = diag_dir.join(&raw_wav_filename);
    let raw_wav_bytes = AudioCodec::encode_raw_wav(
        raw_bytes,
        native_format.channels,
        native_format.sample_rate,
        native_format.bits_per_sample,
        native_format.is_float,
    )?;
    std::fs::write(&raw_wav_path, &raw_wav_bytes).map_err(|e| {
        VoiceError::Io(format!(
            "Falha ao gravar áudio bruto de diagnóstico ('{}'): {}",
            raw_wav_path.display(),
            e
        ))
    })?;

    // 2. Salva arquivo WAV canônico (16kHz, mono, PCM 16-bit)
    let canonical_wav_filename = format!("canonical_16k_{}.wav", session_id);
    let canonical_wav_path = diag_dir.join(&canonical_wav_filename);
    let canonical_format = crate::contracts::voice::AudioFormat::default();
    let canonical_wav_bytes = AudioCodec::encode_wav(canonical_samples, &canonical_format)?;
    std::fs::write(&canonical_wav_path, &canonical_wav_bytes).map_err(|e| {
        VoiceError::Io(format!(
            "Falha ao gravar áudio canônico de diagnóstico ('{}'): {}",
            canonical_wav_path.display(),
            e
        ))
    })?;

    // 3. Medições acústicas do sinal canônico
    let acoustic_canonical = compute_acoustic_metrics(canonical_samples, 16000);

    // 4. Salva relatório em JSON
    let report_filename = format!("diag_{}.json", session_id);
    let report_path = diag_dir.join(&report_filename);

    let report = AudioDiagnosticsReport {
        session_id: session_id.to_string(),
        timestamp,
        device_id,
        device_name,
        native_format: native_format.clone(),
        timings,
        buffer_metrics,
        acoustic_canonical,
        acoustic_normalized: None,
        stt_timings: None,
        stt_config: None,
        raw_wav_path: Some(raw_wav_path.to_string_lossy().to_string()),
        canonical_wav_path: Some(canonical_wav_path.to_string_lossy().to_string()),
        normalized_wav_path: None,
        report_path: Some(report_path.to_string_lossy().to_string()),
    };

    let json_bytes = serde_json::to_vec_pretty(&report).map_err(|e| {
        VoiceError::Io(format!(
            "Falha ao serializar relatório JSON de diagnóstico: {}",
            e
        ))
    })?;
    std::fs::write(&report_path, &json_bytes).map_err(|e| {
        VoiceError::Io(format!(
            "Falha ao gravar arquivo JSON de diagnóstico ('{}'): {}",
            report_path.display(),
            e
        ))
    })?;

    Ok(report)
}

/// Atualiza o relatório de diagnóstico existente com a versão do áudio normalizada pelo STT.
pub fn update_diagnostic_with_normalized(
    report: &mut AudioDiagnosticsReport,
    normalized_samples: &[i16],
    normalized_wav_bytes: &[u8],
) -> Result<(), VoiceError> {
    let diag_dir = Path::new(DIAGNOSTICS_DIR);
    let norm_filename = format!("normalized_16k_{}.wav", report.session_id);
    let norm_path = diag_dir.join(&norm_filename);
    std::fs::write(&norm_path, normalized_wav_bytes).map_err(|e| {
        VoiceError::Io(format!(
            "Falha ao gravar áudio normalizado de diagnóstico ('{}'): {}",
            norm_path.display(),
            e
        ))
    })?;

    let norm_metrics = compute_acoustic_metrics(normalized_samples, 16000);
    report.acoustic_normalized = Some(norm_metrics);
    report.normalized_wav_path = Some(norm_path.to_string_lossy().to_string());

    if let Some(ref report_path_str) = report.report_path {
        let json_bytes = serde_json::to_vec_pretty(&report).map_err(|e| {
            VoiceError::Io(format!(
                "Falha ao serializar atualização JSON de diagnóstico: {}",
                e
            ))
        })?;
        let _ = std::fs::write(report_path_str, &json_bytes);
    }
    Ok(())
}

/// Atualiza o relatório de diagnóstico com métricas de temporização e parâmetros do STT.
pub fn update_diagnostic_with_stt(
    report: &mut AudioDiagnosticsReport,
    timings: &SttTimingMetrics,
    config: &SttDecodingConfigSummary,
) -> Result<(), VoiceError> {
    report.stt_timings = Some(timings.clone());
    report.stt_config = Some(config.clone());

    if let Some(ref report_path_str) = report.report_path {
        let json_bytes = serde_json::to_vec_pretty(&report).map_err(|e| {
            VoiceError::Io(format!(
                "Falha ao serializar atualização JSON de diagnóstico com STT: {}",
                e
            ))
        })?;
        let _ = std::fs::write(report_path_str, &json_bytes);
    }
    Ok(())
}

/// Exibe no console um sumário claro, técnico e visual do diagnóstico acústico.
pub fn print_diagnostic_summary(report: &AudioDiagnosticsReport) {
    println!("\n╔══════════════════════════════════════════════════════════════════════════════╗");
    println!("║              RELATÓRIO DIAGNÓSTICO DE ÁUDIO FÍSICO E CAPTURA                 ║");
    println!("╚══════════════════════════════════════════════════════════════════════════════╝");
    println!("• ID da Sessão:        {}", report.session_id);
    println!("• Timestamp UTC:       {}", report.timestamp);
    if let Some(ref dev_name) = report.device_name {
        println!("• Dispositivo:         {}", dev_name);
    }
    if let Some(ref dev_id) = report.device_id {
        println!("• Endpoint ID:         {}", dev_id);
    }

    println!("\n--- Formato Nativo do Driver (WASAPI) ---");
    let fmt = &report.native_format;
    println!(
        "• Tag / Codificação:   {} (hex: {}) — {}",
        fmt.format_tag, fmt.format_tag_hex, fmt.format_tag_name
    );
    println!(
        "• Canais:              {} | Taxa Nativa: {} Hz | Bits: {} bits ({})",
        fmt.channels,
        fmt.sample_rate,
        fmt.bits_per_sample,
        if fmt.is_float {
            "IEEE Float"
        } else {
            "PCM Inteiro"
        }
    );
    println!(
        "• Alinhamento / Taxa:  BlockAlign: {} bytes | Bytes/s: {}",
        fmt.block_align, fmt.avg_bytes_per_sec
    );
    if let Some(ref guid) = fmt.sub_format_guid {
        println!("• SubFormat GUID:      {}", guid);
    }

    println!("\n--- Métricas de Temporização e Ciclo de Vida ---");
    let t = &report.timings;
    println!(
        "• Duração Solicitada:  {:.2}s ({} ms)",
        t.requested_duration_ms as f64 / 1000.0,
        t.requested_duration_ms
    );
    println!(
        "• Setup COM + WASAPI:  {:.2}s ({} ms)",
        t.init_duration_ms as f64 / 1000.0,
        t.init_duration_ms
    );
    println!(
        "• Gravação Efetiva:    {:.2}s ({} ms)",
        t.effective_capture_ms as f64 / 1000.0,
        t.effective_capture_ms
    );
    println!(
        "• Amostras Geradas:    {:.2}s ({} ms)",
        t.samples_duration_ms as f64 / 1000.0,
        t.samples_duration_ms
    );

    println!("\n--- Métricas de Buffer e Pacotes WASAPI ---");
    let b = &report.buffer_metrics;
    println!(
        "• Total de Pacotes:    {} | Frames Lidos: {} | Bytes Lidos: {}",
        b.total_packets, b.total_raw_frames, b.total_raw_bytes
    );
    println!(
        "• Buffers Silenciosos: {} | Falhas/Glitches de Descontinuidade: {}",
        b.silent_buffer_events, b.discontinuity_glitches
    );

    println!("\n--- Métricas Acústicas do Sinal Canônico (16kHz Mono) ---");
    let ac = &report.acoustic_canonical;
    println!(
        "• Amostras / Energia:  {} amostras | RMS: {:.1} ({:.2} dBFS)",
        ac.sample_count, ac.rms, ac.rms_dbfs
    );
    println!(
        "• Pico / Clipping:     Pico: {} ({:.2} dBFS) | Clipping: {} ({:.2}%)",
        ac.peak, ac.peak_dbfs, ac.clipping_samples, ac.clipping_ratio_pct
    );
    println!(
        "• Silêncio Estimado:   {:.1}% do tempo",
        ac.silence_ratio_pct
    );

    if let Some(ref an) = report.acoustic_normalized {
        println!("\n--- Métricas Acústicas Pós-Normalização STT ---");
        println!(
            "• Amostras / Energia:  {} amostras | RMS: {:.1} ({:.2} dBFS)",
            an.sample_count, an.rms, an.rms_dbfs
        );
        println!(
            "• Pico / Clipping:     Pico: {} ({:.2} dBFS) | Clipping: {} ({:.2}%)",
            an.peak, an.peak_dbfs, an.clipping_samples, an.clipping_ratio_pct
        );
    }

    if let Some(ref cfg) = report.stt_config {
        println!("\n--- Configuração de Decodificação STT (Whisper) ---");
        println!("• Idioma Solicitado:   {}", cfg.language_requested);
        println!(
            "• Prompt Lexical:      {}",
            if cfg.prompt_configured {
                cfg.prompt_preview.as_deref().unwrap_or("Ativo")
            } else {
                "Desativado (Baseline neutro)"
            }
        );
        println!(
            "• Threads (-t):        {} | Beam Size (-bs): {} | Best-of (-bo): {}",
            cfg.threads
                .map(|t| t.to_string())
                .unwrap_or_else(|| "default (4)".into()),
            cfg.beam_size
                .map(|b| b.to_string())
                .unwrap_or_else(|| "default (5)".into()),
            cfg.best_of
                .map(|b| b.to_string())
                .unwrap_or_else(|| "default (5)".into())
        );
        println!(
            "• Fallback Temperatura:{}",
            if cfg.no_fallback {
                " --no-fallback (Sem retentativas de temperatura)"
            } else {
                " Padrão (Fallbacks automáticos 0.2, 0.4, 0.6 ativados)"
            }
        );
    }

    if let Some(ref st) = report.stt_timings {
        println!("\n--- Decomposição de Latência do STT (Wall-clock & Whisper) ---");
        println!("• Prep / Mono Downmix: {:.2} ms", st.audio_prep_ms);
        println!("• Normalização & Enc:  {:.2} ms", st.audio_norm_ms);
        println!("• Gravação Temp WAV:   {:.2} ms", st.temp_write_ms);
        println!(
            "• Subprocesso Whisper: {:.2} ms ({:.2}s)",
            st.subprocess_wall_ms,
            st.subprocess_wall_ms / 1000.0
        );
        if let Some(load) = st.whisper_load_ms {
            println!("  ├─ Modelo Load:      {:.2} ms", load);
        }
        if let Some(mel) = st.whisper_mel_ms {
            println!("  ├─ Mel Spectrogram:  {:.2} ms", mel);
        }
        if let Some(sample) = st.whisper_sample_ms {
            println!("  ├─ Sampling:         {:.2} ms", sample);
        }
        if let Some(enc) = st.whisper_encode_ms {
            println!("  ├─ Encode:           {:.2} ms", enc);
        }
        if let Some(dec) = st.whisper_decode_ms {
            println!("  ├─ Decode:           {:.2} ms", dec);
        }
        if let Some(prompt) = st.whisper_prompt_ms {
            println!("  ├─ Prompt Time:      {:.2} ms", prompt);
        }
        if let Some(tot) = st.whisper_total_ms {
            println!(
                "  ├─ Whisper Total:    {:.2} ms ({:.2}s)",
                tot,
                tot / 1000.0
            );
        }
        if let Some(fb) = st.whisper_fallbacks {
            println!("  └─ Fallbacks Usados: {}", fb);
        }
        println!("• Pós-processamento:   {:.2} ms", st.post_process_ms);
        println!(
            "• Latência Total STT:  {:.2} ms ({:.2}s)",
            st.total_stt_wall_ms,
            st.total_stt_wall_ms / 1000.0
        );
    }

    println!("\n--- Artefatos Diagnósticos Gravados em Disco ---");
    if let Some(ref p) = report.raw_wav_path {
        println!("• Áudio Bruto Nativo:  {}", p);
    }
    if let Some(ref p) = report.canonical_wav_path {
        println!("• Áudio Canônico 16k:  {}", p);
    }
    if let Some(ref p) = report.normalized_wav_path {
        println!("• Áudio Normalizado:   {}", p);
    }
    if let Some(ref p) = report.report_path {
        println!("• Metadados JSON:      {}", p);
    }
    println!("════════════════════════════════════════════════════════════════════════════════\n");
}
