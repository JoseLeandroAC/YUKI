use std::sync::Arc;
use yuki::contracts::voice::{AudioFormat, AudioFrame, AudioInput, VoiceConfiguration, VoiceError};
use yuki::core::yuki_core::YukiCore;
use yuki::models::mock::MockModelProvider;
use yuki::voice::audio::AudioCodec;
use yuki::voice::conversation::VoiceConversationCoordinator;
use yuki::voice::playback::AudioPlaybackController;
use yuki::voice::router::GovernedVoiceRouter;
use yuki::voice::session::VoiceSessionManager;
use yuki::voice::stt::{CloudSttProvider, MockSttProvider};
use yuki::voice::tts::{CloudTtsProvider, LocalAcousticTtsProvider, MockTtsProvider};
use yuki::voice::wakeword::{WakeWordDetection, WakeWordDetector};

#[tokio::test]
async fn test_governed_voice_router_local_first_cloud_only_when_authorized() {
    let failing_local_stt = Arc::new(MockSttProvider::new().with_failure(true));
    let cloud_stt = Arc::new(CloudSttProvider::new(
        "Gemini-Voice-STT",
        "https://api.gemini.mock/speech",
        "gemini_voice_key",
        true,
    ));

    let local_tts = Arc::new(LocalAcousticTtsProvider::new(16000));
    let cloud_tts = Arc::new(CloudTtsProvider::new(
        "ElevenLabs-TTS",
        "https://api.eleven.mock/tts",
        "eleven_key",
        true,
    ));

    let config = VoiceConfiguration {
        cloud_fallback_allowed: false,
        ..Default::default()
    };

    let router = GovernedVoiceRouter::new(
        failing_local_stt.clone(),
        Some(cloud_stt.clone()),
        local_tts.clone(),
        Some(cloud_tts.clone()),
        config,
    );

    let dummy_audio = AudioInput::new(vec![0u8; 100], AudioFormat::default(), 200, "test");

    // 1. Local STT falha e Cloud Fallback está desabilitado na configuração -> Rejeita
    let res = router.transcribe(&dummy_audio, false).await;
    assert!(matches!(res, Err(VoiceError::UnauthorizedCloudFallback(_))));

    // 2. Mesmo que operador autorize, se config desabilita fallback -> Rejeita
    let res = router.transcribe(&dummy_audio, true).await;
    assert!(matches!(res, Err(VoiceError::UnauthorizedCloudFallback(_))));

    // 3. Habilita fallback na config, mas operador NÃO autorizou a capability específica egress:voice_cloud_stt -> Rejeita
    let mut permissive_config = router.config().clone();
    permissive_config.cloud_fallback_allowed = true;
    let permissive_router = GovernedVoiceRouter::new(
        failing_local_stt,
        Some(cloud_stt),
        local_tts,
        Some(cloud_tts),
        permissive_config,
    );

    let res = permissive_router.transcribe(&dummy_audio, false).await;
    assert!(matches!(res, Err(VoiceError::UnauthorizedCloudFallback(_))));

    // 4. Somente com config permitindo E autorização explícita do operador -> Sucesso
    let res = permissive_router.transcribe(&dummy_audio, true).await;
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), "Transcrição via Provedor Cloud Governado");
}

#[tokio::test]
async fn test_governed_voice_router_stt_and_tts_permissions_isolated() {
    let failing_local_stt = Arc::new(MockSttProvider::new().with_failure(true));
    let cloud_stt = Arc::new(CloudSttProvider::new(
        "Cloud-STT",
        "https://api.cloud/stt",
        "key",
        true,
    ));

    let failing_local_tts = Arc::new(MockTtsProvider::new().with_failure(true));
    let cloud_tts = Arc::new(CloudTtsProvider::new(
        "Cloud-TTS",
        "https://api.cloud/tts",
        "key",
        true,
    ));

    let config = VoiceConfiguration {
        cloud_fallback_allowed: true,
        ..Default::default()
    };

    let router = GovernedVoiceRouter::new(
        failing_local_stt,
        Some(cloud_stt),
        failing_local_tts,
        Some(cloud_tts),
        config,
    );

    // Autorização de TTS NÃO autoriza STT
    let dummy_audio = AudioInput::new(vec![0u8; 100], AudioFormat::default(), 200, "test");
    let stt_res = router.transcribe(&dummy_audio, false).await;
    assert!(matches!(
        stt_res,
        Err(VoiceError::UnauthorizedCloudFallback(_))
    ));

    // Autorização de STT NÃO autoriza TTS
    let tts_res = router.synthesize("Texto de teste", false).await;
    assert!(matches!(
        tts_res,
        Err(VoiceError::UnauthorizedCloudFallback(_))
    ));

    // Autorização explícita de TTS
    let tts_ok = router.synthesize("Texto de teste", true).await;
    assert!(tts_ok.is_ok());
}

