use std::collections::VecDeque;
use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use yuki::contracts::voice::{
    AudioFormat, AudioInput, AudioOutput, ResamplerKind, SpeechToTextProvider, SttTimingMetrics,
    TextToSpeechProvider, TtsSynthesisKind, VoiceConfiguration, VoiceError, VoiceProviderMetadata,
    VoiceProviderType,
};
use yuki::core::yuki_core::YukiCore;
use yuki::models::{
    diagnose_gemini_provider_readiness, GeminiCredentialSource, GeminiProviderAdapter,
    HealthStatus, ModelError, ModelGatewayConfig, ModelMetadata, ModelProvider, ModelRequest,
    ModelResponse,
};
use yuki::security::credentials::{CompositeCredentialBroker, SecretMaterial, SecretRef};
use yuki::security::vault::{CredentialVault, VaultSecretStore};
use yuki::voice::audio::AudioCodec;
use yuki::voice::conversation::VoiceConversationCoordinator;
use yuki::voice::live::{VoiceLiveListener, VoiceLiveListenerConfig};
use yuki::voice::playback::{AudioPlaybackController, AudioPlaybackSink, PlaybackStatus};
use yuki::voice::router::GovernedVoiceRouter;
use yuki::voice::session::VoiceSessionManager;
use yuki::voice::stt::{
    trim_silence_edges, trim_silence_edges_adaptive, LocalWhisperConfig, LocalWhisperSttProvider,
};
use yuki::voice::tts::LocalPiperTtsProvider;
use yuki::voice::wakeword::{LiveWakeWordSpotter, WakeWordDetector};

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn make_silence_block() -> Vec<i16> {
    vec![0i16; 1280]
}

/// Gera ruído de sala realista equivalente ao Realtek físico (~14 RMS, pico <= 42).
fn make_room_floor_block(seed: usize) -> Vec<i16> {
    (0usize..1280)
        .map(|i| {
            let h = ((i.wrapping_mul(1103515245).wrapping_add(12345 + seed * 97)) >> 16) as i32;
            ((h % 41) - 20) as i16
        })
        .collect()
}

/// Gera fala modulada de baixa amplitude realista (~58 a ~120 RMS, bem abaixo do antigo limiar 300.0!).
fn make_quiet_modulated_speech_block(block_idx: usize, base_peak: f32) -> Vec<i16> {
    let sr = 16000.0f32;
    let f0 = 210.0f32 + ((block_idx % 3) as f32) * 35.0;
    let block_mod = 0.78 + ((block_idx % 4) as f32) * 0.14;
    (0..1280)
        .map(|i| {
            let t = i as f32 / sr;
            // Envelope silábico assimétrico dentro do bloco de 80ms para simular articulação humana real
            let env = 0.40 + 0.60 * ((std::f32::consts::PI * (i as f32) / 960.0).sin().abs());
            let carrier = (2.0 * std::f32::consts::PI * f0 * t).sin() * 0.7
                + (2.0 * std::f32::consts::PI * (f0 * 2.0) * t).sin() * 0.3;
            (carrier * base_peak * block_mod * env) as i16
        })
        .collect()
}

/// Gera ruído contínuo estacionário (ex.: ventilador constante com variância ínfima).
fn make_stationary_hum_block() -> Vec<i16> {
    let sr = 16000.0f32;
    (0..1280)
        .map(|i| ((2.0 * std::f32::consts::PI * 320.0 * (i as f32 / sr)).sin() * 480.0) as i16)
        .collect()
}

struct DeterministicSpotter {
    state: Mutex<(usize, usize)>,
    resets: AtomicUsize,
}

impl DeterministicSpotter {
    fn new() -> Self {
        Self {
            state: Mutex::new((0, 0)),
            resets: AtomicUsize::new(0),
        }
    }

    fn make_trigger_block() -> Vec<i16> {
        let mut b = make_quiet_modulated_speech_block(0, 320.0);
        b[0] = 28_888;
        b[1] = -28_888;
        b
    }
}

impl LiveWakeWordSpotter for DeterministicSpotter {
    fn process_chunk(&self, pcm_1280: &[i16]) -> Result<f32, VoiceError> {
        let mut st = self.state.lock().unwrap();
        st.0 += 1;
        if pcm_1280.len() >= 2 && pcm_1280[0] == 28_888 && pcm_1280[1] == -28_888 {
            st.1 = 16;
        }
        if st.0 < 16 {
            return Ok(0.0);
        }
        if st.1 > 0 {
            st.1 -= 1;
            return Ok(0.95);
        }
        Ok(0.001)
    }

    fn reset(&self) {
        self.resets.fetch_add(1, Ordering::SeqCst);
        let mut st = self.state.lock().unwrap();
        *st = (0, 0);
    }

    fn name(&self) -> &str {
        "DeterministicSpotter"
    }
}

struct SilentTestNeuralTts {
    meta: VoiceProviderMetadata,
}

impl SilentTestNeuralTts {
    fn new() -> Self {
        Self {
            meta: VoiceProviderMetadata::new(
                "SilentTestNeuralTts",
                VoiceProviderType::LocalEdge,
                vec!["pt-BR".to_string()],
                22050,
                false,
                "1.0",
            )
            .with_synthesis_kind(TtsSynthesisKind::NeuralSpeech),
        }
    }
}

