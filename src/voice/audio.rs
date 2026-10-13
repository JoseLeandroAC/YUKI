use crate::contracts::voice::{AudioFormat, VoiceError};
pub use crate::voice::resample::{AudioResampler, ResamplerKind};

/// Utilitários de codificação, decodificação, validação e processamento de áudio PCM/WAV.
pub struct AudioCodec;

impl AudioCodec {
    /// Decodifica um payload WAV PCM (16-bit) e retorna o formato e as amostras i16.
    pub fn decode_wav(bytes: &[u8]) -> Result<(AudioFormat, Vec<i16>), VoiceError> {
        if bytes.len() < 44 {
            return Err(VoiceError::InvalidAudioFormat(
                "Payload menor que o cabeçalho mínimo WAV de 44 bytes".to_string(),
            ));
        }

        // Valida assinaturas RIFF e WAVE
        if &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
            return Err(VoiceError::InvalidAudioFormat(
                "Assinatura RIFF/WAVE ausente ou inválida".to_string(),
            ));
        }

        let mut offset = 12;
        let mut fmt_opt: Option<AudioFormat> = None;
        let mut data_samples: Option<Vec<i16>> = None;

        while offset + 8 <= bytes.len() {
            let chunk_id = &bytes[offset..offset + 4];
            let chunk_size = u32::from_le_bytes([
                bytes[offset + 4],
                bytes[offset + 5],
                bytes[offset + 6],
                bytes[offset + 7],
            ]) as usize;
            offset += 8;

            if chunk_id == b"fmt " {
                if chunk_size < 16 || offset + chunk_size > bytes.len() {
                    return Err(VoiceError::InvalidAudioFormat(
                        "Chunk 'fmt ' truncado".to_string(),
                    ));
                }
                let audio_format = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
                if audio_format != 1 {
                    // 1 = PCM linear não comprimido
                    return Err(VoiceError::InvalidAudioFormat(format!(
                        "Formato de compressão não suportado: {} (esperado PCM linear = 1)",
                        audio_format
                    )));
                }

                let channels = u16::from_le_bytes([bytes[offset + 2], bytes[offset + 3]]);
                let sample_rate = u32::from_le_bytes([
                    bytes[offset + 4],
                    bytes[offset + 5],
                    bytes[offset + 6],
                    bytes[offset + 7],
                ]);
                let bits_per_sample = u16::from_le_bytes([bytes[offset + 14], bytes[offset + 15]]);

                if bits_per_sample != 16 {
                    return Err(VoiceError::InvalidAudioFormat(format!(
                        "Bits por amostra não suportados: {} (esperado 16-bit)",
                        bits_per_sample
                    )));
                }

                fmt_opt = Some(AudioFormat {
                    sample_rate,
                    channels,
                    bits_per_sample,
                });
                offset += chunk_size;
            } else if chunk_id == b"data" {
                if offset + chunk_size > bytes.len() {
                    return Err(VoiceError::InvalidAudioFormat(
                        "Chunk 'data' truncado em relação ao tamanho declarado".to_string(),
                    ));
                }
                let num_samples = chunk_size / 2;
                let mut samples = Vec::with_capacity(num_samples);
                for i in 0..num_samples {
                    let s_offset = offset + (i * 2);
                    let sample = i16::from_le_bytes([bytes[s_offset], bytes[s_offset + 1]]);
                    samples.push(sample);
                }
                data_samples = Some(samples);
                offset += chunk_size;
            } else {
                // Pula chunks desconhecidos (ex: LIST, JUNK, etc.)
                offset += chunk_size;
            }
        }

        let format = fmt_opt.ok_or_else(|| {
            VoiceError::InvalidAudioFormat("Chunk 'fmt ' não encontrado no arquivo WAV".to_string())
        })?;

        let samples = data_samples.ok_or_else(|| {
            VoiceError::InvalidAudioFormat("Chunk 'data' não encontrado no arquivo WAV".to_string())
        })?;

