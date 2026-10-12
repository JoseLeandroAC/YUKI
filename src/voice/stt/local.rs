use crate::contracts::identifiers::generate_unique_id;
use crate::contracts::voice::{
    AudioInput, SpeechToTextProvider, VoiceError, VoiceProviderMetadata, VoiceProviderType,
};
use crate::voice::audio::AudioCodec;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::time::Duration;

/// Configuração do adaptador local Whisper para reconhecimento de fala offline.
#[derive(Debug, Clone)]
pub struct LocalWhisperConfig {
    pub binary_path: Option<PathBuf>,
    pub model_path: Option<PathBuf>,
    pub language: String,
    pub timeout: Duration,
    pub max_duration_ms: u64,
    pub max_bytes: usize,
}

impl Default for LocalWhisperConfig {
    fn default() -> Self {
        let binary_path = std::env::var_os("YUKI_WHISPER_BIN")
            .map(PathBuf::from)
            .or_else(|| {
                // Heurística de binário no PATH ou diretório padrão
                which_whisper()
            });

        let model_path = std::env::var_os("YUKI_WHISPER_MODEL").map(PathBuf::from);

        Self {
            binary_path,
            model_path,
            language: "pt".to_string(),
            timeout: Duration::from_secs(30),
            max_duration_ms: 30_000,
            max_bytes: 5 * 1024 * 1024,
        }
    }
}

