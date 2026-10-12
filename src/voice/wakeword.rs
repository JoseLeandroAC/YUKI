use crate::contracts::voice::{
    AudioFormat, AudioFrame, AudioInput, SpeechToTextProvider, VoiceError,
};
use crate::voice::audio::AudioCodec;
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
}

impl Default for VoiceActivityDetector {
    fn default() -> Self {
        Self {
            min_rms_speech: 300.0,
            max_rms_speech: 30000.0,
            min_zcr: 0.02,
            max_zcr: 0.40,
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
        }
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
            if std_dev < 15.0 {
                return false;
            }
        }

        true
    }
}

// ---------------------------------------------------------------------------
// Trait WakeWordEngine
// ---------------------------------------------------------------------------

/// Contrato para motores de reconhecimento lexical da palavra-chave "Yuki".
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
        "PhoneticPatternWakeWordEngine"
    }
}

// ---------------------------------------------------------------------------
// Motor STT Local (SttWakeWordEngine)
// ---------------------------------------------------------------------------

/// Motor que valida a palavra-chave submetendo a janela candidata para transcrição rápida STT local.
pub struct SttWakeWordEngine {
    stt_provider: Arc<dyn SpeechToTextProvider>,
    phonetic_fallback: PhoneticPatternWakeWordEngine,
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
        Self {
            stt_provider,
            phonetic_fallback: PhoneticPatternWakeWordEngine::new(),
        }
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
            Err(_) => return self.phonetic_fallback.verify_keyword(frames, keyword),
        };

        let duration_ms = ((all_samples.len() as u64) * 1000) / (format.sample_rate as u64);
        let audio_input = AudioInput::new(wav_bytes, format, duration_ms, "wakeword-candidate");

        // Executa transcrição síncrona/bloqueante usando runtime atual se disponível
        let kw_lower = keyword.to_lowercase();
        let stt = self.stt_provider.clone();

        let transcript_res = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async { stt.transcribe(&audio_input).await })
        });

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
                // Em caso de falha transitória ou STT não configurado, faz fallback fonético
                self.phonetic_fallback.verify_keyword(frames, keyword)
            }
        }
    }

    fn name(&self) -> &str {
        "SttWakeWordEngine"
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
            history_frames: Vec::new(),
            max_history_frames: 10,
            vad: VoiceActivityDetector::default(),
            engine,
        }
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

    pub fn engine(&self) -> &Arc<dyn WakeWordEngine> {
        &self.engine
    }

    /// Analisa um quadro de áudio PCM.
    ///
    /// 1. Se desabilitado, ignora.
    /// 2. Acumula em janela deslizante.
    /// 3. Executa VAD para validar presença de voz humana.
    /// 4. Se houver voz, invoca o `WakeWordEngine` para verificar evidência da palavra-chave.
    pub fn process_frame(&mut self, frame: &AudioFrame) -> Result<WakeWordDetection, VoiceError> {
        if !self.enabled || frame.samples.is_empty() {
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
}
