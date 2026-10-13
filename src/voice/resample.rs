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

/// Resampler de áudio PCM com equivalência matemática estrita entre processamento em lote
/// (`resample_oneshot`) e processamento contínuo em chunks (`process_streaming` + `flush`).
#[derive(Debug, Clone)]
pub struct AudioResampler {
    in_rate: u32,
    out_rate: u32,
    kind: ResamplerKind,
    filter_taps: Vec<f32>,
    half_taps: usize,

    // Estado interno para streaming contínuo sem perda nem duplicação de amostras
    input_history: Vec<i16>,
    history_start_idx: usize,
    total_in_samples: usize,
    emitted_out_samples: usize,
    first_sample: Option<i16>,
    last_sample: Option<i16>,
}

impl Default for AudioResampler {
    fn default() -> Self {
        Self::new(48000, 16000, ResamplerKind::default())
    }
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
        let (filter_taps, half_taps) =
            if kind == ResamplerKind::FirAntialiasing && in_rate != out_rate {
                let cutoff = (out_rate as f64 * 0.45).min(Self::DEFAULT_CUTOFF_HZ);
                let taps = design_kaiser_sinc_lpf(
                    Self::DEFAULT_FIR_TAPS,
                    cutoff,
                    in_rate as f64,
                    Self::DEFAULT_KAISER_BETA,
                );
                let half = (taps.len() - 1) / 2;
                (taps, half)
            } else {
                (Vec::new(), 0)
            };