impl TextToSpeechProvider for SilentTestNeuralTts {
    fn metadata(&self) -> &VoiceProviderMetadata {
        &self.meta
    }

    fn synthesize<'a>(
        &'a self,
        text: &'a str,
        _config: Option<&'a VoiceConfiguration>,
    ) -> Pin<Box<dyn Future<Output = Result<AudioOutput, VoiceError>> + Send + 'a>> {
        Box::pin(async move {
            if text.trim().is_empty() {
                return Err(VoiceError::SynthesisFailed("Texto vazio".to_string()));
            }
            let fmt = AudioFormat {
                sample_rate: 22050,
                channels: 1,
                bits_per_sample: 16,
            };
            let samples = vec![0i16; (22050 * 60) / 1000];
            let wav = AudioCodec::encode_wav(&samples, &fmt)?;
            Ok(AudioOutput::new(wav, fmt, 60, "pt_BR-dii-female-test"))
        })
    }

    fn health<'a>(&'a self) -> Pin<Box<dyn Future<Output = Result<(), VoiceError>> + Send + 'a>> {
        Box::pin(async { Ok(()) })
    }
}

/// Provedor STT espião que inspeciona exatamente o buffer WAV entregue pelo `VoiceLiveListener`.
struct CapturingAudioStt {
    captured_inputs: Arc<Mutex<Vec<(u64, usize)>>>,
    queued_texts: Mutex<VecDeque<String>>,
    meta: VoiceProviderMetadata,
}

impl CapturingAudioStt {
    fn new(texts: Vec<&str>, captured_inputs: Arc<Mutex<Vec<(u64, usize)>>>) -> Self {
        Self {
            captured_inputs,
            queued_texts: Mutex::new(texts.into_iter().map(String::from).collect()),
            meta: VoiceProviderMetadata::new(
                "CapturingAudioStt",
                VoiceProviderType::LocalEdge,
                vec!["pt-BR".to_string()],
                16000,
                false,
                "1.0",
            ),
        }
    }
}

impl SpeechToTextProvider for CapturingAudioStt {
    fn metadata(&self) -> &VoiceProviderMetadata {
        &self.meta
    }

    fn transcribe<'a>(
        &'a self,
        audio: &'a AudioInput,
    ) -> Pin<Box<dyn Future<Output = Result<String, VoiceError>> + Send + 'a>> {
        let (_, samples) = AudioCodec::decode_wav(&audio.payload).expect("WAV válido no STT");
        self.captured_inputs
            .lock()
            .unwrap()
            .push((audio.duration_ms, samples.len()));
        let out = self
            .queued_texts
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_default();
        Box::pin(async move { Ok(out) })
    }

    fn health<'a>(&'a self) -> Pin<Box<dyn Future<Output = Result<(), VoiceError>> + Send + 'a>> {
        Box::pin(async { Ok(()) })
    }

    fn last_timings(&self) -> Option<SttTimingMetrics> {
        None
    }
}

// ============================================================================
// PENDÊNCIA 1 — Fim do corte prematuro em 0,65s em fala baixa e pausa após vírgula ("Yuki, ...")
// ============================================================================
#[tokio::test]
async fn test_1_eos_no_premature_065s_cutoff_on_low_volume_and_prosodic_pause() {
    let captured_inputs = Arc::new(Mutex::new(Vec::new()));
    let stt = Arc::new(CapturingAudioStt::new(
        vec!["Yuki, quanto que é 2 mais 2?"],
        captured_inputs.clone(),
    ));
    let tts = Arc::new(SilentTestNeuralTts::new());
    let router = Arc::new(
        GovernedVoiceRouter::new(stt, None, tts, None, VoiceConfiguration::default())
            .with_require_intelligible_tts(true),
    );
    let coordinator = Arc::new(VoiceConversationCoordinator::new(
        Arc::new(YukiCore::new()),
        router,
        Arc::new(VoiceSessionManager::new()),
        Arc::new(AudioPlaybackController::for_test()),
        Arc::new(tokio::sync::Mutex::new(WakeWordDetector::default())),
        None,
    ));

    // Usa silence_hangover_ms = 650 (o exato valor onde antes ocorria o corte em 0,65s!)
    let cfg = VoiceLiveListenerConfig {
        silence_hangover_ms: 650,
        follow_up_secs: 1,
        acoustic_cooldown_ms: 20,
        wake_cooldown_ms: 30,
        ..Default::default()
    };

    let spotter = Arc::new(DeterministicSpotter::new());
    let listener = VoiceLiveListener::with_spotter(cfg, coordinator, spotter);
    let (tx, rx) = std::sync::mpsc::channel::<Vec<i16>>();
    let shutdown = Arc::new(AtomicBool::new(true));
    let shutdown_prod = shutdown.clone();

    let producer = std::thread::spawn(move || {
        // 1. Warmup com ruído real de sala (~14 RMS)
        for k in 0..16 {
            let _ = tx.send(make_room_floor_block(k));
            std::thread::sleep(Duration::from_millis(5));
        }

        // 2. Disparo da palavra "Yuki"
        let _ = tx.send(DeterministicSpotter::make_trigger_block());

        // 3. Pausa prosódica natural de ~720ms após a vírgula ("Yuki, ...") — antes cortava em 650ms!
        for k in 0..9 {
            let _ = tx.send(make_room_floor_block(100 + k));
            std::thread::sleep(Duration::from_millis(80));
        }

        // 4. Pergunta "quanto que é 2 mais 2?" falada em volume baixo Realtek (~85 RMS -> caindo a ~62 RMS no "2?" final,
        //    totalizando 22 blocos * 80ms = 1760ms de fala contínua abaixo do antigo limiar 300.0!)
        for k in 0..22 {
            let peak = if k >= 16 { 155.0 } else { 215.0 };
            let _ = tx.send(make_quiet_modulated_speech_block(k, peak));
            std::thread::sleep(Duration::from_millis(80));
        }

        // 5. Silêncio final de ~800ms para acionar EOS legítimo após a palavra "2?"
        for k in 0..10 {
            let _ = tx.send(make_room_floor_block(200 + k));
            std::thread::sleep(Duration::from_millis(80));
        }

        std::thread::sleep(Duration::from_millis(200));
        shutdown_prod.store(false, Ordering::SeqCst);
    });

    let stats = listener
        .run_event_loop(rx, shutdown)
        .await
        .expect("Loop conversacional deve concluir de forma limpa");
    producer.join().unwrap();

    assert_eq!(stats.total_turns, 1, "Deve capturar um único turno íntegro");

    let inputs = captured_inputs.lock().unwrap();
    assert_eq!(inputs.len(), 1);
    let (captured_ms, captured_samples) = inputs[0];
    println!(
        "[Pendência 1] Frase baixa + pausa de vírgula capturada integralmente: {}ms ({} amostras)",
        captured_ms, captured_samples
    );
    assert!(
        captured_ms >= 3200,
        "A frase jamais pode ser cortada prematuramente em 650ms! Esperado >= 3200ms (pré-roll + pausa 720ms + fala baixa 1760ms + hangover), obtido: {}ms",
        captured_ms
    );
}

