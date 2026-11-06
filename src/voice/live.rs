use crate::contracts::voice::{AudioFormat, AudioFrame, AudioInput, ResamplerKind, VoiceError};
use crate::voice::audio::AudioCodec;
use crate::voice::capture::start_wasapi_continuous_stream;
use crate::voice::conversation::VoiceConversationCoordinator;
use crate::voice::playback::PlaybackStatus;
use crate::voice::wakeword::{LiveWakeWordSpotter, OpenWakeWordEngine, VoiceActivityDetector};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Estado atual da máquina de estados conversacional contínua.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveConversationState {
    /// Escutando continuamente pelo microfone aguardando a palavra de ativação "Yuki".
    AwaitingWakeWord,
    /// Palavra de ativação detectada (ou fala em follow-up); capturando a frase do usuário até o silêncio.
    CapturingPhrase,
    /// Processando o turno completo (STT -> Core -> TTS -> Playback).
    ProcessingTurn,
    /// Janela pós-resposta onde o usuário pode falar diretamente sem repetir "Yuki".
    FollowUpWindow,
    /// Conversação encerrada pelo operador ou sinal de terminação (Ctrl+C).
    Terminated,
}

impl std::fmt::Display for LiveConversationState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AwaitingWakeWord => write!(f, "AwaitingWakeWord"),
            Self::CapturingPhrase => write!(f, "CapturingPhrase"),
            Self::ProcessingTurn => write!(f, "ProcessingTurn"),
            Self::FollowUpWindow => write!(f, "FollowUpWindow"),
            Self::Terminated => write!(f, "Terminated"),
        }
    }
}

/// Configuração operacional do listener de voz em tempo real.
#[derive(Debug, Clone)]
pub struct VoiceLiveListenerConfig {
    /// Limiar de ativação para a palavra "Yuki" (0.0 a 1.0). Padrão: 0.45.
    pub threshold: f32,
    /// Duração da janela conversacional de follow-up em segundos. Padrão: 7s.
    pub follow_up_secs: u64,
    /// Tempo de silêncio contínuo após fala para declarar End of Speech (EOS). Padrão: 950ms.
    pub silence_hangover_ms: u64,
    /// Janela de tolerância inicial logo após o disparo de Wake Word antes do primeiro bloco de fala pós-gatilho
    /// (preserva pausas prosódicas após a vírgula em "Yuki, ..."). `None` = proporcional (`1.5x silence_hangover_ms`, teto 1450ms).
    pub initial_speech_grace_ms: Option<u64>,
    /// Piso mínimo de energia RMS (16-bit PCM) para continuidade de fala durante `CapturingPhrase`. Padrão: 52.0.
    pub speech_continuation_rms_floor: f32,
    /// Limiar mínimo de energia RMS (16-bit PCM) para disparo de fala na janela `FollowUpWindow`. Padrão: 72.0.
    pub followup_min_rms: f32,
    /// Tempo máximo de gravação de uma única frase em segundos. Padrão: 10s.
    pub max_phrase_secs: u64,
    /// Tamanho do buffer circular de pré-roll em milissegundos. Padrão: 600ms.
    pub pre_roll_ms: usize,
    /// Cooldown acústico após término físico do playback para rejeitar ecos residuais. Padrão: 450ms.
    pub acoustic_cooldown_ms: u64,
    /// Cooldown adicional antes de rearmar a detecção de Wake Word após turno/erro/expiração. Padrão: 600ms.
    pub wake_cooldown_ms: u64,
    /// Blocos consecutivos de 80ms confirmados pelo VAD multi-frame para disparar captura na janela de follow-up. Padrão: 3 (240ms).
    pub followup_confirm_frames: usize,
    /// Identificador do dispositivo de entrada (None para padrão do sistema).
    pub input_device: Option<String>,
    /// Identificador do dispositivo de saída (None para padrão do sistema).
    pub output_device: Option<String>,
    /// Tipo de algoritmo de reamostragem do microfone.
    pub resampler_kind: ResamplerKind,
    /// Permissão de fallback para provedores em nuvem (STT).
    pub allow_cloud_stt: bool,
    /// Permissão de fallback para provedores em nuvem (TTS).
    pub allow_cloud_tts: bool,
    /// Frases ou palavras-chave de despedida/cancelamento.
    pub exit_phrases: Vec<String>,
}

impl Default for VoiceLiveListenerConfig {
    fn default() -> Self {
        Self {
            threshold: 0.45,
            follow_up_secs: 7,
            silence_hangover_ms: 950,
            initial_speech_grace_ms: None,
            speech_continuation_rms_floor: 52.0,
            followup_min_rms: 72.0,
            max_phrase_secs: 10,
            pre_roll_ms: 600,
            acoustic_cooldown_ms: 450,
            wake_cooldown_ms: 600,
            followup_confirm_frames: 3,
            input_device: None,
            output_device: None,
            resampler_kind: ResamplerKind::FirAntialiasing,
            allow_cloud_stt: false,
            allow_cloud_tts: false,
            exit_phrases: vec![
                "tchau".to_string(),
                "adeus".to_string(),
                "até logo".to_string(),
                "cancelar".to_string(),
                "só isso".to_string(),
                "encerrar".to_string(),
                "obrigado".to_string(),
                "obrigada".to_string(),
                "valeu".to_string(),
                "dispensada".to_string(),
                "pare".to_string(),
                "parar".to_string(),
            ],
        }
    }
}