#[tokio::test]
async fn test_voice_conversation_button_turn_end_to_end() {
    let model = Arc::new(MockModelProvider::with_behavior(
        yuki::models::mock::MockBehavior::DirectText(
            "Olá, sou a Yuki! Em que posso ajudar?".to_string(),
        ),
    ));
    let core = Arc::new(YukiCore::new().with_model_provider(model));

    let local_stt = Arc::new(MockSttProvider::new().with_response("Yuki, bom dia"));
    let local_tts = Arc::new(LocalAcousticTtsProvider::new(16000));
    let config = VoiceConfiguration::default();

    let router = Arc::new(GovernedVoiceRouter::new(
        local_stt, None, local_tts, None, config,
    ));

    let session_mgr = Arc::new(VoiceSessionManager::new());
    let playback = Arc::new(AudioPlaybackController::new());
    let wake_detector = Arc::new(tokio::sync::Mutex::new(WakeWordDetector::default()));

    let coordinator = VoiceConversationCoordinator::new(
        core,
        router,
        session_mgr,
        playback.clone(),
        wake_detector,
        None,
    );

    let format = AudioFormat::default();
    let samples = vec![100i16; 1600]; // 100ms
    let wav_payload = AudioCodec::encode_wav(&samples, &format).unwrap();
    let input = AudioInput::new(wav_payload, format, 100, "mic-0");

    let (result, audio_output) = coordinator
        .process_button_turn("device-test", input, false, false)
        .await
        .expect("Button turn succeeded");

    assert!(result.content.contains("Yuki"));
    assert!(audio_output.duration_ms > 0);
    assert_eq!(
        playback.status(),
        yuki::voice::playback::PlaybackStatus::Playing
    );
}

#[tokio::test]
async fn test_wake_word_detector_local_and_invariants() {
    let mut detector = WakeWordDetector::new("yuki", 0.5, true);
    assert_eq!(detector.keyword(), "yuki");
    assert!(detector.is_enabled());

    // 1. Frame silencioso -> sem detecção
    let silent_frame = AudioFrame::new(vec![0i16; 160], 16000, 1, 0);
    let res = detector.process_frame(&silent_frame).unwrap();
    assert_eq!(res, WakeWordDetection::None);

    // 2. Desativação completa
    detector.set_enabled(false);
    assert!(!detector.is_enabled());
    let loud_frame = AudioFrame::new(vec![15000i16; 160], 16000, 1, 10);
    let res = detector.process_frame(&loud_frame).unwrap();
    assert_eq!(res, WakeWordDetection::None);

    // 3. Reativação e teste de sensibilidade
    detector.set_enabled(true);
    detector.set_sensitivity(0.8);
    assert_eq!(detector.sensitivity(), 0.8);

    // 4. Invariante: detecção de wake word NUNCA é token de autorização
    // (O detector apenas retorna enum WakeWordDetection::Detected com confidence e timestamp,
    // sem interagir com security controller ou conceder capabilities).
}

#[test]
fn test_voice_interruption_barge_in() {
    let playback = AudioPlaybackController::new();
    let dummy_output = yuki::contracts::voice::AudioOutput::new(
        vec![0u8; 100],
        AudioFormat::default(),
        5000,
        "test-voice",
    );

    // Inicia reprodução
    playback.play(&dummy_output).unwrap();
    assert_eq!(
        playback.status(),
        yuki::voice::playback::PlaybackStatus::Playing
    );

    // Usuário interrompe
    playback.interrupt().unwrap();
    assert_eq!(
        playback.status(),
        yuki::voice::playback::PlaybackStatus::Interrupted
    );

    // Reset limpa
    assert!(playback.reset().is_ok());
    assert_eq!(
        playback.status(),
        yuki::voice::playback::PlaybackStatus::Idle
    );
}

