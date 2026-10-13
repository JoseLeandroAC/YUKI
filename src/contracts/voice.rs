use crate::contracts::identifiers::now_utc;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Formato canônico de áudio para processamento de voz na Yuki.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioFormat {
    pub sample_rate: u32,
    pub channels: u16,
    pub bits_per_sample: u16,
}

impl Default for AudioFormat {
    fn default() -> Self {
        Self {
            sample_rate: 16000,
            channels: 1,
            bits_per_sample: 16,
        }
    }
}

/// Fragmento individual de áudio PCM.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioFrame {
    pub samples: Vec<i16>,
    pub sample_rate: u32,
    pub channels: u16,
    pub timestamp_ms: u64,
}

impl AudioFrame {
    pub fn new(samples: Vec<i16>, sample_rate: u32, channels: u16, timestamp_ms: u64) -> Self {
        Self {
            samples,
            sample_rate,
            channels,
            timestamp_ms,
        }
    }

    pub fn duration_ms(&self) -> u64 {
        if self.sample_rate == 0 || self.channels == 0 {
            return 0;
        }
        ((self.samples.len() as u64) * 1000) / (self.sample_rate as u64 * self.channels as u64)
    }

    pub fn is_silent(&self, threshold_rms: f32) -> bool {
        self.rms_energy() < threshold_rms
    }

    pub fn rms_energy(&self) -> f32 {
        if self.samples.is_empty() {
            return 0.0;
        }
        let sum_sq: f64 = self.samples.iter().map(|&s| (s as f64) * (s as f64)).sum();
        let mean = sum_sq / (self.samples.len() as f64);
        mean.sqrt() as f32
    }
}

/// Detalhes do formato de áudio nativo reportado pelo driver do dispositivo no Windows/WASAPI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeAudioFormatInfo {
    pub format_tag: u16,
    pub format_tag_hex: String,
    pub format_tag_name: String,
    pub is_extensible: bool,
    pub is_float: bool,
    pub channels: u16,
    pub sample_rate: u32,
    pub bits_per_sample: u16,
    pub block_align: u16,
    pub avg_bytes_per_sec: u32,
    pub sub_format_guid: Option<String>,
}

/// Métricas de ciclo de vida e temporização da captura de áudio.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureTimingMetrics {
    pub requested_duration_ms: u64,
    pub init_duration_ms: u64,
    pub effective_capture_ms: u64,
    pub samples_duration_ms: u64,
}

/// Métricas de pacotes e buffers recebidos do subsistema WASAPI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BufferMetrics {
    pub total_packets: usize,
    pub total_raw_frames: usize,
    pub total_raw_bytes: usize,
    pub silent_buffer_events: usize,
    pub discontinuity_glitches: usize,
}

/// Métricas acústicas calculadas sobre o buffer de amostras.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AcousticMetrics {
    pub sample_count: usize,
    pub rms: f32,
    pub rms_dbfs: f32,
    pub peak: i16,
    pub peak_dbfs: f32,
    pub clipping_samples: usize,
    pub clipping_ratio_pct: f32,
    pub silence_ratio_pct: f32,
}

/// Métricas granulares de temporização e decomposição de latência do STT.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SttTimingMetrics {
    /// Tempo de decodificação do áudio de entrada e conversão para mono (ms)
    pub audio_prep_ms: f64,
    /// Tempo de normalização de amplitude e re-encoding do WAV (ms)
    pub audio_norm_ms: f64,
    /// Tempo de gravação do arquivo temporário em disco (ms)
    pub temp_write_ms: f64,
    /// Tempo total de execução do subprocesso whisper-cli (spawn até término) medido pela Yuki (ms)
    pub subprocess_wall_ms: f64,
    /// Tempo de carregamento do modelo informado pelo Whisper (ms), se disponível
    pub whisper_load_ms: Option<f32>,
    /// Tempo de processamento do Mel spectrogram informado pelo Whisper (ms), se disponível
    pub whisper_mel_ms: Option<f32>,
    /// Tempo de amostragem (sampling) informado pelo Whisper (ms), se disponível
    pub whisper_sample_ms: Option<f32>,
    /// Tempo do encoder informado pelo Whisper (ms), se disponível
    pub whisper_encode_ms: Option<f32>,
    /// Tempo do decoder informado pelo Whisper (ms), se disponível
    pub whisper_decode_ms: Option<f32>,
    /// Tempo do prompt processing informado pelo Whisper (ms), se disponível
    pub whisper_prompt_ms: Option<f32>,
    /// Tempo total de processamento reportado pelo Whisper no stderr (ms), se disponível
    pub whisper_total_ms: Option<f32>,
    /// Número de fallbacks de temperatura acionados durante a decodificação
    pub whisper_fallbacks: Option<u32>,
    /// Tempo de sanitização e higienização do texto transcrito (ms)
    pub post_process_ms: f64,
    /// Latência total de ponta a ponta do STT (da entrada de áudio à entrega do texto) (ms)
    pub total_stt_wall_ms: f64,
}

