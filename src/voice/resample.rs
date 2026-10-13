pub use crate::contracts::voice::ResamplerKind;

/// Função de Bessel modificada de primeira espécie e ordem zero: I_0(x).
/// Utilizada na síntese exata da janela de Kaiser.
pub fn bessel_i0(x: f64) -> f64 {
    let mut sum = 1.0f64;
    let mut term = 1.0f64;
    let half_x = x / 2.0f64;
    for k in 1..=50 {
        term *= (half_x / k as f64) * (half_x / k as f64);
        sum += term;
        if term < 1e-15 * sum {
            break;
        }
    }
    sum
}

/// Gera os coeficientes de um filtro FIR passa-baixa de fase linear simétrico Type I
/// utilizando truncamento por sinc com janela de Kaiser.
///
/// Parâmetros:
/// - `num_taps`: número de coeficientes (deve ser ímpar para simetria exata e atraso de grupo inteiro).
/// - `cutoff_hz`: frequência de corte em Hertz (-6 dB).
/// - `sample_rate_hz`: taxa de amostragem em Hertz.
/// - `beta`: parâmetro de conformação da janela de Kaiser (beta ~ 5.65 fornece ~50 dB de atenuação de stopband).
pub fn design_kaiser_sinc_lpf(
    num_taps: usize,
    cutoff_hz: f64,
    sample_rate_hz: f64,
    beta: f64,
) -> Vec<f32> {
    assert!(
        num_taps % 2 == 1,
        "num_taps deve ser ímpar para filtro Type I de fase linear (atraso inteiro)"
    );
    let m = (num_taps - 1) as f64;
    let half_m = m / 2.0;
    let fc_norm = cutoff_hz / sample_rate_hz;
    let denom_i0 = bessel_i0(beta);

    let mut taps = Vec::with_capacity(num_taps);
    let mut sum = 0.0f64;

    for i in 0..num_taps {
        let n = i as f64;
        let delta = n - half_m;

        // Resposta impulsiva do filtro passa-baixa ideal (função sinc normalizada)
        let sinc_val = if delta.abs() < 1e-9 {
            2.0 * fc_norm
        } else {
            (2.0 * std::f64::consts::PI * fc_norm * delta).sin() / (std::f64::consts::PI * delta)
        };

        // Ponderação da janela de Kaiser
        let r = 2.0 * n / m - 1.0;
        let arg = (1.0 - r * r).max(0.0).sqrt();
        let kaiser_val = bessel_i0(beta * arg) / denom_i0;

        let w = sinc_val * kaiser_val;
        taps.push(w);
        sum += w;
    }

    // Normalização rigorosa para ganho unitário DC (0 dB em 0 Hz)
    taps.into_iter().map(|w| (w / sum) as f32).collect()
}

/// Avalia a resposta em frequência analítica |H(e^jω)| de um filtro FIR simétrico em dB.
pub fn evaluate_frequency_response_db(taps: &[f32], freq_hz: f64, sample_rate_hz: f64) -> f64 {
    let omega = 2.0 * std::f64::consts::PI * (freq_hz / sample_rate_hz);
    let mut re = 0.0f64;
    let mut im = 0.0f64;

    for (n, &h) in taps.iter().enumerate() {
        let angle = omega * (n as f64);
        re += (h as f64) * angle.cos();
        im -= (h as f64) * angle.sin();
    }

    let mag = (re * re + im * im).sqrt();
    if mag > 1e-12 {
        20.0 * mag.log10()
    } else {
        -240.0
    }
}

/// Resampler de áudio PCM multi-estratégia com suporte a filtragem antialiasing e baseline linear.
#[derive(Debug, Clone)]
pub struct AudioResampler {
    in_rate: u32,
    out_rate: u32,
    kind: ResamplerKind,
    filter_taps: Vec<f32>,
    streaming_buffer: Vec<f32>,
}

impl AudioResampler {
    /// Número canônico de taps para o filtro antialiasing 48 kHz -> 16 kHz.
    pub const DEFAULT_FIR_TAPS: usize = 97;
    /// Frequência de corte padrão (-6 dB) posicionada abaixo do Nyquist alvo (8.000 Hz).
    pub const DEFAULT_CUTOFF_HZ: f64 = 7200.0;
    /// Parâmetro de forma Kaiser que fornece >= 50 dB de atenuação de banda de rejeição.
    pub const DEFAULT_KAISER_BETA: f64 = 5.65;