// ============================================================================
// PENDÊNCIA 2 — Robustez do STT: preservação de fala baixa, números finais e comandos curtos sem truques artificiais
// ============================================================================
#[tokio::test]
async fn test_2_adaptive_trim_and_live_stt_preserve_quiet_speech_numbers_and_short_commands() {
    // Parte A (Determinística em qualquer runner CI):
    // Prova matemática de que o antigo `trim_silence_edges(..., 160.0, 250)` amputava o final suave da frase,
    // enquanto `trim_silence_edges_adaptive` preserva 100% das sílabas finais suaves!
    let mut synthetic_phrase: Vec<i16> = Vec::new();
    // 640ms silêncio de sala inicial (~14 RMS)
    for k in 0..8 {
        synthetic_phrase.extend_from_slice(&make_room_floor_block(k));
    }
    // "Yuki" inicial mais forte (~220 RMS, 320ms)
    for k in 0..4 {
        synthetic_phrase.extend_from_slice(&make_quiet_modulated_speech_block(k, 480.0));
    }
    // "... quanto que é 2 mais 2?" em entonação descendente baixa (~68 RMS, pico ~195, 1600ms = 25600 amostras)
    for k in 0..20 {
        synthetic_phrase.extend_from_slice(&make_quiet_modulated_speech_block(k, 195.0));
    }
    // 800ms silêncio de sala final (~14 RMS)
    for k in 0..10 {
        synthetic_phrase.extend_from_slice(&make_room_floor_block(50 + k));
    }

    let old_trimmed = trim_silence_edges(&synthetic_phrase, 16000, 160.0, 250);
    let adaptive_trimmed = trim_silence_edges_adaptive(&synthetic_phrase, 16000);

    println!(
        "[Pendência 2A] Amostras originais: {} | Trim antigo (160 RMS): {} | Trim adaptativo: {}",
        synthetic_phrase.len(),
        old_trimmed.len(),
        adaptive_trimmed.len()
    );
    assert!(
        old_trimmed.len() < 20_000,
        "Confirma causa raiz: o trim antigo em 160 RMS amputava o resto suave da pergunta ({} amostras)",
        old_trimmed.len()
    );
    assert!(
        adaptive_trimmed.len() >= 24 * 1280,
        "O trim adaptativo DEVE preservar tanto 'Yuki' quanto os 1600ms finais de fala baixa ({} amostras)",
        adaptive_trimmed.len()
    );
    assert!(
        adaptive_trimmed.len() < synthetic_phrase.len() - 6000,
        "O trim adaptativo ainda deve remover as bordas mortas de silêncio ({} vs {})",
        adaptive_trimmed.len(),
        synthetic_phrase.len()
    );

    // Parte B (Medição real Whisper + Piper quando disponíveis localmente no host):
    let piper = LocalPiperTtsProvider::default();
    let whisper = LocalWhisperSttProvider::new(
        LocalWhisperConfig::default()
            .with_language("pt")
            .with_live_optimizations(),
    );
    if !piper.is_environment_ready() || !whisper.is_environment_ready() {
        eprintln!(
            "[Pendência 2B] Modelos locais Piper/Whisper ausentes neste runner; pulando medição acústica real."
        );
        return;
    }

    let _ = piper.warmup().await;

    // 1. Silêncio físico Realtek deve continuar retornando vazio sem alucinações
    let realtek_silence = Path::new("data/voice_diag/canonical_16k_diag_1a142036d81_1.wav");
    if realtek_silence.exists() {
        let bytes = std::fs::read(realtek_silence).unwrap();
        let input = AudioInput::new(bytes, AudioFormat::default(), 5010, "realtek_silence");
        let res = whisper.transcribe_with_metadata(&input).await.unwrap();
        assert!(
            res.text.is_empty(),
            "Silêncio físico Realtek jamais pode alucinar texto: obtido '{}'",
            res.text
        );
    }

    // Helper: sintetiza fala PT-BR, converte para 16kHz com filtro FIR antialiasing e atenua para volume baixo Realtek (~85 RMS, final ~62 RMS)
    let build_quiet_realtek_utterance = |wav_22k: &[u8]| -> AudioInput {
        let (fmt, pcm_22k) = AudioCodec::decode_wav(wav_22k).unwrap();
        let pcm_16k = AudioCodec::resample(
            &pcm_22k,
            fmt.sample_rate,
            16000,
            ResamplerKind::FirAntialiasing,
        );
        let mut mixed: Vec<i16> = Vec::with_capacity(8000 + pcm_16k.len() + 12800);
        for k in 0..6 {
            mixed.extend_from_slice(&make_room_floor_block(k));
        }
        let n = pcm_16k.len();
        for (i, &s) in pcm_16k.iter().enumerate() {
            // Atenua Piper (~8000 RMS) para ~90 RMS e reduz o terço final para ~65 RMS
            let gain = if i > (n * 2) / 3 { 0.011f32 } else { 0.016f32 };
            let noise = (((i.wrapping_mul(1103515245).wrapping_add(777)) >> 16) % 29) as i32 - 14;
            let v = noise + (s as f32 * gain).round() as i32;
            mixed.push(v.clamp(-32768, 32767) as i16);
        }
        for k in 0..10 {
            mixed.extend_from_slice(&make_room_floor_block(30 + k));
        }
        let dur_ms = (mixed.len() as u64 * 1000) / 16000;
        let wav = AudioCodec::encode_wav(&mixed, &AudioFormat::default()).unwrap();
        AudioInput::new(wav, AudioFormat::default(), dur_ms, "quiet_realtek_sim")
    };

    // Caso 1: Frase com números repetidos no final em fala baixa ("dois mais dois")
    let synth_math = piper
        .synthesize("Yuki, quanto que é dois mais dois?", None)
        .await
        .unwrap();
    let quiet_math_in = build_quiet_realtek_utterance(&synth_math.payload);
    let math_res = whisper
        .transcribe_with_metadata(&quiet_math_in)
        .await
        .unwrap();
    let math_norm = math_res.text.to_lowercase();
    let count_two = math_norm.matches("dois").count() + math_norm.matches('2').count();
    println!(
        "[Pendência 2B] Fala baixa numérica transcrita em {:.2}s: \"{}\"",
        math_res.elapsed.as_secs_f64(),
        math_res.text
    );
    assert!(
        count_two >= 2 && !math_norm.contains("zero") && !math_norm.contains('0'),
        "Ambos os números 'dois' devem ser preservados sem cair para 'mais 0': obtido '{}'",
        math_res.text
    );

    // Caso 2: Comando curto numérico/operacional em fala baixa ("Somar dez.")
    let synth_cmd = piper.synthesize("Somar dez.", None).await.unwrap();
    let quiet_cmd_in = build_quiet_realtek_utterance(&synth_cmd.payload);
    let cmd_res = whisper
        .transcribe_with_metadata(&quiet_cmd_in)
        .await
        .unwrap();
    let cmd_norm = cmd_res.text.to_lowercase();
    println!(
        "[Pendência 2B] Comando curto em fala baixa transcrito em {:.2}s: \"{}\"",
        cmd_res.elapsed.as_secs_f64(),
        cmd_res.text
    );
    assert!(
        cmd_norm.contains("dez") || cmd_norm.contains("10"),
        "Comando curto numérico em fala baixa deve preservar o número: obtido '{}'",
        cmd_res.text
    );
}