/// Resumo da configuração de decodificação utilizada na transcrição.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SttDecodingConfigSummary {
    pub language_requested: String,
    pub prompt_configured: bool,
    pub prompt_preview: Option<String>,
    pub threads: Option<u32>,
    pub beam_size: Option<u32>,
    pub best_of: Option<u32>,
    pub no_fallback: bool,
}

/// Estratégia de reamostragem de áudio aplicada na cadeia de captura e conversão.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResamplerKind {
    /// Reamostragem linear direta (baseline histórico do Marco 8.1.1).
    LinearBaseline,
    /// Reamostragem com filtro passa-baixa FIR antialiasing de fase linear (simétrico Type I).
    #[default]
    FirAntialiasing,
}

impl fmt::Display for ResamplerKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LinearBaseline => write!(f, "linear_baseline"),
            Self::FirAntialiasing => write!(f, "fir_antialiasing"),
        }
    }
}

impl std::str::FromStr for ResamplerKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().trim() {
            "linear" | "baseline" | "linear_baseline" => Ok(Self::LinearBaseline),
            "fir" | "antialiasing" | "fir_antialiasing" => Ok(Self::FirAntialiasing),
            other => Err(format!(
                "Estratégia de reamostragem desconhecida: '{}'. Válidas: 'linear_baseline', 'fir_antialiasing'",
                other
            )),
        }
    }
}

/// Relatório estruturado de diagnóstico de áudio físico e inferência.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioDiagnosticsReport {
    pub session_id: String,
    pub timestamp: String,
    pub device_id: Option<String>,
    pub device_name: Option<String>,
    pub native_format: NativeAudioFormatInfo,
    pub timings: CaptureTimingMetrics,
    pub buffer_metrics: BufferMetrics,
    pub acoustic_canonical: AcousticMetrics,
    pub acoustic_normalized: Option<AcousticMetrics>,
    #[serde(default)]
    pub stt_timings: Option<SttTimingMetrics>,
    #[serde(default)]
    pub stt_config: Option<SttDecodingConfigSummary>,
    #[serde(default)]
    pub resampler_kind: Option<ResamplerKind>,
    pub raw_wav_path: Option<String>,
    pub canonical_wav_path: Option<String>,
    pub normalized_wav_path: Option<String>,
    pub report_path: Option<String>,
}

/// Áudio de entrada para reconhecimento (STT).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioInput {
    pub payload: Vec<u8>,
    pub format: AudioFormat,
    pub duration_ms: u64,
    pub source: String,
    pub captured_at: DateTime<Utc>,
    #[serde(default)]
    pub diagnostics: Option<AudioDiagnosticsReport>,
}

impl AudioInput {
    pub fn new(
        payload: Vec<u8>,
        format: AudioFormat,
        duration_ms: u64,
        source: impl Into<String>,
    ) -> Self {
        Self {
            payload,
            format,
            duration_ms,
            source: source.into(),
            captured_at: now_utc(),
            diagnostics: None,
        }
    }

    pub fn with_diagnostics(mut self, diag: AudioDiagnosticsReport) -> Self {
        self.diagnostics = Some(diag);
        self
    }

    pub fn byte_size(&self) -> usize {
        self.payload.len()
    }
}

