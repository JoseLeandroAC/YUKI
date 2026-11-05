use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use yuki::contracts::voice::{
    AudioFormat, AudioInput, AudioOutput, SpeechToTextProvider, SttTimingMetrics,
    TextToSpeechProvider, TtsSynthesisKind, VoiceConfiguration, VoiceError, VoiceProviderMetadata,
    VoiceProviderType,
};
use yuki::core::yuki_core::YukiCore;
use yuki::models::{
    format_active_model_provider_banner, resolve_conversational_model_provider_from_env,
    HealthStatus, ModelError, ModelMetadata, ModelProvider, ModelRequest, ModelResponse,
};
use yuki::security::credentials::SecretMaterial;
use yuki::security::vault::CredentialVault;
use yuki::voice::audio::AudioCodec;
use yuki::voice::conversation::VoiceConversationCoordinator;
use yuki::voice::live::{VoiceLiveListener, VoiceLiveListenerConfig};
use yuki::voice::playback::{
    classify_sample_format, convert_samples_to_wasapi, resample_pcm_linear,
    AudioPlaybackController, ParsedWaveFormat, PlaybackEnvironment, WasapiSampleClassification,
};
use yuki::voice::router::GovernedVoiceRouter;
use yuki::voice::session::VoiceSessionManager;
use yuki::voice::stt::{
    compute_dynamic_audio_ctx, trim_silence_edges, LocalWhisperConfig, LocalWhisperSttProvider,
};
use yuki::voice::tts::{
    resolve_conversational_local_tts, LocalAcousticTtsProvider, LocalPiperTtsProvider,
};
use yuki::voice::wakeword::{LiveWakeWordSpotter, OpenWakeWordEngine, WakeWordDetector};

static ENV_TEST_LOCK: Mutex<()> = Mutex::new(());

/// Gera um bloco canônico de 80ms (1280 amostras @ 16kHz) de fala modulada sintética
/// compatível com o `VoiceActivityDetector` (ZCR ~0.044 em [0.02, 0.35], RMS > 300 e variância multi-bloco >= 15).
fn make_speech_block(block_idx: usize) -> Vec<i16> {
    let amp = 4200.0 + ((block_idx % 4) as f32) * 1800.0;
    let freq = 350.0f32;
    let sr = 16000.0f32;
    (0..1280)
        .map(|i| ((i as f32 * freq * 2.0 * std::f32::consts::PI / sr).sin() * amp) as i16)
        .collect()
}

/// Gera ruído estacionário constante (ex.: ventoinha/hum com RMS ~367 > 300, porém std_dev ~0 < 15).
fn make_stationary_noise_block() -> Vec<i16> {
    let amp = 520.0f32;
    let freq = 350.0f32;
    let sr = 16000.0f32;
    (0..1280)
        .map(|i| ((i as f32 * freq * 2.0 * std::f32::consts::PI / sr).sin() * amp) as i16)
        .collect()
}

fn make_silence_block() -> Vec<i16> {
    vec![0i16; 1280]
}

/// Simula fielmente a janela deslizante de 16 blocos do `OpenWakeWordEngine`:
/// - Exige 16 blocos de warmup após criação ou `reset()` antes de pontuar > 0.0;
/// - Quando um bloco marcado com sentinela de wake word entra na janela, mantém score elevado
///   pelos 15 blocos subsequentes (~1,2s), A MENOS que `reset()` seja invocado!
///
/// Isso reproduz exatamente a causa raiz das falsas reativações `0.96 -> 0.99 -> 0.98` se `reset()` faltar.
struct StickyWindowMockSpotter {
    state: Mutex<StickySpotterState>,
    reset_calls: AtomicUsize,
}

struct StickySpotterState {
    chunk_count: usize,
    sticky_remaining: usize,
}

impl StickyWindowMockSpotter {
    fn new() -> Self {
        Self {
            state: Mutex::new(StickySpotterState {
                chunk_count: 0,
                sticky_remaining: 0,
            }),
            reset_calls: AtomicUsize::new(0),
        }
    }

    fn reset_count(&self) -> usize {
        self.reset_calls.load(Ordering::SeqCst)
    }

    /// Cria um bloco de 80ms com assinatura determinística da palavra "Yuki" no primeiro sample.
    fn make_wake_trigger_block() -> Vec<i16> {
        let mut b = make_speech_block(1);
        b[0] = 29_999;
        b[1] = -29_999;
        b
    }
}

impl LiveWakeWordSpotter for StickyWindowMockSpotter {
    fn process_chunk(&self, pcm_1280: &[i16]) -> Result<f32, VoiceError> {
        let mut st = self.state.lock().unwrap();
        st.chunk_count += 1;

        let is_trigger = pcm_1280.len() >= 2 && pcm_1280[0] == 29_999 && pcm_1280[1] == -29_999;
        if is_trigger {
            st.sticky_remaining = 16;
        }

        if st.chunk_count < 16 {
            return Ok(0.0);
        }

        if st.sticky_remaining > 0 {
            let score = match st.sticky_remaining {
                16 => 0.96,
                15 => 0.99,
                _ => 0.98,
            };
            st.sticky_remaining -= 1;
            return Ok(score);
        }

        Ok(0.001)
    }

    fn reset(&self) {
        self.reset_calls.fetch_add(1, Ordering::SeqCst);
        let mut st = self.state.lock().unwrap();
        st.chunk_count = 0;
        st.sticky_remaining = 0;
    }

    fn name(&self) -> &str {
        "StickyWindowMockSpotter"
    }
}

/// Provedor TTS neural determinístico silencioso para testes de conversação (categoria `NeuralSpeech`, 22050Hz).
struct DeterministicNeuralTts {
    sample_rate: u32,
    meta: VoiceProviderMetadata,
}

impl DeterministicNeuralTts {
    fn new_22050() -> Self {
        Self {
            sample_rate: 22050,
            meta: VoiceProviderMetadata::new(
                "DeterministicNeuralTts (pt_BR-dii-female)",
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

impl TextToSpeechProvider for DeterministicNeuralTts {
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
                return Err(VoiceError::SynthesisFailed(
                    "Texto vazio para síntese".to_string(),
                ));
            }
            let fmt = AudioFormat {
                sample_rate: self.sample_rate,
                channels: 1,
                bits_per_sample: 16,
            };
            // 40ms de WAV válido para execução rápida em suíte de testes
            let num_samples = (self.sample_rate as usize * 40) / 1000;
            let samples = vec![0i16; num_samples];
            let wav = AudioCodec::encode_wav(&samples, &fmt)?;
            Ok(AudioOutput::new(wav, fmt, 40, "pt_BR-dii-female-test"))
        })
    }

