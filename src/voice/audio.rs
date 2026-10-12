use crate::contracts::voice::{AudioFormat, VoiceError};

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
            VoiceError::InvalidAudioFormat(
                "Chunk 'data' não encontrado no arquivo WAV".to_string(),
            )
        })?;

        Ok((format, samples))
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
        samples.iter().filter(|&&s| s.abs() >= threshold).count()
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

        let duration_ms = ((samples_len as u64) * 1000)
            / (format.sample_rate as u64 * format.channels as u64);

        if duration_ms > max_duration_ms {
            return Err(VoiceError::AudioTooLong {
                duration_ms,
                max_ms: max_duration_ms,
            });
        }

        Ok(duration_ms)
    }
}
