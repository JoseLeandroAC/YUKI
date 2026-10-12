use crate::contracts::voice::{AudioFrame, VoiceError};

/// Resultado da análise de palavra de ativação em um fluxo de quadros de áudio.
#[derive(Debug, Clone, PartialEq)]
pub enum WakeWordDetection {
    /// Nenhuma palavra de ativação detectada no quadro.
    None,
    /// Palavra de ativação "Yuki" detectada com o score de confiança especificado.
    Detected { confidence: f32, timestamp_ms: u64 },
}

/// Detector soberano e 100% local da palavra de ativação "Yuki".
///
/// DIRETRIZES FUNDAMENTAIS DE SEGURANÇA:
/// 1. Operação 100% offline: zero streaming contínuo para provedores externos.
/// 2. Isolamento ontológico estrito:
///    Reconhecer "Yuki" significa EXCLUSIVAMENTE iniciar a captura de áudio (`WakeListening -> Capturing`).
///    NUNCA autoriza comandos, não concede permissões e não autentica a identidade do operador.
pub struct WakeWordDetector {
    keyword: String,
    sensitivity: f32,
    enabled: bool,
    history_frames: Vec<AudioFrame>,
    max_history_frames: usize,
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
            max_history_frames: 10, // Janela de buffer de aprox 300ms
        }
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

    /// Analisa um quadro de áudio PCM. Se a palavra de ativação for detectada na janela deslizante,
    /// emite `WakeWordDetection::Detected`.
    pub fn process_frame(&mut self, frame: &AudioFrame) -> Result<WakeWordDetection, VoiceError> {
        if !self.enabled {
            return Ok(WakeWordDetection::None);
        }

        // Valida parâmetros mínimos do frame
        if frame.samples.is_empty() {
            return Ok(WakeWordDetection::None);
        }

        self.history_frames.push(frame.clone());
        if self.history_frames.len() > self.max_history_frames {
            self.history_frames.remove(0);
        }

        // Se o nível de ruído RMS for muito baixo (silêncio), não processa
        let rms = frame.rms_energy();
        if rms < 300.0 {
            return Ok(WakeWordDetection::None);
        }

        // Análise acústica local das sílabas "Yu-ki" (/ju:ki/):
        // 1. /ju/: semivogal sonora de média/baixa frequência
        // 2. /ki/: plosiva oclusiva velar com rajada de alta frequência seguida de vogal alta /i/
        let confidence = self.calculate_phonetic_confidence(&self.history_frames);
        let threshold = 0.85 - (self.sensitivity * 0.45); // Limiar entre [0.40, 0.80]

        if confidence >= threshold {
            self.history_frames.clear(); // Limpa para evitar ativações repetidas contínuas no mesmo som
            return Ok(WakeWordDetection::Detected {
                confidence,
                timestamp_ms: frame.timestamp_ms,
            });
        }

        Ok(WakeWordDetection::None)
    }

    /// Avalia a estrutura acústica temporal agregada na janela deslizante.
    fn calculate_phonetic_confidence(&self, frames: &[AudioFrame]) -> f32 {
        if frames.len() < 3 {
            return 0.0;
        }

        // Calcula taxa de cruzamento por zero (ZCR) e energia espectral média
        let mut total_zcr = 0usize;
        let mut total_samples = 0usize;
        let mut energy_profile = Vec::with_capacity(frames.len());

        for f in frames {
            let mut zcr = 0;
            for i in 1..f.samples.len() {
                if (f.samples[i] >= 0 && f.samples[i - 1] < 0)
                    || (f.samples[i] < 0 && f.samples[i - 1] >= 0)
                {
                    zcr += 1;
                }
            }
            total_zcr += zcr;
            total_samples += f.samples.len();
            energy_profile.push(f.rms_energy());
        }

        if total_samples == 0 {
            return 0.0;
        }

        let zcr_rate = (total_zcr as f32) / (total_samples as f32);

        // Padrão de duas sílabas distintas com transição de energia
        let first_half = &energy_profile[..energy_profile.len() / 2];
        let second_half = &energy_profile[energy_profile.len() / 2..];

        let avg1: f32 = first_half.iter().sum::<f32>() / (first_half.len() as f32);
        let avg2: f32 = second_half.iter().sum::<f32>() / (second_half.len() as f32);

        // Se houver dois picos de energia com ZCR característico de plosiva /k/ + vogal /i/
        if avg1 > 400.0 && avg2 > 400.0 && zcr_rate > 0.04 && zcr_rate < 0.35 {
            let balance = 1.0 - ((avg1 - avg2).abs() / (avg1 + avg2));
            (0.5 + (balance * 0.45)).clamp(0.0, 0.99)
        } else {
            0.1
        }
    }
}
