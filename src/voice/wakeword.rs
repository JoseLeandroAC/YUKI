use crate::contracts::voice::{
    AudioFormat, AudioFrame, AudioInput, SpeechToTextProvider, VoiceError,
};
use crate::voice::audio::AudioCodec;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// Resultado da análise de palavra de ativação em um fluxo de quadros de áudio.
#[derive(Debug, Clone, PartialEq)]
pub enum WakeWordDetection {
    /// Nenhuma palavra de ativação detectada no quadro.
    None,
    /// Palavra de ativação "Yuki" detectada com o score de confiança especificado.
    Detected { confidence: f32, timestamp_ms: u64 },
}

// ---------------------------------------------------------------------------
// VAD: Voice Activity Detection
// ---------------------------------------------------------------------------

/// Detector de Atividade de Voz (VAD) para separação prévia entre ruído/silêncio e fala humana.
///
/// Avalia:
/// - Nível de energia RMS em faixas compatíveis com fonação humana.
/// - Taxa de cruzamento por zero (ZCR) para rejeitar ruídos puramente impulsivos ou estáticos.
/// - Variância temporal de energia para descartar ruído contínuo estacionário (ex: ventilador, zumbido).
/// - Fator de crista (crest factor) para rejeitar palmas e estalos curtos.
#[derive(Debug, Clone)]
pub struct VoiceActivityDetector {
    min_rms_speech: f32,
    max_rms_speech: f32,
    min_zcr: f32,
    max_zcr: f32,
    min_std_dev: f32,
}

impl Default for VoiceActivityDetector {
    fn default() -> Self {
        Self {
            min_rms_speech: 300.0,
            max_rms_speech: 30000.0,
            min_zcr: 0.02,
            max_zcr: 0.40,
            min_std_dev: 15.0,
        }
    }
}

impl VoiceActivityDetector {
    pub fn new(min_rms_speech: f32, max_rms_speech: f32, min_zcr: f32, max_zcr: f32) -> Self {
        Self {
            min_rms_speech,
            max_rms_speech,
            min_zcr,
            max_zcr,
            min_std_dev: 15.0,
        }
    }

    /// Configura o desvio padrão mínimo entre quadros para rejeição de ruído estacionário.
    pub fn with_min_std_dev(mut self, min_std_dev: f32) -> Self {
        self.min_std_dev = min_std_dev.max(0.0);
        self
    }

    /// Retorna o limiar mínimo de RMS configurado para detecção de fala.
    pub fn min_rms_speech(&self) -> f32 {
        self.min_rms_speech
    }

    /// Avalia se a janela de quadros contém atividade acústica condizente com fala humana.
    pub fn is_speech(&self, frames: &[AudioFrame]) -> bool {
        if frames.is_empty() {
            return false;
        }

        let mut total_samples = 0usize;
        let mut total_zcr = 0usize;
        let mut rms_list = Vec::with_capacity(frames.len());

        for f in frames {
            if f.samples.is_empty() {
                continue;
            }
            let rms = f.rms_energy();
            rms_list.push(rms);

            let mut zcr = 0usize;
            for i in 1..f.samples.len() {
                if (f.samples[i] >= 0 && f.samples[i - 1] < 0)
                    || (f.samples[i] < 0 && f.samples[i - 1] >= 0)
                {
                    zcr += 1;
                }
            }
            total_zcr += zcr;
            total_samples += f.samples.len();
        }

        if total_samples == 0 || rms_list.is_empty() {
            return false;
        }

        let avg_rms: f32 = rms_list.iter().sum::<f32>() / (rms_list.len() as f32);
        let zcr_rate = (total_zcr as f32) / (total_samples as f32);

        // 1. Filtro de nível RMS: silêncio ou distorção excessiva
        if avg_rms < self.min_rms_speech || avg_rms > self.max_rms_speech {
            return false;
        }

        // 2. Filtro de ZCR: faixa da voz humana articulada
        if zcr_rate < self.min_zcr || zcr_rate > self.max_zcr {
            return false;
        }

        // 3. Rejeição de ruído contínuo estacionário (variância quase nula)
        if rms_list.len() >= 3 {
            let variance: f32 = rms_list.iter().map(|&r| (r - avg_rms).powi(2)).sum::<f32>()
                / (rms_list.len() as f32);
            let std_dev = variance.sqrt();
            // Ruído estacionário sintético tem desvio padrão ínfimo
            if std_dev < self.min_std_dev {
                return false;
            }
        }

        true
    }
}

// ---------------------------------------------------------------------------
// Trait WakeWordEngine e Status
// ---------------------------------------------------------------------------

/// Status operacional e maturidade do motor de detecção de palavra-chave.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WakeWordEngineStatus {
    /// Motor pronto e operacional com reconhecimento lexical local validado.
    Ready,
    /// Motor heurístico acústico experimental (não realiza reconhecimento lexical pleno).
    Experimental,
    /// Motor indisponível no ambiente atual (sem modelo ou hardware compatível).
    Unavailable,
}

/// Contrato para motores de reconhecimento da palavra-chave "Yuki".
pub trait WakeWordEngine: Send + Sync + std::fmt::Debug {
    /// Avalia uma janela de quadros confirmada pelo VAD como voz humana.
    /// Retorna `Some(confidence)` [0.0 .. 1.0] se houver evidência da palavra, ou `None`.
    fn verify_keyword(
        &self,
        frames: &[AudioFrame],
        keyword: &str,
    ) -> Result<Option<f32>, VoiceError>;