/// Áudio de saída gerado por síntese (TTS).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioOutput {
    pub payload: Vec<u8>,
    pub format: AudioFormat,
    pub duration_ms: u64,
    pub voice_name: String,
    pub synthesized_at: DateTime<Utc>,
}

impl AudioOutput {
    pub fn new(
        payload: Vec<u8>,
        format: AudioFormat,
        duration_ms: u64,
        voice_name: impl Into<String>,
    ) -> Self {
        Self {
            payload,
            format,
            duration_ms,
            voice_name: voice_name.into(),
            synthesized_at: now_utc(),
        }
    }

    pub fn byte_size(&self) -> usize {
        self.payload.len()
    }
}

/// Classificação do provedor de voz (soberania e privacidade).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VoiceProviderType {
    /// Executado 100% em CPU/GPU local, sem qualquer transmissão externa.
    LocalEdge,
    /// Provedor externo acessado sob autorização explícita e Credential Vault.
    GovernedCloud,
    /// Mock para testes determinísticos isolados.
    Mock,
}

/// Metadados descritivos de um provedor de STT ou TTS.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceProviderMetadata {
    pub provider_name: String,
    pub provider_type: VoiceProviderType,
    pub supported_languages: Vec<String>,
    pub sample_rate: u32,
    pub supports_streaming: bool,
    pub version: String,
}

impl VoiceProviderMetadata {
    pub fn new(
        provider_name: impl Into<String>,
        provider_type: VoiceProviderType,
        supported_languages: Vec<String>,
        sample_rate: u32,
        supports_streaming: bool,
        version: impl Into<String>,
    ) -> Self {
        Self {
            provider_name: provider_name.into(),
            provider_type,
            supported_languages,
            sample_rate,
            supports_streaming,
            version: version.into(),
        }
    }

    pub fn is_offline(&self) -> bool {
        matches!(
            self.provider_type,
            VoiceProviderType::LocalEdge | VoiceProviderType::Mock
        )
    }
}

/// Configurações gerais do subsistema de voz da Yuki.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceConfiguration {
    pub language: String,
    pub sample_rate: u32,
    pub max_duration_seconds: u32,
    pub max_bytes: usize,
    pub wake_word_enabled: bool,
    pub wake_word: String,
    pub wake_word_sensitivity: f32,
    pub cloud_fallback_allowed: bool,
    pub local_first: bool,
    pub voice_name: String,
    pub speaking_rate: f32,
    pub pitch: f32,
    pub session_timeout_secs: u64,
}

impl Default for VoiceConfiguration {
    fn default() -> Self {
        Self {
            language: "pt-BR".to_string(),
            sample_rate: 16000,
            max_duration_seconds: 30,
            max_bytes: 2 * 1024 * 1024, // 2 MiB
            wake_word_enabled: true,
            wake_word: "yuki".to_string(),
            wake_word_sensitivity: 0.5,
            cloud_fallback_allowed: false, // Default estritamente seguro: sem cloud por padrão
            local_first: true,
            voice_name: "yuki-default".to_string(),
            speaking_rate: 1.0,
            pitch: 1.0,
            session_timeout_secs: 15,
        }
    }
}

/// Estados formais da sessão conversacional de voz.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VoiceSessionState {
    Idle,
    WakeListening,
    Capturing,
    Transcribing,
    Processing,
    Synthesizing,
    Speaking,
    Interrupted,
    Failed(String),
}

impl fmt::Display for VoiceSessionState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Idle => write!(f, "Idle"),
            Self::WakeListening => write!(f, "WakeListening"),
            Self::Capturing => write!(f, "Capturing"),
            Self::Transcribing => write!(f, "Transcribing"),
            Self::Processing => write!(f, "Processing"),
            Self::Synthesizing => write!(f, "Synthesizing"),
            Self::Speaking => write!(f, "Speaking"),
            Self::Interrupted => write!(f, "Interrupted"),
            Self::Failed(err) => write!(f, "Failed: {}", err),
        }
    }
}