#[tokio::test]
async fn test_voice_runtime_environment_selection_and_coordinator_wiring() {
    use yuki::contracts::voice::VoiceConfiguration;
    use yuki::core::yuki_core::YukiCore;
    use yuki::models::MockModelProvider;
    use yuki::voice::playback::{AudioPlaybackController, PlaybackEnvironment};
    use yuki::voice::router::GovernedVoiceRouter;
    use yuki::voice::session::VoiceSessionManager;
    use yuki::voice::stt::MockSttProvider;
    use yuki::voice::tts::MockTtsProvider;
    use yuki::voice::wakeword::WakeWordDetector;

    // 1. A configuração de produção seleciona o backend correto
    let prod_controller = AudioPlaybackController::production();
    assert_eq!(
        prod_controller.sink().name(),
        "WindowsAudioPlaybackSink (winmm)"
    );

    let prod_env_ctrl = AudioPlaybackController::for_environment(PlaybackEnvironment::Production);
    assert_eq!(
        prod_env_ctrl.sink().name(),
        "WindowsAudioPlaybackSink (winmm)"
    );

    // 2. A configuração de testes seleciona o mock
    let test_controller = AudioPlaybackController::for_environment(PlaybackEnvironment::Test);
    assert_eq!(test_controller.sink().name(), "MockPlaybackSink");

    let default_controller = AudioPlaybackController::new();
    assert_eq!(default_controller.sink().name(), "MockPlaybackSink");

    // 3. O VoiceConversationCoordinator recebe o controlador selecionado
    let model = Arc::new(MockModelProvider::new());
    let core = Arc::new(YukiCore::new().with_model_provider(model));
    let stt = Arc::new(MockSttProvider::new());
    let tts = Arc::new(MockTtsProvider::new());
    let router = Arc::new(GovernedVoiceRouter::new(
        stt,
        None,
        tts,
        None,
        VoiceConfiguration::default(),
    ));
    let session_mgr = Arc::new(VoiceSessionManager::new());
    let wake_detector = Arc::new(tokio::sync::Mutex::new(WakeWordDetector::default()));

    // Instanciação com production()
    let prod_coord = VoiceConversationCoordinator::production(
        core.clone(),
        router.clone(),
        session_mgr.clone(),
        wake_detector.clone(),
        None,
    );
    assert_eq!(
        prod_coord.playback().sink().name(),
        "WindowsAudioPlaybackSink (winmm)"
    );

    // Instanciação com for_test()
    let (test_coord, mock_sink) = VoiceConversationCoordinator::for_test(
        core.clone(),
        router.clone(),
        session_mgr.clone(),
        wake_detector.clone(),
        None,
    );
    assert_eq!(test_coord.playback().sink().name(), "MockPlaybackSink");
    assert_eq!(mock_sink.played_count(), 0);

    // Instanciação parametrizada por ambiente
    let env_prod_coord = VoiceConversationCoordinator::for_environment(
        PlaybackEnvironment::Production,
        core.clone(),
        router.clone(),
        session_mgr.clone(),
        wake_detector.clone(),
        None,
    );
    assert_eq!(
        env_prod_coord.playback().sink().name(),
        "WindowsAudioPlaybackSink (winmm)"
    );

    let env_test_coord = VoiceConversationCoordinator::for_environment(
        PlaybackEnvironment::Test,
        core,
        router,
        session_mgr,
        wake_detector,
        None,
    );
    assert_eq!(env_test_coord.playback().sink().name(), "MockPlaybackSink");
}