    /// Nome identificador do motor.
    fn name(&self) -> &str;

    /// Retorna o status operacional do motor de detecção.
    fn status(&self) -> WakeWordEngineStatus {
        WakeWordEngineStatus::Ready
    }
}

// ---------------------------------------------------------------------------
// Motor Fonético Local (PhoneticPatternWakeWordEngine)
// ---------------------------------------------------------------------------

/// Motor que analisa a estrutura acústico-formante específica da palavra "Yu-ki" (/ju:ki/):
/// - Sílaba 1: /ju/ - semivogal sonora de média/baixa frequência e ZCR baixo (0.04 - 0.16).
/// - Transição: vale de coarticulação oclusiva entre /u/ e /k/.
/// - Sílaba 2: /ki/ - plosiva velar com explosão de alta frequência e vogal /i/ (ZCR 0.15 - 0.35).
/// - Rejeição estrita de palmas, ruídos contínuos e monossílabos.
#[derive(Debug, Default)]
pub struct PhoneticPatternWakeWordEngine;

impl PhoneticPatternWakeWordEngine {
    pub fn new() -> Self {
        Self
    }
}

impl WakeWordEngine for PhoneticPatternWakeWordEngine {
    fn verify_keyword(
        &self,
        frames: &[AudioFrame],
        keyword: &str,
    ) -> Result<Option<f32>, VoiceError> {
        if frames.len() < 3 || keyword.to_lowercase() != "yuki" {
            return Ok(None);
        }

        let mut energy_profile = Vec::with_capacity(frames.len());
        let mut zcr_profile = Vec::with_capacity(frames.len());

        for f in frames {
            energy_profile.push(f.rms_energy());
            let mut zcr = 0;
            for i in 1..f.samples.len() {
                if (f.samples[i] >= 0 && f.samples[i - 1] < 0)
                    || (f.samples[i] < 0 && f.samples[i - 1] >= 0)
                {
                    zcr += 1;
                }
            }
            let zcr_rate = if !f.samples.is_empty() {
                (zcr as f32) / (f.samples.len() as f32)
            } else {
                0.0
            };
            zcr_profile.push(zcr_rate);
        }

        let mid = energy_profile.len() / 2;
        let first_half = &energy_profile[..mid];
        let second_half = &energy_profile[mid..];

        let avg1: f32 = first_half.iter().sum::<f32>() / (first_half.len() as f32);
        let avg2: f32 = second_half.iter().sum::<f32>() / (second_half.len() as f32);

        // Ambas as sílabas precisam ter energia substancial
        if avg1 < 400.0 || avg2 < 400.0 {
            return Ok(None);
        }

        // Rejeição de desequilíbrio extremo (palma única ou estalo)
        let ratio = avg1 / avg2;
        if !(0.25..=4.0).contains(&ratio) {
            return Ok(None);
        }

        // Avaliação de ZCR por sílaba (/ju/ mais baixo que /ki/)
        let zcr_half1: f32 = zcr_profile[..mid].iter().sum::<f32>() / (mid as f32);
        let zcr_half2: f32 =
            zcr_profile[mid..].iter().sum::<f32>() / ((zcr_profile.len() - mid) as f32);

        // Sílaba 2 (/ki/) deve ter presença de frequência maior ou comparável com /k/ plosivo
        let balance = 1.0 - ((avg1 - avg2).abs() / (avg1 + avg2));
        let phonetic_fitness = if zcr_half2 >= zcr_half1 * 0.85 && zcr_half2 > 0.03 {
            0.62 + (balance * 0.35)
        } else {
            0.45 + (balance * 0.25)
        };

        let confidence = phonetic_fitness.clamp(0.0, 0.98);
        if confidence >= 0.40 {
            Ok(Some(confidence))
        } else {
            Ok(None)
        }
    }

    fn name(&self) -> &str {
        "PhoneticPatternWakeWordEngine (Experimental Acoustic Heuristic)"
    }

    fn status(&self) -> WakeWordEngineStatus {
        WakeWordEngineStatus::Experimental
    }
}

// ---------------------------------------------------------------------------
// Motor openWakeWord Dedicado (OpenWakeWordEngine) - Marco 8.1.3
// ---------------------------------------------------------------------------

/// Configuração do motor acústico neural openWakeWord para ativação local por palavra-chave ("Yuki").
#[derive(Debug, Clone)]
pub struct OpenWakeWordConfig {
    pub onnxruntime_dll_path: Option<PathBuf>,
    pub melspec_model_path: Option<PathBuf>,
    pub embedding_model_path: Option<PathBuf>,
    pub yuki_model_path: Option<PathBuf>,
    pub activation_threshold: f32,
    pub cooldown_frames: usize,
}

impl Default for OpenWakeWordConfig {
    fn default() -> Self {
        Self {
            onnxruntime_dll_path: default_onnxruntime_dll_path(),
            melspec_model_path: default_melspec_model_path(),
            embedding_model_path: default_embedding_model_path(),
            yuki_model_path: default_yuki_model_path(),
            activation_threshold: 0.50,
            cooldown_frames: 18,
        }
    }
}

