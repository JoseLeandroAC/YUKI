use crate::contracts::identifiers::generate_unique_id;
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
    pub length_scale: f32,
    pub noise_scale: f32,
    pub noise_w: f32,
    pub timeout: Duration,
}

impl Default for LocalPiperConfig {
    fn default() -> Self {
        let binary_path = std::env::var_os("YUKI_PIPER_BIN")
            .map(PathBuf::from)
            .or_else(find_local_piper_bin)
            .or_else(which_piper);

        let model_path = std::env::var_os("YUKI_PIPER_MODEL")
            .map(PathBuf::from)
            .or_else(find_local_piper_model);

        let config_json = std::env::var_os("YUKI_PIPER_CONFIG")
            .map(PathBuf::from)
            .or_else(|| model_path.as_deref().and_then(find_local_piper_config));

        Self {
            binary_path,
            model_path,
            config_json,
            length_scale: 1.0,
            noise_scale: 0.55,
            noise_w: 0.70,
            timeout: Duration::from_secs(15),
        }
    }
}

fn find_local_piper_bin() -> Option<PathBuf> {
    let local_dirs = [
        PathBuf::from("bin/piper"),
        PathBuf::from("bin"),
        PathBuf::from("."),
    ];
    let candidates = ["piper.exe", "piper"];
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

fn find_local_piper_model() -> Option<PathBuf> {
    let local_dirs = [PathBuf::from("models/piper"), PathBuf::from("models")];
    let preferred = [
        "pt_BR-faber-medium.onnx",
        "pt_BR-cadu-medium.onnx",
        "pt_BR-edresson-low.onnx",
        "pt_BR-jeff-medium.onnx",
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
                if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("onnx") {
                    return Some(path);
                }
            }
        }
    }
    None
}

fn find_local_piper_config(model_path: &std::path::Path) -> Option<PathBuf> {
    let p1 = PathBuf::from(format!("{}.json", model_path.display()));
    if p1.is_file() {
        return Some(p1);
    }
    let p2 = model_path.with_extension("json");
    if p2.is_file() {
        return Some(p2);
    }
    None
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

/// Metadados informativos de modelos Piper conhecidos e locais.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PiperVoiceModelInfo {
    pub id: String,
    pub name: String,
    pub quality: String,
    pub sample_rate: u32,
    pub speaker_gender: String,
    pub is_official: bool,
    pub is_installed: bool,
    pub local_path: Option<PathBuf>,
    pub download_onnx_url: String,
    pub download_json_url: String,
    pub description: String,
}