        Self {
            in_rate,
            out_rate,
            kind,
            filter_taps,
            half_taps,
            input_history: Vec::new(),
            history_start_idx: 0,
            total_in_samples: 0,
            emitted_out_samples: 0,
            first_sample: None,
            last_sample: None,
        }
    }

    /// Cria um resampler otimizado especificamente para a conversão canônica de hardware 48 kHz -> 16 kHz (FIR).
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

    /// Retorna o total de amostras de entrada alimentadas desde o início ou último reset.
    pub fn total_in_samples(&self) -> usize {
        self.total_in_samples
    }

    /// Retorna o total de amostras de saída já emitidas.
    pub fn emitted_out_samples(&self) -> usize {
        self.emitted_out_samples
    }

    /// Limpa o estado interno de streaming para reutilização limpa da instância.
    pub fn reset(&mut self) {
        self.input_history.clear();
        self.history_start_idx = 0;
        self.total_in_samples = 0;
        self.emitted_out_samples = 0;
        self.first_sample = None;
        self.last_sample = None;
    }

    /// Política documentada de cálculo de comprimento de saída para uma dada quantidade de amostras de entrada.
    ///
    /// Para taxa 1:1, retorna `in_len`.
    /// Para `in_len == 0`, retorna 0.
    /// Para decimação com `in_len > 0`, calcula `round(in_len / ratio)`, garantindo pelo menos 1 amostra
    /// para entradas unitárias (evitando perda de impulsos curtos).
    pub fn calculate_out_len(in_len: usize, in_rate: u32, out_rate: u32) -> usize {
        if in_rate == out_rate || in_len == 0 {
            return in_len;
        }
        let ratio = in_rate as f64 / out_rate as f64;
        let rounded = ((in_len as f64) / ratio).round() as usize;
        if in_len > 0 && rounded == 0 {
            1
        } else {
            rounded
        }
    }

    /// Obtém uma amostra pelo índice absoluto `abs_idx`, aplicando padding constante consistente
    /// nas fronteiras temporais:
    /// - `abs_idx < 0`: replica `first_sample`
    /// - `abs_idx >= total_in_samples`: replica `last_sample`
    #[inline]
    fn sample_at(&self, abs_idx: isize) -> i16 {
        if abs_idx < 0 {
            self.first_sample.unwrap_or(0)
        } else if abs_idx >= self.total_in_samples as isize {
            self.last_sample.unwrap_or(0)
        } else {
            let u_idx = abs_idx as usize;
            if u_idx >= self.history_start_idx {
                let offset = u_idx - self.history_start_idx;
                if offset < self.input_history.len() {
                    self.input_history[offset]
                } else {
                    self.last_sample.unwrap_or(0)
                }
            } else {
                self.first_sample.unwrap_or(0)
            }
        }
    }

    /// Processa um bloco arbitrário de áudio em streaming.
    ///
    /// Emite apenas as amostras cuja janela de convolução centrada já está inteiramente disponível
    /// dentro das amostras recebidas até o momento, retendo em buffer as amostras necessárias para
    /// as janelas seguintes.
    pub fn process_streaming(&mut self, chunk: &[i16]) -> Vec<i16> {
        if chunk.is_empty() {
            return Vec::new();
        }

        if self.in_rate == self.out_rate {
            self.total_in_samples += chunk.len();
            self.emitted_out_samples += chunk.len();
            return chunk.to_vec();
        }

        if self.first_sample.is_none() && !chunk.is_empty() {
            self.first_sample = Some(chunk[0]);
        }
        self.last_sample = Some(chunk[chunk.len() - 1]);
        self.input_history.extend_from_slice(chunk);
        self.total_in_samples += chunk.len();

        let mut out = Vec::new();

        match self.kind {
            ResamplerKind::FirAntialiasing => {
                if self.in_rate == 48000 && self.out_rate == 16000 {
                    // Decimação 3:1 centrada em m * 3.
                    // Para emitir m, precisamos de amostras até (m * 3 + half_taps).
                    while self.emitted_out_samples * 3 + self.half_taps < self.total_in_samples {
                        let m = self.emitted_out_samples;
                        let center = (m * 3) as isize;
                        let mut acc = 0.0f32;
                        for (j, &h) in self.filter_taps.iter().enumerate() {
                            let abs_idx = center + (j as isize) - (self.half_taps as isize);
                            let s = self.sample_at(abs_idx);
                            acc += (s as f32) * h;
                        }
                        out.push(acc.clamp(-32768.0, 32767.0).round() as i16);
                        self.emitted_out_samples += 1;
                    }

                    // Poda do histórico de amostras que já não serão mais acessadas por janelas futuras
                    let min_needed_idx =
                        (self.emitted_out_samples * 3).saturating_sub(self.half_taps);
                    if min_needed_idx > self.history_start_idx {
                        let to_drain =
                            (min_needed_idx - self.history_start_idx).min(self.input_history.len());
                        self.input_history.drain(..to_drain);
                        self.history_start_idx += to_drain;
                    }
                } else {
                    // Taxas arbitrárias com FIR
                    let ratio = self.in_rate as f64 / self.out_rate as f64;
                    loop {
                        let m = self.emitted_out_samples;
                        let center = ((m as f64) * ratio).round() as usize;
                        if center + self.half_taps >= self.total_in_samples {
                            break;
                        }
                        let mut acc = 0.0f32;
                        for (j, &h) in self.filter_taps.iter().enumerate() {
                            let abs_idx =
                                (center as isize) + (j as isize) - (self.half_taps as isize);
                            let s = self.sample_at(abs_idx);
                            acc += (s as f32) * h;
                        }
                        out.push(acc.clamp(-32768.0, 32767.0).round() as i16);
                        self.emitted_out_samples += 1;
                    }
                    let center_next = ((self.emitted_out_samples as f64) * ratio).round() as usize;
                    let min_needed_idx = center_next.saturating_sub(self.half_taps);
                    if min_needed_idx > self.history_start_idx {
                        let to_drain =
                            (min_needed_idx - self.history_start_idx).min(self.input_history.len());
                        self.input_history.drain(..to_drain);
                        self.history_start_idx += to_drain;
                    }
                }
            }
            ResamplerKind::LinearBaseline => {
                let ratio = self.in_rate as f64 / self.out_rate as f64;
                loop {
                    let m = self.emitted_out_samples;
                    let in_pos = (m as f64) * ratio;
                    let idx = in_pos.floor() as usize;
                    if idx + 1 >= self.total_in_samples {
                        break;
                    }
                    let frac = in_pos - (idx as f64);
                    let s0 = self.sample_at(idx as isize) as f64;
                    let s1 = self.sample_at((idx + 1) as isize) as f64;
                    let interpolated = (1.0 - frac) * s0 + frac * s1;
                    out.push(interpolated.clamp(-32768.0, 32767.0).round() as i16);
                    self.emitted_out_samples += 1;
                }
                let min_needed_idx = (self.emitted_out_samples as f64 * ratio).floor() as usize;
                if min_needed_idx > self.history_start_idx {
                    let to_drain =
                        (min_needed_idx - self.history_start_idx).min(self.input_history.len());
                    self.input_history.drain(..to_drain);
                    self.history_start_idx += to_drain;
                }
            }
        }

        out
    }

    /// Operação de finalização do streaming.
    ///
    /// Informa o término da transmissão física e descarrega todas as amostras restantes
    /// necessárias para atingir a contagem exata `calculate_out_len(total_in_samples)`,
    /// aplicando o padding constante da cauda para completar as janelas de convolução finais.
    pub fn flush(&mut self) -> Vec<i16> {
        if self.in_rate == self.out_rate || self.total_in_samples == 0 {
            return Vec::new();
        }

        let target_out =
            Self::calculate_out_len(self.total_in_samples, self.in_rate, self.out_rate);
        if self.emitted_out_samples >= target_out {
            return Vec::new();
        }

        let mut out = Vec::new();

        match self.kind {
            ResamplerKind::FirAntialiasing => {
                if self.in_rate == 48000 && self.out_rate == 16000 {
                    while self.emitted_out_samples < target_out {
                        let m = self.emitted_out_samples;
                        let center = (m * 3) as isize;
                        let mut acc = 0.0f32;
                        for (j, &h) in self.filter_taps.iter().enumerate() {
                            let abs_idx = center + (j as isize) - (self.half_taps as isize);
                            let s = self.sample_at(abs_idx);
                            acc += (s as f32) * h;
                        }
                        out.push(acc.clamp(-32768.0, 32767.0).round() as i16);
                        self.emitted_out_samples += 1;
                    }
                } else {
                    let ratio = self.in_rate as f64 / self.out_rate as f64;
                    while self.emitted_out_samples < target_out {
                        let m = self.emitted_out_samples;
                        let center = ((m as f64) * ratio).round() as isize;
                        let mut acc = 0.0f32;
                        for (j, &h) in self.filter_taps.iter().enumerate() {
                            let abs_idx = center + (j as isize) - (self.half_taps as isize);
                            let s = self.sample_at(abs_idx);
                            acc += (s as f32) * h;
                        }
                        out.push(acc.clamp(-32768.0, 32767.0).round() as i16);
                        self.emitted_out_samples += 1;
                    }
                }
            }
            ResamplerKind::LinearBaseline => {
                let ratio = self.in_rate as f64 / self.out_rate as f64;
                while self.emitted_out_samples < target_out {
                    let m = self.emitted_out_samples;
                    let in_pos = (m as f64) * ratio;
                    let idx = in_pos.floor() as usize;
                    let frac = in_pos - (idx as f64);
                    let s0 = self.sample_at(idx as isize) as f64;
                    let s1 = self.sample_at((idx + 1) as isize) as f64;
                    let interpolated = (1.0 - frac) * s0 + frac * s1;
                    out.push(interpolated.clamp(-32768.0, 32767.0).round() as i16);
                    self.emitted_out_samples += 1;
                }
            }
        }

        self.input_history.clear();
        out
    }

    /// Reamostragem completa sobre a própria instância com limpeza estrita de estado antes e depois.
    ///
    /// Permite reutilizar instâncias pré-alocadas (preservando coeficientes de filtro pré-calculados)
    /// garantindo que nenhum estado residual de processamentos anteriores contamine a nova saída.
    pub fn resample(&mut self, samples: &[i16]) -> Vec<i16> {
        if self.in_rate == self.out_rate || samples.is_empty() {
            return samples.to_vec();
        }
        self.reset();
        let mut out = self.process_streaming(samples);
        out.extend(self.flush());
        self.reset();
        out
    }

    /// Reamostragem completa de uma gravação fechada (modo lote).
    ///
    /// Implementada com equivalência matemática estrita ao pipeline de streaming:
    /// `resample_oneshot(audio) == process_streaming(audio) + flush()`.
    pub fn resample_oneshot(
        samples: &[i16],
        in_rate: u32,
        out_rate: u32,
        kind: ResamplerKind,
    ) -> Vec<i16> {
        if in_rate == out_rate || samples.is_empty() {
            return samples.to_vec();
        }
        let mut resampler = Self::new(in_rate, out_rate, kind);
        resampler.resample(samples)
    }

    /// Reamostragem linear direta das amostras mono de entrada para a taxa alvo.
    /// Preservada integralmente como baseline matemático e funcional de referência.
    pub fn resample_linear(samples: &[i16], in_rate: u32, out_rate: u32) -> Vec<i16> {
        Self::resample_oneshot(samples, in_rate, out_rate, ResamplerKind::LinearBaseline)
    }
}