    fn health<'a>(&'a self) -> Pin<Box<dyn Future<Output = Result<(), VoiceError>> + Send + 'a>> {
        Box::pin(async { Ok(()) })
    }
}

/// Provedor STT determinístico baseado em fila de transcrições e métricas instrumentadas.
struct QueueSttProvider {
    outputs: Mutex<VecDeque<Result<String, VoiceError>>>,
    timings: Mutex<Option<SttTimingMetrics>>,
    meta: VoiceProviderMetadata,
}

impl QueueSttProvider {
    fn from_texts(texts: Vec<&str>) -> Self {
        let q = texts.into_iter().map(|t| Ok(t.to_string())).collect();
        Self {
            outputs: Mutex::new(q),
            timings: Mutex::new(Some(SttTimingMetrics {
                audio_prep_ms: 1.2,
                audio_norm_ms: 2.4,
                temp_write_ms: 0.8,
                subprocess_wall_ms: 920.0,
                whisper_load_ms: Some(180.0),
                whisper_mel_ms: Some(14.0),
                whisper_sample_ms: Some(2.0),
                whisper_encode_ms: Some(610.0),
                whisper_decode_ms: Some(95.0),
                whisper_prompt_ms: Some(12.0),
                whisper_total_ms: Some(913.0),
                whisper_fallbacks: Some(0),
                post_process_ms: 0.5,
                total_stt_wall_ms: 924.9,
            })),
            meta: VoiceProviderMetadata::new(
                "QueueSttProvider",
                VoiceProviderType::LocalEdge,
                vec!["pt-BR".to_string()],
                16000,
                false,
                "1.0",
            ),
        }
    }

    fn failing(msg: &str) -> Self {
        let mut q = VecDeque::new();
        q.push_back(Err(VoiceError::TranscriptionFailed(msg.to_string())));
        Self {
            outputs: Mutex::new(q),
            timings: Mutex::new(None),
            meta: VoiceProviderMetadata::new(
                "QueueSttProvider",
                VoiceProviderType::LocalEdge,
                vec!["pt-BR".to_string()],
                16000,
                false,
                "1.0",
            ),
        }
    }
}

impl SpeechToTextProvider for QueueSttProvider {
    fn metadata(&self) -> &VoiceProviderMetadata {
        &self.meta
    }

    fn transcribe<'a>(
        &'a self,
        _audio: &'a AudioInput,
    ) -> Pin<Box<dyn Future<Output = Result<String, VoiceError>> + Send + 'a>> {
        Box::pin(async move {
            let mut q = self.outputs.lock().unwrap();
            q.pop_front().unwrap_or_else(|| Ok(String::new()))
        })
    }

    fn health<'a>(&'a self) -> Pin<Box<dyn Future<Output = Result<(), VoiceError>> + Send + 'a>> {
        Box::pin(async { Ok(()) })
    }

    fn last_timings(&self) -> Option<SttTimingMetrics> {
        self.timings.lock().unwrap().clone()
    }
}