    /// Cria uma nova instância de resampler configurada para as taxas especificadas.
    pub fn new(in_rate: u32, out_rate: u32, kind: ResamplerKind) -> Self {
        let filter_taps = if kind == ResamplerKind::FirAntialiasing && in_rate != out_rate {
            let cutoff = (out_rate as f64 * 0.45).min(Self::DEFAULT_CUTOFF_HZ);
            design_kaiser_sinc_lpf(
                Self::DEFAULT_FIR_TAPS,
                cutoff,
                in_rate as f64,
                Self::DEFAULT_KAISER_BETA,
            )
        } else {
            Vec::new()
        };

        Self {
            in_rate,
            out_rate,
            kind,
            filter_taps,
            streaming_buffer: Vec::new(),
        }
    }

    /// Cria um resampler otimizado especificamente para a conversão canônica de hardware 48 kHz -> 16 kHz.
    pub fn fir_48k_to_16k() -> Self {
        Self::new(48000, 16000, ResamplerKind::FirAntialiasing)
    }

    /// Cria um resampler com a estratégia linear baseline para medições comparativas.
    pub fn linear(in_rate: u32, out_rate: u32) -> Self {
        Self::new(in_rate, out_rate, ResamplerKind::LinearBaseline)
    }

    /// Retorna a estratégia de reamostragem ativa.
    pub fn kind(&self) -> ResamplerKind {
        self.kind
    }

    /// Retorna a taxa de amostragem de entrada.
    pub fn in_rate(&self) -> u32 {
        self.in_rate
    }

    /// Retorna a taxa de amostragem de saída.
    pub fn out_rate(&self) -> u32 {
        self.out_rate
    }

    /// Retorna uma referência aos coeficientes do filtro FIR (vazio se modo linear ou 1:1).
    pub fn filter_taps(&self) -> &[f32] {
        &self.filter_taps
    }

    /// Limpa o histórico do buffer de streaming interno.
    pub fn reset(&mut self) {
        self.streaming_buffer.clear();
    }

    /// Processa um bloco de áudio contínuo em streaming mantendo a continuidade do buffer
    /// entre chamadas consecutivas para eliminar glitches/estalos nas junções de buffers WASAPI.
    pub fn process_streaming(&mut self, chunk: &[i16]) -> Vec<i16> {
        if self.in_rate == self.out_rate || chunk.is_empty() {
            return chunk.to_vec();
        }

        match self.kind {
            ResamplerKind::LinearBaseline => {
                Self::resample_linear(chunk, self.in_rate, self.out_rate)
            }
            ResamplerKind::FirAntialiasing => {
                if self.in_rate == 48000 && self.out_rate == 16000 {
                    // Decimação 3:1 polifásica com histórico contínuo
                    let num_taps = self.filter_taps.len();
                    self.streaming_buffer
                        .extend(chunk.iter().map(|&s| s as f32));

                    let mut out = Vec::new();
                    while self.streaming_buffer.len() >= num_taps {
                        let mut acc = 0.0f32;
                        for i in 0..num_taps {
                            acc += self.streaming_buffer[i] * self.filter_taps[i];
                        }
                        out.push(acc.clamp(-32768.0, 32767.0).round() as i16);

                        // Avança 3 amostras na entrada (decimação 3:1)
                        if self.streaming_buffer.len() >= 3 {
                            self.streaming_buffer.drain(..3);
                        } else {
                            break;
                        }
                    }
                    out
                } else {
                    // Fallback para taxa não inteira em streaming
                    Self::resample_oneshot(chunk, self.in_rate, self.out_rate, self.kind)
                }
            }
        }
    }