impl VoiceSessionState {
    /// Determina se uma transição de estado é formalmente válida.
    pub fn can_transition_to(&self, next: &VoiceSessionState) -> bool {
        use VoiceSessionState::*;
        match (self, next) {
            // Qualquer estado pode ir para Failed, Interrupted ou Idle (reset)
            (_, Failed(_)) | (_, Interrupted) | (_, Idle) => true,

            // De Idle podemos iniciar escuta por palavra-chave ou captura direta por botão
            (Idle, WakeListening) => true,
            (Idle, Capturing) => true,

            // De WakeListening para Capturing quando palavra-chave é detectada
            (WakeListening, Capturing) => true,

            // De Capturing para Transcribing ao encerrar a fala
            (Capturing, Transcribing) => true,

            // De Transcribing para Processing ao produzir texto reconhecido
            (Transcribing, Processing) => true,

            // De Processing para Synthesizing ao produzir texto de resposta
            (Processing, Synthesizing) => true,

            // De Synthesizing para Speaking ao produzir áudio
            (Synthesizing, Speaking) => true,

            // De Speaking de volta para Idle (conclusão) ou WakeListening (modo conversacional contínuo)
            (Speaking, WakeListening) => true,

            // De Interrupted para Idle ou WakeListening
            (Interrupted, WakeListening) => true,

            // Qualquer outra transição é inválida
            _ => false,
        }
    }
}

/// Erros tipados do subsistema de voz.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VoiceError {
    #[error("Formato de áudio inválido: {0}")]
    InvalidAudioFormat(String),

    #[error("Áudio excede duração máxima permitida: {duration_ms}ms (máx: {max_ms}ms)")]
    AudioTooLong { duration_ms: u64, max_ms: u64 },

    #[error("Áudio excede limite de memória: {bytes} bytes (máx: {max_bytes} bytes)")]
    AudioTooLarge { bytes: usize, max_bytes: usize },

    #[error("Provedor de voz indisponível: {0}")]
    ProviderUnavailable(String),

    #[error("Falha no reconhecimento de fala (STT): {0}")]
    TranscriptionFailed(String),

    #[error("Falha na síntese de fala (TTS): {0}")]
    SynthesisFailed(String),

    #[error("Falha na captura de áudio: {0}")]
    CaptureFailed(String),

    #[error("Falha na reprodução de áudio: {0}")]
    PlaybackFailed(String),

    #[error("Dispositivo de áudio indisponível ou inacessível: {0}")]
    DeviceUnavailable(String),

    #[error("Transição de estado inválida: de {from} para {to}")]
    InvalidStateTransition { from: String, to: String },

    #[error("Conflito de sessão de voz ativa no dispositivo: {0}")]
    SessionConflict(String),

    #[error("Fallback em nuvem não autorizado pela política de segurança: {0}")]
    UnauthorizedCloudFallback(String),

    #[error("Operação de voz expirou por timeout ({0})")]
    Timeout(String),

    #[error("Operação de voz foi interrompida")]
    Interrupted,

    #[error("Operação de voz foi cancelada")]
    Cancelled,

    #[error("Erro de E/S no subsistema de voz: {0}")]
    Io(String),
}

use std::future::Future;
use std::pin::Pin;

/// Contrato para provedores de Reconhecimento de Fala (STT).
pub trait SpeechToTextProvider: Send + Sync {
    fn metadata(&self) -> &VoiceProviderMetadata;
    fn health<'a>(&'a self) -> Pin<Box<dyn Future<Output = Result<(), VoiceError>> + Send + 'a>>;
    fn transcribe<'a>(
        &'a self,
        audio: &'a AudioInput,
    ) -> Pin<Box<dyn Future<Output = Result<String, VoiceError>> + Send + 'a>>;
}

/// Contrato para provedores de Síntese de Fala (TTS).
pub trait TextToSpeechProvider: Send + Sync {
    fn metadata(&self) -> &VoiceProviderMetadata;
    fn health<'a>(&'a self) -> Pin<Box<dyn Future<Output = Result<(), VoiceError>> + Send + 'a>>;
    fn synthesize<'a>(
        &'a self,
        text: &'a str,
        voice_params: Option<&'a VoiceConfiguration>,
    ) -> Pin<Box<dyn Future<Output = Result<AudioOutput, VoiceError>> + Send + 'a>>;
}