// ============================================================================
// TESTE A — Proibição do fallback "oinoin" (`DiagnosticAcoustic`) e validação neural PT-BR
// ============================================================================
#[tokio::test]
async fn test_a_neural_tts_vs_diagnostic_acoustic_enforcement_and_intelligibility() {
    let acoustic = LocalAcousticTtsProvider::default();
    assert_eq!(
        acoustic.metadata().synthesis_kind,
        TtsSynthesisKind::DiagnosticAcoustic
    );
    assert!(!acoustic.metadata().is_intelligible_speech());

    let piper = LocalPiperTtsProvider::default();
    assert_eq!(
        piper.metadata().synthesis_kind,
        TtsSynthesisKind::NeuralSpeech
    );
    assert!(piper.metadata().is_intelligible_speech());

    // 1. GovernedVoiceRouter em modo conversacional DEVE rejeitar DiagnosticAcoustic (fail-closed)
    //    sem mascarar a causa primária local como mero erro de nuvem não autorizada.
    let cloud_called = Arc::new(AtomicBool::new(false));
    struct SpyCloudTts {
        called: Arc<AtomicBool>,
        meta: VoiceProviderMetadata,
    }
    impl TextToSpeechProvider for SpyCloudTts {
        fn metadata(&self) -> &VoiceProviderMetadata {
            &self.meta
        }
        fn synthesize<'a>(
            &'a self,
            _text: &'a str,
            _config: Option<&'a VoiceConfiguration>,
        ) -> Pin<Box<dyn Future<Output = Result<AudioOutput, VoiceError>> + Send + 'a>> {
            self.called.store(true, Ordering::SeqCst);
            Box::pin(async { unreachable!("Cloud TTS jamais deve ser chamado sem permissão") })
        }
        fn health<'a>(
            &'a self,
        ) -> Pin<Box<dyn Future<Output = Result<(), VoiceError>> + Send + 'a>> {
            Box::pin(async { Ok(()) })
        }
    }

    // 1a. Cenário real do `yuki voice live` / `voice turn` (`cloud_tts: None`):
    // Preserva diretamente `VoiceError::ProviderUnavailable` com contexto de fallback desabilitado.
    let router_local_only = GovernedVoiceRouter::new(
        Arc::new(QueueSttProvider::from_texts(vec!["teste"])),
        None,
        Arc::new(LocalAcousticTtsProvider::default()),
        None,
        VoiceConfiguration::default(),
    )
    .with_require_intelligible_tts(true);

    let err_local = router_local_only
        .synthesize("Olá José, tudo bem?", false)
        .await
        .expect_err("Modo conversacional deve bloquear LocalAcousticTtsProvider");
    let err_local_msg = err_local.to_string();
    assert!(
        matches!(err_local, VoiceError::ProviderUnavailable(_)),
        "Sem cloud_tts configurado, deve preservar erro primário ProviderUnavailable: {}",
        err_local_msg
    );
    assert!(
        err_local_msg.contains("DiagnosticAcoustic")
            || err_local_msg.contains("LocalAcousticTtsProvider"),
        "Mensagem de erro deve explicitar rejeição do gerador harmônico de diagnóstico: {}",
        err_local_msg
    );
    assert!(
        err_local_msg.to_lowercase().contains("fallback em nuvem"),
        "Mensagem deve informar secundariamente o status do fallback em nuvem: {}",
        err_local_msg
    );

    // 1b. Cenário governado com `cloud_tts: Some(SpyCloudTts)` e sem autorização de nuvem:
    // Mantém o contrato `UnauthorizedCloudFallback`, mas incluindo a causa primária local sem acionar a nuvem.
    let router_rejecting = GovernedVoiceRouter::new(
        Arc::new(QueueSttProvider::from_texts(vec!["teste"])),
        None,
        Arc::new(LocalAcousticTtsProvider::default()),
        Some(Arc::new(SpyCloudTts {
            called: cloud_called.clone(),
            meta: VoiceProviderMetadata::new(
                "SpyCloudTts",
                VoiceProviderType::GovernedCloud,
                vec!["pt-BR".to_string()],
                24000,
                false,
                "1.0",
            ),
        })),
        VoiceConfiguration::default(),
    )
    .with_require_intelligible_tts(true);

    let err = router_rejecting
        .synthesize("Olá José, tudo bem?", false)
        .await
        .expect_err("Modo conversacional com cloud não autorizado deve falhar fechado");
    let err_msg = err.to_string();
    assert!(
        matches!(err, VoiceError::UnauthorizedCloudFallback(_)),
        "Com cloud_tts configurado e não autorizado, deve retornar UnauthorizedCloudFallback: {}",
        err_msg
    );
    assert!(
        err_msg.contains("DiagnosticAcoustic") || err_msg.contains("LocalAcousticTtsProvider"),
        "UnauthorizedCloudFallback deve preservar a causa primária local: {}",
        err_msg
    );
    assert!(
        err_msg.to_lowercase().contains("fallback em nuvem"),
        "Mensagem deve informar secundariamente o bloqueio do fallback em nuvem: {}",
        err_msg
    );
    assert!(
        !cloud_called.load(Ordering::SeqCst),
        "Nenhum egresso para nuvem pode ocorrer sem autorização explícita"
    );

    // 2. GovernedVoiceRouter aceita provedor NeuralSpeech normalmente
    let router_neural = GovernedVoiceRouter::new(
        Arc::new(QueueSttProvider::from_texts(vec!["teste"])),
        None,
        Arc::new(DeterministicNeuralTts::new_22050()),
        None,
        VoiceConfiguration::default(),
    )
    .with_require_intelligible_tts(true);

    let ok_audio = router_neural
        .synthesize("Olá José, tudo bem?", false)
        .await
        .expect("Provedor NeuralSpeech deve ser aceito em modo conversacional");
    assert_eq!(ok_audio.format.sample_rate, 22050);

    // 3. Se Piper neural + Whisper locais estiverem instalados na máquina de homologação,
    //    valida síntese neural feminina real (daemon quente) + inteligibilidade acústica via STT roundtrip!
    if piper.is_environment_ready() {
        let resolved = resolve_conversational_local_tts(false)
            .expect("Com Piper instalado, resolve_conversational_local_tts(false) deve selecionar NeuralSpeech");
        assert_eq!(
            resolved.metadata().synthesis_kind,
            TtsSynthesisKind::NeuralSpeech
        );

        piper
            .warmup()
            .await
            .expect("Warmup do daemon Piper deve iniciar com sucesso");
        let t0 = std::time::Instant::now();
        let neural_out = piper
            .synthesize("Dois mais dois é quatro.", None)
            .await
            .expect("Síntese neural real no Piper deve produzir WAV válido");
        let warm_ms = t0.elapsed().as_millis();
        println!(
            "[Teste A] Síntese neural real '{}' concluída em {}ms ({}Hz, {} bytes)",
            neural_out.voice_name,
            warm_ms,
            neural_out.format.sample_rate,
            neural_out.payload.len()
        );
        assert!(neural_out.payload.starts_with(b"RIFF"));
        assert!(neural_out.payload.len() > 4000);
        assert_eq!(neural_out.format.channels, 1);
        assert_eq!(neural_out.format.bits_per_sample, 16);

        let whisper = LocalWhisperSttProvider::new(
            LocalWhisperConfig::default()
                .with_language("pt")
                .with_live_optimizations(),
        );
        if whisper.is_environment_ready() {
            let (fmt, pcm_22k) = AudioCodec::decode_wav(&neural_out.payload).unwrap();
            let pcm_16k = resample_pcm_linear(&pcm_22k, fmt.sample_rate, 16000);
            let fmt_16k = AudioFormat {
                sample_rate: 16000,
                channels: 1,
                bits_per_sample: 16,
            };
            let wav_16k = AudioCodec::encode_wav(&pcm_16k, &fmt_16k).unwrap();
            let duration_ms = (pcm_16k.len() as u64 * 1000) / 16000;
            let stt_in = AudioInput::new(wav_16k, fmt_16k, duration_ms, "test_a_roundtrip");
            let transcript = whisper
                .transcribe(&stt_in)
                .await
                .expect("Whisper deve transcrever fala neural sintetizada");
            println!("[Teste A] Roundtrip Piper -> Whisper: \"{}\"", transcript);
            let norm = transcript.to_lowercase();
            assert!(
                norm.contains("quatro") || norm.contains("4"),
                "Áudio neural sintetizado deve ser inteligível para STT PT-BR (obtido: '{}')",
                transcript
            );
        }
    }
}