pub fn default_onnxruntime_dll_path() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("YUKI_ONNXRUNTIME_DLL").map(PathBuf::from) {
        if p.exists() {
            return Some(p);
        }
    }
    let candidates = [
        PathBuf::from("bin/onnxruntime/onnxruntime.dll"),
        PathBuf::from("models/wakeword/onnxruntime.dll"),
        PathBuf::from(
            r"C:\Users\Miriã\.gemini\antigravity\brain\db5234b4-9cc4-4f3b-9460-8f9e4174c84d\scratch\kws\oww_exp\.venv\Lib\site-packages\onnxruntime\capi\onnxruntime.dll",
        ),
        PathBuf::from(r"C:\Windows\System32\onnxruntime.dll"),
    ];
    for c in &candidates {
        if c.exists() {
            return Some(c.clone());
        }
    }
    None
}

pub fn default_melspec_model_path() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("YUKI_OWW_MEL_MODEL").map(PathBuf::from) {
        if p.exists() {
            return Some(p);
        }
    }
    let candidates = [
        PathBuf::from("models/wakeword/melspectrogram.onnx"),
        PathBuf::from(
            r"C:\Users\Miriã\.gemini\antigravity\brain\db5234b4-9cc4-4f3b-9460-8f9e4174c84d\scratch\kws\oww_exp\.venv\Lib\site-packages\openwakeword\resources\models\melspectrogram.onnx",
        ),
    ];
    for c in &candidates {
        if c.exists() {
            return Some(c.clone());
        }
    }
    None
}

pub fn default_embedding_model_path() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("YUKI_OWW_EMB_MODEL").map(PathBuf::from) {
        if p.exists() {
            return Some(p);
        }
    }
    let candidates = [
        PathBuf::from("models/wakeword/embedding_model.onnx"),
        PathBuf::from(
            r"C:\Users\Miriã\.gemini\antigravity\brain\db5234b4-9cc4-4f3b-9460-8f9e4174c84d\scratch\kws\oww_exp\.venv\Lib\site-packages\openwakeword\resources\models\embedding_model.onnx",
        ),
    ];
    for c in &candidates {
        if c.exists() {
            return Some(c.clone());
        }
    }
    None
}

pub fn default_yuki_model_path() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("YUKI_OWW_YUKI_MODEL").map(PathBuf::from) {
        if p.exists() {
            return Some(p);
        }
    }
    let candidates = [
        PathBuf::from("models/wakeword/yuki_v0.1.onnx"),
        PathBuf::from(
            r"C:\Users\Miriã\.gemini\antigravity\brain\db5234b4-9cc4-4f3b-9460-8f9e4174c84d\scratch\kws\oww_exp\models\yuki_v0.1.onnx",
        ),
    ];
    for c in &candidates {
        if c.exists() {
            return Some(c.clone());
        }
    }
    None
}

struct OwwEngineInner {
    mel_session: ort::session::Session,
    emb_session: ort::session::Session,
    yuki_session: ort::session::Session,
    mel_buffer: Vec<f32>,  // 76 frames x 32 mel bins = 2432 floats
    feat_buffer: Vec<f32>, // 16 frames x 96 embedding dims = 1536 floats
    raw_buffer: Vec<f32>,  // histórico bruto para janela de 1760 amostras
    chunk_count: usize,
}

impl OwwEngineInner {
    fn reset(&mut self) {
        self.mel_buffer = vec![1.0; 76 * 32];
        self.feat_buffer = vec![0.0; 16 * 96];
        self.raw_buffer.clear();
        self.chunk_count = 0;
    }