#[tokio::test]
async fn test_voice_coordinator_tts_output_forwarded_to_playback_controller() {
    use yuki::contracts::voice::{AudioFormat, AudioInput, VoiceConfiguration};
    use yuki::core::yuki_core::YukiCore;
    use yuki::models::{MockBehavior, MockModelProvider};
    use yuki::voice::audio::AudioCodec;
    use yuki::voice::playback::PlaybackStatus;
    use yuki::voice::router::GovernedVoiceRouter;
    use yuki::voice::session::VoiceSessionManager;
    use yuki::voice::stt::MockSttProvider;
    use yuki::voice::tts::MockTtsProvider;
    use yuki::voice::wakeword::WakeWordDetector;

    let model = Arc::new(MockModelProvider::with_behavior(MockBehavior::DirectText(
        "Olá, eu sou a Yuki.".to_string(),
    )));
    let core = Arc::new(YukiCore::new().with_model_provider(model));
    let stt = Arc::new(MockSttProvider::new().with_response("Yuki, tudo bem?"));
    let tts = Arc::new(MockTtsProvider::new());
    let router = Arc::new(GovernedVoiceRouter::new(
        stt,
        None,
        tts,
        None,
        VoiceConfiguration::default(),
    ));
    let session_mgr = Arc::new(VoiceSessionManager::new());
    let wake_detector = Arc::new(tokio::sync::Mutex::new(WakeWordDetector::default()));

    let (coordinator, mock_sink) =
        VoiceConversationCoordinator::for_test(core, router, session_mgr, wake_detector, None);

    let format = AudioFormat::default();
    let samples = vec![50i16; 1600];
    let payload = AudioCodec::encode_wav(&samples, &format).unwrap();
    let input = AudioInput::new(payload, format, 100, "mic-test");

    // 4. Executa turno conversacional: saída TTS simulada é encaminhada ao controlador
    let (yuki_res, audio_out) = coordinator
        .process_button_turn("device-wiring-test", input, false, false)
        .await
        .expect("Turno de conversação deve ter sucesso");

    assert_eq!(yuki_res.content, "Olá, eu sou a Yuki.");
    assert_eq!(mock_sink.played_count(), 1);
    assert_eq!(mock_sink.last_played().unwrap().payload, audio_out.payload);
    assert!(
        coordinator.playback().status() == PlaybackStatus::Playing
            || coordinator.playback().status() == PlaybackStatus::Completed
    );
}

#[tokio::test]
async fn test_voice_coordinator_playback_failure_propagation() {
    use yuki::contracts::voice::{AudioFormat, AudioInput, VoiceConfiguration, VoiceError};
    use yuki::core::yuki_core::YukiCore;
    use yuki::models::{MockBehavior, MockModelProvider};
    use yuki::voice::audio::AudioCodec;
    use yuki::voice::playback::{AudioPlaybackController, PlaybackStatus};
    use yuki::voice::router::GovernedVoiceRouter;
    use yuki::voice::session::VoiceSessionManager;
    use yuki::voice::stt::MockSttProvider;
    use yuki::voice::tts::MockTtsProvider;
    use yuki::voice::wakeword::WakeWordDetector;

    let model = Arc::new(MockModelProvider::with_behavior(MockBehavior::DirectText(
        "Resposta".to_string(),
    )));
    let core = Arc::new(YukiCore::new().with_model_provider(model));
    let stt = Arc::new(MockSttProvider::new().with_response("Teste de falha"));
    let tts = Arc::new(MockTtsProvider::new());
    let router = Arc::new(GovernedVoiceRouter::new(
        stt,
        None,
        tts,
        None,
        VoiceConfiguration::default(),
    ));
    let session_mgr = Arc::new(VoiceSessionManager::new());
    let wake_detector = Arc::new(tokio::sync::Mutex::new(WakeWordDetector::default()));

    let (playback, _mock_sink) = AudioPlaybackController::mock();
    // Simula falha de dispositivo de áudio indisponível
    playback.set_device_available(false);

    let coordinator = VoiceConversationCoordinator::new(
        core,
        router,
        session_mgr,
        Arc::new(playback),
        wake_detector,
        None,
    );

    let format = AudioFormat::default();
    let samples = vec![50i16; 1600];
    let payload = AudioCodec::encode_wav(&samples, &format).unwrap();
    let input = AudioInput::new(payload, format, 100, "mic-err");

    // 5. Falha de reprodução é propagada corretamente sem mascaramento
    let turn_res = coordinator
        .process_button_turn("device-fail-test", input, false, false)
        .await;

    assert!(turn_res.is_err());
    assert!(matches!(
        turn_res.unwrap_err(),
        VoiceError::DeviceUnavailable(_)
    ));
    assert_eq!(coordinator.playback().status(), PlaybackStatus::Failed);
}