// ============================================================================
// TESTE B — Provedor Cognitivo Conversacional: bloqueio de Mock silencioso + CredentialBroker sem vazar segredos
// ============================================================================
#[test]
fn test_b_conversational_model_provider_blocks_silent_mock_and_integrates_broker() {
    let _guard = ENV_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    // Salva variáveis de ambiente originais para restauração determinística
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

    struct EnvRestorer<'a>(Vec<(&'a str, Option<String>)>);
    impl Drop for EnvRestorer<'_> {
        fn drop(&mut self) {
            for (k, v) in &self.0 {
                match v {
                    Some(val) => std::env::set_var(k, val),
                    None => std::env::remove_var(k),
                }
            }
        }
    }
    let _restore = EnvRestorer(saved);

    for k in &vars {
        std::env::remove_var(k);
    }

    // 1. Sem YUKI_MODEL_PROVIDER configurado e allow_mock=false -> Fail-Closed imediato!
    let err_unset = match resolve_conversational_model_provider_from_env(false) {
        Ok(p) => panic!(
            "Não deve permitir fallback silencioso quando YUKI_MODEL_PROVIDER está ausente (obteve: {})",
            p.metadata().provider_name
        ),
        Err(e) => e,
    };
    assert!(err_unset.to_string().contains("YUKI_MODEL_PROVIDER"));

    // 2. Com YUKI_MODEL_PROVIDER="mock" e allow_mock=false -> Fail-Closed!
    std::env::set_var("YUKI_MODEL_PROVIDER", "mock");
    assert!(
        resolve_conversational_model_provider_from_env(false).is_err(),
        "YUKI_MODEL_PROVIDER=mock deve ser bloqueado em voz conversacional sem --mock explícito"
    );

    // 3. Com allow_mock=true -> Mock permitido explicitamente e banner destaca modo diagnóstico
    let mock_prov = resolve_conversational_model_provider_from_env(true)
        .expect("Com --mock explícito, MockModelProvider deve ser permitido");
    let mock_banner = format_active_model_provider_banner(mock_prov.as_ref());
    assert!(mock_banner.contains("MockProvider"));
    assert!(mock_banner.contains("MODO DIAGNÓSTICO"));

    // 4. Com YUKI_MODEL_PROVIDER="gemini" mas sem nenhuma credencial disponível -> Fail-Closed!
    std::env::set_var("YUKI_MODEL_PROVIDER", "gemini");
    let err_no_cred = match resolve_conversational_model_provider_from_env(false) {
        Ok(_) => panic!("Gemini sem chave no env nem vault deve falhar fechado na inicialização"),
        Err(e) => e,
    };
    assert!(matches!(err_no_cred, ModelError::Authentication(_)));

    // 5. Com YUKI_GEMINI_API_KEY definido -> resolve GeminiProviderAdapter sem jamais expor o segredo!
    let fake_env_secret = "yuki-test-sentinel-env-credential-987654";
    std::env::set_var("YUKI_GEMINI_API_KEY", fake_env_secret);
    let gemini_env_prov = resolve_conversational_model_provider_from_env(false)
        .expect("Gemini com YUKI_GEMINI_API_KEY deve resolver via EnvSecretStore");
    let banner_env = format_active_model_provider_banner(gemini_env_prov.as_ref());
    assert!(banner_env.contains("gemini"));
    assert!(
        !banner_env.contains(fake_env_secret),
        "Banner público jamais pode conter o segredo"
    );
    let meta_json = serde_json::to_string(&gemini_env_prov.metadata()).unwrap();
    assert!(!meta_json.contains(fake_env_secret));

    // 6. Com env keys removidas e CredentialVault configurado -> resolve via VaultSecretStore sem vazar segredo nem passphrase!
    std::env::remove_var("YUKI_GEMINI_API_KEY");
    let tmp_dir = std::env::temp_dir().join(format!(
        "yuki_test_b_vault_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&tmp_dir).unwrap();
    let vault_path = tmp_dir.join("vault.json");
    let fake_passphrase = "vault-master-passphrase-never-expose-123!";
    let fake_vault_secret = "yuki-test-sentinel-vault-credential-555666";

    {
        let vault = CredentialVault::create_new(
            &vault_path,
            SecretMaterial::new(fake_passphrase.to_string()),
        )
        .expect("Deve criar cofre temporário");
        vault
            .store(
                "gemini_api_key",
                fake_vault_secret,
                "gemini",
                Some("Chave de teste Marco 8.1.3".to_string()),
            )
            .expect("Deve armazenar segredo cifrado no cofre");
    }

    std::env::set_var("YUKI_VAULT_PATH", &vault_path);
    std::env::set_var("YUKI_VAULT_PASSPHRASE", fake_passphrase);

    let gemini_vault_prov = resolve_conversational_model_provider_from_env(false).expect(
        "Gemini deve resolver credencial cifrada via CredentialVault + CompositeCredentialBroker",
    );
    let banner_vault = format_active_model_provider_banner(gemini_vault_prov.as_ref());
    assert!(banner_vault.contains("gemini"));
    assert!(!banner_vault.contains(fake_vault_secret));
    assert!(!banner_vault.contains(fake_passphrase));

    let _ = std::fs::remove_dir_all(&tmp_dir);
}

// ============================================================================
// TESTE C — Ausência de falsas reativações (`0.96 -> 0.99 -> 0.98`) e rejeição de ruído estacionário
// ============================================================================
#[tokio::test]
async fn test_c_wake_word_no_false_reactivation_after_trigger_and_silence() {
    let spotter = Arc::new(StickyWindowMockSpotter::new());
    let core = Arc::new(YukiCore::new()); // Mock interno apenas para orquestração acústica isolada do teste
    let stt = Arc::new(QueueSttProvider::from_texts(vec![
        "Yuki, olá",
        "FANTASMA_NAO_DEVE_OCORRER",
    ]));
    let tts = Arc::new(DeterministicNeuralTts::new_22050());
    let router = Arc::new(
        GovernedVoiceRouter::new(stt, None, tts, None, VoiceConfiguration::default())
            .with_require_intelligible_tts(true),
    );
    let playback = Arc::new(AudioPlaybackController::for_test());
    let coordinator = Arc::new(VoiceConversationCoordinator::new(
        core,
        router,
        Arc::new(VoiceSessionManager::new()),
        playback,
        Arc::new(tokio::sync::Mutex::new(WakeWordDetector::default())),
        None,
    ));

    let cfg = VoiceLiveListenerConfig {
        threshold: 0.45,
        follow_up_secs: 1,
        silence_hangover_ms: 420,
        max_phrase_secs: 5,
        pre_roll_ms: 320,
        acoustic_cooldown_ms: 30,
        wake_cooldown_ms: 60,
        followup_confirm_frames: 3,
        ..Default::default()
    };

    let listener = VoiceLiveListener::with_spotter(cfg, coordinator, spotter.clone());
    let (tx, rx) = std::sync::mpsc::channel::<Vec<i16>>();
    let shutdown = Arc::new(AtomicBool::new(true));
    let shutdown_producer = shutdown.clone();

    let producer = std::thread::spawn(move || {
        // 1. 16 blocos de silêncio para warmup inicial do motor de Wake Word
        for _ in 0..16 {
            let _ = tx.send(make_silence_block());
            std::thread::sleep(Duration::from_millis(5));
        }
        // 2. Disparo único legítimo da palavra "Yuki" (score 0.96; sem reset ficaria preso em 0.99/0.98!)
        let _ = tx.send(StickyWindowMockSpotter::make_wake_trigger_block());
        for k in 0..4 {
            let _ = tx.send(make_speech_block(k));
            std::thread::sleep(Duration::from_millis(25));
        }
        // 3. Silêncio pós-fala para disparar EOS (~500ms)
        for _ in 0..11 {
            let _ = tx.send(make_silence_block());
            std::thread::sleep(Duration::from_millis(50));
        }
        // 4. Durante a janela de FollowUp (~1s), injeta ruído estacionário constante (RMS ~367 > 300, std_dev ~0)
        //    que no código antigo causava captura fantasma imediata!
        for _ in 0..24 {
            let _ = tx.send(make_stationary_noise_block());
            std::thread::sleep(Duration::from_millis(50));
        }
        // 5. Após expiração do FollowUp de volta a AwaitingWakeWord, envia 24 blocos de silêncio
        for _ in 0..24 {
            let _ = tx.send(make_silence_block());
            std::thread::sleep(Duration::from_millis(15));
        }
        shutdown_producer.store(false, Ordering::SeqCst);
    });

    let stats = listener
        .run_event_loop(rx, shutdown)
        .await
        .expect("Loop conversacional deve concluir de forma limpa");
    producer.join().unwrap();

    assert_eq!(
        stats.total_turns, 1,
        "Apenas o turno legítimo deve executar; ruído estacionário e cauda de embeddings não podem reativar turnos fantasmas"
    );
    assert_eq!(
        stats.follow_up_turns, 0,
        "Ruído estacionário na janela de follow-up não pode acionar turno fantasma"
    );
    assert!(
        spotter.reset_count() >= 3,
        "Spotter deve sofrer reset na partida, imediatamente após o disparo de Wake Word e ao retornar a AwaitingWakeWord (resets={})",
        spotter.reset_count()
    );

    // Validação adicional direta com o motor real `OpenWakeWordEngine` caso disponível localmente
    if OpenWakeWordEngine::is_available() {
        let engine = OpenWakeWordEngine::from_default_paths()
            .expect("Motor OpenWakeWord local deve carregar com sucesso");
        for k in 0..18 {
            let _ = engine.process_chunk(&make_speech_block(k)).unwrap();
        }
        assert!(engine.chunk_count() >= 18);
        engine.reset();
        assert_eq!(
            engine.chunk_count(),
            0,
            "OpenWakeWordEngine::reset() deve zerar chunk_count e exigir novo warmup"
        );
        for _ in 0..20 {
            let score = engine.process_chunk(&make_silence_block()).unwrap();
            assert!(
                score < 0.05,
                "Blocos silenciosos após reset jamais podem produzir score residual alto (score={})",
                score
            );
        }
    }
}

// ============================================================================
// TESTE D — Conversa matemática multi-turno ("2+2" -> "4" -> "E vezes 3?" -> "12") com histórico real no ModelProvider
// ============================================================================
#[tokio::test]
async fn test_d_multi_turn_math_conversation_propagates_history_to_provider() {
    struct ContextualMathProvider {
        captured_requests: Arc<Mutex<Vec<ModelRequest>>>,
    }

    impl ModelProvider for ContextualMathProvider {
        fn generate<'a>(
            &'a self,
            request: &'a ModelRequest,
        ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, ModelError>> + Send + 'a>> {
            self.captured_requests.lock().unwrap().push(request.clone());
            let prompt_lower = request.prompt.to_lowercase();

            // Responde exclusivamente com base no prompt + histórico entregues ao provedor cognitivo:
            // Se o turno 2 ("e vezes três?") chegasse sem o histórico do turno 1 ("quatro"), falharia!
            let answer = if prompt_lower.contains("vezes três") || prompt_lower.contains("vezes 3")
            {
                if prompt_lower.contains("quatro") && prompt_lower.contains("dois mais dois") {
                    "Quatro vezes três é doze."
                } else {
                    "CONTEXTO_AUSENTE_NO_PROVEDOR"
                }
            } else if prompt_lower.contains("dois mais dois") {
                "Dois mais dois é quatro."
            } else {
                "Não compreendi."
            };

            let req_id = request.request_id.clone();
            Box::pin(async move {
                Ok(ModelResponse::text(
                    req_id,
                    "ContextualMathProvider",
                    "contextual-test-v1",
                    answer,
                ))
            })
        }

        fn metadata(&self) -> ModelMetadata {
            ModelMetadata {
                provider_name: "ContextualMathProvider".to_string(),
                model_name: "contextual-test-v1".to_string(),
                version: "1.0".to_string(),
            }
        }

        fn health(&self) -> HealthStatus {
            HealthStatus::Healthy
        }
    }

    let captured_requests = Arc::new(Mutex::new(Vec::new()));
    let provider: Arc<dyn ModelProvider> = Arc::new(ContextualMathProvider {
        captured_requests: captured_requests.clone(),
    });
    let core = Arc::new(YukiCore::new().with_model_provider(provider));

    let stt = Arc::new(QueueSttProvider::from_texts(vec![
        "Yuki, quanto é dois mais dois?",
        "E vezes três?",
    ]));
    let tts = Arc::new(DeterministicNeuralTts::new_22050());
    let router = Arc::new(
        GovernedVoiceRouter::new(stt, None, tts, None, VoiceConfiguration::default())
            .with_require_intelligible_tts(true),
    );
    let playback = Arc::new(AudioPlaybackController::for_test());
    let coordinator = Arc::new(VoiceConversationCoordinator::new(
        core,
        router,
        Arc::new(VoiceSessionManager::new()),
        playback,
        Arc::new(tokio::sync::Mutex::new(WakeWordDetector::default())),
        None,
    ));

    let spotter = Arc::new(StickyWindowMockSpotter::new());
    let cfg = VoiceLiveListenerConfig {
        threshold: 0.45,
        follow_up_secs: 4,
        silence_hangover_ms: 420,
        max_phrase_secs: 5,
        pre_roll_ms: 320,
        acoustic_cooldown_ms: 25,
        wake_cooldown_ms: 50,
        followup_confirm_frames: 3,
        ..Default::default()
    };

    let listener = VoiceLiveListener::with_spotter(cfg, coordinator, spotter);
    let (tx, rx) = std::sync::mpsc::channel::<Vec<i16>>();
    let shutdown = Arc::new(AtomicBool::new(true));
    let shutdown_producer = shutdown.clone();

    let producer = std::thread::spawn(move || {
        // Turno 1: Warmup (16 blocos) + Wake Word "Yuki" + fala "quanto é dois mais dois?" + EOS
        for _ in 0..16 {
            let _ = tx.send(make_silence_block());
            std::thread::sleep(Duration::from_millis(5));
        }
        let _ = tx.send(StickyWindowMockSpotter::make_wake_trigger_block());
        for k in 0..4 {
            let _ = tx.send(make_speech_block(k));
            std::thread::sleep(Duration::from_millis(25));
        }
        for _ in 0..11 {
            let _ = tx.send(make_silence_block());
            std::thread::sleep(Duration::from_millis(50));
        }

        // Aguarda processamento do Turno 1 + abertura da FollowUpWindow (~200ms)
        std::thread::sleep(Duration::from_millis(220));

        // Turno 2 (Follow-Up SEM repetir "Yuki"): envia 6 blocos sustentados de fala modulada ("E vezes três?") + EOS
        for k in 0..6 {
            let _ = tx.send(make_speech_block(k));
            std::thread::sleep(Duration::from_millis(30));
        }
        for _ in 0..11 {
            let _ = tx.send(make_silence_block());
            std::thread::sleep(Duration::from_millis(50));
        }

        // Aguarda conclusão do Turno 2 e encerra sessão
        std::thread::sleep(Duration::from_millis(220));
        shutdown_producer.store(false, Ordering::SeqCst);
    });

    let stats = listener
        .run_event_loop(rx, shutdown)
        .await
        .expect("Sessão multi-turno deve concluir sem erros");
    producer.join().unwrap();

    assert_eq!(stats.total_turns, 2, "Devem ocorrer exatamente 2 turnos");
    assert_eq!(
        stats.follow_up_turns, 1,
        "O segundo turno deve ocorrer na janela de follow-up sem exigir repetição de 'Yuki'"
    );
    assert_eq!(stats.records[0].yuki_text, "Dois mais dois é quatro.");
    assert_eq!(stats.records[1].yuki_text, "Quatro vezes três é doze.");

    let reqs = captured_requests.lock().unwrap();
    assert_eq!(reqs.len(), 2);
    assert!(
        reqs[1].prompt.contains("Usuário: Yuki, quanto é dois mais dois?")
            && reqs[1].prompt.contains("Yuki: Dois mais dois é quatro."),
        "O segundo turno deve entregar conversation_history completo ao ModelProvider! Prompt recebido:\n{}",
        reqs[1].prompt
    );
}