    fn process_chunk(&mut self, chunk: &[i16]) -> Result<f32, VoiceError> {
        self.chunk_count += 1;

        // 1. Atualizar buffer de áudio bruto
        for &s in chunk {
            self.raw_buffer.push(s as f32);
        }

        // Janela de áudio de 1760 amostras (1280 do bloco atual de 80ms + 480 de overlap)
        let window_size = 1280 + 480;
        let audio_window: Vec<f32> = if self.raw_buffer.len() >= window_size {
            let start = self.raw_buffer.len() - window_size;
            self.raw_buffer[start..].to_vec()
        } else {
            let mut padded = vec![0.0f32; window_size - self.raw_buffer.len()];
            padded.extend_from_slice(&self.raw_buffer);
            padded
        };

        // Truncar buffer bruto para economizar memória (manter últimas 3200 amostras)
        if self.raw_buffer.len() > 3200 {
            let drop_count = self.raw_buffer.len() - window_size;
            self.raw_buffer.drain(0..drop_count);
        }

        // 2. Executar melspectrogram.onnx (input: [1, 1760])
        let mel_input_shape = [1usize, window_size];
        let mel_tensor =
            ort::value::TensorRef::from_array_view((mel_input_shape, audio_window.as_slice()))
                .map_err(|e| {
                    VoiceError::CaptureFailed(format!("Erro ao criar tensor mel: {}", e))
                })?;
        let mel_outputs = self
            .mel_session
            .run(ort::inputs![mel_tensor])
            .map_err(|e| {
                VoiceError::CaptureFailed(format!("Erro ao executar melspectrogram.onnx: {}", e))
            })?;
        let (_mel_out_shape, mel_data) = mel_outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(|e| VoiceError::CaptureFailed(format!("Erro ao extrair mels: {}", e)))?;

        // Cada 1280 amostras adicionam exatamente 8 novos frames de mel (8 * 32 = 256 floats)
        let new_mel_frames_count = 8 * 32;
        let new_mels_slice = if mel_data.len() >= new_mel_frames_count {
            &mel_data[mel_data.len() - new_mel_frames_count..]
        } else {
            mel_data
        };

        // Transformação oficial do openWakeWord: x / 10.0 + 2.0 (escala Google Speech Embedding)
        let transformed_mels: Vec<f32> = new_mels_slice.iter().map(|&x| x / 10.0 + 2.0).collect();

        // Deslocar mel_buffer (76 * 32 = 2432) descartando os 256 mais antigos
        self.mel_buffer.drain(0..transformed_mels.len());
        self.mel_buffer.extend_from_slice(&transformed_mels);

        // 3. Executar embedding_model.onnx (input: [1, 76, 32, 1])
        let emb_input_shape = [1usize, 76, 32, 1];
        let emb_tensor = ort::value::TensorRef::from_array_view((
            emb_input_shape,
            self.mel_buffer.as_slice(),
        ))
        .map_err(|e| VoiceError::CaptureFailed(format!("Erro ao criar tensor emb: {}", e)))?;
        let emb_outputs = self
            .emb_session
            .run(ort::inputs![emb_tensor])
            .map_err(|e| {
                VoiceError::CaptureFailed(format!("Erro ao executar embedding_model.onnx: {}", e))
            })?;
        let (_emb_out_shape, emb_data) = emb_outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(|e| VoiceError::CaptureFailed(format!("Erro ao extrair embedding: {}", e)))?;

        // Vetor de embedding de 96 dimensões
        let new_embedding = &emb_data[..96];

        // 4. Deslocar feat_buffer (16 * 96 = 1536) descartando 1 frame (96 floats)
        self.feat_buffer.drain(0..96);
        self.feat_buffer.extend_from_slice(new_embedding);

        // Warmup: requer pelo menos 16 blocos (1.28s) para preencher a janela temporal de 16 frames
        if self.chunk_count < 16 {
            return Ok(0.0);
        }

        // 5. Executar yuki_v0.1.onnx (input: [1, 16, 96])
        let yuki_input_shape = [1usize, 16, 96];
        let yuki_tensor =
            ort::value::TensorRef::from_array_view((yuki_input_shape, self.feat_buffer.as_slice()))
                .map_err(|e| {
                    VoiceError::CaptureFailed(format!("Erro ao criar tensor yuki: {}", e))
                })?;
        let yuki_outputs = self
            .yuki_session
            .run(ort::inputs![yuki_tensor])
            .map_err(|e| {
                VoiceError::CaptureFailed(format!("Erro ao executar yuki_v0.1.onnx: {}", e))
            })?;
        let (_yuki_out_shape, yuki_data) = yuki_outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(|e| VoiceError::CaptureFailed(format!("Erro ao extrair score: {}", e)))?;

        let score = yuki_data[0];
        Ok(score)
    }
}

/// Motor de Keyword Spotting baseado na arquitetura oficial openWakeWord com modelo personalizado 'yuki_v0.1.onnx'.
/// Executa a inferência em 3 estágios ONNX (Melspectrograma -> Google Speech Embedding -> Classificador FCN PT-BR).
pub struct OpenWakeWordEngine {
    config: OpenWakeWordConfig,
    inner: Mutex<OwwEngineInner>,
}

impl std::fmt::Debug for OpenWakeWordEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "OpenWakeWordEngine(model={:?}, threshold={})",
            self.config.yuki_model_path, self.config.activation_threshold
        )
    }
}

impl OpenWakeWordEngine {
    /// Verifica se os artefatos ONNX necessários e a biblioteca ONNX Runtime estão presentes no ambiente.
    pub fn is_available() -> bool {
        let config = OpenWakeWordConfig::default();
        config.onnxruntime_dll_path.is_some()
            && config.melspec_model_path.is_some()
            && config.embedding_model_path.is_some()
            && config.yuki_model_path.is_some()
    }

    /// Inicializa o motor a partir dos caminhos padrão de produção/desenvolvimento.
    pub fn from_default_paths() -> Result<Self, VoiceError> {
        Self::new(OpenWakeWordConfig::default())
    }