// ============================================================================
// PENDÊNCIA 3 — Conexão real do GeminiProviderAdapter via CredentialBroker + higiene absoluta de segredos
// ============================================================================
#[tokio::test]
async fn test_3_gemini_credential_broker_real_wire_protocol_and_readiness_without_secret_leak() {
    // 1. Diagnóstico seguro de prontidão e distinção explícita Antigravity vs API do app
    {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let vars = [
            "YUKI_MODEL_PROVIDER",
            "YUKI_GEMINI_API_KEY",
            "GEMINI_API_KEY",
            "YUKI_GEMINI_SECRET_REF",
            "YUKI_VAULT_PATH",
            "YUKI_VAULT_PASSPHRASE",
        ];
        let saved: Vec<(&str, Option<String>)> =
            vars.iter().map(|&k| (k, std::env::var(k).ok())).collect();
        struct Restore<'a>(Vec<(&'a str, Option<String>)>);
        impl Drop for Restore<'_> {
            fn drop(&mut self) {
                for (k, v) in &self.0 {
                    match v {
                        Some(val) => std::env::set_var(k, val),
                        None => std::env::remove_var(k),
                    }
                }
            }
        }
        let _r = Restore(saved);

        for k in &vars {
            std::env::remove_var(k);
        }
        let missing_vault = std::env::temp_dir().join("yuki_nonexistent_vault_test_v1.json");
        let _ = std::fs::remove_file(&missing_vault);
        std::env::set_var("YUKI_VAULT_PATH", &missing_vault);

        let rep_missing = diagnose_gemini_provider_readiness();
        assert!(!rep_missing.ready_for_live_conversation);
        assert_eq!(
            rep_missing.credential_source,
            GeminiCredentialSource::Missing
        );
        assert!(
            rep_missing.operator_guidance.contains("Antigravity"),
            "Orientação deve esclarecer que assinatura do Antigravity não equivale a API configurada no app"
        );

        let sentinel_secret = "yuki-test-sentinel-broker-readiness-445566";
        std::env::set_var("YUKI_MODEL_PROVIDER", "gemini");
        std::env::set_var("YUKI_GEMINI_API_KEY", sentinel_secret);
        let rep_ready = diagnose_gemini_provider_readiness();
        assert!(rep_ready.ready_for_live_conversation);
        assert_eq!(
            rep_ready.credential_source,
            GeminiCredentialSource::EnvYukiGeminiApiKey
        );
        assert!(!rep_ready.operator_guidance.contains(sentinel_secret));
        assert!(!format!("{:?}", rep_ready).contains(sentinel_secret));
    }

    // 2. Servidor HTTP local simulando endpoint REST oficial do Google Gemini (`:generateContent`)
    //    para validar `GeminiProviderAdapter` + `CredentialVault` + `CompositeCredentialBroker` + histórico multi-turno!
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Bind em loopback local");
    let addr = listener.local_addr().unwrap();
    let captured_wire: Arc<Mutex<Vec<(String, String, String)>>> = Arc::new(Mutex::new(Vec::new()));
    let captured_wire_srv = captured_wire.clone();

    let server_handle = tokio::spawn(async move {
        for turn in 0..2 {
            let (mut socket, _) = listener.accept().await.expect("Aceitar conexão HTTP");
            let mut buf = Vec::with_capacity(8192);
            let mut tmp = [0u8; 2048];
            let header_end_pos;
            loop {
                let n = socket.read(&mut tmp).await.unwrap();
                if n == 0 {
                    panic!("Conexão encerrada antes do cabeçalho HTTP");
                }
                buf.extend_from_slice(&tmp[..n]);
                if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                    header_end_pos = pos + 4;
                    break;
                }
            }
            let headers_str = String::from_utf8_lossy(&buf[..header_end_pos]).to_string();
            let content_len = headers_str
                .lines()
                .find_map(|line| {
                    let lower = line.to_ascii_lowercase();
                    lower
                        .strip_prefix("content-length:")
                        .and_then(|v| v.trim().parse::<usize>().ok())
                })
                .unwrap_or(0);
            while buf.len() < header_end_pos + content_len {
                let n = socket.read(&mut tmp).await.unwrap();
                if n == 0 {
                    break;
                }
                buf.extend_from_slice(&tmp[..n]);
            }
            let request_line = headers_str.lines().next().unwrap_or("").to_string();
            let api_key_hdr = headers_str
                .lines()
                .find_map(|l| {
                    let lower = l.to_ascii_lowercase();
                    if lower.starts_with("x-goog-api-key:") {
                        Some(l.split_once(':').unwrap().1.trim().to_string())
                    } else {
                        None
                    }
                })
                .unwrap_or_default();
            let body_str =
                String::from_utf8_lossy(&buf[header_end_pos..header_end_pos + content_len])
                    .to_string();
            captured_wire_srv
                .lock()
                .unwrap()
                .push((request_line, api_key_hdr, body_str.clone()));

            let reply_text = if turn == 0 {
                "Dois mais dois é quatro."
            } else if body_str.contains("Dois mais dois é quatro.") {
                "Somando dez ao resultado anterior quatro, obtemos quatorze."
            } else {
                "HISTÓRICO_AUSENTE_NA_REQUISIÇÃO_GEMINI"
            };

            let resp_json = serde_json::json!({
                "candidates": [{
                    "content": {
                        "role": "model",
                        "parts": [{ "text": reply_text }]
                    },
                    "finishReason": "STOP"
                }],
                "usageMetadata": {
                    "promptTokenCount": 32,
                    "candidatesTokenCount": 12,
                    "totalTokenCount": 44
                }
            })
            .to_string();

            let http_resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                resp_json.len(),
                resp_json
            );
            socket.write_all(http_resp.as_bytes()).await.unwrap();
        }
    });

    // Cria cofre cifrado temporário e instancia CompositeCredentialBroker real
    let tmp_dir = std::env::temp_dir().join(format!(
        "yuki_v1_broker_wire_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&tmp_dir).unwrap();
    let vault_path = tmp_dir.join("vault.json");
    let vault_pass = "yuki-test-sentinel-passphrase-alpha-99";
    let wire_secret = "yuki-test-sentinel-wire-credential-314159";

    let vault = Arc::new(
        CredentialVault::create_new(&vault_path, SecretMaterial::new(vault_pass.to_string()))
            .unwrap(),
    );
    vault
        .store("gemini_api_key", wire_secret, "gemini", None)
        .unwrap();

    let mut broker = CompositeCredentialBroker::new();
    broker.set_vault(Arc::new(VaultSecretStore::new(vault)));

    let secret_ref = SecretRef::new("secret://vault/gemini_api_key", "gemini-primary").unwrap();
    let gw_cfg = ModelGatewayConfig::default();
    let adapter = GeminiProviderAdapter::new(secret_ref, Arc::new(broker), gw_cfg)
        .unwrap()
        .with_base_url(format!("http://{}", addr));

    assert_eq!(adapter.health(), HealthStatus::Healthy);
    assert!(!format!("{:?}", adapter.metadata()).contains(wire_secret));

    let core = Arc::new(YukiCore::new().with_model_provider(Arc::new(adapter)));
    let captured_stt = Arc::new(Mutex::new(Vec::new()));
    let stt = Arc::new(CapturingAudioStt::new(
        vec![
            "Yuki, quanto é dois mais dois?",
            "Some dez a esse resultado.",
        ],
        captured_stt,
    ));
    let router = Arc::new(
        GovernedVoiceRouter::new(
            stt,
            None,
            Arc::new(SilentTestNeuralTts::new()),
            None,
            VoiceConfiguration::default(),
        )
        .with_require_intelligible_tts(true),
    );
    let coordinator = VoiceConversationCoordinator::new(
        core,
        router,
        Arc::new(VoiceSessionManager::new()),
        Arc::new(AudioPlaybackController::for_test()),
        Arc::new(tokio::sync::Mutex::new(WakeWordDetector::default())),
        None,
    );

    let dummy_wav = AudioCodec::encode_wav(
        &make_quiet_modulated_speech_block(0, 300.0),
        &AudioFormat::default(),
    )
    .unwrap();
    let audio1 = AudioInput::new(dummy_wav.clone(), AudioFormat::default(), 80, "wire_t1");
    let (u1, r1, _, _) = coordinator
        .process_turn_instrumented("wire-dev", audio1, vec![], false, false)
        .await
        .expect("Turno 1 via GeminiProviderAdapter + CredentialVault deve ter sucesso");
    assert_eq!(r1.content, "Dois mais dois é quatro.");

    let history = vec![format!("Usuário: {}", u1), format!("Yuki: {}", r1.content)];
    let audio2 = AudioInput::new(dummy_wav, AudioFormat::default(), 80, "wire_t2");
    let (_, r2, _, _) = coordinator
        .process_turn_instrumented("wire-dev", audio2, history, false, false)
        .await
        .expect("Turno 2 com histórico via GeminiProviderAdapter deve ter sucesso");
    assert_eq!(
        r2.content,
        "Somando dez ao resultado anterior quatro, obtemos quatorze."
    );

    server_handle.await.unwrap();
    let _ = std::fs::remove_dir_all(&tmp_dir);

    let wire_reqs = captured_wire.lock().unwrap();
    assert_eq!(wire_reqs.len(), 2);
    for (req_line, key_hdr, _body) in wire_reqs.iter() {
        assert!(
            !req_line.contains(wire_secret) && !req_line.contains("key="),
            "A chave jamais pode trafegar na URL/query string: {}",
            req_line
        );
        assert_eq!(
            key_hdr, wire_secret,
            "Header x-goog-api-key deve ser injetado via SecretLease do CredentialBroker"
        );
    }
}

// ============================================================================
// PENDÊNCIA 4 — Janela de follow-up de 7s com fala baixa, referência ao turno anterior e fronteira de palavra em saída
// ============================================================================
#[tokio::test]
async fn test_4_seven_second_followup_window_quiet_speech_context_and_word_boundary_exit() {
    struct FollowUpContextProvider {
        prompts: Arc<Mutex<Vec<String>>>,
    }
    impl ModelProvider for FollowUpContextProvider {
        fn generate<'a>(
            &'a self,
            request: &'a ModelRequest,
        ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, ModelError>> + Send + 'a>> {
            self.prompts.lock().unwrap().push(request.prompt.clone());
            let p = request.prompt.to_lowercase();
            let reply = if p.contains("dobro") {
                if p.contains("quatro é maior que dois") {
                    "Quatro é o dobro de dois."
                } else {
                    "SEM_HISTORICO"
                }
            } else {
                "Quatro é maior que dois."
            };
            let id = request.request_id.clone();
            Box::pin(async move {
                Ok(ModelResponse::text(
                    id,
                    "FollowUpContextProvider",
                    "test-v1",
                    reply,
                ))
            })
        }
        fn metadata(&self) -> ModelMetadata {
            ModelMetadata {
                provider_name: "FollowUpContextProvider".to_string(),
                model_name: "test-v1".to_string(),
                version: "1.0".to_string(),
            }
        }
        fn health(&self) -> HealthStatus {
            HealthStatus::Healthy
        }
    }

    let prompts = Arc::new(Mutex::new(Vec::new()));
    let core = Arc::new(
        YukiCore::new().with_model_provider(Arc::new(FollowUpContextProvider {
            prompts: prompts.clone(),
        })),
    );
    // Nota crucial: "compare" contém a substring "pare"! Antes isso encerrava a conversa por engano!
    let stt = Arc::new(CapturingAudioStt::new(
        vec![
            "Yuki, compare dois e quatro.",
            "Qual deles é o dobro?",
            "Obrigado, até logo.",
        ],
        Arc::new(Mutex::new(Vec::new())),
    ));
    let router = Arc::new(
        GovernedVoiceRouter::new(
            stt,
            None,
            Arc::new(SilentTestNeuralTts::new()),
            None,
            VoiceConfiguration::default(),
        )
        .with_require_intelligible_tts(true),
    );
    let coordinator = Arc::new(VoiceConversationCoordinator::new(
        core,
        router,
        Arc::new(VoiceSessionManager::new()),
        Arc::new(AudioPlaybackController::for_test()),
        Arc::new(tokio::sync::Mutex::new(WakeWordDetector::default())),
        None,
    ));

    let cfg = VoiceLiveListenerConfig {
        follow_up_secs: 7,
        silence_hangover_ms: 420,
        acoustic_cooldown_ms: 20,
        wake_cooldown_ms: 30,
        ..Default::default()
    };
    let listener =
        VoiceLiveListener::with_spotter(cfg, coordinator, Arc::new(DeterministicSpotter::new()));

    // Validação direta da fronteira de palavra em comandos de saída
    assert!(
        !listener.is_exit_phrase("Yuki, compare dois e quatro."),
        "'compare' jamais pode acionar saída falsa por substring 'pare'"
    );
    assert!(
        !listener.is_exit_phrase("Parece que vai chover hoje?"),
        "'Parece' jamais pode acionar saída falsa por substring 'pare'"
    );
    assert!(listener.is_exit_phrase("Yuki, pare."));
    assert!(listener.is_exit_phrase("Obrigado, até logo."));

    let (tx, rx) = std::sync::mpsc::channel::<Vec<i16>>();
    let shutdown = Arc::new(AtomicBool::new(true));
    let shutdown_prod = shutdown.clone();

    let producer = std::thread::spawn(move || {
        // Turno 1: Warmup + Wake Word + fala
        for k in 0..16 {
            let _ = tx.send(make_room_floor_block(k));
            std::thread::sleep(Duration::from_millis(5));
        }
        let _ = tx.send(DeterministicSpotter::make_trigger_block());
        for k in 0..5 {
            let _ = tx.send(make_quiet_modulated_speech_block(k, 230.0));
            std::thread::sleep(Duration::from_millis(25));
        }
        for k in 0..11 {
            let _ = tx.send(make_room_floor_block(k));
            std::thread::sleep(Duration::from_millis(45));
        }

        // Durante a janela de follow-up de 7s (~1,1s após resposta), envia primeiro ruído de sala + ruído estacionário
        // (que NÃO podem disparar) e em seguida fala baixa Realtek (~105 RMS, < 300.0!) SEM repetir "Yuki"
        for k in 0..8 {
            let _ = tx.send(make_room_floor_block(90 + k));
            std::thread::sleep(Duration::from_millis(60));
        }
        for _ in 0..6 {
            let _ = tx.send(make_stationary_hum_block());
            std::thread::sleep(Duration::from_millis(60));
        }

        // Fala baixa legítima em follow-up (~105 RMS): "Qual deles é o dobro?"
        for k in 0..6 {
            let _ = tx.send(make_quiet_modulated_speech_block(k, 240.0));
            std::thread::sleep(Duration::from_millis(35));
        }
        for k in 0..11 {
            let _ = tx.send(make_room_floor_block(k));
            std::thread::sleep(Duration::from_millis(45));
        }

        // Turno 3 em follow-up encerrando com "Obrigado, até logo." (após playback de 60ms + cooldown de 20ms concluir)
        std::thread::sleep(Duration::from_millis(280));
        for k in 0..6 {
            let _ = tx.send(make_quiet_modulated_speech_block(k, 240.0));
            std::thread::sleep(Duration::from_millis(35));
        }
        for k in 0..11 {
            let _ = tx.send(make_room_floor_block(k));
            std::thread::sleep(Duration::from_millis(45));
        }

        std::thread::sleep(Duration::from_millis(250));
        shutdown_prod.store(false, Ordering::SeqCst);
    });

    let stats = listener.run_event_loop(rx, shutdown).await.unwrap();
    producer.join().unwrap();

    assert_eq!(stats.total_turns, 3);
    assert_eq!(stats.follow_up_turns, 2);
    assert_eq!(stats.records[1].yuki_text, "Quatro é o dobro de dois.");
}