    /// Reamostragem completa one-shot de uma gravação fechada.
    ///
    /// Aplica zero-phase symmetric padding nas extremidades temporais para que os primeiros
    /// e últimos fonemas da fala não sofram atenuação transitória ou atraso artificial.
    pub fn resample_oneshot(
        samples: &[i16],
        in_rate: u32,
        out_rate: u32,
        kind: ResamplerKind,
    ) -> Vec<i16> {
        if in_rate == out_rate || samples.is_empty() {
            return samples.to_vec();
        }

        match kind {
            ResamplerKind::LinearBaseline => Self::resample_linear(samples, in_rate, out_rate),
            ResamplerKind::FirAntialiasing => {
                if in_rate == 48000 && out_rate == 16000 {
                    let taps = design_kaiser_sinc_lpf(
                        Self::DEFAULT_FIR_TAPS,
                        Self::DEFAULT_CUTOFF_HZ,
                        48000.0,
                        Self::DEFAULT_KAISER_BETA,
                    );
                    let num_taps = taps.len();
                    let half_taps = (num_taps - 1) / 2;
                    let out_len = samples.len() / 3;
                    let mut out = Vec::with_capacity(out_len);

                    let samples_len = samples.len() as isize;

                    for m in 0..out_len {
                        let center_idx = (m * 3) as isize;
                        let mut acc = 0.0f32;

                        for (j, &h) in taps.iter().enumerate() {
                            let in_idx = center_idx + (j as isize) - (half_taps as isize);
                            let s = if in_idx < 0 {
                                samples[0] as f32
                            } else if in_idx >= samples_len {
                                samples[samples.len() - 1] as f32
                            } else {
                                samples[in_idx as usize] as f32
                            };
                            acc += s * h;
                        }

                        out.push(acc.clamp(-32768.0, 32767.0).round() as i16);
                    }

                    out
                } else {
                    // Para outras taxas: pré-filtragem antialiasing seguida de interpolação
                    let cutoff = (out_rate as f64 * 0.45).min(Self::DEFAULT_CUTOFF_HZ);
                    let taps = design_kaiser_sinc_lpf(
                        Self::DEFAULT_FIR_TAPS,
                        cutoff,
                        in_rate as f64,
                        Self::DEFAULT_KAISER_BETA,
                    );
                    let filtered = Self::apply_fir_symmetric(samples, &taps);
                    Self::resample_linear(&filtered, in_rate, out_rate)
                }
            }
        }
    }

    /// Aplica convolução FIR simétrica com preservação das bordas (zero phase alignment).
    fn apply_fir_symmetric(samples: &[i16], taps: &[f32]) -> Vec<i16> {
        let num_taps = taps.len();
        let half_taps = (num_taps - 1) / 2;
        let mut out = Vec::with_capacity(samples.len());
        let samples_len = samples.len() as isize;

        for i in 0..samples.len() {
            let center_idx = i as isize;
            let mut acc = 0.0f32;
            for (j, &h) in taps.iter().enumerate() {
                let in_idx = center_idx + (j as isize) - (half_taps as isize);
                let s = if in_idx < 0 {
                    samples[0] as f32
                } else if in_idx >= samples_len {
                    samples[samples.len() - 1] as f32
                } else {
                    samples[in_idx as usize] as f32
                };
                acc += s * h;
            }
            out.push(acc.clamp(-32768.0, 32767.0).round() as i16);
        }
        out
    }

    /// Reamostragem linear direta das amostras mono de entrada para a taxa alvo.
    /// Preservada integralmente como baseline matemático e funcional de referência.
    pub fn resample_linear(samples: &[i16], in_rate: u32, out_rate: u32) -> Vec<i16> {
        if in_rate == out_rate || samples.is_empty() {
            return samples.to_vec();
        }
        let ratio = in_rate as f64 / out_rate as f64;
        let out_len = ((samples.len() as f64) / ratio).round() as usize;
        let mut out = Vec::with_capacity(out_len);

        for i in 0..out_len {
            let in_pos = i as f64 * ratio;
            let idx = in_pos.floor() as usize;
            let frac = in_pos - (idx as f64);

            let s0 = samples.get(idx).copied().unwrap_or(0) as f64;
            let s1 = samples.get(idx + 1).copied().unwrap_or(s0 as i16) as f64;

            let interpolated = (1.0 - frac) * s0 + frac * s1;
            out.push(interpolated.clamp(-32768.0, 32767.0) as i16);
        }
        out
    }
}