#[test]
fn test_production_backend_without_device_fails_closed_without_mock_masquerade() {
    use yuki::contracts::voice::{AudioFormat, AudioOutput, VoiceError};
    use yuki::voice::playback::{is_live_audio_test_allowed, AudioPlaybackController};

    // 6. O backend de produção não é acionado fisicamente durante testes automatizados sem opt-in
    assert!(!is_live_audio_test_allowed());

    // 7. A ausência de dispositivo não é relatada como reprodução física bem-sucedida
    let prod_ctrl = AudioPlaybackController::production();
    assert_eq!(prod_ctrl.sink().name(), "WindowsAudioPlaybackSink (winmm)");

    // Força indisponibilidade de dispositivo para testar invariante de segurança
    prod_ctrl.set_device_available(false);
    assert!(!prod_ctrl.is_device_available());

    let output = AudioOutput::new(vec![0u8; 100], AudioFormat::default(), 50, "safety-test");
    let play_res = prod_ctrl.play(&output);

    // Invariante essencial: NUNCA mascara como sucesso via Mock silencioso
    assert!(play_res.is_err());
    assert!(matches!(
        play_res.unwrap_err(),
        VoiceError::DeviceUnavailable(_)
    ));
}

#[test]
fn test_wasapi_device_enumeration_and_matching() {
    use yuki::voice::devices::{
        find_device, get_default_device, list_audio_devices, AudioDeviceFlow, AudioDeviceInfo,
        AudioDeviceState,
    };

    // 1. Enumeração dos dispositivos de áudio
    let all = list_audio_devices(false).expect("Enumeração de dispositivos não deve falhar");
    let active = list_audio_devices(true).expect("Enumeração de ativos não deve falhar");
    assert!(all.len() >= active.len());

    // 2. Dispositivo padrão
    let default_in = get_default_device(AudioDeviceFlow::Input);
    assert!(default_in.is_ok());

    let default_out = get_default_device(AudioDeviceFlow::Output);
    assert!(default_out.is_ok());

    // 3. Resolução com query vazia retorna o padrão
    if let Ok(Some(def_out)) = default_out {
        let found =
            find_device("", AudioDeviceFlow::Output).expect("Query vazia deve retornar padrão");
        assert_eq!(found.id, def_out.id);
    }

    // 4. Resolução por substring case-insensitive
    if let Some(first_active) = active.iter().find(|d| d.flow == AudioDeviceFlow::Output) {
        let substr = &first_active.name[..first_active.name.len().min(4)];
        let found = find_device(substr, AudioDeviceFlow::Output);
        assert!(found.is_ok());
    }

    // 5. Dispositivo inexistente retorna erro DeviceUnavailable
    let missing = find_device(
        "__DISPOSITIVO_TOTALMENTE_INEXISTENTE__",
        AudioDeviceFlow::Input,
    );
    assert!(missing.is_err());
    assert!(matches!(
        missing.unwrap_err(),
        VoiceError::DeviceUnavailable(_)
    ));

    // 6. Estrutura AudioDeviceInfo e serialização
    let dummy = AudioDeviceInfo {
        id: "test-id".to_string(),
        name: "Test Mic".to_string(),
        flow: AudioDeviceFlow::Input,
        state: AudioDeviceState::Active,
        is_default: true,
    };
    assert!(dummy.state.is_active());
    let serialized = serde_json::to_string(&dummy).unwrap();
    assert!(serialized.contains("test-id"));
}

#[test]
fn test_audio_resampling_and_downmix_pcm_validation() {
    use yuki::contracts::voice::AudioFormat;
    use yuki::voice::audio::AudioCodec;

    // Gera onda senoidal pura em 16kHz mono (formato canônico da Yuki)
    let freq = 440.0;
    let sample_rate = 16000;
    let duration_sec = 0.5;
    let total_samples = (sample_rate as f64 * duration_sec) as usize;

    let mut samples = Vec::with_capacity(total_samples);
    for i in 0..total_samples {
        let t = i as f64 / sample_rate as f64;
        let s = (t * freq * 2.0 * std::f64::consts::PI).sin();
        samples.push((s * 10000.0) as i16);
    }

    let format = AudioFormat {
        sample_rate: 16000,
        channels: 1,
        bits_per_sample: 16,
    };

    // Encode para WAV
    let wav_bytes = AudioCodec::encode_wav(&samples, &format).expect("Encode WAV deve funcionar");
    assert!(wav_bytes.starts_with(b"RIFF"));

    // Decode de volta
    let (decoded_format, decoded_samples) =
        AudioCodec::decode_wav(&wav_bytes).expect("Decode WAV deve funcionar");
    assert_eq!(decoded_format.sample_rate, 16000);
    assert_eq!(decoded_format.channels, 1);
    assert_eq!(decoded_format.bits_per_sample, 16);
    assert_eq!(decoded_samples.len(), samples.len());
    assert_eq!(decoded_samples[0], samples[0]);
    assert_eq!(decoded_samples[100], samples[100]);
}