/// Registro acústico e temporal de um turno conversacional executado.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LiveTurnRecord {
    pub turn_index: usize,
    pub is_follow_up: bool,
    pub wake_word_score: f32,
    pub wake_word_inference_ms: f32,
    pub phrase_duration_ms: u64,
    pub eos_hangover_ms: u64,
    pub stt_duration_ms: u64,
    pub core_duration_ms: u64,
    pub tts_duration_ms: u64,
    pub playback_duration_ms: u64,
    pub total_to_playback_ms: u64,
    pub total_audible_latency_ms: u64,
    pub user_text: String,
    pub yuki_text: String,
}

/// Estatísticas agregadas de desempenho e latência da sessão ao vivo.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct VoiceLatencyStats {
    pub total_turns: usize,
    pub follow_up_turns: usize,
    pub empty_stt_discards: usize,
    pub p50_ww_ms: f64,
    pub p95_ww_ms: f64,
    pub p50_stt_ms: f64,
    pub p95_stt_ms: f64,
    pub p50_core_ms: f64,
    pub p95_core_ms: f64,
    pub p50_tts_ms: f64,
    pub p95_tts_ms: f64,
    pub p50_playback_ms: f64,
    pub p95_playback_ms: f64,
    pub p50_audible_ms: f64,
    pub p95_audible_ms: f64,
    pub records: Vec<LiveTurnRecord>,
}

impl VoiceLatencyStats {
    pub fn compute(records: Vec<LiveTurnRecord>) -> Self {
        Self::compute_with_discards(records, 0)
    }

    pub fn compute_with_discards(records: Vec<LiveTurnRecord>, empty_stt_discards: usize) -> Self {
        let total_turns = records.len();
        let follow_up_turns = records.iter().filter(|r| r.is_follow_up).count();

        if total_turns == 0 {
            return Self {
                total_turns: 0,
                follow_up_turns: 0,
                empty_stt_discards,
                records,
                ..Default::default()
            };
        }

        let mut ww: Vec<f64> = records
            .iter()
            .map(|r| r.wake_word_inference_ms as f64)
            .collect();
        let mut stt: Vec<f64> = records.iter().map(|r| r.stt_duration_ms as f64).collect();
        let mut core: Vec<f64> = records.iter().map(|r| r.core_duration_ms as f64).collect();
        let mut tts: Vec<f64> = records.iter().map(|r| r.tts_duration_ms as f64).collect();
        let mut pb: Vec<f64> = records
            .iter()
            .map(|r| r.playback_duration_ms as f64)
            .collect();
        let mut aud: Vec<f64> = records
            .iter()
            .map(|r| r.total_audible_latency_ms as f64)
            .collect();

        Self {
            total_turns,
            follow_up_turns,
            empty_stt_discards,
            p50_ww_ms: percentile(&mut ww, 0.50),
            p95_ww_ms: percentile(&mut ww, 0.95),
            p50_stt_ms: percentile(&mut stt, 0.50),
            p95_stt_ms: percentile(&mut stt, 0.95),
            p50_core_ms: percentile(&mut core, 0.50),
            p95_core_ms: percentile(&mut core, 0.95),
            p50_tts_ms: percentile(&mut tts, 0.50),
            p95_tts_ms: percentile(&mut tts, 0.95),
            p50_playback_ms: percentile(&mut pb, 0.50),
            p95_playback_ms: percentile(&mut pb, 0.95),
            p50_audible_ms: percentile(&mut aud, 0.50),
            p95_audible_ms: percentile(&mut aud, 0.95),
            records,
        }
    }

    pub fn print_summary_report(&self) {
        println!("\n========================================================");
        println!("       RELATÓRIO DE DESEMPENHO VOCAL DA YUKI (LIVE)");
        println!("========================================================");
        println!("Total de turnos processados: {}", self.total_turns);
        println!("Turnos em follow-up direto : {}", self.follow_up_turns);
        println!("Segmentos vazios descartados: {}", self.empty_stt_discards);
        println!("--------------------------------------------------------");
        println!("MÉTRICAS TEMPORAIS (p50 / p95):");
        println!(
            "  - Inferência Wake Word (T_ww) : {:>6.1} ms  |  p95: {:>6.1} ms",
            self.p50_ww_ms, self.p95_ww_ms
        );
        println!(
            "  - Transcrição Whisper  (T_stt): {:>6.1} ms  |  p95: {:>6.1} ms",
            self.p50_stt_ms, self.p95_stt_ms
        );
        println!(
            "  - Processamento Core   (T_core): {:>6.1} ms |  p95: {:>6.1} ms",
            self.p50_core_ms, self.p95_core_ms
        );
        println!(
            "  - Síntese Piper TTS    (T_tts): {:>6.1} ms  |  p95: {:>6.1} ms",
            self.p50_tts_ms, self.p95_tts_ms
        );
        println!(
            "  - Duração de Fala      (T_play): {:>6.1} ms |  p95: {:>6.1} ms",
            self.p50_playback_ms, self.p95_playback_ms
        );
        println!("--------------------------------------------------------");
        println!("  >>> LATÊNCIA AUDÍVEL TOTAL (T_total_audible):");
        println!(
            "      p50: {:>6.1} ms   |   p95: {:>6.1} ms",
            self.p50_audible_ms, self.p95_audible_ms
        );
        println!("========================================================\n");
    }
}