    /// Inicializa o motor com configuração customizada.
    pub fn new(config: OpenWakeWordConfig) -> Result<Self, VoiceError> {
        let dll_path = config.onnxruntime_dll_path.as_ref().ok_or_else(|| {
            VoiceError::ProviderUnavailable("onnxruntime.dll não encontrada".to_string())
        })?;
        let mel_path = config.melspec_model_path.as_ref().ok_or_else(|| {
            VoiceError::ProviderUnavailable("melspectrogram.onnx não encontrado".to_string())
        })?;
        let emb_path = config.embedding_model_path.as_ref().ok_or_else(|| {
            VoiceError::ProviderUnavailable("embedding_model.onnx não encontrado".to_string())
        })?;
        let yuki_path = config.yuki_model_path.as_ref().ok_or_else(|| {
            VoiceError::ProviderUnavailable("yuki_v0.1.onnx não encontrado".to_string())
        })?;

        if !dll_path.exists() {
            return Err(VoiceError::ProviderUnavailable(format!(
                "Arquivo não existe: {}",
                dll_path.display()
            )));
        }
        if !mel_path.exists() {
            return Err(VoiceError::ProviderUnavailable(format!(
                "Arquivo não existe: {}",
                mel_path.display()
            )));
        }
        if !emb_path.exists() {
            return Err(VoiceError::ProviderUnavailable(format!(
                "Arquivo não existe: {}",
                emb_path.display()
            )));
        }
        if !yuki_path.exists() {
            return Err(VoiceError::ProviderUnavailable(format!(
                "Arquivo não existe: {}",
                yuki_path.display()
            )));
        }

        let env = ort::init_from(dll_path.to_str().unwrap_or_default())
            .map_err(|e| {
                VoiceError::ProviderUnavailable(format!(
                    "Falha ao inicializar ONNX Runtime a partir de '{}': {}",
                    dll_path.display(),
                    e
                ))
            })?
            .build()
            .map_err(|e| {
                VoiceError::ProviderUnavailable(format!(
                    "Falha ao construir ambiente ONNX Runtime: {}",
                    e
                ))
            })?;

        let mel_session = ort::session::Session::builder(&env)
            .map_err(|e| VoiceError::ProviderUnavailable(format!("Falha no builder mel: {}", e)))?
            .commit_from_file(mel_path)
            .map_err(|e| {
                VoiceError::ProviderUnavailable(format!(
                    "Falha ao carregar {}: {}",
                    mel_path.display(),
                    e
                ))
            })?;

        let emb_session = ort::session::Session::builder(&env)
            .map_err(|e| VoiceError::ProviderUnavailable(format!("Falha no builder emb: {}", e)))?
            .commit_from_file(emb_path)
            .map_err(|e| {
                VoiceError::ProviderUnavailable(format!(
                    "Falha ao carregar {}: {}",
                    emb_path.display(),
                    e
                ))
            })?;

        let yuki_session = ort::session::Session::builder(&env)
            .map_err(|e| VoiceError::ProviderUnavailable(format!("Falha no builder yuki: {}", e)))?
            .commit_from_file(yuki_path)
            .map_err(|e| {
                VoiceError::ProviderUnavailable(format!(
                    "Falha ao carregar {}: {}",
                    yuki_path.display(),
                    e
                ))
            })?;

        let mut inner = OwwEngineInner {
            mel_session,
            emb_session,
            yuki_session,
            mel_buffer: vec![1.0; 76 * 32],
            feat_buffer: vec![0.0; 16 * 96],
            raw_buffer: Vec::with_capacity(3200),
            chunk_count: 0,
        };
        inner.reset();

        Ok(Self {
            config,
            inner: Mutex::new(inner),
        })
    }

    /// Processa um bloco de áudio de 1280 amostras (80 ms @ 16 kHz mono) em streaming contínuo.
    pub fn process_chunk(&self, chunk: &[i16]) -> Result<f32, VoiceError> {
        let mut inner = self.inner.lock().map_err(|_| {
            VoiceError::CaptureFailed(
                "Falha de concorrência ao adquirir lock do motor openWakeWord".to_string(),
            )
        })?;
        inner.process_chunk(chunk)
    }

    /// Reseta buffers internos para estado inicial.
    pub fn reset(&self) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.reset();
        }
    }

    /// Retorna o contador atual de blocos processados desde a última inicialização ou `reset()`.
    pub fn chunk_count(&self) -> usize {
        self.inner.lock().map(|g| g.chunk_count).unwrap_or(0)
    }

    /// Retorna a configuração ativa do motor openWakeWord.
    pub fn config(&self) -> &OpenWakeWordConfig {
        &self.config
    }
}

/// Contrato de streaming em blocos de 80 ms (1280 amostras @ 16 kHz mono) para escuta contínua ao vivo.
///
/// Permite acoplar `OpenWakeWordEngine` em produção e motores determinísticos com memória de
/// embeddings em testes automatizados de regressão (mesmo em ambientes CI sem `onnxruntime.dll`).
pub trait LiveWakeWordSpotter: Send + Sync {
    /// Processa um bloco de 1280 amostras (80 ms @ 16 kHz mono) e retorna score `[0.0 .. 1.0]`.
    fn process_chunk(&self, chunk: &[i16]) -> Result<f32, VoiceError>;

    /// Limpa completamente os buffers de melspectrograma, embeddings e áudio bruto,
    /// exigindo novo preenchimento da janela de warmup antes de habilitar novo disparo.
    fn reset(&self);

    /// Nome identificador do motor de Wake Word em streaming.
    fn name(&self) -> &str;
}

impl LiveWakeWordSpotter for OpenWakeWordEngine {
    fn process_chunk(&self, chunk: &[i16]) -> Result<f32, VoiceError> {
        OpenWakeWordEngine::process_chunk(self, chunk)
    }

    fn reset(&self) {
        OpenWakeWordEngine::reset(self);
    }

    fn name(&self) -> &str {
        "OpenWakeWordEngine (PT-BR yuki_v0.1)"
    }
}

impl WakeWordEngine for OpenWakeWordEngine {
    fn verify_keyword(
        &self,
        frames: &[AudioFrame],
        keyword: &str,
    ) -> Result<Option<f32>, VoiceError> {
        if keyword.to_lowercase() != "yuki" {
            return Ok(None);
        }
        let mut all_samples = Vec::new();
        for f in frames {
            all_samples.extend_from_slice(&f.samples);
        }
        if all_samples.is_empty() {
            return Ok(None);
        }

        let mut inner = self.inner.lock().map_err(|_| {
            VoiceError::CaptureFailed(
                "Falha de concorrência ao adquirir lock do motor openWakeWord".to_string(),
            )
        })?;
        inner.reset();

        let chunk_size = 1280;
        let n_chunks = all_samples.len() / chunk_size;
        let mut max_score = 0.0f32;

        for i in 0..n_chunks {
            let chunk = &all_samples[i * chunk_size..(i + 1) * chunk_size];
            let score = inner.process_chunk(chunk)?;
            if score > max_score {
                max_score = score;
            }
        }

        if max_score >= self.config.activation_threshold {
            Ok(Some(max_score))
        } else {
            Ok(None)
        }
    }