// ============================================================================
// TESTE E — Recuperação calma após transcrição vazia (sem loop infinito)
// ============================================================================
#[tokio::test]
async fn test_e_empty_stt_recovery_without_infinite_loop() {
    let spotter = Arc::new(StickyWindowMockSpotter::new());
    let core = Arc::new(YukiCore::new());
    // Turno 1 retorna transcrição vazia ""; Turno 2 retorna despedida legítima "Yuki, obrigado, só isso"
    let stt = Arc::new(QueueSttProvider::from_texts(vec![
        "   ",
        "Yuki, obrigado, só isso",
    ]));
    let tts = Arc::new(DeterministicNeuralTts::new_22050());
    let router = Arc::new(
        GovernedVoiceRouter::new(stt, None, tts, None, VoiceConfiguration::default())
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
        threshold: 0.45,
        follow_up_secs: 1,
        silence_hangover_ms: 420,
        max_phrase_secs: 5,
        pre_roll_ms: 320,
        acoustic_cooldown_ms: 20,
        wake_cooldown_ms: 40,
        followup_confirm_frames: 3,
        ..Default::default()
    };

    let listener = VoiceLiveListener::with_spotter(cfg, coordinator, spotter.clone());
    let (tx, rx) = std::sync::mpsc::channel::<Vec<i16>>();
    let shutdown = Arc::new(AtomicBool::new(true));
    let shutdown_producer = shutdown.clone();

    let producer = std::thread::spawn(move || {
        // Tentativa 1 -> STT retorna vazio -> deve voltar calmamente a AwaitingWakeWord sem loopar!
        for _ in 0..16 {
            let _ = tx.send(make_silence_block());
            std::thread::sleep(Duration::from_millis(5));
        }
        let _ = tx.send(StickyWindowMockSpotter::make_wake_trigger_block());
        for k in 0..4 {
            let _ = tx.send(make_speech_block(k));
            std::thread::sleep(Duration::from_millis(25));
        }
        for _ in 0..11 {
            let _ = tx.send(make_silence_block());
            std::thread::sleep(Duration::from_millis(50));
        }

        // Aguarda recuperação calma + wake_cooldown_ms (40ms)
        std::thread::sleep(Duration::from_millis(120));

        // Envia 16 blocos de silêncio pós-reset (comprovando ausência de loop em silêncio) e depois novo disparo legítimo com frase de despedida
        for _ in 0..16 {
            let _ = tx.send(make_silence_block());
            std::thread::sleep(Duration::from_millis(5));
        }
        let _ = tx.send(StickyWindowMockSpotter::make_wake_trigger_block());
        for k in 0..4 {
            let _ = tx.send(make_speech_block(k));
            std::thread::sleep(Duration::from_millis(25));
        }
        for _ in 0..11 {
            let _ = tx.send(make_silence_block());
            std::thread::sleep(Duration::from_millis(50));
        }
        std::thread::sleep(Duration::from_millis(220));
        shutdown_producer.store(false, Ordering::SeqCst);
    });

    let stats = tokio::time::timeout(
        Duration::from_secs(8),
        listener.run_event_loop(rx, shutdown),
    )
    .await
    .expect("Sessão não pode entrar em loop infinito após STT vazio")
    .expect("Loop deve encerrar graciosamente na despedida do Turno 2");
    producer.join().unwrap();

    assert_eq!(
        stats.empty_stt_discards, 1,
        "Deve contabilizar exatamente 1 descarte calmo por STT vazio"
    );
    assert_eq!(
        stats.total_turns, 1,
        "Deve concluir normalmente o turno válido subsequente"
    );
}

