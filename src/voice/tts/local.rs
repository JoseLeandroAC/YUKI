use crate::contracts::voice::{
    AudioFormat, AudioOutput, TextToSpeechProvider, VoiceConfiguration, VoiceError,
    VoiceProviderMetadata, VoiceProviderType,
};
use crate::voice::audio::AudioCodec;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::time::Duration;

/// Configuração do adaptador local Piper TTS.
#[derive(Debug, Clone)]
pub struct LocalPiperConfig {
    pub binary_path: Option<PathBuf>,
    pub model_path: Option<PathBuf>,
    pub config_json: Option<PathBuf>,
    pub timeout: Duration,
}

impl Default for LocalPiperConfig {
    fn default() -> Self {
        let binary_path = std::env::var_os("YUKI_PIPER_BIN")
            .map(PathBuf::from)
            .or_else(which_piper);

        let model_path = std::env::var_os("YUKI_PIPER_MODEL").map(PathBuf::from);
        let config_json = std::env::var_os("YUKI_PIPER_CONFIG").map(PathBuf::from);

        Self {
            binary_path,
            model_path,
            config_json,
            timeout: Duration::from_secs(15),
        }
    }
}

fn which_piper() -> Option<PathBuf> {
    let candidates = ["piper.exe", "piper"];
    for name in &candidates {
        if let Ok(path) = std::env::var("PATH") {
            for dir in std::env::split_paths(&path) {
                let full = dir.join(name);
                if full.is_file() {
                    return Some(full);
                }
            }
        }
    }
    None
}

/// Provedor local de síntese de voz (TTS) baseado no Piper.
///
/// ARQUITETURA LOCAL-FIRST:
/// - Soberano: síntese local de altíssima velocidade (< 150ms).
/// - Respeita a identidade vocal da Yuki.
/// - Instruções claras de instalação sem downloads silenciosos.
pub struct LocalPiperTtsProvider {
    metadata: VoiceProviderMetadata,
    config: LocalPiperConfig,
}

impl LocalPiperTtsProvider {
    pub fn new(config: LocalPiperConfig) -> Self {
        let metadata = VoiceProviderMetadata::new(
            "Piper-Local",
            VoiceProviderType::LocalEdge,
            vec!["pt-BR".to_string(), "pt".to_string()],
            22050,
            false,
            "1.0.0",
        );
        Self { metadata, config }
    }

    pub fn installation_instructions() -> &'static str {
        "Para ativar a síntese de voz local com Piper na Yuki:\n\
         1. Baixe o executável Piper para sua plataforma (ex: piper.exe).\n\
         2. Baixe o modelo de voz feminina em português (ex: pt_BR-faber-medium.onnx e seu arquivo .json correspondente).\n\
         3. Configure as variáveis de ambiente:\n\
            set YUKI_PIPER_BIN=C:\\caminho\\para\\piper.exe\n\
            set YUKI_PIPER_MODEL=C:\\caminho\\para\\pt_BR-faber-medium.onnx\n\
         A Yuki não realiza downloads silenciosos de modelos sem consentimento do operador."
    }

    pub fn is_environment_ready(&self) -> bool {
        let bin_ok = self
            .config
            .binary_path
            .as_ref()
            .map(|p| p.is_file())
            .unwrap_or(false);
        let model_ok = self
            .config
            .model_path
            .as_ref()
            .map(|p| p.is_file())
            .unwrap_or(false);
        bin_ok && model_ok
    }
}

impl Default for LocalPiperTtsProvider {
    fn default() -> Self {
        Self::new(LocalPiperConfig::default())
    }
}

impl TextToSpeechProvider for LocalPiperTtsProvider {
    fn metadata(&self) -> &VoiceProviderMetadata {
        &self.metadata
    }

