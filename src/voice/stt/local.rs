use crate::contracts::identifiers::generate_unique_id;
use crate::contracts::voice::{
    AudioInput, SpeechToTextProvider, SttDecodingConfigSummary, SttTimingMetrics, VoiceError,
    VoiceProviderMetadata, VoiceProviderType,
};
use crate::voice::audio::AudioCodec;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::time::Duration;

/// Contexto lexical padrão de referência para português brasileiro.
pub const YUKI_DEFAULT_VOCABULARY_PROMPT_PT: &str =
    "Yuki, FUVEST, OBMEP, Engenharia de Computação.";

/// Retorna o contexto lexical padrão recomendado conforme o idioma solicitado.
/// Retorna `None` para modo 'auto' ou idiomas sem preset para evitar enviesamento.
pub fn default_lexical_prompt_for_language(lang: &str) -> Option<&'static str> {
    match normalize_language_code(lang).as_str() {
        "pt" => Some(YUKI_DEFAULT_VOCABULARY_PROMPT_PT),
        "en" => Some("Yuki, artificial intelligence, computer science."),
        "es" => Some("Yuki, inteligencia artificial, ingeniería."),
        "ja" => Some("Yuki、人工知能、コンピュータ工学。"),
        _ => None,
    }
}

/// Valida limites do número de threads do Whisper (1 a 64).
pub fn validate_threads(threads: u32) -> Result<u32, VoiceError> {
    if threads == 0 || threads > 64 {
        Err(VoiceError::InvalidAudioFormat(format!(
            "Número de threads inválido: {} (deve estar entre 1 e 64)",
            threads
        )))
    } else {
        Ok(threads)
    }
}

/// Valida limites do tamanho de feixe beam search (1 a 10).
pub fn validate_beam_size(beam_size: u32) -> Result<u32, VoiceError> {
    if beam_size == 0 || beam_size > 10 {
        Err(VoiceError::InvalidAudioFormat(format!(
            "Tamanho de feixe (beam size) inválido: {} (deve estar entre 1 e 10)",
            beam_size
        )))
    } else {
        Ok(beam_size)
    }
}

/// Valida limites de melhores hipóteses mantidas (1 a 10).
pub fn validate_best_of(best_of: u32) -> Result<u32, VoiceError> {
    if best_of == 0 || best_of > 10 {
        Err(VoiceError::InvalidAudioFormat(format!(
            "Número de melhores hipóteses (best-of) inválido: {} (deve estar entre 1 e 10)",
            best_of
        )))
    } else {
        Ok(best_of)
    }
}

/// Configuração do adaptador local Whisper para reconhecimento de fala offline.
#[derive(Debug, Clone)]
pub struct LocalWhisperConfig {
    pub binary_path: Option<PathBuf>,
    pub model_path: Option<PathBuf>,
    pub language: String,
    pub timeout: Duration,
    pub max_duration_ms: u64,
    pub max_bytes: usize,
    /// Contexto lexical inicial opcional para condicionamento fonético do decodificador (--prompt)
    pub initial_prompt: Option<String>,
    /// Número explícito de threads para o cálculo do Whisper (-t / --threads)
    pub threads: Option<u32>,
    /// Tamanho do feixe para busca beam search (-bs / --beam-size)
    pub beam_size: Option<u32>,
    /// Número de melhores hipóteses a manter (-bo / --best-of)
    pub best_of: Option<u32>,
    /// Desativa o fallback de temperatura ao decodificar (-nf / --no-fallback)
    pub no_fallback: bool,
}

impl LocalWhisperConfig {
    /// Permite configurar explicitamente o idioma alvo para a transcrição ('pt', 'en', 'es', 'ja', 'auto', etc.).
    pub fn with_language(mut self, lang: impl Into<String>) -> Self {
        self.language = normalize_language_code(&lang.into());
        self
    }