// ============================================================================
// TESTE F — Cancelamento gracioso (Ctrl+C / shutdown_flag) e retomada limpa na mesma sessão
// ============================================================================
#[tokio::test]
async fn test_f_graceful_cancellation_and_clean_session_resumption() {
    let session_manager = Arc::new(VoiceSessionManager::new());
    let core = Arc::new(YukiCore::new());
    let stt = Arc::new(QueueSttProvider::from_texts(vec![
        "Yuki, teste pós-cancelamento, obrigado",
    ]));
    let tts = Arc::new(DeterministicNeuralTts::new_22050());
    let router = Arc::new(
        GovernedVoiceRouter::new(stt, None, tts, None, VoiceConfiguration::default())
            .with_require_intelligible_tts(true),
    );
    let coordinator = Arc::new(VoiceConversationCoordinator::new(
        core,
        router,
        session_manager.clone(),
        Arc::new(AudioPlaybackController::for_test()),
        Arc::new(tokio::sync::Mutex::new(WakeWordDetector::default())),
        None,
    ));

    let cfg = VoiceLiveListenerConfig {
        silence_hangover_ms: 420,
        acoustic_cooldown_ms: 20,
        wake_cooldown_ms: 30,
        ..Default::default()
    };

    // Execução 1: inicia captura de frase e cancela via shutdown_flag no meio da fala
    let spotter1 = Arc::new(StickyWindowMockSpotter::new());
    let listener1 = VoiceLiveListener::with_spotter(cfg.clone(), coordinator.clone(), spotter1);
    let (tx1, rx1) = std::sync::mpsc::channel::<Vec<i16>>();
    let shutdown1 = Arc::new(AtomicBool::new(true));
    let shutdown1_trigger = shutdown1.clone();

    std::thread::spawn(move || {
        for _ in 0..16 {
            let _ = tx1.send(make_silence_block());
        }
        let _ = tx1.send(StickyWindowMockSpotter::make_wake_trigger_block());
        let _ = tx1.send(make_speech_block(0));
        std::thread::sleep(Duration::from_millis(80));
        shutdown1_trigger.store(false, Ordering::SeqCst);
    });

    let stats1 = tokio::time::timeout(
        Duration::from_secs(2),
        listener1.run_event_loop(rx1, shutdown1),
    )
    .await
    .expect("Cancelamento deve encerrar prontamente sem travar")
    .expect("Cancelamento gracioso deve retornar Ok(stats)");
    assert_eq!(stats1.total_turns, 0);

    // Execução 2: reutiliza exatamente o MESMO coordinator + session_manager e completa um turno inteiro!
    let spotter2 = Arc::new(StickyWindowMockSpotter::new());
    let listener2 = VoiceLiveListener::with_spotter(cfg, coordinator, spotter2);
    let (tx2, rx2) = std::sync::mpsc::channel::<Vec<i16>>();
    let shutdown2 = Arc::new(AtomicBool::new(true));
    let shutdown2_producer = shutdown2.clone();

    std::thread::spawn(move || {
        for _ in 0..16 {
            let _ = tx2.send(make_silence_block());
            std::thread::sleep(Duration::from_millis(5));
        }
        let _ = tx2.send(StickyWindowMockSpotter::make_wake_trigger_block());
        for k in 0..4 {
            let _ = tx2.send(make_speech_block(k));
            std::thread::sleep(Duration::from_millis(25));
        }
        for _ in 0..11 {
            let _ = tx2.send(make_silence_block());
            std::thread::sleep(Duration::from_millis(50));
        }
        std::thread::sleep(Duration::from_millis(220));
        shutdown2_producer.store(false, Ordering::SeqCst);
    });

    let stats2 = tokio::time::timeout(
        Duration::from_secs(5),
        listener2.run_event_loop(rx2, shutdown2),
    )
    .await
    .expect("Retomada não pode bloquear por resíduo de sessão")
    .expect("Segunda sessão após cancelamento deve completar turno limpo");
    assert_eq!(stats2.total_turns, 1);
}

