use crate::contracts::identifiers::generate_unique_id;
use crate::contracts::voice::{
    AudioFormat, AudioOutput, TextToSpeechProvider, TtsSynthesisKind, VoiceConfiguration,
    VoiceError, VoiceProviderMetadata, VoiceProviderType,
};
use crate::voice::audio::AudioCodec;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
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
    // Prioriza vozes neurais femininas em PT-BR compatíveis com a identidade vocal da Yuki
    let preferred = [
        "pt_BR-dii-female.onnx",
        "dii_pt-BR.onnx",
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
        if dir.ends_with("piper") {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("onnx") {
                        return Some(path);
                    }
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

fn find_espeak_data_dir(bin_path: &std::path::Path) -> Option<PathBuf> {
    if let Some(env_dir) = std::env::var_os("YUKI_PIPER_ESPEAK_DATA") {
        let p = PathBuf::from(env_dir);
        if p.is_dir() {
            return Some(p);
        }
    }
    if let Some(parent) = bin_path.parent() {
        let sibling = parent.join("espeak-ng-data");
        if sibling.is_dir() {
            return Some(sibling);
        }
    }
    let fallback = PathBuf::from("bin/piper/espeak-ng-data");
    if fallback.is_dir() {
        return Some(fallback);
    }
    None
}

/// Converte um caminho para forma segura em ASCII para executáveis C/C++ legados no Windows
/// (como `espeak-ng` dentro do `piper.exe`, que falha em diretórios com acentos como `Miriã` ou `JOSÉ`).
pub fn to_ascii_safe_cli_path(path: &std::path::Path) -> PathBuf {
    if let Ok(cwd) = std::env::current_dir() {
        if let Ok(rel) = path.strip_prefix(&cwd) {
            if !rel.as_os_str().is_empty() && rel.to_string_lossy().is_ascii() {
                return rel.to_path_buf();
            }
        }
    }

    if path.to_string_lossy().is_ascii() {
        return path.to_path_buf();
    }

    #[cfg(windows)]
    {
        use std::os::windows::ffi::{OsStrExt, OsStringExt};
        extern "system" {
            fn GetShortPathNameW(
                lpszLongPath: *const u16,
                lpszShortPath: *mut u16,
                cchBuffer: u32,
            ) -> u32;
        }

        let wide: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        unsafe {
            let len = GetShortPathNameW(wide.as_ptr(), std::ptr::null_mut(), 0);
            if len > 0 {
                let mut buf = vec![0u16; len as usize];
                let written = GetShortPathNameW(wide.as_ptr(), buf.as_mut_ptr(), len);
                if written > 0 && (written as usize) < buf.len() {
                    let short_os = std::ffi::OsString::from_wide(&buf[..written as usize]);
                    return PathBuf::from(short_os);
                }
            }
        }
    }

    path.to_path_buf()
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

struct PiperWarmDaemon {
    child: tokio::process::Child,
    stdin: tokio::process::ChildStdin,
    stdout: tokio::process::ChildStdout,
    length_scale_key: u32,
}

fn select_ascii_temp_wav_path() -> PathBuf {
    let fname = format!("yuki_tts_{}.wav", generate_unique_id("tts"));
    let local_cache_dir = PathBuf::from("data/tts_cache");
    if std::fs::create_dir_all(&local_cache_dir).is_ok() {
        return local_cache_dir.join(fname);
    }
    to_ascii_safe_cli_path(&std::env::temp_dir()).join(fname)
}

/// Provedor local de síntese de voz (TTS) baseado no Piper.
///
/// ARQUITETURA LOCAL-FIRST:
/// - Soberano: síntese neural local sem egresso de dados.
/// - Respeita a identidade vocal da Yuki (prioriza voz feminina PT-BR quando presente).
/// - Mantém processo quente (`--json-input`) sob lock assíncrono com `kill_on_drop(true)`,
///   evitando recarregar o modelo ONNX a cada turno (~350-450ms por frase).
/// - Converte diretórios com acentuação no Windows (`espeak-ng-data`) para caminhos ASCII/8.3 seguros.
pub struct LocalPiperTtsProvider {
    metadata: VoiceProviderMetadata,
    config: LocalPiperConfig,
    daemon: tokio::sync::Mutex<Option<PiperWarmDaemon>>,
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
        )
        .with_synthesis_kind(TtsSynthesisKind::NeuralSpeech);

        Self {
            metadata,
            config,
            daemon: tokio::sync::Mutex::new(None),
        }
    }

    pub fn active_voice_id(&self) -> String {
        self.config
            .model_path
            .as_deref()
            .and_then(|p| p.file_stem())
            .and_then(|s| s.to_str())
            .unwrap_or("yuki-piper-ptbr")
            .to_string()
    }

    pub fn config(&self) -> &LocalPiperConfig {
        &self.config
    }

    pub fn installation_instructions() -> &'static str {
        Self::download_instructions()
    }

    pub fn download_instructions() -> &'static str {
        "Para ativar a síntese de voz neural local com Piper na Yuki:\n\
         1. Baixe o executável oficial do Piper para Windows x64:\n\
            Repositório: https://github.com/rhasspy/piper/releases\n\
            Extraia piper.exe e espeak-ng-data em ./bin/piper/.\n\
         2. Baixe um modelo neural pt-BR (.onnx e .onnx.json) em ./models/piper/:\n\
            - Voz neural feminina PT-BR recomendada para a identidade da Yuki (OpenVoiceOS/pipertts_pt-BR_dii):\n\
              Invoke-WebRequest -Uri \"https://huggingface.co/OpenVoiceOS/pipertts_pt-BR_dii/resolve/main/dii_pt-BR.onnx\" -OutFile \"models/piper/pt_BR-dii-female.onnx\"\n\
              Invoke-WebRequest -Uri \"https://huggingface.co/OpenVoiceOS/pipertts_pt-BR_dii/resolve/main/dii_pt-BR.onnx.json\" -OutFile \"models/piper/pt_BR-dii-female.onnx.json\"\n\
            - Nota de transparência: os locutores oficiais de rhasspy/piper-voices para pt-BR (faber, cadu, edresson, jeff) são masculinos:\n\
              Invoke-WebRequest -Uri \"https://huggingface.co/rhasspy/piper-voices/resolve/main/pt/pt_BR/faber/medium/pt_BR-faber-medium.onnx\" -OutFile \"models/piper/pt_BR-faber-medium.onnx\"\n\
         3. Variáveis de ambiente opcionais:\n\
            $env:YUKI_PIPER_BIN = \"bin\\piper\\piper.exe\"\n\
            $env:YUKI_PIPER_MODEL = \"models\\piper\\pt_BR-dii-female.onnx\""
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

    fn spawn_warm_daemon(&self, length_scale: f32) -> Result<PiperWarmDaemon, VoiceError> {
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

        let safe_bin = to_ascii_safe_cli_path(bin_path);
        let safe_model = to_ascii_safe_cli_path(model_path);

        let mut cmd = tokio::process::Command::new(&safe_bin);
        cmd.kill_on_drop(true);
        cmd.arg("-m").arg(&safe_model);
        if let Some(ref cfg) = self.config.config_json {
            if cfg.is_file() {
                cmd.arg("-c").arg(to_ascii_safe_cli_path(cfg));
            }
        }
        if let Some(espeak_dir) = find_espeak_data_dir(bin_path) {
            cmd.arg("--espeak_data")
                .arg(to_ascii_safe_cli_path(&espeak_dir));
        }
        cmd.arg("--json-input");
        cmd.arg("--length_scale")
            .arg(format!("{:.2}", length_scale));
        cmd.arg("--noise_scale")
            .arg(format!("{:.2}", self.config.noise_scale));
        cmd.arg("--noise_w")
            .arg(format!("{:.2}", self.config.noise_w));
        cmd.stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null());

        let mut child = cmd.spawn().map_err(|e| {
            VoiceError::SynthesisFailed(format!("Falha ao iniciar processo piper: {}", e))
        })?;

        let stdin = child.stdin.take().ok_or_else(|| {
            VoiceError::SynthesisFailed("Stdin do processo piper não capturado".to_string())
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            VoiceError::SynthesisFailed("Stdout do processo piper não capturado".to_string())
        })?;

        let scale_key = (length_scale * 100.0).round() as u32;
        Ok(PiperWarmDaemon {
            child,
            stdin,
            stdout,
            length_scale_key: scale_key,
        })
    }

    /// Pré-aquece o motor neural Piper carregando o modelo ONNX antes do primeiro turno conversacional.
    pub async fn warmup(&self) -> Result<(), VoiceError> {
        let _ = self.synthesize("Olá.", None).await?;
        Ok(())
    }

    async fn synthesize_oneshot(
        &self,
        text: &str,
        effective_length_scale: f32,
        temp_out: &std::path::Path,
    ) -> Result<(), VoiceError> {
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

        let safe_bin = to_ascii_safe_cli_path(bin_path);
        let safe_model = to_ascii_safe_cli_path(model_path);
        let safe_out = to_ascii_safe_cli_path(temp_out);

        let timeout_duration = self.config.timeout;
        let mut cmd = tokio::process::Command::new(&safe_bin);
        cmd.kill_on_drop(true);
        cmd.arg("-m").arg(&safe_model);
        if let Some(ref cfg) = self.config.config_json {
            if cfg.is_file() {
                cmd.arg("-c").arg(to_ascii_safe_cli_path(cfg));
            }
        }
        if let Some(espeak_dir) = find_espeak_data_dir(bin_path) {
            cmd.arg("--espeak_data")
                .arg(to_ascii_safe_cli_path(&espeak_dir));
        }
        cmd.arg("-f").arg(&safe_out);
        cmd.arg("--length_scale")
            .arg(format!("{:.2}", effective_length_scale));
        cmd.arg("--noise_scale")
            .arg(format!("{:.2}", self.config.noise_scale));
        cmd.arg("--noise_w")
            .arg(format!("{:.2}", self.config.noise_w));
        cmd.stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| {
            VoiceError::SynthesisFailed(format!("Falha ao iniciar processo piper: {}", e))
        })?;

        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            let mut payload = text.replace(['\r', '\n'], " ");
            payload.push('\n');
            let _ = stdin.write_all(payload.as_bytes()).await;
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

        Ok(())
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

        Box::pin(async move {
            if text_owned.trim().is_empty() {
                return Err(VoiceError::SynthesisFailed(
                    "Texto vazio para síntese".to_string(),
                ));
            }

            if !self.is_environment_ready() {
                let bin_missing = self
                    .config
                    .binary_path
                    .as_ref()
                    .map(|p| !p.is_file())
                    .unwrap_or(true);
                let reason = if bin_missing {
                    "Executável piper local não configurado."
                } else {
                    "Modelo piper não configurado."
                };
                return Err(VoiceError::ProviderUnavailable(format!(
                    "{} {}",
                    reason,
                    Self::installation_instructions()
                )));
            }

            let temp_out = select_ascii_temp_wav_path();
            struct TempCleaner(PathBuf);
            impl Drop for TempCleaner {
                fn drop(&mut self) {
                    let _ = std::fs::remove_file(&self.0);
                }
            }
            let _cleaner = TempCleaner(temp_out.clone());
            let safe_out = to_ascii_safe_cli_path(&temp_out);

            let desired_scale_key = (effective_length_scale * 100.0).round() as u32;
            let clean_text = text_owned.replace(['\r', '\n'], " ");

            // Tenta síntese de baixa latência via processo Piper quente (--json-input).
            // Retiramos o daemon do slot antes do `.await` para garantir segurança total de cancelamento:
            // se a Future for cancelada/dropada no meio da síntese, `PiperWarmDaemon` sofre Drop e encerra
            // o processo filho via `kill_on_drop(true)`, sem deixar estado parcial nos pipes.
            let daemon_attempt: Result<(), VoiceError> = {
                let mut slot = self.daemon.lock().await;
                let mut daemon = match slot.take() {
                    Some(mut existing) => {
                        if existing.length_scale_key == desired_scale_key
                            && matches!(existing.child.try_wait(), Ok(None))
                        {
                            existing
                        } else {
                            let _ = existing.child.start_kill();
                            self.spawn_warm_daemon(effective_length_scale)?
                        }
                    }
                    None => self.spawn_warm_daemon(effective_length_scale)?,
                };

                let req_json = serde_json::json!({
                    "text": clean_text,
                    "output_file": safe_out.to_string_lossy(),
                });
                let mut line_bytes = req_json.to_string().into_bytes();
                line_bytes.push(b'\n');

                let timeout_duration = self.config.timeout;
                let io_res = tokio::time::timeout(timeout_duration, async {
                    use tokio::io::{AsyncReadExt, AsyncWriteExt};
                    daemon.stdin.write_all(&line_bytes).await?;
                    daemon.stdin.flush().await?;

                    let mut ack = Vec::with_capacity(128);
                    let mut byte = [0u8; 1];
                    loop {
                        let n = daemon.stdout.read(&mut byte).await?;
                        if n == 0 {
                            return Err(std::io::Error::new(
                                std::io::ErrorKind::UnexpectedEof,
                                "Processo piper encerrou inesperadamente",
                            ));
                        }
                        if byte[0] == b'\n' {
                            break;
                        }
                        if ack.len() < 4096 {
                            ack.push(byte[0]);
                        }
                    }
                    Ok::<(), std::io::Error>(())
                })
                .await;

                match io_res {
                    Ok(Ok(())) if temp_out.is_file() => {
                        *slot = Some(daemon);
                        Ok(())
                    }
                    Ok(Ok(())) => {
                        let _ = daemon.child.kill().await;
                        Err(VoiceError::SynthesisFailed(
                            "Piper não produziu arquivo de saída esperado".to_string(),
                        ))
                    }
                    Ok(Err(e)) => {
                        let _ = daemon.child.kill().await;
                        Err(VoiceError::SynthesisFailed(format!(
                            "Falha de comunicação com processo piper: {}",
                            e
                        )))
                    }
                    Err(_) => {
                        let _ = daemon.child.kill().await;
                        let _ = daemon.child.wait().await;
                        Err(VoiceError::Timeout(
                            "Síntese de voz excedeu tempo limite".to_string(),
                        ))
                    }
                }
            };

            if let Err(e) = daemon_attempt {
                if matches!(e, VoiceError::Timeout(_)) {
                    return Err(e);
                }
                tracing::debug!(
                    "Fallback para execução Piper one-shot após falha no daemon ({})",
                    e
                );
                self.synthesize_oneshot(&clean_text, effective_length_scale, &temp_out)
                    .await?;
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
                self.active_voice_id(),
            ))
        })
    }
}