        Ok((format, samples))
    }

    /// Decodifica qualquer arquivo WAV (PCM 16/24/32 bits ou IEEE Float 32 bits, mono ou estéreo)
    /// e retorna a taxa de amostragem nativa e as amostras downmixed para mono i16.
    pub fn decode_wav_to_mono_i16(bytes: &[u8]) -> Result<(u32, Vec<i16>), VoiceError> {
        if bytes.len() < 44 {
            return Err(VoiceError::InvalidAudioFormat(
                "Payload menor que o cabeçalho mínimo WAV de 44 bytes".to_string(),
            ));
        }

        if &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
            return Err(VoiceError::InvalidAudioFormat(
                "Assinatura RIFF/WAVE ausente ou inválida".to_string(),
            ));
        }

        let mut offset = 12;
        let mut sample_rate = 0u32;
        let mut channels = 0u16;
        let mut bits_per_sample = 0u16;
        let mut audio_format = 0u16;
        let mut raw_data: Option<&[u8]> = None;

        while offset + 8 <= bytes.len() {
            let chunk_id = &bytes[offset..offset + 4];
            let chunk_size = u32::from_le_bytes([
                bytes[offset + 4],
                bytes[offset + 5],
                bytes[offset + 6],
                bytes[offset + 7],
            ]) as usize;
            offset += 8;

            if chunk_id == b"fmt " {
                if chunk_size < 16 || offset + chunk_size > bytes.len() {
                    return Err(VoiceError::InvalidAudioFormat(
                        "Chunk 'fmt ' truncado".to_string(),
                    ));
                }
                audio_format = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
                channels = u16::from_le_bytes([bytes[offset + 2], bytes[offset + 3]]);
                sample_rate = u32::from_le_bytes([
                    bytes[offset + 4],
                    bytes[offset + 5],
                    bytes[offset + 6],
                    bytes[offset + 7],
                ]);
                bits_per_sample = u16::from_le_bytes([bytes[offset + 14], bytes[offset + 15]]);
                offset += chunk_size;
            } else if chunk_id == b"data" {
                if offset + chunk_size > bytes.len() {
                    return Err(VoiceError::InvalidAudioFormat(
                        "Chunk 'data' truncado".to_string(),
                    ));
                }
                raw_data = Some(&bytes[offset..offset + chunk_size]);
                offset += chunk_size;
            } else {
                offset += chunk_size;
            }
        }

        let data = raw_data
            .ok_or_else(|| VoiceError::InvalidAudioFormat("Chunk 'data' ausente".to_string()))?;

        if channels == 0 || sample_rate == 0 {
            return Err(VoiceError::InvalidAudioFormat(
                "Metadados de áudio inválidos no cabeçalho fmt".to_string(),
            ));
        }

        let ch = channels as usize;
        let mut mono_samples = Vec::new();

        if audio_format == 3 && bits_per_sample == 32 {
            // IEEE Float 32-bit
            let frame_size = ch * 4;
            for frame in data.chunks_exact(frame_size) {
                let mut sum = 0.0f32;
                for c in 0..ch {
                    let sb = [
                        frame[c * 4],
                        frame[c * 4 + 1],
                        frame[c * 4 + 2],
                        frame[c * 4 + 3],
                    ];
                    sum += f32::from_le_bytes(sb);
                }
                let avg = sum / (ch as f32);
                mono_samples.push((avg.clamp(-1.0, 1.0) * 32767.0).round() as i16);
            }
        } else if audio_format == 1 && bits_per_sample == 16 {
            // PCM 16-bit
            let frame_size = ch * 2;
            for frame in data.chunks_exact(frame_size) {
                let mut sum = 0i32;
                for c in 0..ch {
                    let sb = [frame[c * 2], frame[c * 2 + 1]];
                    sum += i16::from_le_bytes(sb) as i32;
                }
                let avg = sum / (ch as i32);
                mono_samples.push(avg.clamp(-32768, 32767) as i16);
            }
        } else if audio_format == 1 && bits_per_sample == 24 {
            // PCM 24-bit
            let frame_size = ch * 3;
            for frame in data.chunks_exact(frame_size) {
                let mut sum = 0i32;
                for c in 0..ch {
                    let b = &frame[c * 3..c * 3 + 3];
                    let sign_ext = if b[2] & 0x80 != 0 { 0xFF } else { 0x00 };
                    let val = i32::from_le_bytes([b[0], b[1], b[2], sign_ext]) >> 8;
                    sum += val;
                }
                let avg = sum / (ch as i32);
                mono_samples.push(avg.clamp(-32768, 32767) as i16);
            }
        } else if audio_format == 1 && bits_per_sample == 32 {
            // PCM 32-bit
            let frame_size = ch * 4;
            for frame in data.chunks_exact(frame_size) {
                let mut sum = 0i64;
                for c in 0..ch {
                    let sb = [
                        frame[c * 4],
                        frame[c * 4 + 1],
                        frame[c * 4 + 2],
                        frame[c * 4 + 3],
                    ];
                    sum += (i32::from_le_bytes(sb) >> 16) as i64;
                }
                let avg = sum / (ch as i64);
                mono_samples.push(avg.clamp(-32768, 32767) as i16);
            }
        } else {
            return Err(VoiceError::InvalidAudioFormat(format!(
                "Combinação não suportada de formato ({}) e profundidade de bits ({})",
                audio_format, bits_per_sample
            )));
        }

        Ok((sample_rate, mono_samples))
    }

    /// Codifica uma sequência de amostras i16 em um contêiner WAV PCM canônico de 44 bytes.
    pub fn encode_wav(samples: &[i16], format: &AudioFormat) -> Result<Vec<u8>, VoiceError> {
        if format.channels == 0 || format.sample_rate == 0 || format.bits_per_sample != 16 {
            return Err(VoiceError::InvalidAudioFormat(format!(
                "Parâmetros de formato inválidos para WAV: {:?}",
                format
            )));
        }

        let data_size = (samples.len() * 2) as u32;
        let file_size = 36 + data_size;
        let byte_rate = format.sample_rate * (format.channels as u32) * 2;
        let block_align = format.channels * 2;

        let mut buffer = Vec::with_capacity(44 + (samples.len() * 2));

        // Header RIFF
        buffer.extend_from_slice(b"RIFF");
        buffer.extend_from_slice(&file_size.to_le_bytes());
        buffer.extend_from_slice(b"WAVE");

        // Chunk fmt
        buffer.extend_from_slice(b"fmt ");
        buffer.extend_from_slice(&16u32.to_le_bytes()); // Tamanho do chunk fmt
        buffer.extend_from_slice(&1u16.to_le_bytes()); // Formato PCM = 1
        buffer.extend_from_slice(&format.channels.to_le_bytes());
        buffer.extend_from_slice(&format.sample_rate.to_le_bytes());
        buffer.extend_from_slice(&byte_rate.to_le_bytes());
        buffer.extend_from_slice(&block_align.to_le_bytes());
        buffer.extend_from_slice(&16u16.to_le_bytes()); // 16 bits por amostra

        // Chunk data
        buffer.extend_from_slice(b"data");
        buffer.extend_from_slice(&data_size.to_le_bytes());
        for &sample in samples {
            buffer.extend_from_slice(&sample.to_le_bytes());
        }

        Ok(buffer)
    }

    /// Codifica um buffer de bytes brutos em um arquivo WAV RIFF no formato nativo do dispositivo
    /// (suporta PCM 16/24/32 bits e IEEE Float 32 bits).
    pub fn encode_raw_wav(
        raw_bytes: &[u8],
        channels: u16,
        sample_rate: u32,
        bits_per_sample: u16,
        is_float: bool,
    ) -> Result<Vec<u8>, VoiceError> {
        if channels == 0 || sample_rate == 0 || bits_per_sample == 0 {
            return Err(VoiceError::InvalidAudioFormat(
                "Parâmetros de formato nativo inválidos para codificação WAV".to_string(),
            ));
        }

        let bytes_per_sample = (bits_per_sample / 8) as u32;
        let block_align = channels * (bits_per_sample / 8);
        let byte_rate = sample_rate * (channels as u32) * bytes_per_sample;
        let data_size = raw_bytes.len() as u32;

        let fmt_chunk_size = if is_float { 18u32 } else { 16u32 };
        let file_size = 12 + (8 + fmt_chunk_size) + (8 + data_size) - 8;

        let mut buffer = Vec::with_capacity(file_size as usize + 8);

        // Header RIFF
        buffer.extend_from_slice(b"RIFF");
        buffer.extend_from_slice(&file_size.to_le_bytes());
        buffer.extend_from_slice(b"WAVE");

        // Chunk fmt
        buffer.extend_from_slice(b"fmt ");
        buffer.extend_from_slice(&fmt_chunk_size.to_le_bytes());

        let format_tag: u16 = if is_float {
            3 /* IEEE FLOAT */
        } else {
            1 /* PCM */
        };
        buffer.extend_from_slice(&format_tag.to_le_bytes());
        buffer.extend_from_slice(&channels.to_le_bytes());
        buffer.extend_from_slice(&sample_rate.to_le_bytes());
        buffer.extend_from_slice(&byte_rate.to_le_bytes());
        buffer.extend_from_slice(&block_align.to_le_bytes());
        buffer.extend_from_slice(&bits_per_sample.to_le_bytes());
        if is_float {
            buffer.extend_from_slice(&0u16.to_le_bytes()); // cbSize = 0
        }

        // Chunk data
        buffer.extend_from_slice(b"data");
        buffer.extend_from_slice(&data_size.to_le_bytes());
        buffer.extend_from_slice(raw_bytes);

        Ok(buffer)
    }

    /// Converte áudio estéreo (2 canais) para mono (1 canal) calculando a média das amostras.
    pub fn to_mono(samples: &[i16], channels: u16) -> Vec<i16> {
        if channels <= 1 {
            return samples.to_vec();
        }
        let ch = channels as usize;
        let mut mono = Vec::with_capacity(samples.len() / ch);
        for chunk in samples.chunks(ch) {
            let sum: i32 = chunk.iter().map(|&s| s as i32).sum();
            let avg = (sum / (chunk.len() as i32)) as i16;
            mono.push(avg);
        }
        mono
    }

    /// Normaliza amplitude das amostras para um pico desejado (ex: 28000 para margem de segurança contra clipping).
    pub fn normalize_amplitude(samples: &mut [i16], target_peak: i16) {
        if samples.is_empty() {
            return;
        }
        let current_peak = samples.iter().map(|&s| s.abs()).max().unwrap_or(0);
        if current_peak == 0 || current_peak == target_peak {
            return;
        }
        let factor = (target_peak as f64) / (current_peak as f64);
        for s in samples.iter_mut() {
            let val = (*s as f64 * factor).round();
            *s = val.clamp(-32768.0, 32767.0) as i16;
        }
    }

    /// Calcula a energia RMS das amostras.
    pub fn calculate_rms(samples: &[i16]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }
        let sum_sq: f64 = samples.iter().map(|&s| (s as f64) * (s as f64)).sum();
        (sum_sq / (samples.len() as f64)).sqrt() as f32
    }

    /// Conta quantas amostras atingiram os limites extremos de saturação (clipping).
    pub fn count_clipping(samples: &[i16], threshold: i16) -> usize {
        let pos_thresh = threshold.saturating_abs();
        let neg_thresh = -pos_thresh;
        samples
            .iter()
            .filter(|&&s| s >= pos_thresh || s <= neg_thresh)
            .count()
    }

    /// Valida restrições de tamanho em bytes e duração em milissegundos.
    pub fn validate_constraints(
        bytes_len: usize,
        samples_len: usize,
        format: &AudioFormat,
        max_bytes: usize,
        max_duration_ms: u64,
    ) -> Result<u64, VoiceError> {
        if bytes_len > max_bytes {
            return Err(VoiceError::AudioTooLarge {
                bytes: bytes_len,
                max_bytes,
            });
        }

        if format.sample_rate == 0 || format.channels == 0 {
            return Err(VoiceError::InvalidAudioFormat(
                "Taxa de amostragem ou canais zerados".to_string(),
            ));
        }

        let duration_ms =
            ((samples_len as u64) * 1000) / (format.sample_rate as u64 * format.channels as u64);

        if duration_ms > max_duration_ms {
            return Err(VoiceError::AudioTooLong {
                duration_ms,
                max_ms: max_duration_ms,
            });
        }

        Ok(duration_ms)
    }

    /// Reamostra uma sequência de amostras PCM i16 mono entre taxas de amostragem especificadas.
    pub fn resample(samples: &[i16], in_rate: u32, out_rate: u32, kind: ResamplerKind) -> Vec<i16> {
        AudioResampler::resample_oneshot(samples, in_rate, out_rate, kind)
    }
}