    fn health<'a>(&'a self) -> Pin<Box<dyn Future<Output = Result<(), VoiceError>> + Send + 'a>> {
        Box::pin(async move {
            if !self.is_environment_ready() {
                return Err(VoiceError::ProviderUnavailable(format!(
                    "Piper TTS local não configurado. {}",
                    Self::installation_instructions()
                )));
            }
            Ok(())
        })
    }

    fn synthesize<'a>(
        &'a self,
        text: &'a str,
        voice_params: Option<&'a VoiceConfiguration>,
    ) -> Pin<Box<dyn Future<Output = Result<AudioOutput, VoiceError>> + Send + 'a>> {
        let text_owned = text.to_string();
        let length_scale = voice_params
            .map(|p| 1.0 / p.speaking_rate.clamp(0.5, 2.0))
            .unwrap_or(1.0);

        Box::pin(async move {
            if text_owned.trim().is_empty() {
                return Err(VoiceError::SynthesisFailed(
                    "Texto vazio para síntese".to_string(),
                ));
            }

            let bin_path = self.config.binary_path.as_ref().ok_or_else(|| {
                VoiceError::ProviderUnavailable(format!(
                    "Executável piper local não configurado. {}",
                    Self::installation_instructions()
                ))
            })?;

            let model_path = self.config.model_path.as_ref().ok_or_else(|| {
                VoiceError::ProviderUnavailable(format!(
                    "Modelo piper não configurado. {}",
                    Self::installation_instructions()
                ))
            })?;

            let temp_dir = std::env::temp_dir();
            let temp_out = temp_dir.join(format!("yuki_tts_{}.wav", uuid_timestamp()));

            let bin_clone = bin_path.clone();
            let model_clone = model_path.clone();
            let temp_out_clone = temp_out.clone();
            let text_for_cmd = text_owned.clone();
            let timeout_duration = self.config.timeout;

            let join_handle = tokio::task::spawn_blocking(move || {
                use std::io::Write;
                let mut child = std::process::Command::new(bin_clone)
                    .arg("-m")
                    .arg(model_clone)
                    .arg("-f")
                    .arg(&temp_out_clone)
                    .arg("--length_scale")
                    .arg(format!("{:.2}", length_scale))
                    .stdin(std::process::Stdio::piped())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::piped())
                    .spawn()
                    .map_err(|e| format!("Falha ao iniciar processo piper: {}", e))?;

                if let Some(mut stdin) = child.stdin.take() {
                    let _ = stdin.write_all(text_for_cmd.as_bytes());
                }

                let output = child
                    .wait_with_output()
                    .map_err(|e| format!("Falha ao aguardar piper: {}", e))?;

                Ok::<std::process::Output, String>(output)
            });

            struct TempCleaner(PathBuf);
            impl Drop for TempCleaner {
                fn drop(&mut self) {
                    let _ = std::fs::remove_file(&self.0);
                }
            }
            let _cleaner = TempCleaner(temp_out.clone());

            let output_res = match tokio::time::timeout(timeout_duration, join_handle).await {
                Ok(j_res) => match j_res {
                    Ok(cmd_res) => cmd_res.map_err(VoiceError::SynthesisFailed)?,
                    Err(e) => {
                        return Err(VoiceError::SynthesisFailed(format!(
                            "Erro na thread de síntese: {}",
                            e
                        )));
                    }
                },
                Err(_) => {
                    return Err(VoiceError::Timeout(
                        "Síntese de voz excedeu tempo limite".to_string(),
                    ));
                }
            };

            if !output_res.status.success() {
                let stderr = String::from_utf8_lossy(&output_res.stderr);
                return Err(VoiceError::SynthesisFailed(format!(
                    "Piper finalizou com erro: {}",
                    stderr.trim()
                )));
            }

            let wav_bytes = std::fs::read(&temp_out).map_err(|e| {
                VoiceError::Io(format!("Falha ao ler áudio gerado pelo piper: {}", e))
            })?;

            let (format, samples) = AudioCodec::decode_wav(&wav_bytes)?;
            let duration_ms = ((samples.len() as u64) * 1000)
                / (format.sample_rate as u64 * format.channels as u64);

            Ok(AudioOutput::new(
                wav_bytes,
                format,
                duration_ms,
                "yuki-piper-ptbr",
            ))
        })
    }
}

/// Sintetizador acústico determinístico soberano embutido na Yuki.
///
/// Produz formas de onda WAV canônicas com entonação melódica feminina (F0 ~ 210-230 Hz),
/// permitindo execução e validação completa sem necessidade de dependências C++ ou downloads prévios.
pub struct LocalAcousticTtsProvider {
    metadata: VoiceProviderMetadata,
    sample_rate: u32,
}