    /// Permite customizar o tempo limite máximo para a inferência do modelo.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Configura o contexto lexical inicial (--prompt). Se vazio, desativa o contexto.
    pub fn with_initial_prompt(mut self, prompt: impl Into<String>) -> Self {
        let p = prompt.into();
        let trimmed = p.trim().to_string();
        self.initial_prompt = if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        };
        self
    }

    /// Configura o contexto lexical padrão recomendado conforme o idioma ativo.
    pub fn with_default_prompt_for_language(mut self) -> Self {
        if let Some(preset) = default_lexical_prompt_for_language(&self.language) {
            self.initial_prompt = Some(preset.to_string());
        }
        self
    }

    /// Define explicitamente o número de threads do Whisper (-t).
    pub fn set_threads(&mut self, threads: u32) -> Result<(), VoiceError> {
        let val = validate_threads(threads)?;
        self.threads = Some(val);
        Ok(())
    }

    /// Configura explicitamente o número de threads do Whisper (-t) em padrão builder.
    pub fn with_threads(mut self, threads: u32) -> Result<Self, VoiceError> {
        self.set_threads(threads)?;
        Ok(self)
    }

    /// Define explicitamente o beam size (-bs).
    pub fn set_beam_size(&mut self, beam_size: u32) -> Result<(), VoiceError> {
        let val = validate_beam_size(beam_size)?;
        self.beam_size = Some(val);
        Ok(())
    }

    /// Configura explicitamente o beam size (-bs) em padrão builder.
    pub fn with_beam_size(mut self, beam_size: u32) -> Result<Self, VoiceError> {
        self.set_beam_size(beam_size)?;
        Ok(self)
    }

    /// Define explicitamente o best-of (-bo).
    pub fn set_best_of(&mut self, best_of: u32) -> Result<(), VoiceError> {
        let val = validate_best_of(best_of)?;
        self.best_of = Some(val);
        Ok(())
    }

    /// Configura explicitamente o best-of (-bo) em padrão builder.
    pub fn with_best_of(mut self, best_of: u32) -> Result<Self, VoiceError> {
        self.set_best_of(best_of)?;
        Ok(self)
    }

    /// Ativa ou desativa o fallback de temperatura (-nf / --no-fallback).
    pub fn with_no_fallback(mut self, no_fallback: bool) -> Self {
        self.no_fallback = no_fallback;
        self
    }

    /// Ativa modo experimental de baixa latência (greedy search com beam_size=1, best_of=1 e sem fallback).
    pub fn with_fast_decoding(mut self) -> Self {
        self.beam_size = Some(1);
        self.best_of = Some(1);
        self.no_fallback = true;
        self
    }

    /// Retorna verdadeiro se a configuração ativa corresponde a modo de decodificação rápida (greedy search).
    pub fn is_fast_mode(&self) -> bool {
        self.beam_size == Some(1) && self.best_of == Some(1) && self.no_fallback
    }
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

        // Suporte soberano a seleção de idioma via variável de ambiente YUKI_WHISPER_LANGUAGE.
        // Se não configurada, preserva o português brasileiro ('pt') como padrão inicial.
        let language = std::env::var("YUKI_WHISPER_LANGUAGE")
            .map(|l| normalize_language_code(&l))
            .unwrap_or_else(|_| "pt".to_string());

        // Timeout padrão de 60s para garantir resiliência em inferência de CPU com modelos médios/pequenos.
        // Pode ser customizado via YUKI_WHISPER_TIMEOUT_SECS.
        let timeout_secs = std::env::var("YUKI_WHISPER_TIMEOUT_SECS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(60);

        let initial_prompt = std::env::var("YUKI_WHISPER_PROMPT").ok().and_then(|p| {
            let t = p.trim().to_string();
            if t.is_empty() {
                None
            } else {
                Some(t)
            }
        });

        let threads = std::env::var("YUKI_WHISPER_THREADS")
            .ok()
            .and_then(|s| s.parse::<u32>().ok())
            .and_then(|t| validate_threads(t).ok());

        let beam_size = std::env::var("YUKI_WHISPER_BEAM_SIZE")
            .ok()
            .and_then(|s| s.parse::<u32>().ok())
            .and_then(|b| validate_beam_size(b).ok());

        let best_of = std::env::var("YUKI_WHISPER_BEST_OF")
            .ok()
            .and_then(|s| s.parse::<u32>().ok())
            .and_then(|b| validate_best_of(b).ok());

        let no_fallback = std::env::var("YUKI_WHISPER_NO_FALLBACK")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        Self {
            binary_path,
            model_path,
            language,
            timeout: Duration::from_secs(timeout_secs),
            max_duration_ms: 30_000,
            max_bytes: 5 * 1024 * 1024,
            initial_prompt,
            threads,
            beam_size,
            best_of,
            no_fallback,
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

/// Normaliza códigos de idioma para o formato canônico esperado pelo Whisper.
/// Trata dialetos e códigos regionais (ex: 'pt-BR' -> 'pt', 'en-US' -> 'en') e maiúsculas.
pub fn normalize_language_code(lang: &str) -> String {
    let trimmed = lang.trim().to_lowercase();
    if trimmed.is_empty() || trimmed == "auto" {
        return "auto".to_string();
    }
    // Trata códigos regionais separados por hífen ou sublinhado
    if let Some((prefix, _)) = trimmed.split_once('-').or_else(|| trimmed.split_once('_')) {
        if !prefix.is_empty() {
            return prefix.to_string();
        }
    }
    trimmed
}

/// Verifica se um código de idioma informado é reconhecido pelo Whisper ou corresponde a 'auto'.
pub fn is_supported_language(lang: &str) -> bool {
    let norm = normalize_language_code(lang);
    if norm == "auto" {
        return true;
    }
    const KNOWN_LANGUAGES: &[&str] = &[
        "pt", "en", "es", "ja", "fr", "de", "it", "zh", "ru", "ko", "ar", "hi", "tr", "pl", "nl",
        "sv", "uk", "vi", "he", "el", "da", "fi", "cs", "id", "ro", "hu", "no", "th", "ca",
    ];
    KNOWN_LANGUAGES.contains(&norm.as_str())
        || (norm.len() == 2 && norm.chars().all(|c| c.is_ascii_alphabetic()))
}

/// Metadados diagnósticos extraídos dos registros (stderr) do processo Whisper.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct WhisperDetectionMetadata {
    /// Idioma detectado automaticamente pelo Whisper (se executado com `-l auto`).
    pub detected_language: Option<String>,
    /// Confiança estimada na detecção de idioma (0.0 a 1.0).
    pub probability: Option<f32>,
    /// Tempo de carregamento do modelo em milissegundos.
    pub load_time_ms: Option<f32>,
    /// Tempo de cálculo do Mel spectrogram em milissegundos.
    pub mel_time_ms: Option<f32>,
    /// Tempo de amostragem (sampling) em milissegundos.
    pub sample_time_ms: Option<f32>,
    /// Tempo do encoder em milissegundos.
    pub encode_time_ms: Option<f32>,
    /// Tempo do decoder em milissegundos.
    pub decode_time_ms: Option<f32>,
    /// Tempo de processamento do prompt em milissegundos.
    pub prompt_time_ms: Option<f32>,
    /// Tempo total de processamento em milissegundos reportado pelo whisper.cpp.
    pub total_time_ms: Option<f32>,
    /// Quantidade de fallbacks de temperatura acionados durante a decodificação.
    pub fallbacks: Option<u32>,
}

fn parse_ms_val(line: &str) -> Option<f32> {
    let idx = line.find('=')?;
    let s = line[idx + 1..].trim();
    let val_str = s.split_whitespace().next().unwrap_or("");
    val_str.parse::<f32>().ok()
}

/// Extrai idioma detectado, probabilidade e métricas de tempo do stderr do whisper.cpp.
pub fn parse_whisper_stderr(stderr: &str) -> WhisperDetectionMetadata {
    let mut meta = WhisperDetectionMetadata::default();

    for line in stderr.lines() {
        let line_trim = line.trim();

        // Linha típica de detecção: "whisper_full_with_state: auto-detected language: pt (p = 0.998412)"
        // ou variações: "auto-detected language: en (p = 0.606967)"
        if let Some(pos) = line_trim.find("auto-detected language:") {
            let rest = line_trim[pos + "auto-detected language:".len()..].trim();
            if let Some(space_or_paren) = rest.find(|c: char| c.is_whitespace() || c == '(') {
                let code = rest[..space_or_paren].trim();
                if !code.is_empty() {
                    meta.detected_language = Some(code.to_string());
                }
            } else if !rest.is_empty() {
                meta.detected_language = Some(rest.to_string());
            }

            if let Some(p_pos) = rest.find("p =") {
                let p_str = rest[p_pos + 3..].trim_start();
                let end = p_str
                    .find(|c: char| c == ')' || c.is_whitespace())
                    .unwrap_or(p_str.len());
                if let Ok(prob) = p_str[..end].parse::<f32>() {
                    meta.probability = Some(prob);
                }
            }
        } else if line_trim.contains("load time =") {
            if let Some(v) = parse_ms_val(line_trim) {
                meta.load_time_ms = Some(v);
            }
        } else if line_trim.contains("mel time =") {
            if let Some(v) = parse_ms_val(line_trim) {
                meta.mel_time_ms = Some(v);
            }
        } else if line_trim.contains("sample time =") {
            if let Some(v) = parse_ms_val(line_trim) {
                meta.sample_time_ms = Some(v);
            }
        } else if line_trim.contains("encode time =") {
            if let Some(v) = parse_ms_val(line_trim) {
                meta.encode_time_ms = Some(v);
            }
        } else if line_trim.contains("decode time =") {
            if let Some(v) = parse_ms_val(line_trim) {
                meta.decode_time_ms = Some(v);
            }
        } else if line_trim.contains("prompt time =") {
            if let Some(v) = parse_ms_val(line_trim) {
                meta.prompt_time_ms = Some(v);
            }
        } else if line_trim.contains("total time =") {
            if let Some(v) = parse_ms_val(line_trim) {
                meta.total_time_ms = Some(v);
            }
        } else if line_trim.contains("fallbacks =") {
            if let Some(idx) = line_trim.find('=') {
                let rest = line_trim[idx + 1..].trim();
                let val_str = rest.split_whitespace().next().unwrap_or("");
                if let Ok(fb) = val_str.parse::<u32>() {
                    meta.fallbacks = Some(fb);
                }
            }
        }
    }

    meta
}

/// Resultado detalhado e tipado de uma transcrição local com Whisper.
#[derive(Debug, Clone)]
pub struct LocalTranscriptionResult {
    /// Texto transcrito e sanitizado.
    pub text: String,
    /// Idioma solicitado/configurado para a transcrição (ex: "pt", "en", "auto").
    pub language_requested: String,
    /// Idioma detectado pelo Whisper quando executado em modo 'auto'.
    pub detected_language: Option<String>,
    /// Nível de confiança ou probabilidade do idioma detectado (0.0 a 1.0).
    pub probability: Option<f32>,
    /// Duração total decorrida medida pelo processo da Yuki.
    pub elapsed: Duration,
    /// Tempo de inferência computacional reportado pelo Whisper (se disponível).
    pub whisper_total_time_ms: Option<f32>,
    /// Caminho do modelo Whisper utilizado.
    pub model_path: PathBuf,
    /// Relatório diagnóstico detalhado de áudio (se presente na entrada).
    pub diagnostics: Option<crate::contracts::voice::AudioDiagnosticsReport>,
    /// Decomposição de latência medida diretamente pela Yuki e reportada pelo Whisper.
    pub timings: SttTimingMetrics,
    /// Resumo dos parâmetros de decodificação utilizados na execução.
    pub decoding_config: SttDecodingConfigSummary,
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

    /// Retorna referência imutável à configuração ativa do Whisper.
    pub fn config(&self) -> &LocalWhisperConfig {
        &self.config
    }

    /// Permite alterar dinamicamente o idioma da transcrição neste provedor.
    pub fn set_language(&mut self, language: impl Into<String>) {
        self.config.language = normalize_language_code(&language.into());
    }

    /// Executa o reconhecimento local de fala retornando metadados diagnósticos estruturados
    /// (incluindo idioma solicitado, idioma detectado pelo Whisper, confiança e tempos de inferência).
    pub async fn transcribe_with_metadata(
        &self,
        audio: &AudioInput,
    ) -> Result<LocalTranscriptionResult, VoiceError> {
        let start_instant = std::time::Instant::now();

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

        // 2. Decodifica o áudio WAV
        let t_prep_start = std::time::Instant::now();
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

        // 4. Converte para mono
        let mut mono_samples = AudioCodec::to_mono(&samples, format.channels);
        let audio_prep_ms = t_prep_start.elapsed().as_secs_f64() * 1000.0;

        let decoding_config_summary = SttDecodingConfigSummary {
            language_requested: self.config.language.clone(),
            prompt_configured: self.config.initial_prompt.is_some(),
            prompt_preview: self.config.initial_prompt.as_ref().map(|p| {
                if p.len() > 60 {
                    format!("{}...", &p[..57])
                } else {
                    p.clone()
                }
            }),
            threads: self.config.threads,
            beam_size: self.config.beam_size,
            best_of: self.config.best_of,
            no_fallback: self.config.no_fallback,
        };

        // 5. Curto-circuito econômico: verifica silêncio acústico ANTES da normalização.
        // Se verificado após a normalização, o piso de ruído seria artificialmente amplificado,
        // mascarando o silêncio e gerando alucinações acústicas no modelo.
        if is_silent_pcm(&mono_samples) {
            tracing::debug!(
                "Áudio identificado como silêncio acústico; transcrição concluída vazia."
            );
            let total_stt_wall_ms = start_instant.elapsed().as_secs_f64() * 1000.0;
            let timings = SttTimingMetrics {
                audio_prep_ms,
                total_stt_wall_ms,
                ..Default::default()
            };
            return Ok(LocalTranscriptionResult {
                text: String::new(),
                language_requested: self.config.language.clone(),
                detected_language: None,
                probability: None,
                elapsed: start_instant.elapsed(),
                whisper_total_time_ms: None,
                model_path: model_path.clone(),
                diagnostics: audio.diagnostics.clone(),
                timings,
                decoding_config: decoding_config_summary,
            });
        }

        // 6. Normaliza amplitude com margem de segurança contra saturação (clipping)
        let t_norm_start = std::time::Instant::now();
        AudioCodec::normalize_amplitude(&mut mono_samples, 28000);

        // Re-codifica áudio WAV normalizado para entrega ao processo do whisper
        let norm_format = crate::contracts::voice::AudioFormat {
            sample_rate: 16000,
            channels: 1,
            bits_per_sample: 16,
        };
        let normalized_wav = AudioCodec::encode_wav(&mono_samples, &norm_format)?;
        let audio_norm_ms = t_norm_start.elapsed().as_secs_f64() * 1000.0;

        let mut diag_opt = audio.diagnostics.clone();
        if let Some(ref mut diag) = diag_opt {
            let _ = crate::voice::diag::update_diagnostic_with_normalized(
                diag,
                &mono_samples,
                &normalized_wav,
            );
        }

        // 7. Cria arquivo temporário atômico para o áudio com alta entropia
        let t_write_start = std::time::Instant::now();
        let temp_dir = std::env::temp_dir();
        let temp_wav_path = temp_dir.join(format!("yuki_stt_{}.wav", generate_unique_id("stt")));

        std::fs::write(&temp_wav_path, &normalized_wav)
            .map_err(|e| VoiceError::Io(format!("Falha ao gravar WAV temporário: {}", e)))?;
        let temp_write_ms = t_write_start.elapsed().as_secs_f64() * 1000.0;

        // Limpa o arquivo temporário ao final da execução (Drop garantido)
        struct TempCleaner(PathBuf);
        impl Drop for TempCleaner {
            fn drop(&mut self) {
                let _ = std::fs::remove_file(&self.0);
            }
        }
        let _cleaner = TempCleaner(temp_wav_path.clone());

        // 8. Executa processo Whisper de forma assíncrona com controle de timeout e término garantido
        let t_sub_start = std::time::Instant::now();
        let timeout_duration = self.config.timeout;
        let mut cmd = tokio::process::Command::new(bin_path);
        cmd.arg("-m")
            .arg(model_path)
            .arg("-f")
            .arg(&temp_wav_path)
            .arg("-l")
            .arg(&self.config.language)
            .arg("--no-timestamps");

        if let Some(ref prompt) = self.config.initial_prompt {
            if !prompt.trim().is_empty() {
                cmd.arg("--prompt").arg(prompt);
            }
        }

        if let Some(threads) = self.config.threads {
            cmd.arg("-t").arg(threads.to_string());
        }

        if let Some(beam_size) = self.config.beam_size {
            cmd.arg("-bs").arg(beam_size.to_string());
        }

        if let Some(best_of) = self.config.best_of {
            cmd.arg("-bo").arg(best_of.to_string());
        }

        if self.config.no_fallback {
            cmd.arg("--no-fallback");
        }

        cmd.stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| {
            VoiceError::TranscriptionFailed(format!("Falha ao iniciar processo whisper: {}", e))
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

        let subprocess_wall_ms = t_sub_start.elapsed().as_secs_f64() * 1000.0;
        let stderr_str = String::from_utf8_lossy(&stderr_bytes);

        if !status.success() {
            return Err(VoiceError::TranscriptionFailed(format!(
                "Processo whisper finalizou com erro (código {:?}): {}",
                status.code(),
                stderr_str.trim()
            )));
        }

        let t_post_start = std::time::Instant::now();
        let raw_text = String::from_utf8_lossy(&stdout_bytes);
        let sanitized = sanitize_whisper_transcript(&raw_text);
        let post_process_ms = t_post_start.elapsed().as_secs_f64() * 1000.0;

        let elapsed = start_instant.elapsed();
        let total_stt_wall_ms = elapsed.as_secs_f64() * 1000.0;

        // Extrai metadados diagnósticos da saída stderr (detecção de idioma e timings)
        let detection_meta = parse_whisper_stderr(&stderr_str);

        let timings = SttTimingMetrics {
            audio_prep_ms,
            audio_norm_ms,
            temp_write_ms,
            subprocess_wall_ms,
            whisper_load_ms: detection_meta.load_time_ms,
            whisper_mel_ms: detection_meta.mel_time_ms,
            whisper_sample_ms: detection_meta.sample_time_ms,
            whisper_encode_ms: detection_meta.encode_time_ms,
            whisper_decode_ms: detection_meta.decode_time_ms,
            whisper_prompt_ms: detection_meta.prompt_time_ms,
            whisper_total_ms: detection_meta.total_time_ms,
            whisper_fallbacks: detection_meta.fallbacks,
            post_process_ms,
            total_stt_wall_ms,
        };

        if let Some(ref mut diag) = diag_opt {
            let _ = crate::voice::diag::update_diagnostic_with_stt(
                diag,
                &timings,
                &decoding_config_summary,
            );
        }

        tracing::info!(
            "Transcrição local Whisper concluída em {:?}: \"{}\" (solicitado: {}, detectado: {:?})",
            elapsed,
            sanitized,
            self.config.language,
            detection_meta.detected_language
        );

        Ok(LocalTranscriptionResult {
            text: sanitized,
            language_requested: self.config.language.clone(),
            detected_language: detection_meta.detected_language,
            probability: detection_meta.probability,
            elapsed,
            whisper_total_time_ms: detection_meta.total_time_ms,
            model_path: model_path.clone(),
            diagnostics: diag_opt,
            timings,
            decoding_config: decoding_config_summary,
        })
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
            let res = self.transcribe_with_metadata(audio).await?;
            Ok(res.text)
        })
    }
}