/// Gerador acústico determinístico soberano estritamente para diagnóstico de hardware da Yuki.
///
/// Produz formas de onda WAV canônicas com entonação melódica harmônica (F0 ~ 210-230 Hz)
/// exclusivamente para validar sinks e dispositivos de áudio; NÃO produz fala inteligível
/// (`TtsSynthesisKind::DiagnosticAcoustic`) e é bloqueado nos modos conversacionais reais.
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
        )
        .with_synthesis_kind(TtsSynthesisKind::DiagnosticAcoustic);
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

/// Resolve o provedor TTS local padrão.
///
/// Se `allow_diagnostic_acoustic` for `false` (padrão conversacional real), NUNCA substitui
/// silenciosamente o Piper por ondas harmônicas artificiais (`DiagnosticAcoustic`).
pub fn resolve_default_local_tts(allow_diagnostic_acoustic: bool) -> Arc<dyn TextToSpeechProvider> {
    let piper = LocalPiperTtsProvider::default();
    if piper.is_environment_ready() || !allow_diagnostic_acoustic {
        Arc::new(piper)
    } else {
        Arc::new(LocalAcousticTtsProvider::default())
    }
}

/// Resolve e valida estritamente um provedor TTS local para conversação real.
///
/// Falha fechado (`Err(VoiceError::ProviderUnavailable)`) se apenas o gerador diagnóstico estiver
/// disponível e `allow_diagnostic_acoustic` não tiver sido solicitado explicitamente.
pub fn resolve_conversational_local_tts(
    allow_diagnostic_acoustic: bool,
) -> Result<Arc<dyn TextToSpeechProvider>, VoiceError> {
    if allow_diagnostic_acoustic {
        return Ok(Arc::new(LocalAcousticTtsProvider::default()));
    }
    let piper = LocalPiperTtsProvider::default();
    if !piper.is_environment_ready() {
        return Err(VoiceError::ProviderUnavailable(format!(
            "Nenhum motor TTS neural inteligível (NeuralSpeech) está pronto para conversação real. \
             O fallback acústico diagnóstico (ondas harmônicas) está bloqueado em produção.\n{}",
            LocalPiperTtsProvider::installation_instructions()
        )));
    }
    Ok(Arc::new(piper))
}