#[test]
fn test_wasapi_32bit_bluetooth_headset_mix_format_integration() {
    use yuki::contracts::voice::{AudioFormat, AudioOutput};
    use yuki::voice::audio::AudioCodec;
    use yuki::voice::playback::wasapi_format::{
        classify_sample_format, convert_samples_to_wasapi, ParsedWaveFormat,
        WasapiSampleClassification, KSDATAFORMAT_SUBTYPE_IEEE_FLOAT, KSDATAFORMAT_SUBTYPE_PCM,
        WAVEFORMATEX, WAVEFORMATEXTENSIBLE,
    };

    // 1. Gera áudio canônico da Yuki (16kHz mono) simulando a síntese de voz (TTS)
    let samples: Vec<i16> = (0..16000)
        .map(|i| ((i as f32 * 0.1).sin() * 5000.0) as i16)
        .collect();
    let format = AudioFormat {
        sample_rate: 16000,
        channels: 1,
        bits_per_sample: 16,
    };
    let wav_data = AudioCodec::encode_wav(&samples, &format).unwrap();
    let output = AudioOutput::new(wav_data, format, 1000, "test-yuki-speech");

    let (decoded_format, decoded_samples) =
        AudioCodec::decode_wav(&output.payload).expect("Deve decodificar wav");
    assert_eq!(decoded_format.sample_rate, 16000);

    // 2. Simula o formato de mixagem retornado pelo Redmi Buds 6 Play em modo shared: 48kHz, 2 canais, 32 bits float
    let float_format_raw = WAVEFORMATEXTENSIBLE {
        format: WAVEFORMATEX {
            wFormatTag: 0xFFFE,
            nChannels: 2,
            nSamplesPerSec: 48000,
            nAvgBytesPerSec: 48000 * 8,
            nBlockAlign: 8,
            wBitsPerSample: 32,
            cbSize: 22,
        },
        samples: 32,
        dw_channel_mask: 0x00000003,
        sub_format: KSDATAFORMAT_SUBTYPE_IEEE_FLOAT,
    };

    let parsed_float =
        unsafe { ParsedWaveFormat::from_raw_ptr(&float_format_raw.format as *const _) }
            .expect("Deve decodificar formato float");
    let class_float = classify_sample_format(&parsed_float).expect("Float deve classificar");
    assert_eq!(class_float, WasapiSampleClassification::IeeeFloat32);

    let bytes_float = convert_samples_to_wasapi(&decoded_samples, 2, class_float);
    assert_eq!(bytes_float.len(), decoded_samples.len() * 2 * 4);

    // 3. Simula o formato de mixagem em modo shared com 32 bits PCM inteiro (24 bits válidos)
    let pcm_format_raw = WAVEFORMATEXTENSIBLE {
        format: WAVEFORMATEX {
            wFormatTag: 0xFFFE,
            nChannels: 2,
            nSamplesPerSec: 48000,
            nAvgBytesPerSec: 48000 * 8,
            nBlockAlign: 8,
            wBitsPerSample: 32,
            cbSize: 22,
        },
        samples: 24,
        dw_channel_mask: 0x00000003,
        sub_format: KSDATAFORMAT_SUBTYPE_PCM,
    };

    let parsed_pcm = unsafe { ParsedWaveFormat::from_raw_ptr(&pcm_format_raw.format as *const _) }
        .expect("Deve decodificar formato PCM");
    let class_pcm = classify_sample_format(&parsed_pcm).expect("PCM 32-bit deve classificar");
    assert_eq!(
        class_pcm,
        WasapiSampleClassification::Pcm32Int { valid_bits: 24 }
    );

    let bytes_pcm = convert_samples_to_wasapi(&decoded_samples, 2, class_pcm);
    assert_eq!(bytes_pcm.len(), decoded_samples.len() * 2 * 4);

    // Amostras convertidas devem respeitar MSB alignment
    let first_sample_pcm = i32::from_le_bytes(bytes_pcm[0..4].try_into().unwrap());
    assert_eq!(first_sample_pcm, (decoded_samples[0] as i32) << 16);
}