/// Retorna o catálogo oficial de vozes pt-BR do Piper e modelos locais customizados detectados.
pub fn get_piper_voice_catalog() -> Vec<PiperVoiceModelInfo> {
    let mut catalog = vec![
        PiperVoiceModelInfo {
            id: "pt_BR-faber-medium".to_string(),
            name: "Faber (Médio)".to_string(),
            quality: "medium".to_string(),
            sample_rate: 22050,
            speaker_gender: "Masculino".to_string(),
            is_official: true,
            is_installed: false,
            local_path: None,
            download_onnx_url: "https://huggingface.co/rhasspy/piper-voices/resolve/main/pt/pt_BR/faber/medium/pt_BR-faber-medium.onnx".to_string(),
            download_json_url: "https://huggingface.co/rhasspy/piper-voices/resolve/main/pt/pt_BR/faber/medium/pt_BR-faber-medium.onnx.json".to_string(),
            description: "Modelo oficial pt-BR com maior estabilidade e clareza fonêmica. Locutor masculino.".to_string(),
        },
        PiperVoiceModelInfo {
            id: "pt_BR-cadu-medium".to_string(),
            name: "Cadu (Médio)".to_string(),
            quality: "medium".to_string(),
            sample_rate: 22050,
            speaker_gender: "Masculino".to_string(),
            is_official: true,
            is_installed: false,
            local_path: None,
            download_onnx_url: "https://huggingface.co/rhasspy/piper-voices/resolve/main/pt/pt_BR/cadu/medium/pt_BR-cadu-medium.onnx".to_string(),
            download_json_url: "https://huggingface.co/rhasspy/piper-voices/resolve/main/pt/pt_BR/cadu/medium/pt_BR-cadu-medium.onnx.json".to_string(),
            description: "Locutor oficial pt-BR com entonação coloquial brasileira dinâmica. Locutor masculino.".to_string(),
        },
        PiperVoiceModelInfo {
            id: "pt_BR-edresson-low".to_string(),
            name: "Edresson (Leve)".to_string(),
            quality: "low".to_string(),
            sample_rate: 16000,
            speaker_gender: "Masculino".to_string(),
            is_official: true,
            is_installed: false,
            local_path: None,
            download_onnx_url: "https://huggingface.co/rhasspy/piper-voices/resolve/main/pt/pt_BR/edresson/low/pt_BR-edresson-low.onnx".to_string(),
            download_json_url: "https://huggingface.co/rhasspy/piper-voices/resolve/main/pt/pt_BR/edresson/low/pt_BR-edresson-low.onnx.json".to_string(),
            description: "Modelo ultraleve oficial pt-BR para inferência de alta velocidade e baixo consumo. Locutor masculino.".to_string(),
        },
        PiperVoiceModelInfo {
            id: "pt_BR-jeff-medium".to_string(),
            name: "Jeff (Médio)".to_string(),
            quality: "medium".to_string(),
            sample_rate: 22050,
            speaker_gender: "Masculino".to_string(),
            is_official: true,
            is_installed: false,
            local_path: None,
            download_onnx_url: "https://huggingface.co/rhasspy/piper-voices/resolve/main/pt/pt_BR/jeff/medium/pt_BR-jeff-medium.onnx".to_string(),
            download_json_url: "https://huggingface.co/rhasspy/piper-voices/resolve/main/pt/pt_BR/jeff/medium/pt_BR-jeff-medium.onnx.json".to_string(),
            description: "Locutor alternativo da comunidade pt-BR. Locutor masculino.".to_string(),
        },
    ];

    let search_dirs = [PathBuf::from("models/piper"), PathBuf::from("models")];
    for model in &mut catalog {
        for dir in &search_dirs {
            let onnx_path = dir.join(format!("{}.onnx", model.id));
            if onnx_path.is_file() {
                model.is_installed = true;
                model.local_path = Some(onnx_path);
                break;
            }
        }
    }

    // Detecta modelos adicionais em models/piper/
    for dir in &search_dirs {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("onnx") {
                    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                    if !catalog.iter().any(|m| m.id == stem) {
                        catalog.push(PiperVoiceModelInfo {
                            id: stem.to_string(),
                            name: format!("Custom ({})", stem),
                            quality: "custom".to_string(),
                            sample_rate: 22050,
                            speaker_gender: "Custom / Comunitário".to_string(),
                            is_official: false,
                            is_installed: true,
                            local_path: Some(path),
                            download_onnx_url: String::new(),
                            download_json_url: String::new(),
                            description: "Modelo ONNX local personalizado ou comunitário."
                                .to_string(),
                        });
                    }
                }
            }
        }
    }

    catalog
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
        Self::download_instructions()
    }

    pub fn download_instructions() -> &'static str {
        "Para ativar a síntese de voz neural local com Piper na Yuki:\n\
         1. Baixe o executável oficial do Piper para sua plataforma:\n\
            Repositório: https://github.com/rhasspy/piper/releases\n\
            Windows x64: piper_windows_amd64.zip -> extrair executável piper.exe em ./bin/piper/ ou no PATH.\n\
         2. Baixe o modelo neural pt-BR (.onnx) e respectivo metadado (.onnx.json):\n\
            Catálogo Hugging Face: https://huggingface.co/rhasspy/piper-voices/tree/main/pt/pt_BR\n\
            Candidatas oficiais:\n\
            - pt_BR-faber-medium.onnx (maior clareza fonêmica)\n\
            - pt_BR-cadu-medium.onnx (entonação coloquial brasileira)\n\
            - pt_BR-edresson-low.onnx (ultraleve de baixa latência)\n\
            Destino dos modelos: ./models/piper/\n\
         3. Script PowerShell para download manual (zero downloads automáticos):\n\
            Invoke-WebRequest -Uri \"https://huggingface.co/rhasspy/piper-voices/resolve/main/pt/pt_BR/faber/medium/pt_BR-faber-medium.onnx\" -OutFile \"models/piper/pt_BR-faber-medium.onnx\"\n\
            Invoke-WebRequest -Uri \"https://huggingface.co/rhasspy/piper-voices/resolve/main/pt/pt_BR/faber/medium/pt_BR-faber-medium.onnx.json\" -OutFile \"models/piper/pt_BR-faber-medium.onnx.json\"\n\
         4. Variáveis de ambiente opcionais:\n\
            set YUKI_PIPER_BIN=C:\\caminho\\para\\piper.exe\n\
            set YUKI_PIPER_MODEL=C:\\caminho\\para\\pt_BR-faber-medium.onnx\n\
         \n\
         LIMITAÇÃO TÉCNICA E TRANSPARÊNCIA:\n\
         O catálogo upstream da Rhasspy para pt-BR possui modelos com locutores masculinos.\n\
         Para usar voz feminina nativa com a Yuki, posicione um modelo ONNX comunitário em ./models/piper/\n\
         ou mantenha o sintetizador acústico de diagnóstico padrão com F0=220Hz (tom afável melódico)."
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
        let effective_length_scale = if let Some(p) = voice_params {
            (self.config.length_scale / p.speaking_rate.clamp(0.5, 2.0)).clamp(0.5, 2.0)
        } else {
            self.config.length_scale
        };
        let effective_noise_scale = self.config.noise_scale;
        let effective_noise_w = self.config.noise_w;

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
            let temp_out = temp_dir.join(format!("yuki_tts_{}.wav", generate_unique_id("tts")));

            // Limpa o arquivo temporário ao final da execução (Drop garantido)
            struct TempCleaner(PathBuf);
            impl Drop for TempCleaner {
                fn drop(&mut self) {
                    let _ = std::fs::remove_file(&self.0);
                }
            }
            let _cleaner = TempCleaner(temp_out.clone());

            let timeout_duration = self.config.timeout;
            let mut cmd = tokio::process::Command::new(bin_path);
            cmd.arg("-m").arg(model_path);
            if let Some(ref cfg) = self.config.config_json {
                if cfg.is_file() {
                    cmd.arg("-c").arg(cfg);
                }
            }
            cmd.arg("-f").arg(&temp_out);
            cmd.arg("--length_scale")
                .arg(format!("{:.2}", effective_length_scale));
            cmd.arg("--noise_scale")
                .arg(format!("{:.2}", effective_noise_scale));
            cmd.arg("--noise_w")
                .arg(format!("{:.2}", effective_noise_w));
            cmd.stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::piped());

            let mut child = cmd.spawn().map_err(|e| {
                VoiceError::SynthesisFailed(format!("Falha ao iniciar processo piper: {}", e))
            })?;

            if let Some(mut stdin) = child.stdin.take() {
                use tokio::io::AsyncWriteExt;
                let _ = stdin.write_all(text_owned.as_bytes()).await;
            }

            let mut stderr = child.stderr.take().ok_or_else(|| {
                VoiceError::SynthesisFailed("Stderr do processo piper não capturado".to_string())
            })?;

            let mut stderr_bytes = Vec::new();

            let status_res = tokio::time::timeout(timeout_duration, async {
                use tokio::io::AsyncReadExt;
                let read_err = stderr.read_to_end(&mut stderr_bytes);
                let wait_child = child.wait();
                let (_, status) = tokio::try_join!(read_err, wait_child)?;
                Ok::<std::process::ExitStatus, std::io::Error>(status)
            })
            .await;

            let status = match status_res {
                Ok(Ok(st)) => st,
                Ok(Err(e)) => {
                    return Err(VoiceError::SynthesisFailed(format!(
                        "Erro ao aguardar processo piper: {}",
                        e
                    )));
                }
                Err(_) => {
                    // Previne processos órfãos: encerra imediatamente o processo filho
                    let _ = child.kill().await;
                    let _ = child.wait().await;
                    return Err(VoiceError::Timeout(
                        "Síntese de voz excedeu tempo limite".to_string(),
                    ));
                }
            };

            if !status.success() {
                let stderr_str = String::from_utf8_lossy(&stderr_bytes);
                return Err(VoiceError::SynthesisFailed(format!(
                    "Piper finalizou com erro: {}",
                    stderr_str.trim()
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

/// Gerador acústico determinístico soberano de diagnóstico e fallback offline da Yuki.
///
/// Produz formas de onda WAV canônicas com entonação melódica harmônica feminina (F0 ~ 210-230 Hz),
/// permitindo execução, diagnóstico de hardware e testes ponta-a-ponta em ambientes sem o motor Piper instalado.
///
/// NOTA DE TRANSPARÊNCIA: Este provedor sintetiza tons senoidais puros com envelopes de amplitude
/// para validação de integridade acústica; não produz fala fonêmica neural inteligível.
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
            "Yuki-Acoustic-Diagnostic-Fallback",
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