fn which_whisper() -> Option<PathBuf> {
    // Procura por executáveis conhecidos whisper.cpp / whisper
    let candidates = [
        "whisper-cli.exe",
        "whisper.exe",
        "whisper-cli",
        "whisper",
        "main.exe",
        "main",
    ];
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

/// Provedor local de reconhecimento de fala (STT) baseado no whisper.cpp/Whisper.
///
/// ARQUITETURA LOCAL-FIRST:
/// - Soberano: opera 100% em CPU/GPU local, sem qualquer transmissão à nuvem.
/// - Seguro: valida formato, tamanho e duração do áudio antes de qualquer processamento.
/// - Transparente: não realiza downloads silenciosos de modelos sem consentimento.
/// - Resiliente: reporta instruções claras de instalação se o modelo ou executável não forem encontrados.
pub struct LocalWhisperSttProvider {
    metadata: VoiceProviderMetadata,
    config: LocalWhisperConfig,
}

impl LocalWhisperSttProvider {
    pub fn new(config: LocalWhisperConfig) -> Self {
        let metadata = VoiceProviderMetadata::new(
            "Whisper-Local",
            VoiceProviderType::LocalEdge,
            vec!["pt-BR".to_string(), "pt".to_string(), "en".to_string()],
            16000,
            false,
            "1.0.0",
        );
        Self { metadata, config }
    }

    /// Retorna instruções claras de instalação para o operador caso os recursos locais estejam ausentes.
    pub fn installation_instructions() -> &'static str {
        "Para ativar o reconhecimento de fala local com whisper.cpp na Yuki:\n\
         1. Baixe ou compile o executável whisper.cpp (ex: whisper-cli.exe ou main.exe).\n\
         2. Baixe o modelo Whisper quantizado desejado (ex: ggml-small.bin ou ggml-base.bin de HuggingFace/ggerganov/whisper.cpp).\n\
         3. Configure as variáveis de ambiente:\n\
            set YUKI_WHISPER_BIN=C:\\caminho\\para\\whisper-cli.exe\n\
            set YUKI_WHISPER_MODEL=C:\\caminho\\para\\ggml-small.bin\n\
         Ou posicione-os em ./models/whisper/.\n\
         A Yuki não realiza downloads silenciosos para garantir a integridade e soberania do operador."
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

impl Default for LocalWhisperSttProvider {
    fn default() -> Self {
        Self::new(LocalWhisperConfig::default())
    }
}

impl SpeechToTextProvider for LocalWhisperSttProvider {
    fn metadata(&self) -> &VoiceProviderMetadata {
        &self.metadata
    }

    fn health<'a>(&'a self) -> Pin<Box<dyn Future<Output = Result<(), VoiceError>> + Send + 'a>> {
        Box::pin(async move {
            if !self.is_environment_ready() {
                let bin_missing = self.config.binary_path.is_none();
                let model_missing = self.config.model_path.is_none();
                return Err(VoiceError::ProviderUnavailable(format!(
                    "Whisper local não configurado (binário ausente: {}, modelo ausente: {}). {}",
                    bin_missing,
                    model_missing,
                    Self::installation_instructions()
                )));
            }
            Ok(())
        })
    }

    fn transcribe<'a>(
        &'a self,
        audio: &'a AudioInput,
    ) -> Pin<Box<dyn Future<Output = Result<String, VoiceError>> + Send + 'a>> {
        Box::pin(async move {
            // 1. Valida restrições de memória e duração
            if audio.payload.len() > self.config.max_bytes {
                return Err(VoiceError::AudioTooLarge {
                    bytes: audio.payload.len(),
                    max_bytes: self.config.max_bytes,
                });
            }

            if audio.duration_ms > self.config.max_duration_ms {
                return Err(VoiceError::AudioTooLong {
                    duration_ms: audio.duration_ms,
                    max_ms: self.config.max_duration_ms,
                });
            }

            // 2. Decodifica e normaliza o áudio
            let (format, samples) = AudioCodec::decode_wav(&audio.payload)?;
            if format.sample_rate != 16000 {
                return Err(VoiceError::InvalidAudioFormat(format!(
                    "Taxa de amostragem incompatível: {}Hz (esperado 16000Hz)",
                    format.sample_rate
                )));
            }

            // Garante mono
            let mut mono_samples = AudioCodec::to_mono(&samples, format.channels);
            // Normaliza amplitude para 28000
            AudioCodec::normalize_amplitude(&mut mono_samples, 28000);

            // Re-codifica áudio WAV normalizado para entrega ao processo do whisper
            let norm_format = crate::contracts::voice::AudioFormat {
                sample_rate: 16000,
                channels: 1,
                bits_per_sample: 16,
            };
            let normalized_wav = AudioCodec::encode_wav(&mono_samples, &norm_format)?;

            // 3. Verifica disponibilidade do ambiente
            let bin_path = self.config.binary_path.as_ref().ok_or_else(|| {
                VoiceError::ProviderUnavailable(format!(
                    "Executável whisper local não configurado. {}",
                    Self::installation_instructions()
                ))
            })?;

            let model_path = self.config.model_path.as_ref().ok_or_else(|| {
                VoiceError::ProviderUnavailable(format!(
                    "Arquivo de modelo whisper não configurado. {}",
                    Self::installation_instructions()
                ))
            })?;

            // 4. Cria arquivo temporário atômico para o áudio com alta entropia
            let temp_dir = std::env::temp_dir();
            let temp_wav_path = temp_dir.join(format!("yuki_stt_{}.wav", generate_unique_id("stt")));

            std::fs::write(&temp_wav_path, &normalized_wav)
                .map_err(|e| VoiceError::Io(format!("Falha ao gravar WAV temporário: {}", e)))?;

            // Limpa o arquivo temporário ao final da execução (Drop garantido)
            struct TempCleaner(PathBuf);
            impl Drop for TempCleaner {
                fn drop(&mut self) {
                    let _ = std::fs::remove_file(&self.0);
                }
            }
            let _cleaner = TempCleaner(temp_wav_path.clone());

            // 5. Executa processo Whisper de forma assíncrona com controle de timeout e término garantido
            let timeout_duration = self.config.timeout;
            let mut child = tokio::process::Command::new(bin_path)
                .arg("-m")
                .arg(model_path)
                .arg("-f")
                .arg(&temp_wav_path)
                .arg("-l")
                .arg(&self.config.language)
                .arg("--no-timestamps")
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .map_err(|e| {
                    VoiceError::TranscriptionFailed(format!(
                        "Falha ao iniciar processo whisper: {}",
                        e
                    ))
                })?;

            let mut stdout = child.stdout.take().ok_or_else(|| {
                VoiceError::TranscriptionFailed("Stdout do processo não capturado".to_string())
            })?;
            let mut stderr = child.stderr.take().ok_or_else(|| {
                VoiceError::TranscriptionFailed("Stderr do processo não capturado".to_string())
            })?;

            let mut stdout_bytes = Vec::new();
            let mut stderr_bytes = Vec::new();

            let status_res = tokio::time::timeout(timeout_duration, async {
                use tokio::io::AsyncReadExt;
                let read_out = stdout.read_to_end(&mut stdout_bytes);
                let read_err = stderr.read_to_end(&mut stderr_bytes);
                let wait_child = child.wait();
                let (_, _, status) = tokio::try_join!(read_out, read_err, wait_child)?;
                Ok::<std::process::ExitStatus, std::io::Error>(status)
            })
            .await;

            let status = match status_res {
                Ok(Ok(st)) => st,
                Ok(Err(e)) => {
                    return Err(VoiceError::TranscriptionFailed(format!(
                        "Erro ao aguardar processo whisper: {}",
                        e
                    )));
                }
                Err(_) => {
                    // Previne processos órfãos: encerra imediatamente o processo filho
                    let _ = child.kill().await;
                    let _ = child.wait().await;
                    return Err(VoiceError::Timeout(format!(
                        "Reconhecimento de fala excedeu o limite de {:?}",
                        timeout_duration
                    )));
                }
            };

            if !status.success() {
                let err_msg = String::from_utf8_lossy(&stderr_bytes);
                return Err(VoiceError::TranscriptionFailed(format!(
                    "Processo whisper finalizou com erro (código {:?}): {}",
                    status.code(),
                    err_msg.trim()
                )));
            }

            let text = String::from_utf8_lossy(&stdout_bytes)
                .trim()
                .to_string();
            Ok(text)
        })
    }
}