// ============================================================================
// PENDÊNCIAS 5 & 6 — Dreno físico do buffer de hardware no Playback + imunidade contra realimentação acústica e Ctrl+C
// ============================================================================
#[derive(Debug)]
struct DelayedHardwareDrainSink {
    render_until: Mutex<Option<Instant>>,
    interrupted: AtomicBool,
    extra_drain_ms: u64,
}

impl DelayedHardwareDrainSink {
    fn new(extra_drain_ms: u64) -> Self {
        Self {
            render_until: Mutex::new(None),
            interrupted: AtomicBool::new(false),
            extra_drain_ms,
        }
    }
}

impl AudioPlaybackSink for DelayedHardwareDrainSink {
    fn play(&self, output: &AudioOutput) -> Result<(), VoiceError> {
        self.interrupted.store(false, Ordering::SeqCst);
        let total_hw_ms = output.duration_ms + self.extra_drain_ms;
        *self.render_until.lock().unwrap() =
            Some(Instant::now() + Duration::from_millis(total_hw_ms));
        Ok(())
    }

    fn interrupt(&self) -> Result<(), VoiceError> {
        self.interrupted.store(true, Ordering::SeqCst);
        *self.render_until.lock().unwrap() = None;
        Ok(())
    }

    fn is_device_available(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        "DelayedHardwareDrainSink"
    }