// ============================================================================
// TESTE G — Conversão PCM 22050Hz (Piper Neural) -> 48000Hz / 44100Hz WASAPI e enumeração segura
// ============================================================================
#[test]
fn test_g_wasapi_device_enumeration_and_22050hz_pcm_format_conversion() {
    // 1. Sintetiza 1,000s de senoide pura em 440 Hz a 22050 Hz (taxa nativa dos modelos Piper VITS PT-BR)
    let in_rate = 22050u32;
    let src_22k: Vec<i16> = (0..in_rate as usize)
        .map(|i| {
            let t = i as f64 / in_rate as f64;
            ((2.0 * std::f64::consts::PI * 440.0 * t).sin() * 16000.0) as i16
        })
        .collect();

    // 2. Reamostra para 48000 Hz (mix format típico do Windows WASAPI) e 44100 Hz
    let resampled_48k = resample_pcm_linear(&src_22k, 22050, 48000);
    assert_eq!(resampled_48k.len(), 48000);

    let resampled_44k = resample_pcm_linear(&src_22k, 22050, 44100);
    assert_eq!(resampled_44k.len(), 44100);

    // Verifica preservação de amplitude e ausência de clipping/DC offset em 48 kHz
    let peak_48k = resampled_48k
        .iter()
        .map(|s| s.unsigned_abs())
        .max()
        .unwrap();
    assert!(
        (15800..=16005).contains(&peak_48k),
        "Amplitude de pico em 48kHz deve preservar fielmente o sinal de 22050Hz (peak={})",
        peak_48k
    );

    // 3. Conversão para o formato WASAPI negociado (IEEE Float 32-bit estéreo 48kHz)
    let parsed_mix = ParsedWaveFormat {
        tag: 0xFFFE,
        channels: 2,
        sample_rate: 48000,
        avg_bytes_per_sec: 48000 * 8,
        block_align: 8,
        bits_per_sample: 32,
        cb_size: 22,
        valid_bits_per_sample: Some(32),
        channel_mask: Some(0x3),
        sub_format_data1: Some(3),
        is_ks_sub_format: true,
    };
    let class = classify_sample_format(&parsed_mix).expect("Deve classificar IEEE Float 32-bit");
    assert_eq!(class, WasapiSampleClassification::IeeeFloat32);

    let wasapi_bytes = convert_samples_to_wasapi(&resampled_48k, 2, class);
    assert_eq!(
        wasapi_bytes.len(),
        48000 * 2 * 4,
        "1s estéreo IEEE Float32 @ 48kHz deve conter exatamente 384.000 bytes"
    );
    for &chunk in wasapi_bytes.as_chunks::<4>().0 {
        let sample_f32 = f32::from_le_bytes(chunk);
        assert!(
            sample_f32.is_finite() && (-1.0..=1.0).contains(&sample_f32),
            "Amostra WASAPI Float32 fora da faixa válida: {}",
            sample_f32
        );
    }

    // 4. Seleção segura de controlador sem emitir som físico em `cargo test`
    let disabled_ctrl = AudioPlaybackController::for_environment(PlaybackEnvironment::Disabled);
    assert_eq!(
        disabled_ctrl.sink().name(),
        "WindowsAudioPlaybackSink (WASAPI Core Audio)"
    );

    #[cfg(windows)]
    {
        let devices = yuki::voice::devices::list_audio_devices(false)
            .expect("Enumeração WASAPI de dispositivos no Windows deve executar sem falha COM");
        println!(
            "[Teste G] Dispositivos WASAPI ativos enumerados no host Windows: {}",
            devices.len()
        );
    }
}