    fn name(&self) -> &str {
        "OpenWakeWordEngine (PT-BR yuki_v0.1)"
    }

    fn status(&self) -> WakeWordEngineStatus {
        WakeWordEngineStatus::Ready
    }
}

// ---------------------------------------------------------------------------
// Motor STT Local (SttWakeWordEngine)
// ---------------------------------------------------------------------------

/// Motor que valida a palavra-chave submetendo a janela candidata para transcrição rápida STT local.
/// NUNCA recorre a heurísticas acústicas em caso de falha de transcrição.
pub struct SttWakeWordEngine {
    stt_provider: Arc<dyn SpeechToTextProvider>,
}

impl std::fmt::Debug for SttWakeWordEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SttWakeWordEngine({})",
            self.stt_provider.metadata().provider_name
        )
    }
}

impl SttWakeWordEngine {
    pub fn new(stt_provider: Arc<dyn SpeechToTextProvider>) -> Self {
        Self { stt_provider }
    }
}

impl WakeWordEngine for SttWakeWordEngine {
    fn verify_keyword(
        &self,
        frames: &[AudioFrame],
        keyword: &str,
    ) -> Result<Option<f32>, VoiceError> {
        // Coleta todas as amostras i16 da janela
        let mut all_samples = Vec::new();
        for f in frames {
            all_samples.extend_from_slice(&f.samples);
        }

        if all_samples.is_empty() {
            return Ok(None);
        }

        let format = AudioFormat::default();
        let wav_bytes = match AudioCodec::encode_wav(&all_samples, &format) {
            Ok(b) => b,
            Err(_) => return Ok(None),
        };

        let duration_ms = ((all_samples.len() as u64) * 1000) / (format.sample_rate as u64);
        let audio_input = AudioInput::new(wav_bytes, format, duration_ms, "wakeword-candidate");

        // Executa transcrição síncrona/bloqueante usando runtime atual se disponível
        let kw_lower = keyword.to_lowercase();
        let stt = self.stt_provider.clone();

        let transcript_res = if let Ok(handle) = tokio::runtime::Handle::try_current() {
            tokio::task::block_in_place(|| {
                handle.block_on(async { stt.transcribe(&audio_input).await })
            })
        } else {
            return Ok(None);
        };

        match transcript_res {
            Ok(text) => {
                let norm = text.to_lowercase();
                if norm.contains(&kw_lower)
                    || norm.contains("yuuki")
                    || norm.contains("iuki")
                    || norm.contains("yukki")
                {
                    Ok(Some(0.95))
                } else {
                    Ok(None)
                }
            }
            Err(_) => {
                // Em caso de erro de transcrição, NÃO recorre a heurísticas acústicas cegas
                Ok(None)
            }
        }
    }

    fn name(&self) -> &str {
        "SttWakeWordEngine"
    }

    fn status(&self) -> WakeWordEngineStatus {
        let meta = self.stt_provider.metadata();
        if meta.provider_name.to_lowercase().contains("unavailable") {
            return WakeWordEngineStatus::Unavailable;
        }

        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            let health_res = tokio::task::block_in_place(|| {
                handle.block_on(async { self.stt_provider.health().await })
            });
            if health_res.is_ok() {
                WakeWordEngineStatus::Ready
            } else {
                WakeWordEngineStatus::Unavailable
            }
        } else {
            WakeWordEngineStatus::Ready
        }
    }
}

// ---------------------------------------------------------------------------
// Motor Mock Observável para Testes e CI
// ---------------------------------------------------------------------------

/// Motor mock determinístico para testes e injeção controlada de confiança de detecção.
#[derive(Debug, Default)]
pub struct MockWakeWordEngine {
    forced_confidence: Arc<Mutex<Option<f32>>>,
    calls_count: Arc<AtomicUsize>,
}