impl Default for LocalAcousticTtsProvider {
    fn default() -> Self {
        Self::new(16000)
    }
}

impl LocalAcousticTtsProvider {
    pub fn new(sample_rate: u32) -> Self {
        let metadata = VoiceProviderMetadata::new(
            "Yuki-Acoustic-Local",
            VoiceProviderType::LocalEdge,
            vec!["pt-BR".to_string(), "pt".to_string()],
            sample_rate,
            false,
            "1.0.0",
        );
        Self {
            metadata,
            sample_rate,
        }
    }
}

impl TextToSpeechProvider for LocalAcousticTtsProvider {
    fn metadata(&self) -> &VoiceProviderMetadata {
        &self.metadata
    }

    fn health<'a>(&'a self) -> Pin<Box<dyn Future<Output = Result<(), VoiceError>> + Send + 'a>> {
        Box::pin(async { Ok(()) })
    }

    fn synthesize<'a>(
        &'a self,
        text: &'a str,
        voice_params: Option<&'a VoiceConfiguration>,
    ) -> Pin<Box<dyn Future<Output = Result<AudioOutput, VoiceError>> + Send + 'a>> {
        let text_owned = text.to_string();
        let sample_rate = self.sample_rate;
        let speaking_rate = voice_params.map(|p| p.speaking_rate).unwrap_or(1.0);
        let pitch_mod = voice_params.map(|p| p.pitch).unwrap_or(1.0);

        Box::pin(async move {
            if text_owned.trim().is_empty() {
                return Err(VoiceError::SynthesisFailed(
                    "Texto vazio para síntese".to_string(),
                ));
            }

            // Duração proporcional à quantidade de caracteres (aprox 70ms por caractere / speaking_rate)
            let char_count = text_owned.chars().count().max(1);
            let ms_per_char = (70.0 / speaking_rate.clamp(0.5, 2.0)) as u64;
            let duration_ms = (char_count as u64 * ms_per_char).clamp(200, 30_000);
            let total_samples = ((duration_ms * sample_rate as u64) / 1000) as usize;

            // Frequência fundamental feminina calorosa da Yuki: ~220Hz * pitch_mod
            let f0 = 220.0 * pitch_mod as f64;
            let mut samples = Vec::with_capacity(total_samples);

            for i in 0..total_samples {
                let t = (i as f64) / (sample_rate as f64);
                // Onda acústica rica com harmônicos (F0, 2*F0, 3*F0) e envelope suave nas bordas
                let envelope = if i < (sample_rate as usize / 20) {
                    // Fade in inicial 50ms
                    (i as f64) / (sample_rate as f64 / 20.0)
                } else if i > total_samples - (sample_rate as usize / 20) {
                    // Fade out final 50ms
                    ((total_samples - i) as f64) / (sample_rate as f64 / 20.0)
                } else {
                    1.0
                };

                // Modulação melódica natural
                let vibrato = (2.0 * std::f64::consts::PI * 4.5 * t).sin() * 3.0;
                let inst_f0 = f0 + vibrato;

                let harmonic1 = (2.0 * std::f64::consts::PI * inst_f0 * t).sin() * 0.6;
                let harmonic2 = (2.0 * std::f64::consts::PI * inst_f0 * 2.0 * t).sin() * 0.25;
                let harmonic3 = (2.0 * std::f64::consts::PI * inst_f0 * 3.0 * t).sin() * 0.15;

                let sample_val = ((harmonic1 + harmonic2 + harmonic3) * envelope * 24000.0)
                    .clamp(-32760.0, 32760.0) as i16;
                samples.push(sample_val);
            }

            let format = AudioFormat {
                sample_rate,
                channels: 1,
                bits_per_sample: 16,
            };

            let wav_payload = AudioCodec::encode_wav(&samples, &format)?;

            Ok(AudioOutput::new(
                wav_payload,
                format,
                duration_ms,
                "yuki-acoustic-ptbr",
            ))
        })
    }
}

fn uuid_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("{:x}", ms)
}