fn percentile(values: &mut [f64], pct: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let rank = (values.len() - 1) as f64 * pct;
    let lower = rank.floor() as usize;
    let upper = rank.ceil() as usize;
    if lower == upper {
        values[lower]
    } else {
        let weight = rank - lower as f64;
        values[lower] * (1.0 - weight) + values[upper] * weight
    }
}

enum SpotterHandle {
    OpenWakeWord(Arc<tokio::sync::Mutex<OpenWakeWordEngine>>),
    Custom(Arc<dyn LiveWakeWordSpotter>),
}

impl SpotterHandle {
    async fn process_chunk(&self, chunk: &[i16]) -> Result<f32, VoiceError> {
        match self {
            Self::OpenWakeWord(m) => m.lock().await.process_chunk(chunk),
            Self::Custom(s) => s.process_chunk(chunk),
        }
    }

    async fn reset(&self) {
        match self {
            Self::OpenWakeWord(m) => m.lock().await.reset(),
            Self::Custom(s) => s.reset(),
        }
    }
}

/// Avalia se um bloco canônico de 80ms (1280 amostras @ 16kHz) contém continuidade real de fala,
/// preservando sílabas baixas e números finais (ex.: microfones integrados em ~60–250 RMS)
/// enquanto descarta silêncio de sala (~14 RMS) e ruído estacionário contínuo (zumbido/ventoinha).
fn is_speech_continuation_block(
    block: &[i16],
    vad_window: &[AudioFrame],
    effective_min_rms: f32,
) -> bool {
    if block.is_empty() {
        return false;
    }

    let mut total_sq: f64 = 0.0;
    let mut zcr: usize = 0;
    for (i, &s) in block.iter().enumerate() {
        total_sq += (s as f64) * (s as f64);
        if i > 0 && ((s >= 0 && block[i - 1] < 0) || (s < 0 && block[i - 1] >= 0)) {
            zcr += 1;
        }
    }
    let block_rms = (total_sq / block.len() as f64).sqrt() as f32;
    let zcr_rate = zcr as f32 / block.len() as f32;

    if !(0.012..=0.48).contains(&zcr_rate) {
        return false;
    }

    // Sub-janelas de 20ms (320 amostras @ 16kHz) para preservar consoantes/finais curtos de baixa amplitude
    let sub_len = 320usize;
    let mut sub_rms = Vec::with_capacity(4);
    let mut active_subs = 0usize;
    // Limiar dinâmico para picos/plosivas curtas: calibra com base no piso efetivo,
    // evitando disparos falsos com o ruído/chiado residual de microfones Bluetooth (típico em 190–240 pico)
    let peak_plosive_thresh = (effective_min_rms * 3.5).clamp(280.0, 600.0) as i16;

    for chunk in block.chunks(sub_len) {
        if chunk.is_empty() {
            continue;
        }
        let mut sq = 0.0f64;
        let mut peak = 0i16;
        for &s in chunk {
            let av = s.saturating_abs();
            if av > peak {
                peak = av;
            }
            sq += (s as f64) * (s as f64);
        }
        let r = (sq / chunk.len() as f64).sqrt() as f32;
        sub_rms.push(r);
        if r >= effective_min_rms || peak >= peak_plosive_thresh {
            active_subs += 1;
        }
    }

    if block_rms < effective_min_rms && (active_subs < 2 || block_rms < (effective_min_rms * 0.65))
    {
        return false;
    }

    // Rejeita ruído contínuo estacionário (ex.: ventilador constante ou chiado de Bluetooth wireless)
    if vad_window.len() >= 3 && !sub_rms.is_empty() {
        let sub_mean = sub_rms.iter().sum::<f32>() / sub_rms.len() as f32;
        let sub_var =
            sub_rms.iter().map(|&r| (r - sub_mean).powi(2)).sum::<f32>() / sub_rms.len() as f32;
        let sub_std = sub_var.sqrt();

        let macro_rms: Vec<f32> = vad_window.iter().map(|f| f.rms_energy()).collect();
        let macro_mean = macro_rms.iter().sum::<f32>() / macro_rms.len() as f32;
        let macro_var = macro_rms
            .iter()
            .map(|&r| (r - macro_mean).powi(2))
            .sum::<f32>()
            / macro_rms.len() as f32;
        let macro_std = macro_var.sqrt();

        if macro_std < 3.5 && sub_std < 3.5 {
            return false;
        }
    }

    true
}

/// Orquestrador governado do ouvinte de voz contínuo da Yuki.
pub struct VoiceLiveListener {
    config: VoiceLiveListenerConfig,
    coordinator: Arc<VoiceConversationCoordinator>,
    spotter: SpotterHandle,
    followup_vad: VoiceActivityDetector,
}

impl VoiceLiveListener {
    pub fn new(
        config: VoiceLiveListenerConfig,
        coordinator: Arc<VoiceConversationCoordinator>,
        wakeword_engine: Arc<tokio::sync::Mutex<OpenWakeWordEngine>>,
    ) -> Self {
        let followup_vad =
            VoiceActivityDetector::new(config.followup_min_rms.max(45.0), 30000.0, 0.015, 0.45)
                .with_min_std_dev(8.0);
        Self {
            config,
            coordinator,
            spotter: SpotterHandle::OpenWakeWord(wakeword_engine),
            followup_vad,
        }
    }

