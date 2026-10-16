use serde::{Deserialize, Serialize};

/// Configuração para detecção experimental de silêncio acústico por janelas temporais (Marco 8.1.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowedSilenceConfig {
    /// Tamanho de cada janela temporal em milissegundos (padrão: 20 ms).
    pub window_ms: usize,
    /// Limiar de energia RMS para classificar uma janela individual como ativa/fonação (padrão: 80.0, aprox. -52.2 dBFS).
    pub active_rms_threshold: f32,
    /// Limiar de pico absoluto para classificar uma janela individual como ativa (padrão: 250, aprox. -42.3 dBFS).
    pub active_peak_threshold: i16,
    /// Número mínimo de janelas ativas consecutivas para confirmar fala sustentada e descartar transientes/cliques isolados (padrão: 3 janelas = 60 ms).
    pub min_consecutive_active_windows: usize,
    /// Razão máxima de janelas de silêncio (em porcentagem) acima da qual o áudio requer validação estrita de persistência (padrão: 85.0%).
    pub max_silence_ratio_pct: f32,
    /// Desvio padrão mínimo de RMS entre janelas para diferenciar fala articulada de ruído contínuo estacionário (padrão: 10.0).
    pub min_variance_std_dev: f32,
}

impl Default for WindowedSilenceConfig {
    fn default() -> Self {
        Self {
            window_ms: 20,
            active_rms_threshold: 80.0,
            active_peak_threshold: 250,
            min_consecutive_active_windows: 3,
            max_silence_ratio_pct: 85.0,
            min_variance_std_dev: 10.0,
        }
    }
}

/// Modalidade selecionada para detecção de silêncio acústico no STT local.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum SilenceDetectorKind {
    /// Heurística legado do Marco 8.1 (`rms < 120.0 && max_abs < 250`). Preservada como baseline.
    #[default]
    Legacy,
    /// Detecção experimental por janelas temporais com persistência e variabilidade.
    Windowed(WindowedSilenceConfig),
}

/// Relatório granular de análise acústica de atividade vocal por janelas temporais.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowedSilenceAnalysis {
    pub total_windows: usize,
    pub silent_windows: usize,
    pub active_windows: usize,
    pub silence_ratio_pct: f32,
    pub max_consecutive_active: usize,
    pub rms_mean: f32,
    pub rms_std_dev: f32,
    pub peak_global: i16,
    pub is_silent: bool,
    pub rejection_reason: Option<String>,
}

/// Avalia blocos temporais do sinal PCM de 16 bits para separar silêncio/ruído de fonação legítima.
pub fn analyze_acoustic_activity(
    samples: &[i16],
    sample_rate: u32,
    config: &WindowedSilenceConfig,
) -> WindowedSilenceAnalysis {
    if samples.is_empty() {
        return WindowedSilenceAnalysis {
            total_windows: 0,
            silent_windows: 0,
            active_windows: 0,
            silence_ratio_pct: 100.0,
            max_consecutive_active: 0,
            rms_mean: 0.0,
            rms_std_dev: 0.0,
            peak_global: 0,
            is_silent: true,
            rejection_reason: Some("ZeroSamples".to_string()),
        };
    }

    let rate = if sample_rate > 0 { sample_rate } else { 16000 };
    let window_samples =
        ((rate as f64 * (config.window_ms as f64 / 1000.0)).round() as usize).max(1);

    let mut total_windows = 0usize;
    let mut silent_windows = 0usize;
    let mut active_windows = 0usize;
    let mut current_consecutive = 0usize;
    let mut max_consecutive_active = 0usize;
    let mut peak_global: i16 = 0;
    let mut window_rms_list: Vec<f32> = Vec::with_capacity(samples.len() / window_samples + 1);

    for chunk in samples.chunks(window_samples) {
        total_windows += 1;
        let mut sum_sq: f64 = 0.0;
        let mut w_peak: i16 = 0;

        for &s in chunk {
            let abs_s = s.saturating_abs();
            if abs_s > w_peak {
                w_peak = abs_s;
            }
            if abs_s > peak_global {
                peak_global = abs_s;
            }
            sum_sq += (s as f64) * (s as f64);
        }

        let w_rms = (sum_sq / chunk.len() as f64).sqrt() as f32;
        window_rms_list.push(w_rms);

        let is_window_active =
            w_rms >= config.active_rms_threshold || w_peak >= config.active_peak_threshold;
        if is_window_active {
            active_windows += 1;
            current_consecutive += 1;
            if current_consecutive > max_consecutive_active {
                max_consecutive_active = current_consecutive;
            }
        } else {
            silent_windows += 1;
            current_consecutive = 0;
        }
    }

    let silence_ratio_pct = if total_windows > 0 {
        (silent_windows as f32 / total_windows as f32) * 100.0
    } else {
        100.0
    };

    let rms_mean = if !window_rms_list.is_empty() {
        window_rms_list.iter().sum::<f32>() / window_rms_list.len() as f32
    } else {
        0.0
    };

    let rms_variance = if window_rms_list.len() > 1 {
        let var_sum: f32 = window_rms_list
            .iter()
            .map(|&r| (r - rms_mean).powi(2))
            .sum();
        var_sum / window_rms_list.len() as f32
    } else {
        0.0
    };
    let rms_std_dev = rms_variance.sqrt();

    // Regras de classificação heurística de silêncio
    let mut is_silent = false;
    let mut rejection_reason: Option<String> = None;

    if active_windows == 0 {
        is_silent = true;
        rejection_reason = Some("AllSilentWindows".to_string());
    } else if max_consecutive_active < config.min_consecutive_active_windows {
        // Transientes pontuais isolados (estalo RF Bluetooth, chaveamento elétrico de < 60ms)
        is_silent = true;
        rejection_reason = Some(format!(
            "TransientSpikesWithoutSustainedSpeech (max_consecutive: {} < {})",
            max_consecutive_active, config.min_consecutive_active_windows
        ));
    } else if rms_std_dev < config.min_variance_std_dev && rms_mean < 180.0 {
        // Ruído estacionário contínuo artificial (hiss estático contínuo com variância ínfima)
        is_silent = true;
        rejection_reason = Some(format!(
            "StationaryContinuousNoise (std_dev: {:.2} < {:.2})",
            rms_std_dev, config.min_variance_std_dev
        ));
    }

    WindowedSilenceAnalysis {
        total_windows,
        silent_windows,
        active_windows,
        silence_ratio_pct,
        max_consecutive_active,
        rms_mean,
        rms_std_dev,
        peak_global,
        is_silent,
        rejection_reason,
    }
}

/// Avalia se um buffer PCM é silêncio utilizando a detecção experimental por janelas temporais.
pub fn is_silent_windowed(
    samples: &[i16],
    sample_rate: u32,
    config: &WindowedSilenceConfig,
) -> bool {
    analyze_acoustic_activity(samples, sample_rate, config).is_silent
}