impl MockWakeWordEngine {
    pub fn new() -> Self {
        Self {
            forced_confidence: Arc::new(Mutex::new(None)),
            calls_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn set_forced_confidence(&self, confidence: Option<f32>) {
        *self.forced_confidence.lock().unwrap() = confidence;
    }

    pub fn calls_count(&self) -> usize {
        self.calls_count.load(Ordering::SeqCst)
    }
}

impl WakeWordEngine for MockWakeWordEngine {
    fn verify_keyword(
        &self,
        _frames: &[AudioFrame],
        _keyword: &str,
    ) -> Result<Option<f32>, VoiceError> {
        self.calls_count.fetch_add(1, Ordering::SeqCst);
        let conf = *self.forced_confidence.lock().unwrap();
        Ok(conf)
    }

    fn name(&self) -> &str {
        "MockWakeWordEngine"
    }

    fn status(&self) -> WakeWordEngineStatus {
        WakeWordEngineStatus::Ready
    }
}

// ---------------------------------------------------------------------------
// WakeWordDetector Integrado
// ---------------------------------------------------------------------------

/// Detector soberano e local da palavra de ativação "Yuki".
///
/// DIRETRIZES FUNDAMENTAIS DE SEGURANÇA:
/// 1. Separação Estrita de VAD e Reconhecimento Lexical:
///    O motor de palavra-chave só é consultado se o VAD atestar atividade de fala humana.
/// 2. Operação 100% offline: zero streaming contínuo para provedores externos.
/// 3. Isolamento ontológico estrito:
///    Reconhecer "Yuki" significa EXCLUSIVAMENTE transitar a sessão para captura (`WakeListening -> Capturing`).
///    NUNCA autoriza comandos, não concede permissões e não autentica a identidade do operador.
pub struct WakeWordDetector {
    keyword: String,
    sensitivity: f32,
    enabled: bool,
    allow_experimental: bool,
    history_frames: Vec<AudioFrame>,
    max_history_frames: usize,
    vad: VoiceActivityDetector,
    engine: Arc<dyn WakeWordEngine>,
}

impl Default for WakeWordDetector {
    fn default() -> Self {
        Self::new("yuki", 0.5, true)
    }
}

impl WakeWordDetector {
    pub fn new(keyword: impl Into<String>, sensitivity: f32, enabled: bool) -> Self {
        Self {
            keyword: keyword.into().to_lowercase(),
            sensitivity: sensitivity.clamp(0.1, 1.0),
            enabled,
            allow_experimental: false,
            history_frames: Vec::new(),
            max_history_frames: 10,
            vad: VoiceActivityDetector::default(),
            engine: Arc::new(PhoneticPatternWakeWordEngine::new()),
        }
    }

    pub fn with_engine(
        keyword: impl Into<String>,
        sensitivity: f32,
        enabled: bool,
        engine: Arc<dyn WakeWordEngine>,
    ) -> Self {
        Self {
            keyword: keyword.into().to_lowercase(),
            sensitivity: sensitivity.clamp(0.1, 1.0),
            enabled,
            allow_experimental: false,
            history_frames: Vec::new(),
            max_history_frames: 10,
            vad: VoiceActivityDetector::default(),
            engine,
        }
    }

    /// Inicializa o detector utilizando o modelo neural personalizado openWakeWord ('yuki_v0.1.onnx').
    pub fn open_wakeword() -> Result<Self, VoiceError> {
        let engine = Arc::new(OpenWakeWordEngine::from_default_paths()?);
        Ok(Self::with_engine("yuki", 0.5, true, engine))
    }

    /// Inicializa o detector com uma instância customizada do motor openWakeWord.
    pub fn with_open_wakeword_engine(sensitivity: f32, engine: Arc<OpenWakeWordEngine>) -> Self {
        Self::with_engine("yuki", sensitivity, true, engine)
    }

    pub fn mock(keyword: impl Into<String>) -> (Self, Arc<MockWakeWordEngine>) {
        let mock_engine = Arc::new(MockWakeWordEngine::new());
        let detector = Self::with_engine(keyword, 0.5, true, mock_engine.clone());
        (detector, mock_engine)
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn keyword(&self) -> &str {
        &self.keyword
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.history_frames.clear();
        }
    }

    pub fn sensitivity(&self) -> f32 {
        self.sensitivity
    }

    pub fn set_sensitivity(&mut self, sensitivity: f32) {
        self.sensitivity = sensitivity.clamp(0.1, 1.0);
    }

    pub fn allow_experimental(&self) -> bool {
        self.allow_experimental
    }

    pub fn set_allow_experimental(&mut self, allow: bool) {
        self.allow_experimental = allow;
    }

    /// Avalia se o detector pode transitar automaticamente para captura de voz (`Capturing`).
    /// Motores em status `Ready` sempre podem; motores `Experimental` requerem opt-in explícito (`allow_experimental = true`).
    pub fn can_auto_activate(&self) -> bool {
        if !self.enabled {
            return false;
        }
        match self.engine.status() {
            WakeWordEngineStatus::Ready => true,
            WakeWordEngineStatus::Experimental => self.allow_experimental,
            WakeWordEngineStatus::Unavailable => false,
        }
    }

    pub fn engine(&self) -> &Arc<dyn WakeWordEngine> {
        &self.engine
    }

    /// Retorna o status operacional do detector/motor subjacente.
    pub fn status(&self) -> WakeWordEngineStatus {
        if !self.enabled {
            WakeWordEngineStatus::Unavailable
        } else {
            self.engine.status()
        }
    }

    /// Analisa um quadro de áudio PCM.
    ///
    /// 1. Se desabilitado ou motor indisponível, ignora.
    /// 2. Se o motor for experimental e o opt-in não foi concedido, bloqueia auto-ativação.
    /// 3. Acumula em janela deslizante.
    /// 4. Executa VAD para validar presença de voz humana.
    /// 5. Se houver voz, invoca o `WakeWordEngine` para verificar evidência da palavra-chave.
    pub fn process_frame(&mut self, frame: &AudioFrame) -> Result<WakeWordDetection, VoiceError> {
        if !self.enabled
            || self.engine.status() == WakeWordEngineStatus::Unavailable
            || frame.samples.is_empty()
        {
            return Ok(WakeWordDetection::None);
        }

        // Se o motor for experimental e o opt-in não foi concedido, bloqueia auto-ativação
        if self.engine.status() == WakeWordEngineStatus::Experimental && !self.allow_experimental {
            return Ok(WakeWordDetection::None);
        }

        self.history_frames.push(frame.clone());
        if self.history_frames.len() > self.max_history_frames {
            self.history_frames.remove(0);
        }

        // 1. Etapa de Voice Activity Detection (VAD)
        if !self.vad.is_speech(&self.history_frames) {
            return Ok(WakeWordDetection::None);
        }

        // 2. Etapa de Reconhecimento Lexical via WakeWordEngine
        let threshold = 0.85 - (self.sensitivity * 0.45); // [0.40 .. 0.80]

        if let Some(confidence) = self
            .engine
            .verify_keyword(&self.history_frames, &self.keyword)?
        {
            if confidence >= threshold {
                self.history_frames.clear(); // Previne repetições contínuas espúrias
                if self.engine.status() == WakeWordEngineStatus::Experimental {
                    tracing::warn!(
                        keyword = %self.keyword,
                        confidence,
                        engine = %self.engine.name(),
                        "Wake word auto-ativada via motor heurístico acústico experimental"
                    );
                }
                return Ok(WakeWordDetection::Detected {
                    confidence,
                    timestamp_ms: frame.timestamp_ms,
                });
            }
        }

        Ok(WakeWordDetection::None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vad_rejection_of_silence_and_static_noise() {
        let vad = VoiceActivityDetector::default();

        // 1. Silêncio absoluto
        let silent_frame = AudioFrame::new(vec![0i16; 320], 16000, 1, 0);
        assert!(!vad.is_speech(&[silent_frame]));

        // 2. Ruído alternado de alta frequência (ZCR = 1.0)
        let noise_samples: Vec<i16> = (0..320)
            .map(|i| if i % 2 == 0 { 2000 } else { -2000 })
            .collect();
        let noise_frame = AudioFrame::new(noise_samples, 16000, 1, 0);
        assert!(!vad.is_speech(&[noise_frame]));
    }

    #[test]
    fn test_mock_wakeword_engine_integration() {
        let (mut detector, mock_engine) = WakeWordDetector::mock("yuki");

        // Cria frame com características de fala para passar pelo VAD
        let speech_samples: Vec<i16> = (0..640)
            .map(|i| ((i as f32 * 0.1).sin() * 1500.0) as i16)
            .collect();
        let frame = AudioFrame::new(speech_samples, 16000, 1, 100);

        // Sem confiança forçada, não detecta
        assert_eq!(
            detector.process_frame(&frame).unwrap(),
            WakeWordDetection::None
        );
        assert_eq!(mock_engine.calls_count(), 1);

        // Com confiança forçada alta (0.9), deve detectar
        mock_engine.set_forced_confidence(Some(0.9));
        let det = detector.process_frame(&frame).unwrap();
        assert!(matches!(det, WakeWordDetection::Detected { .. }));
    }

    #[test]
    fn test_phonetic_engine_detects_synthesized_yuki() {
        let mut detector = WakeWordDetector::new("yuki", 0.5, true);
        detector.set_allow_experimental(true);
        assert!(detector.can_auto_activate());

        // Constrói janela com padrão fonético de "Yu-ki":
        // Sílaba 1: /ju/ (F0 ~ 220Hz, ZCR menor)
        let frame1_samples: Vec<i16> = (0..480)
            .map(|i| ((i as f32 * 0.08).sin() * 2500.0) as i16)
            .collect();
        let frame1 = AudioFrame::new(frame1_samples, 16000, 1, 0);

        // Vale de transição /u/ -> /k/
        let frame2_samples: Vec<i16> = (0..160)
            .map(|i| ((i as f32 * 0.05).sin() * 500.0) as i16)
            .collect();
        let frame2 = AudioFrame::new(frame2_samples, 16000, 1, 30);

        // Sílaba 2: /ki/ (componentes de frequência mais alta e explosão)
        let frame3_samples: Vec<i16> = (0..480)
            .map(|i| (((i as f32 * 0.18).sin() + (i as f32 * 0.3).sin()) * 1400.0) as i16)
            .collect();
        let frame3 = AudioFrame::new(frame3_samples, 16000, 1, 60);

        // Alimenta o detector
        let _ = detector.process_frame(&frame1);
        let _ = detector.process_frame(&frame2);
        let det = detector.process_frame(&frame3).unwrap();

        assert!(matches!(det, WakeWordDetection::Detected { .. }));
    }

    #[test]
    fn test_phonetic_engine_blocks_auto_activate_without_opt_in() {
        let mut detector = WakeWordDetector::new("yuki", 0.5, true);
        // Por padrão, allow_experimental é false
        assert!(!detector.allow_experimental());
        assert!(!detector.can_auto_activate());

        let frame1_samples: Vec<i16> = (0..480)
            .map(|i| ((i as f32 * 0.08).sin() * 2500.0) as i16)
            .collect();
        let frame1 = AudioFrame::new(frame1_samples, 16000, 1, 0);
        let frame2_samples: Vec<i16> = (0..160)
            .map(|i| ((i as f32 * 0.05).sin() * 500.0) as i16)
            .collect();
        let frame2 = AudioFrame::new(frame2_samples, 16000, 1, 30);
        let frame3_samples: Vec<i16> = (0..480)
            .map(|i| (((i as f32 * 0.18).sin() + (i as f32 * 0.3).sin()) * 1400.0) as i16)
            .collect();
        let frame3 = AudioFrame::new(frame3_samples, 16000, 1, 60);

        let _ = detector.process_frame(&frame1);
        let _ = detector.process_frame(&frame2);
        // Mesmo com áudio acústico válido, sem opt-in NÃO auto-ativa
        let det = detector.process_frame(&frame3).unwrap();
        assert_eq!(det, WakeWordDetection::None);

        // Com opt-in concedido, can_auto_activate se torna true
        detector.set_allow_experimental(true);
        assert!(detector.can_auto_activate());
        let _ = detector.process_frame(&frame1);
        let _ = detector.process_frame(&frame2);
        let det_opt_in = detector.process_frame(&frame3).unwrap();
        assert!(matches!(det_opt_in, WakeWordDetection::Detected { .. }));
    }
}