    /// Construtor alternativo aceitando qualquer implementação de `LiveWakeWordSpotter`
    /// (ex.: motores determinísticos em testes de regressão CI sem dependência de DLL externa).
    pub fn with_spotter(
        config: VoiceLiveListenerConfig,
        coordinator: Arc<VoiceConversationCoordinator>,
        spotter: Arc<dyn LiveWakeWordSpotter>,
    ) -> Self {
        let followup_vad =
            VoiceActivityDetector::new(config.followup_min_rms.max(45.0), 30000.0, 0.015, 0.45)
                .with_min_std_dev(8.0);
        Self {
            config,
            coordinator,
            spotter: SpotterHandle::Custom(spotter),
            followup_vad,
        }
    }

    /// Executa o loop conversacional conectando diretamente ao microfone do sistema operacional via WASAPI.
    pub async fn run_with_live_mic(
        &self,
        shutdown_flag: Arc<AtomicBool>,
    ) -> Result<VoiceLatencyStats, VoiceError> {
        let (_stream_controller, rx_samples) = start_wasapi_continuous_stream(
            self.config.input_device.clone(),
            self.config.resampler_kind,
        )?;

        self.run_event_loop(rx_samples, shutdown_flag).await
    }

    /// Executa o loop conversacional consumindo amostras PCM de um canal pré-configurado (útil para testes).
    pub async fn run_event_loop(
        &self,
        rx_samples: std::sync::mpsc::Receiver<Vec<i16>>,
        shutdown_flag: Arc<AtomicBool>,
    ) -> Result<VoiceLatencyStats, VoiceError> {
        // Garante que o motor de Wake Word inicia com buffers limpos e exige warmup completo (16 blocos)
        self.spotter.reset().await;

        let mut state = LiveConversationState::AwaitingWakeWord;
        let mut state_changed_instant = Instant::now();
        let mut wake_cooldown_until: Option<Instant> = None;

        let pre_roll_samples_capacity = (16000 * self.config.pre_roll_ms) / 1000;
        let mut pre_roll_buffer: Vec<i16> = Vec::with_capacity(pre_roll_samples_capacity);

        let mut phrase_buffer: Vec<i16> = Vec::new();
        let mut vad_window: Vec<AudioFrame> = Vec::with_capacity(4);
        let mut consecutive_speech_blocks: usize = 0;
        let mut trailing_silent_blocks: usize = 0;

        let mut phrase_start_instant: Option<Instant> = None;
        let mut last_speech_instant: Option<Instant> = None;
        let mut post_trigger_speech_detected = false;
        let mut ambient_rms_ema: f32 = 16.0;

        let mut turn_records: Vec<LiveTurnRecord> = Vec::new();
        let mut empty_stt_discards: usize = 0;
        let mut conversation_history: Vec<String> = Vec::new();

        let mut last_ww_score: f32 = 0.0;
        let mut last_ww_time_ms: f32 = 0.0;
        let mut is_current_turn_follow_up = false;

        // Bloco PCM para o openWakeWord e VAD: 80 ms = 1280 amostras em 16 kHz mono
        let block_size = 1280usize;
        let max_queue_samples = 96_000usize; // limite de segurança de 6s contra acúmulo extremo
        let mut incoming_pcm_queue: Vec<i16> = Vec::new();

        println!("\n[Yuki] Sistema de voz contínuo iniciado.");
        println!(
            "[Yuki] Diga 'Yuki' para chamar ou fale após a resposta durante a janela de {}s.",
            self.config.follow_up_secs
        );
        println!(
            "[Yuki] Pressione Ctrl+C a qualquer momento para encerrar com relatório completo.\n"
        );

        while shutdown_flag.load(Ordering::SeqCst) {
            // 1. Drena amostras recebidas do canal WASAPI sem bloquear
            let mut got_samples = false;
            if self.coordinator.playback().status() == PlaybackStatus::Playing {
                // Se estivermos em playback ativo, descarta pacotes de entrada (supressão acústica de auto-gatilho)
                while let Ok(samples) = rx_samples.try_recv() {
                    if !samples.is_empty() {
                        got_samples = true;
                    }
                }
                incoming_pcm_queue.clear();
                pre_roll_buffer.clear();
            } else {
                // Drena incrementalmente até 2 blocos por iteração para preservar fronteiras exatas entre estados
                while incoming_pcm_queue.len() < block_size * 2 {
                    match rx_samples.try_recv() {
                        Ok(samples) => {
                            if !samples.is_empty() {
                                got_samples = true;
                                incoming_pcm_queue.extend_from_slice(&samples);
                            }
                        }
                        Err(_) => break,
                    }
                }
                if incoming_pcm_queue.len() > max_queue_samples {
                    let excess = incoming_pcm_queue.len() - max_queue_samples;
                    incoming_pcm_queue.drain(0..excess);
                }
            }

            // 2. Transições e processamento por estado
            match state {
                LiveConversationState::AwaitingWakeWord => {
                    while incoming_pcm_queue.len() >= block_size {
                        let block: Vec<i16> = incoming_pcm_queue.drain(0..block_size).collect();

                        // Durante o cooldown pós-turno/pós-erro, descarta reverberação residual sem contaminar buffers do spotter
                        if let Some(until) = wake_cooldown_until {
                            if Instant::now() < until {
                                pre_roll_buffer.clear();
                                continue;
                            }
                            wake_cooldown_until = None;
                        }

                        // Atualiza estimativa adaptativa do piso de ruído de sala apenas em quadros calmos
                        {
                            let mut sq = 0.0f64;
                            let mut pk = 0i16;
                            for &s in &block {
                                let av = s.saturating_abs();
                                if av > pk {
                                    pk = av;
                                }
                                sq += (s as f64) * (s as f64);
                            }
                            let b_rms = (sq / block.len() as f64).sqrt() as f32;
                            if b_rms < 55.0 && pk < 260 {
                                ambient_rms_ema =
                                    (0.90 * ambient_rms_ema + 0.10 * b_rms).clamp(8.0, 65.0);
                            }
                        }

                        // Atualiza buffer circular de pré-roll
                        if pre_roll_samples_capacity > 0 {
                            pre_roll_buffer.extend_from_slice(&block);
                            if pre_roll_buffer.len() > pre_roll_samples_capacity {
                                let excess = pre_roll_buffer.len() - pre_roll_samples_capacity;
                                pre_roll_buffer.drain(0..excess);
                            }
                        }

                        // Avalia palavra de ativação no motor openWakeWord
                        let t_infer_start = Instant::now();
                        let score_res = self.spotter.process_chunk(&block).await;
                        let infer_ms = t_infer_start.elapsed().as_secs_f32() * 1000.0;
                        let score = score_res.unwrap_or(0.0);

                        if score >= self.config.threshold {
                            // CRÍTICO: limpa imediatamente feat_buffer (16x96) e mel_buffer (76x32) do openWakeWord
                            // para impedir re-ativações fantasmas (0.99 / 0.98) em silêncio nos ciclos seguintes.
                            self.spotter.reset().await;

                            println!(
                                "\n[Yuki] >> Palavra de ativação detectada! (score: {:.2}, infer: {:.1}ms) Ouvindo...",
                                score, infer_ms
                            );
                            last_ww_score = score;
                            last_ww_time_ms = infer_ms;
                            is_current_turn_follow_up = false;

                            // Inicializa buffer de captura com pré-roll para não perder o início da fala
                            phrase_buffer.clear();
                            phrase_buffer.extend_from_slice(&pre_roll_buffer);
                            pre_roll_buffer.clear();
                            vad_window.clear();
                            consecutive_speech_blocks = 0;
                            trailing_silent_blocks = 0;
                            post_trigger_speech_detected = false;

                            phrase_start_instant = Some(Instant::now());
                            last_speech_instant = Some(Instant::now());

                            state = LiveConversationState::CapturingPhrase;
                            state_changed_instant = Instant::now();
                            break;
                        }
                    }
                }

                LiveConversationState::FollowUpWindow => {
                    let elapsed = state_changed_instant.elapsed();
                    if elapsed >= Duration::from_secs(self.config.follow_up_secs) {
                        println!(
                            "[Yuki] Janela de follow-up expirada. Retornando para escuta de 'Yuki'..."
                        );
                        self.spotter.reset().await;
                        pre_roll_buffer.clear();
                        vad_window.clear();
                        consecutive_speech_blocks = 0;
                        trailing_silent_blocks = 0;
                        if self.config.wake_cooldown_ms > 0 {
                            wake_cooldown_until = Some(
                                Instant::now()
                                    + Duration::from_millis(self.config.wake_cooldown_ms),
                            );
                        }
                        state = LiveConversationState::AwaitingWakeWord;
                        state_changed_instant = Instant::now();
                        continue;
                    }

                    // Na janela de follow-up, exigimos confirmação multi-frame de fala articulada
                    // (janela deslizante >= 3 quadros para ativar rejeição de ruído estacionário por variância no VAD)
                    let required_confirm = self.config.followup_confirm_frames.max(1);
                    let min_vad_window = required_confirm.min(3);

                    while incoming_pcm_queue.len() >= block_size {
                        let block: Vec<i16> = incoming_pcm_queue.drain(0..block_size).collect();

                        // Atualiza pré-roll (600ms preserva todo o início da fala durante os 240ms de confirmação)
                        if pre_roll_samples_capacity > 0 {
                            pre_roll_buffer.extend_from_slice(&block);
                            if pre_roll_buffer.len() > pre_roll_samples_capacity {
                                let excess = pre_roll_buffer.len() - pre_roll_samples_capacity;
                                pre_roll_buffer.drain(0..excess);
                            }
                        }

                        let frame = AudioFrame::new(block, 16000, 1, 0);
                        let frame_rms = frame.rms_energy();
                        if frame_rms < 55.0 {
                            ambient_rms_ema =
                                (0.90 * ambient_rms_ema + 0.10 * frame_rms).clamp(8.0, 65.0);
                        }
                        let followup_gate_rms =
                            (ambient_rms_ema * 3.0).max(self.followup_vad.min_rms_speech());
                        let current_non_silent = !frame.is_silent(followup_gate_rms);

                        if current_non_silent {
                            if consecutive_speech_blocks == 0 {
                                vad_window.clear();
                            }
                            consecutive_speech_blocks += 1;
                            if vad_window.len() >= 4 {
                                vad_window.remove(0);
                            }
                            vad_window.push(frame);
                        } else {
                            consecutive_speech_blocks = 0;
                            vad_window.clear();
                        }

                        if consecutive_speech_blocks >= required_confirm
                            && vad_window.len() >= min_vad_window
                            && self.followup_vad.is_speech(&vad_window)
                        {
                            println!(
                                "\n[Yuki] >> Fala detectada na janela de follow-up! Ouvindo..."
                            );
                            last_ww_score = 1.0;
                            last_ww_time_ms = 0.0;
                            is_current_turn_follow_up = true;

                            phrase_buffer.clear();
                            phrase_buffer.extend_from_slice(&pre_roll_buffer);
                            pre_roll_buffer.clear();
                            consecutive_speech_blocks = 0;
                            trailing_silent_blocks = 0;
                            // Na janela de follow-up a fala já foi confirmada por 3 quadros consecutivos
                            post_trigger_speech_detected = true;

                            phrase_start_instant = Some(Instant::now());
                            last_speech_instant = Some(Instant::now());

                            state = LiveConversationState::CapturingPhrase;
                            state_changed_instant = Instant::now();
                            break;
                        }
                    }
                }

                LiveConversationState::CapturingPhrase => {
                    let effective_cont_rms = (ambient_rms_ema * 2.5)
                        .max(self.config.speech_continuation_rms_floor)
                        .clamp(42.0, 140.0);

                    // Enquanto a fala pós-gatilho ainda não iniciou (ex.: pausa de vírgula após "Yuki,"),
                    // respeita janela inicial de tolerância antes de declarar EOS.
                    let calc_required_silence_ms = |post_speech: bool| -> u64 {
                        if post_speech {
                            self.config.silence_hangover_ms
                        } else {
                            self.config
                                .initial_speech_grace_ms
                                .unwrap_or_else(|| {
                                    (self.config.silence_hangover_ms.saturating_mul(3) / 2)
                                        .min(1450)
                                })
                                .max(self.config.silence_hangover_ms)
                        }
                    };

                    let mut triggered_turn = false;

                    // Avalia continuidade de fala bloco a bloco (80ms = 1280 amostras @ 16kHz),
                    // combinando tempo acústico real de amostras com relógio monotônico.
                    while incoming_pcm_queue.len() >= block_size {
                        let eval_block: Vec<i16> =
                            incoming_pcm_queue.drain(0..block_size).collect();
                        phrase_buffer.extend_from_slice(&eval_block);

                        let frame = AudioFrame::new(eval_block.clone(), 16000, 1, 0);
                        if vad_window.len() >= 4 {
                            vad_window.remove(0);
                        }
                        vad_window.push(frame);

                        if is_speech_continuation_block(
                            &eval_block,
                            &vad_window,
                            effective_cont_rms,
                        ) {
                            last_speech_instant = Some(Instant::now());
                            post_trigger_speech_detected = true;
                            trailing_silent_blocks = 0;
                        } else {
                            trailing_silent_blocks += 1;
                        }

                        let wall_phrase_ms = phrase_start_instant
                            .map(|i| i.elapsed().as_millis() as u64)
                            .unwrap_or(0);
                        let acoustic_phrase_ms = (phrase_buffer.len() as u64 * 1000) / 16000;
                        let effective_phrase_ms = wall_phrase_ms.max(acoustic_phrase_ms);

                        let wall_silence_ms = last_speech_instant
                            .map(|i| i.elapsed().as_millis() as u64)
                            .unwrap_or(0);
                        let acoustic_silence_ms = (trailing_silent_blocks as u64) * 80;
                        let effective_silence_ms = wall_silence_ms.max(acoustic_silence_ms);

                        let required_silence_ms =
                            calc_required_silence_ms(post_trigger_speech_detected);
                        let reached_no_speech_timeout =
                            !post_trigger_speech_detected && effective_phrase_ms >= 2500;
                        let reached_eos = (effective_silence_ms >= required_silence_ms
                            && effective_phrase_ms >= 400)
                            || reached_no_speech_timeout;
                        let reached_max = effective_phrase_ms >= self.config.max_phrase_secs * 1000;

                        if reached_eos || reached_max {
                            println!(
                                "[Yuki] Fim da fala detectado (duração: {:.2}s, silêncio: {}ms). Processando turno...",
                                effective_phrase_ms as f32 / 1000.0,
                                effective_silence_ms
                            );
                            state = LiveConversationState::ProcessingTurn;
                            state_changed_instant = Instant::now();
                            triggered_turn = true;
                            break;
                        }
                    }

                    if !triggered_turn {
                        let wall_phrase_ms = phrase_start_instant
                            .map(|i| i.elapsed().as_millis() as u64)
                            .unwrap_or(0);
                        let acoustic_phrase_ms =
                            ((phrase_buffer.len() + incoming_pcm_queue.len()) as u64 * 1000)
                                / 16000;
                        let effective_phrase_ms = wall_phrase_ms.max(acoustic_phrase_ms);

                        let wall_silence_ms = last_speech_instant
                            .map(|i| i.elapsed().as_millis() as u64)
                            .unwrap_or(0);
                        let acoustic_silence_ms = (trailing_silent_blocks as u64) * 80;
                        let effective_silence_ms = wall_silence_ms.max(acoustic_silence_ms);

                        let required_silence_ms =
                            calc_required_silence_ms(post_trigger_speech_detected);
                        let reached_no_speech_timeout =
                            !post_trigger_speech_detected && effective_phrase_ms >= 2500;
                        let reached_eos = (effective_silence_ms >= required_silence_ms
                            && effective_phrase_ms >= 400)
                            || reached_no_speech_timeout;
                        let reached_max = effective_phrase_ms >= self.config.max_phrase_secs * 1000;

                        if reached_eos || reached_max {
                            if !incoming_pcm_queue.is_empty() {
                                phrase_buffer.extend_from_slice(&incoming_pcm_queue);
                                incoming_pcm_queue.clear();
                            }
                            println!(
                                "[Yuki] Fim da fala detectado (duração: {:.2}s, silêncio: {}ms). Processando turno...",
                                effective_phrase_ms as f32 / 1000.0,
                                effective_silence_ms
                            );
                            state = LiveConversationState::ProcessingTurn;
                            state_changed_instant = Instant::now();
                        }
                    }
                }

                LiveConversationState::ProcessingTurn => {
                    // Limpa filas residuais de estado antes de decodificar a frase capturada
                    incoming_pcm_queue.clear();
                    vad_window.clear();
                    consecutive_speech_blocks = 0;
                    trailing_silent_blocks = 0;

                    if phrase_buffer.len() < 3200 {
                        // Menos de 200ms de áudio é ruído irrelevante
                        println!(
                            "[Yuki] Áudio capturado muito curto para decodificação. Descartando."
                        );
                        self.spotter.reset().await;
                        while rx_samples.try_recv().is_ok() {}
                        pre_roll_buffer.clear();
                        if self.config.wake_cooldown_ms > 0 {
                            wake_cooldown_until = Some(
                                Instant::now()
                                    + Duration::from_millis(self.config.wake_cooldown_ms),
                            );
                        }
                        state = LiveConversationState::AwaitingWakeWord;
                        state_changed_instant = Instant::now();
                        continue;
                    }

                    // Codifica buffer PCM em WAV canônico
                    let wav_bytes =
                        match AudioCodec::encode_wav(&phrase_buffer, &AudioFormat::default()) {
                            Ok(b) => b,
                            Err(e) => {
                                eprintln!("[Yuki] Falha ao codificar áudio WAV: {}", e);
                                self.spotter.reset().await;
                                while rx_samples.try_recv().is_ok() {}
                                pre_roll_buffer.clear();
                                if self.config.wake_cooldown_ms > 0 {
                                    wake_cooldown_until = Some(
                                        Instant::now()
                                            + Duration::from_millis(self.config.wake_cooldown_ms),
                                    );
                                }
                                state = LiveConversationState::AwaitingWakeWord;
                                state_changed_instant = Instant::now();
                                continue;
                            }
                        };

                    let phrase_duration_ms = (phrase_buffer.len() as u64 * 1000) / 16000;
                    phrase_buffer.clear();
                    let audio_input = AudioInput::new(
                        wav_bytes,
                        AudioFormat::default(),
                        phrase_duration_ms,
                        "wasapi-live",
                    );

                    let turn_start_instant = Instant::now();
                    let session_device_id = self
                        .config
                        .input_device
                        .as_deref()
                        .unwrap_or("live-endpoint");

                    // Processa turno com governança de sessão, histórico e instrumentação de timing
                    let turn_result = self
                        .coordinator
                        .process_turn_instrumented(
                            session_device_id,
                            audio_input,
                            conversation_history.clone(),
                            self.config.allow_cloud_stt,
                            self.config.allow_cloud_tts,
                        )
                        .await;

                    match turn_result {
                        Ok((transcribed_text, yuki_res, audio_out, timings)) => {
                            let _total_turn_time = turn_start_instant.elapsed().as_millis() as u64;
                            let eos_hangover_ms = self.config.silence_hangover_ms;
                            let total_audible_ms = eos_hangover_ms + timings.total_to_playback_ms;

                            println!("\n--------------------------------------------------------");
                            println!("Usuário: \"{}\"", transcribed_text);
                            println!("Yuki:    \"{}\"", yuki_res.content);
                            println!("--------------------------------------------------------");
                            println!(
                                "Métricas: STT: {}ms | Core: {}ms | TTS: {}ms | Até som: {}ms (audível: {}ms)",
                                timings.stt_duration_ms,
                                timings.core_duration_ms,
                                timings.tts_duration_ms,
                                timings.total_to_playback_ms,
                                total_audible_ms
                            );
                            if let Some(stt_t) = self.coordinator.router().last_stt_timings() {
                                println!(
                                    "  [STT] prep+trim: {:.1}ms | norm: {:.1}ms | wav: {:.1}ms | proc: {:.0}ms (load: {:.0}ms, encode: {:.0}ms, decode: {:.0}ms) | post: {:.1}ms",
                                    stt_t.audio_prep_ms,
                                    stt_t.audio_norm_ms,
                                    stt_t.temp_write_ms,
                                    stt_t.subprocess_wall_ms,
                                    stt_t.whisper_load_ms.unwrap_or(0.0),
                                    stt_t.whisper_encode_ms.unwrap_or(0.0),
                                    stt_t.whisper_decode_ms.unwrap_or(0.0),
                                    stt_t.post_process_ms,
                                );
                            }

                            let playback_duration_ms = audio_out.duration_ms;

                            // Registra métricas do turno
                            turn_records.push(LiveTurnRecord {
                                turn_index: turn_records.len() + 1,
                                is_follow_up: is_current_turn_follow_up,
                                wake_word_score: last_ww_score,
                                wake_word_inference_ms: last_ww_time_ms,
                                phrase_duration_ms,
                                eos_hangover_ms,
                                stt_duration_ms: timings.stt_duration_ms,
                                core_duration_ms: timings.core_duration_ms,
                                tts_duration_ms: timings.tts_duration_ms,
                                playback_duration_ms,
                                total_to_playback_ms: timings.total_to_playback_ms,
                                total_audible_latency_ms: total_audible_ms,
                                user_text: transcribed_text.clone(),
                                yuki_text: yuki_res.content.clone(),
                            });

                            // Atualiza histórico conversacional (mantém até 8 turnos recentes)
                            conversation_history.push(format!("Usuário: {}", transcribed_text));
                            conversation_history.push(format!("Yuki: {}", yuki_res.content));
                            if conversation_history.len() > 16 {
                                conversation_history.drain(0..2);
                            }

                            // Aguarda conclusão do playback se estiver tocando (interrompendo cedo se Ctrl+C ocorrer)
                            let pb_controller = self.coordinator.playback();
                            let pb_timeout = Instant::now();
                            while shutdown_flag.load(Ordering::SeqCst)
                                && pb_controller.status() == PlaybackStatus::Playing
                                && pb_timeout.elapsed()
                                    < Duration::from_millis(playback_duration_ms + 1000)
                            {
                                tokio::time::sleep(Duration::from_millis(50)).await;
                            }

                            // Cooldown acústico para evitar pegar o eco da própria fala nos microfones
                            if shutdown_flag.load(Ordering::SeqCst)
                                && self.config.acoustic_cooldown_ms > 0
                            {
                                tokio::time::sleep(Duration::from_millis(
                                    self.config.acoustic_cooldown_ms,
                                ))
                                .await;
                            }

                            // Drena qualquer áudio capturado durante a fala/cooldown e reseta buffers do spotter
                            while rx_samples.try_recv().is_ok() {}
                            incoming_pcm_queue.clear();
                            pre_roll_buffer.clear();
                            vad_window.clear();
                            consecutive_speech_blocks = 0;
                            trailing_silent_blocks = 0;
                            self.spotter.reset().await;

                            // Avalia se o usuário expressou intenção de encerramento
                            let is_exit = self.is_exit_phrase(&transcribed_text);

                            if is_exit {
                                println!("[Yuki] Despedida ou encerramento reconhecido. Até logo!");
                                conversation_history.clear();
                                if self.config.wake_cooldown_ms > 0 {
                                    wake_cooldown_until = Some(
                                        Instant::now()
                                            + Duration::from_millis(self.config.wake_cooldown_ms),
                                    );
                                }
                                state = LiveConversationState::AwaitingWakeWord;
                                state_changed_instant = Instant::now();
                            } else {
                                println!(
                                    "[Yuki] Aguardando continuação por {}s... (fale diretamente)",
                                    self.config.follow_up_secs
                                );
                                state = LiveConversationState::FollowUpWindow;
                                state_changed_instant = Instant::now();
                            }
                        }
                        Err(e) => {
                            let is_empty_stt = matches!(
                                &e,
                                VoiceError::TranscriptionFailed(msg)
                                    if msg.contains("Nenhuma fala reconhecida")
                            );
                            if is_empty_stt {
                                empty_stt_discards += 1;
                                println!(
                                    "[Yuki] Nenhuma fala inteligível reconhecida no segmento capturado. Retornando para escuta de 'Yuki'..."
                                );
                            } else {
                                eprintln!("[Yuki] Erro ao processar turno: {}", e);
                            }

                            if shutdown_flag.load(Ordering::SeqCst)
                                && self.config.acoustic_cooldown_ms > 0
                            {
                                tokio::time::sleep(Duration::from_millis(
                                    self.config.acoustic_cooldown_ms,
                                ))
                                .await;
                            }

                            while rx_samples.try_recv().is_ok() {}
                            incoming_pcm_queue.clear();
                            pre_roll_buffer.clear();
                            vad_window.clear();
                            consecutive_speech_blocks = 0;
                            trailing_silent_blocks = 0;
                            self.spotter.reset().await;

                            if self.config.wake_cooldown_ms > 0 {
                                wake_cooldown_until = Some(
                                    Instant::now()
                                        + Duration::from_millis(self.config.wake_cooldown_ms),
                                );
                            }
                            state = LiveConversationState::AwaitingWakeWord;
                            state_changed_instant = Instant::now();
                        }
                    }
                }

                LiveConversationState::Terminated => {
                    break;
                }
            }

            // Se não há trabalho imediato pendente na fila nem transição síncrona, dorme 10ms para poupar CPU
            let has_pending_work = got_samples
                || incoming_pcm_queue.len() >= block_size
                || state == LiveConversationState::ProcessingTurn;
            if !has_pending_work {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }

        // Em caso de encerramento por Ctrl+C durante playback, interrompe o sink limpo
        let _ = self.coordinator.playback().interrupt();

        println!("\n[Yuki] Encerrando sessão de conversação ao vivo...");
        let stats = VoiceLatencyStats::compute_with_discards(turn_records, empty_stt_discards);
        stats.print_summary_report();

        Ok(stats)
    }

    /// Avalia se a fala do usuário corresponde a um comando de despedida ou saída,
    /// utilizando fronteiras de palavra para não confundir vocábulos como "compare" ou "parece" com "pare".
    pub fn is_exit_phrase(&self, text: &str) -> bool {
        let normalized: String = text
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { ' ' })
            .collect();
        let words: Vec<&str> = normalized.split_whitespace().collect();
        if words.is_empty() {
            return false;
        }

        for phrase in &self.config.exit_phrases {
            let p_norm: String = phrase
                .to_lowercase()
                .chars()
                .map(|c| if c.is_alphanumeric() { c } else { ' ' })
                .collect();
            let p_words: Vec<&str> = p_norm.split_whitespace().collect();
            if !p_words.is_empty()
                && words
                    .windows(p_words.len())
                    .any(|win| win == p_words.as_slice())
            {
                return true;
            }
        }
        false
    }
}
