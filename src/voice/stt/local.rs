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
            .or_else(find_local_whisper_bin)
            .or_else(which_whisper);

        let model_path = std::env::var_os("YUKI_WHISPER_MODEL")
            .map(PathBuf::from)
            .or_else(find_local_whisper_model);

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

fn find_local_whisper_bin() -> Option<PathBuf> {
    let local_dirs = [
        PathBuf::from("bin/whisper"),
        PathBuf::from("bin"),
        PathBuf::from("."),
    ];
    let candidates = [
        "whisper-cli.exe",
        "whisper.exe",
        "main.exe",
        "whisper-cli",
        "whisper",
        "main",
    ];
    for dir in &local_dirs {
        for candidate in &candidates {
            let p = dir.join(candidate);
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}

fn find_local_whisper_model() -> Option<PathBuf> {
    let local_dirs = [PathBuf::from("models/whisper"), PathBuf::from("models")];
    let preferred = [
        "ggml-small.bin",
        "ggml-base.bin",
        "ggml-medium.bin",
        "ggml-tiny.bin",
        "ggml-large-v3.bin",
    ];
    for dir in &local_dirs {
        for name in &preferred {
            let p = dir.join(name);
            if p.is_file() {
                return Some(p);
            }
        }
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("bin") {
                    return Some(path);
                }
            }
        }
    }
    None
}

/// Enumera modelos Whisper quantizados (.bin) encontrados nos diretórios locais da Yuki.
pub fn list_local_whisper_models() -> Vec<PathBuf> {
    let mut models = Vec::new();
    let search_dirs = [PathBuf::from("models/whisper"), PathBuf::from("models")];
    for dir in &search_dirs {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("bin") {
                    models.push(path);
                }
            }
        }
    }
    models.sort();
    models.dedup();
    models
}

/// Detecta se um bloco PCM de 16 bits corresponde a silêncio acústico ou ruído imperceptível.
pub fn is_silent_pcm(samples: &[i16]) -> bool {
    if samples.is_empty() {
        return true;
    }
    let mut sum_sq: f64 = 0.0;
    let mut max_abs: i16 = 0;
    for &s in samples {
        let abs = s.abs();
        if abs > max_abs {
            max_abs = abs;
        }
        sum_sq += (s as f64) * (s as f64);
    }
    let rms = (sum_sq / samples.len() as f64).sqrt();
    rms < 120.0 && max_abs < 250
}

/// Remove formatações de tempo e tags especiais (ex: `[00:00.000 --> 00:02.000]`, `[BLANK_AUDIO]`).
pub fn sanitize_whisper_transcript(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.contains("[BLANK_AUDIO]") {
        return String::new();
    }
    let mut cleaned = String::with_capacity(trimmed.len());
    let mut in_bracket = false;
    for c in trimmed.chars() {
        if c == '[' {
            in_bracket = true;
        } else if c == ']' {
            in_bracket = false;
        } else if !in_bracket {
            cleaned.push(c);
        }
    }
    cleaned.trim().to_string()
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
    for dir in &candidates {
        if let Ok(path) = std::env::var("PATH") {
            for d in std::env::split_paths(&path) {
                let full = d.join(dir);
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
         1. Baixe o executável whisper.cpp oficial para sua plataforma:\n\
            Repositório: https://github.com/ggerganov/whisper.cpp/releases\n\
            Windows x64: extraia whisper-cli.exe em ./bin/whisper/ ou adicione ao PATH.\n\
         2. Baixe o modelo Whisper quantizado GGML em português brasileiro:\n\
            Repositório Hugging Face: https://huggingface.co/ggerganov/whisper.cpp\n\
            Modelos recomendados:\n\
            - ggml-small.bin (~460MB - melhor equilíbrio entre acurácia pt-BR e latência)\n\
            - ggml-base.bin (~140MB - menor consumo computacional)\n\
            Destino: ./models/whisper/ggml-small.bin\n\
         3. Script PowerShell para download manual (zero downloads silenciosos):\n\
            Invoke-WebRequest -Uri \"https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin\" -OutFile \"models/whisper/ggml-small.bin\"\n\
         4. Variáveis de ambiente opcionais:\n\
            set YUKI_WHISPER_BIN=C:\\caminho\\para\\whisper-cli.exe\n\
            set YUKI_WHISPER_MODEL=C:\\caminho\\para\\ggml-small.bin\n\
         A Yuki opera com soberania local e nunca baixa executáveis ou modelos sem consentimento expresso."
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

            // 3. Garante que os recursos locais estão configurados antes do processamento
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

            // 4. Garante mono e normaliza amplitude
            let mut mono_samples = AudioCodec::to_mono(&samples, format.channels);
            AudioCodec::normalize_amplitude(&mut mono_samples, 28000);

            // 5. Curto-circuito econômico: se o áudio for silêncio, evita chamada externa ao processo
            if is_silent_pcm(&mono_samples) {
                tracing::debug!(
                    "Áudio identificado como silêncio acústico; transcrição concluída vazia."
                );
                return Ok(String::new());
            }

            // 6. Inicia medição de latência
            let start_instant = std::time::Instant::now();

            // Re-codifica áudio WAV normalizado para entrega ao processo do whisper
            let norm_format = crate::contracts::voice::AudioFormat {
                sample_rate: 16000,
                channels: 1,
                bits_per_sample: 16,
            };
            let normalized_wav = AudioCodec::encode_wav(&mono_samples, &norm_format)?;

            // 7. Cria arquivo temporário atômico para o áudio com alta entropia
            let temp_dir = std::env::temp_dir();
            let temp_wav_path =
                temp_dir.join(format!("yuki_stt_{}.wav", generate_unique_id("stt")));

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

            // 8. Executa processo Whisper de forma assíncrona com controle de timeout e término garantido
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

            let elapsed = start_instant.elapsed();
            let raw_text = String::from_utf8_lossy(&stdout_bytes);
            let sanitized = sanitize_whisper_transcript(&raw_text);
            tracing::info!(
                "Transcrição local Whisper concluída em {:?}: \"{}\"",
                elapsed,
                sanitized
            );
            Ok(sanitized)
        })
    }
}