    fn is_actively_rendering(&self) -> bool {
        if self.interrupted.load(Ordering::SeqCst) {
            return false;
        }
        self.render_until
            .lock()
            .unwrap()
            .map(|until| Instant::now() < until)
            .unwrap_or(false)
    }
}

#[tokio::test]
async fn test_5_playback_hardware_render_drain_guard_and_acoustic_feedback_immunity() {
    // Simula cauda física de 450ms no buffer circular WASAPI além dos 60ms nominais do WAV!
    let hw_sink = Arc::new(DelayedHardwareDrainSink::new(450));
    let playback = Arc::new(AudioPlaybackController::with_sink(hw_sink.clone()));

    let stt = Arc::new(CapturingAudioStt::new(
        vec!["Yuki, diga olá.", "ECO_FANTASMA_PROIBIDO"],
        Arc::new(Mutex::new(Vec::new())),
    ));
    let router = Arc::new(
        GovernedVoiceRouter::new(
            stt,
            None,
            Arc::new(SilentTestNeuralTts::new()),
            None,
            VoiceConfiguration::default(),
        )
        .with_require_intelligible_tts(true),
    );
    let coordinator = Arc::new(VoiceConversationCoordinator::new(
        Arc::new(YukiCore::new()),
        router,
        Arc::new(VoiceSessionManager::new()),
        playback.clone(),
        Arc::new(tokio::sync::Mutex::new(WakeWordDetector::default())),
        None,
    ));

    let cfg = VoiceLiveListenerConfig {
        follow_up_secs: 1,
        silence_hangover_ms: 410,
        acoustic_cooldown_ms: 60,
        wake_cooldown_ms: 60,
        ..Default::default()
    };
    let listener =
        VoiceLiveListener::with_spotter(cfg, coordinator, Arc::new(DeterministicSpotter::new()));
    let (tx, rx) = std::sync::mpsc::channel::<Vec<i16>>();
    let shutdown = Arc::new(AtomicBool::new(true));
    let shutdown_prod = shutdown.clone();
    let pb_observer = playback.clone();

    let producer = std::thread::spawn(move || {
        struct ShutdownOnDrop(Arc<AtomicBool>);
        impl Drop for ShutdownOnDrop {
            fn drop(&mut self) {
                self.0.store(false, Ordering::SeqCst);
            }
        }
        let _shutdown_guard = ShutdownOnDrop(shutdown_prod.clone());

        for _ in 0..16 {
            let _ = tx.send(make_silence_block());
            std::thread::sleep(Duration::from_millis(4));
        }
        let _ = tx.send(DeterministicSpotter::make_trigger_block());
        for k in 0..4 {
            let _ = tx.send(make_quiet_modulated_speech_block(k, 260.0));
            std::thread::sleep(Duration::from_millis(20));
        }
        for _ in 0..8 {
            let _ = tx.send(make_silence_block());
            std::thread::sleep(Duration::from_millis(20));
        }

        // Aguarda o playback entrar em estado Playing (durante a cauda de dreno de hardware)
        let wait_start = Instant::now();
        while pb_observer.status() != PlaybackStatus::Playing
            && wait_start.elapsed() < Duration::from_millis(800)
        {
            std::thread::sleep(Duration::from_millis(10));
        }

        // Aos +80ms após início do playback (quando os 60ms nominais do WAV já transcorreram,
        // mas a cauda física de 450ms WASAPI ainda está drenando), verifica Playing:
        std::thread::sleep(Duration::from_millis(80));
        assert_eq!(
            pb_observer.status(),
            PlaybackStatus::Playing,
            "AudioPlaybackController deve permanecer Playing enquanto o buffer físico WASAPI ainda drena!"
        );

        // Injeta rajada de eco acústico contendo gatilhos de Wake Word e fala alta dos alto-falantes:
        // O controlador DEVE continuar reportando Playing e descartar 100% do eco!
        for k in 0..6 {
            let _ = tx.send(DeterministicSpotter::make_trigger_block());
            let _ = tx.send(make_quiet_modulated_speech_block(k, 1200.0));
            std::thread::sleep(Duration::from_millis(10));
        }

        // Aguarda a conclusão física do dreno de hardware
        let pb_wait_end = Instant::now();
        while pb_observer.status() == PlaybackStatus::Playing
            && pb_wait_end.elapsed() < Duration::from_millis(1000)
        {
            std::thread::sleep(Duration::from_millis(20));
        }

        // Envia silêncio durante a janela de follow-up (1s) para confirmar ausência de ativação espúria
        for _ in 0..22 {
            let _ = tx.send(make_silence_block());
            std::thread::sleep(Duration::from_millis(50));
        }
    });

    let stats = listener.run_event_loop(rx, shutdown).await.unwrap();
    producer.join().unwrap();

    assert_eq!(
        stats.total_turns, 1,
        "O eco acústico durante a cauda física do buffer WASAPI jamais pode auto-ativar um segundo turno!"
    );
    assert_eq!(stats.follow_up_turns, 0);
}