// ============================================================================
// TESTE H — Instrumentação ponta a ponta, `--audio-ctx` dinâmico, trim de silêncio e preservação de erro primário
// ============================================================================
#[tokio::test]
async fn test_h_end_to_end_latency_instrumentation_and_dynamic_audio_ctx() {
    // 1. Escalonamento dinâmico de `--audio-ctx` (evita codificar 30s = 1500 frames para falas curtas!)
    assert_eq!(
        compute_dynamic_audio_ctx(32_000, 16000),
        256,
        "Fala curta de 2.0s deve limitar encoder do Whisper a 256 frames (~5.1s) em vez de 1500 (30s)"
    );
    assert_eq!(
        compute_dynamic_audio_ctx(72_000, 16000),
        384,
        "Fala de 4.5s deve escalar encoder para ~384 frames com margem segura"
    );
    assert_eq!(
        compute_dynamic_audio_ctx(464_000, 16000),
        1500,
        "Fala longa próxima de 30s deve respeitar teto máximo de 1500 frames"
    );

    // 2. Trim conservador de silêncio nas bordas preservando margem de ataque/cauda (`pad_ms`)
    let mut padded_pcm = vec![0i16; 8000]; // 500ms silêncio inicial
    for k in 0..10 {
        padded_pcm.extend_from_slice(&make_speech_block(k)); // 800ms fala
    }
    padded_pcm.extend(std::iter::repeat_n(0i16, 12000)); // 750ms silêncio final (hangover)

    let trimmed = trim_silence_edges(&padded_pcm, 16000, 120.0, 120);
    assert!(
        trimmed.len() < padded_pcm.len() - 12000,
        "Trim deve remover silêncio morto das extremidades ({} -> {} amostras)",
        padded_pcm.len(),
        trimmed.len()
    );
    assert!(
        trimmed.len() >= 10 * 1280,
        "Trim jamais pode amputar a fala útil central ({} amostras)",
        trimmed.len()
    );

    // 3. Exposição de `SttTimingMetrics` via `GovernedVoiceRouter::last_stt_timings()`
    let stt = Arc::new(QueueSttProvider::from_texts(vec!["Olá Yuki"]));
    let tts = Arc::new(DeterministicNeuralTts::new_22050());
    let router = GovernedVoiceRouter::new(stt, None, tts, None, VoiceConfiguration::default());

    let dummy_wav = AudioCodec::encode_wav(&make_speech_block(0), &AudioFormat::default()).unwrap();
    let input = AudioInput::new(dummy_wav.clone(), AudioFormat::default(), 80, "test_h");
    let text = router.transcribe(&input, false).await.unwrap();
    assert_eq!(text, "Olá Yuki");

    let metrics = router
        .last_stt_timings()
        .expect("GovernedVoiceRouter deve expor SttTimingMetrics detalhados do último turno");
    assert_eq!(metrics.whisper_encode_ms, Some(610.0));
    assert_eq!(metrics.whisper_decode_ms, Some(95.0));

    // 4. Preservação da causa primária local no STT quando fallback em nuvem está desautorizado (Problema 5)
    let failing_router = GovernedVoiceRouter::new(
        Arc::new(QueueSttProvider::failing(
            "Timeout ou binário whisper-cli corrompido (código 127)",
        )),
        None,
        Arc::new(DeterministicNeuralTts::new_22050()),
        None,
        VoiceConfiguration::default(),
    );
    let stt_err = failing_router
        .transcribe(&input, false)
        .await
        .expect_err("Falha primária local deve propagar erro");
    let stt_err_msg = stt_err.to_string();
    assert!(
        matches!(stt_err, VoiceError::TranscriptionFailed(_)),
        "Não pode substituir TranscriptionFailed por UnauthorizedCloudFallback: {}",
        stt_err_msg
    );
    assert!(
        stt_err_msg.contains("código 127")
            && stt_err_msg.to_lowercase().contains("fallback em nuvem"),
        "Deve preservar causa primária local E informar status do fallback: {}",
        stt_err_msg
    );
}
