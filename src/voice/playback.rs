use crate::contracts::voice::{AudioOutput, VoiceError};
use crate::voice::audio::AudioCodec;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// Trait que abstrai o sink físico ou simulado de reprodução de áudio da Yuki.
pub trait AudioPlaybackSink: Send + Sync + std::fmt::Debug {
    /// Envia o buffer de áudio para reprodução.
    fn play(&self, output: &AudioOutput) -> Result<(), VoiceError>;

    /// Interrompe imediatamente qualquer reprodução física ativa no hardware.
    fn interrupt(&self) -> Result<(), VoiceError>;

    /// Verifica se há dispositivo de saída disponível no sistema operacional.
    fn is_device_available(&self) -> bool;

    /// Nome descritivo da implementação de sink.
    fn name(&self) -> &str;
}

/// Estado atual da reprodução de áudio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackStatus {
    Idle,
    Playing,
    Completed,
    Interrupted,
    Stopped,
    Failed,
}

/// Política de autorização para emissão sonora física no subsistema de áudio.
///
/// Invariante de Segurança Acústica:
/// A execução da suíte de testes (`cargo test`) DEVE ser 100% silenciosa por construção,
/// sem depender de inspeção de caminhos de arquivos executáveis (`deps`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioOutputPolicy {
    /// Produção: emite áudio diretamente no hardware via API nativa do sistema operacional (winmm::PlaySoundA).
    Production,
    /// Teste com opt-in explícito: só emite áudio físico no hardware se a variável de ambiente `YUKI_LIVE_AUDIO_TESTS=1`
    /// estiver presente. Sem a variável, valida e armazena o buffer de áudio na memória, mantendo silêncio absoluto.
    OptInTest,
    /// Silenciado por construção: nunca aciona o hardware de áudio; valida o buffer e o mantém na memória.
    Silenced,
}

// ---------------------------------------------------------------------------
// Backend Windows nativo via winmm.dll
// ---------------------------------------------------------------------------

#[cfg(windows)]
#[allow(clippy::upper_case_acronyms)]
mod winmm {
    use std::os::raw::{c_char, c_void};
    pub type BOOL = i32;
    pub type DWORD = u32;
    pub type UINT = u32;
    pub type LPCSTR = *const c_char;
    pub type HMODULE = *mut c_void;

    pub const SND_ASYNC: DWORD = 0x0001;
    pub const SND_NODEFAULT: DWORD = 0x0002;
    pub const SND_MEMORY: DWORD = 0x0004;

    #[link(name = "winmm")]
    extern "system" {
        pub fn PlaySoundA(pszSound: LPCSTR, hmod: HMODULE, fdwSound: DWORD) -> BOOL;
        pub fn waveOutGetNumDevs() -> UINT;
    }
}

/// Verifica se a execução atual possui autorização explícita para emissão de áudio físico em testes.
///
/// Invariante de Segurança Acústica:
/// A execução de testes (`cargo test`) DEVE ser 100% silenciosa por padrão.
/// Emissão física em alto-falantes só é permitida com opt-in explícito via:
/// `YUKI_LIVE_AUDIO_TESTS=1`.
pub fn is_live_audio_test_allowed() -> bool {
    std::env::var("YUKI_LIVE_AUDIO_TESTS").as_deref() == Ok("1")
}

/// Sink concreto para Windows usando APIs nativas multimídia (winmm.dll).
///
/// ARQUITETURA E GERENCIAMENTO DE MEMÓRIA (WinMM Safety Invariants):
/// 1. **Ciclo de Vida de Memória do `PlaySoundA`**:
///    Quando invocado com `SND_MEMORY | SND_ASYNC`, o driver do Windows lê o ponteiro de memória
///    de forma concorrente e assíncrona. O Windows NÃO duplica o buffer internamente na API;
///    ele faz streaming diretamente da memória do processo.
/// 2. **Buffer Pinned em Memória**:
///    Para prevenir corrupção de memória e use-after-free, o `WindowsAudioPlaybackSink` armazena
///    uma referência `Arc<Vec<u8>>` em `current_wav_buffer` durante todo o período em que o som
///    pode estar sendo tocado pelo driver de áudio do kernel.
/// 3. **Interrupção Atômica e Implementação de `Drop`**:
///    Ao substituir um buffer ou destruir a instância (`Drop`), invocamos `PlaySoundA(NULL, NULL, 0)`.
///    Isso força a interrupção imediata da leitura do hardware antes que o ponteiro anterior seja liberado.
/// 4. **Conclusão Estimada vs Confirmação Física**:
///    A API clássica `PlaySoundA` não fornece callbacks assíncronos de finalização (como `waveOutWrite`
///    com eventos de cabeçalho). O `AudioPlaybackController` rastreia o ciclo de vida via temporizador
///    baseado no número de amostras PCM e sample rate (`is_completion_estimated() -> true`).
#[derive(Debug)]
pub struct WindowsAudioPlaybackSink {
    policy: AudioOutputPolicy,
    current_wav_buffer: Arc<Mutex<Option<Arc<Vec<u8>>>>>,
}

impl Drop for WindowsAudioPlaybackSink {
    fn drop(&mut self) {
        #[cfg(windows)]
        {
            unsafe {
                winmm::PlaySoundA(std::ptr::null(), std::ptr::null_mut(), 0);
            }
        }
    }
}

impl Default for WindowsAudioPlaybackSink {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowsAudioPlaybackSink {
    /// Construtor seguro padrão: utiliza `AudioOutputPolicy::OptInTest`.
    /// Garante que testes que instanciem este sink diretamente não emitam som sem autorização explícita.
    pub fn new() -> Self {
        Self::with_policy(AudioOutputPolicy::OptInTest)
    }

    /// Construtor de produção: emite áudio físico diretamente no hardware sem depender de variáveis de teste.
    pub fn production() -> Self {
        Self::with_policy(AudioOutputPolicy::Production)
    }

    /// Construtor para testes com opt-in: só emite som se `YUKI_LIVE_AUDIO_TESTS=1`.
    pub fn opt_in_test() -> Self {
        Self::with_policy(AudioOutputPolicy::OptInTest)
    }

    /// Construtor estritamente silenciado: nunca emite som no hardware.
    pub fn silenced() -> Self {
        Self::with_policy(AudioOutputPolicy::Silenced)
    }

    /// Construtor parametrizado com a política desejada.
    pub fn with_policy(policy: AudioOutputPolicy) -> Self {
        Self {
            policy,
            current_wav_buffer: Arc::new(Mutex::new(None)),
        }
    }

    /// Retorna a política de saída de áudio ativa.
    pub fn policy(&self) -> AudioOutputPolicy {
        self.policy
    }

    /// Prepara um buffer de áudio WAV válido a partir do payload fornecido.
    #[allow(clippy::chunks_exact_to_as_chunks, clippy::manual_is_multiple_of)]
    fn prepare_wav_bytes(&self, output: &AudioOutput) -> Result<Vec<u8>, VoiceError> {
        if output.payload.starts_with(b"RIFF") && output.payload.len() >= 44 {
            // Já é um contêiner WAV válido
            Ok(output.payload.clone())
        } else if output.payload.len() % 2 == 0 {
            // Trata como PCM 16-bit linear e envelopa em WAV canônico
            let samples: Vec<i16> = output
                .payload
                .chunks_exact(2)
                .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
                .collect();
            AudioCodec::encode_wav(&samples, &output.format)
        } else {
            Err(VoiceError::InvalidAudioFormat(
                "Buffer de áudio inválido: tamanho ímpar incompatível com PCM 16-bit".to_string(),
            ))
        }
    }
}

impl AudioPlaybackSink for WindowsAudioPlaybackSink {
    fn play(&self, output: &AudioOutput) -> Result<(), VoiceError> {
        #[cfg(windows)]
        {
            let should_emit_physical = match self.policy {
                AudioOutputPolicy::Production => true,
                AudioOutputPolicy::OptInTest => is_live_audio_test_allowed(),
                AudioOutputPolicy::Silenced => false,
            };

            if !should_emit_physical {
                // Em modo não-físico (teste sem opt-in ou silenciado), valida a integridade
                // do buffer WAV e retém o buffer em memória para inspeção, mantendo silêncio absoluto.
                let wav_bytes = Arc::new(self.prepare_wav_bytes(output)?);
                let mut buf_lock = self.current_wav_buffer.lock().unwrap();
                *buf_lock = Some(wav_bytes);
                return Ok(());
            }

            if !self.is_device_available() {
                return Err(VoiceError::DeviceUnavailable(
                    "Nenhum dispositivo de reprodução de áudio detectado no sistema operacional (waveOut)"
                        .to_string(),
                ));
            }

            let wav_bytes = Arc::new(self.prepare_wav_bytes(output)?);

            let mut buf_lock = self.current_wav_buffer.lock().unwrap();
            // Interrompe reprodução prévia no driver antes de registrar novo buffer
            unsafe {
                winmm::PlaySoundA(std::ptr::null(), std::ptr::null_mut(), 0);
            }
            *buf_lock = Some(wav_bytes.clone());

            let ptr = wav_bytes.as_ptr() as winmm::LPCSTR;
            let flags = winmm::SND_MEMORY | winmm::SND_ASYNC | winmm::SND_NODEFAULT;
            let success = unsafe { winmm::PlaySoundA(ptr, std::ptr::null_mut(), flags) };
            if success == 0 {
                *buf_lock = None;
                return Err(VoiceError::PlaybackFailed(
                    "Falha ao acionar winmm::PlaySoundA no subsistema de áudio do Windows"
                        .to_string(),
                ));
            }
            Ok(())
        }
        #[cfg(not(windows))]
        {
            let _ = output;
            Err(VoiceError::PlaybackFailed(
                "WindowsAudioPlaybackSink é suportado apenas em sistemas Windows".to_string(),
            ))
        }
    }

    fn interrupt(&self) -> Result<(), VoiceError> {
        #[cfg(windows)]
        {
            // Interrompe imediatamente qualquer som tocando de forma assíncrona
            unsafe {
                winmm::PlaySoundA(std::ptr::null(), std::ptr::null_mut(), 0);
            }
            let mut buf_lock = self.current_wav_buffer.lock().unwrap();
            *buf_lock = None;
            Ok(())
        }
        #[cfg(not(windows))]
        {
            Ok(())
        }
    }

    fn is_device_available(&self) -> bool {
        #[cfg(windows)]
        {
            let devs = unsafe { winmm::waveOutGetNumDevs() };
            devs > 0
        }
        #[cfg(not(windows))]
        {
            false
        }
    }

    fn name(&self) -> &str {
        "WindowsAudioPlaybackSink (winmm)"
    }
}

// ---------------------------------------------------------------------------
// Sink Mock Observável para Testes e CI Headless
// ---------------------------------------------------------------------------

/// Sink observável em memória para validação determinística e ambientes CI sem placa de som.
#[derive(Debug)]
pub struct MockPlaybackSink {
    played_outputs: Arc<Mutex<Vec<AudioOutput>>>,
    interrupted_count: Arc<AtomicUsize>,
    device_available: Arc<AtomicBool>,
    interrupt_fails: Arc<AtomicBool>,
}

impl Default for MockPlaybackSink {
    fn default() -> Self {
        Self::new()
    }
}

impl MockPlaybackSink {
    pub fn new() -> Self {
        Self {
            played_outputs: Arc::new(Mutex::new(Vec::new())),
            interrupted_count: Arc::new(AtomicUsize::new(0)),
            device_available: Arc::new(AtomicBool::new(true)),
            interrupt_fails: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn with_device_available(available: bool) -> Self {
        let sink = Self::new();
        sink.set_device_available(available);
        sink
    }

    pub fn set_device_available(&self, available: bool) {
        self.device_available.store(available, Ordering::SeqCst);
    }

    pub fn set_interrupt_fails(&self, fails: bool) {
        self.interrupt_fails.store(fails, Ordering::SeqCst);
    }

    pub fn is_interrupt_failing(&self) -> bool {
        self.interrupt_fails.load(Ordering::SeqCst)
    }

    pub fn played_count(&self) -> usize {
        self.played_outputs.lock().unwrap().len()
    }

    pub fn last_played(&self) -> Option<AudioOutput> {
        self.played_outputs.lock().unwrap().last().cloned()
    }

    pub fn interrupted_count(&self) -> usize {
        self.interrupted_count.load(Ordering::SeqCst)
    }

    pub fn clear(&self) {
        self.played_outputs.lock().unwrap().clear();
        self.interrupted_count.store(0, Ordering::SeqCst);
        self.interrupt_fails.store(false, Ordering::SeqCst);
    }
}

impl AudioPlaybackSink for MockPlaybackSink {
    fn play(&self, output: &AudioOutput) -> Result<(), VoiceError> {
        if !self.is_device_available() {
            return Err(VoiceError::DeviceUnavailable(
                "Dispositivo de reprodução mock configurado como indisponível".to_string(),
            ));
        }
        self.played_outputs.lock().unwrap().push(output.clone());
        Ok(())
    }

    fn interrupt(&self) -> Result<(), VoiceError> {
        if self.interrupt_fails.load(Ordering::SeqCst) {
            return Err(VoiceError::PlaybackFailed(
                "Falha simulada na interrupção do sink de áudio".to_string(),
            ));
        }
        self.interrupted_count.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn is_device_available(&self) -> bool {
        self.device_available.load(Ordering::SeqCst)
    }

    fn name(&self) -> &str {
        "MockPlaybackSink"
    }
}

// ---------------------------------------------------------------------------
// AudioPlaybackController
// ---------------------------------------------------------------------------

/// Estado interno encapsulado sob único Mutex para prevenir deadlocks e condições de corrida entre campos.
#[derive(Debug)]
struct PlaybackStateInner {
    status: PlaybackStatus,
    current_output: Option<AudioOutput>,
    device_available_override: Option<bool>,
    playback_timing: Option<(std::time::Instant, u64)>,
}

impl Default for PlaybackStateInner {
    fn default() -> Self {
        Self {
            status: PlaybackStatus::Idle,
            current_output: None,
            device_available_override: None,
            playback_timing: None,
        }
    }
}

/// Controlador seguro de saída e reprodução de áudio da Yuki.
///
/// Garante:
/// - Acoplamento direto com o subsistema de áudio via `AudioPlaybackSink`.
/// - Prevenção estrita de sobreposição acidental de fala.
/// - Interrupção física instantânea na detecção de fala do operador (barge-in).
/// - Propagação rigorosa de falhas em interrupção com transição para `PlaybackStatus::Failed`.
/// - Rejeição segura de novo áudio caso a interrupção da reprodução anterior falhe.
/// - Tratamento explícito de dispositivo de áudio indisponível ou inacessível.
/// - Limpeza imediata de buffers e recursos na finalização ou cancelamento.
/// - Proteção de concorrência com mutex único atômico (`PlaybackStateInner`).
/// - Silêncio absoluto por padrão na suíte de testes (`MockPlaybackSink` por construção).
#[derive(Debug, Clone)]
pub struct AudioPlaybackController {
    state: Arc<Mutex<PlaybackStateInner>>,
    sink: Arc<dyn AudioPlaybackSink>,
}

impl Default for AudioPlaybackController {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioPlaybackController {
    /// Construtor padrão seguro: inicializa com `MockPlaybackSink`.
    ///
    /// Por construção, qualquer chamada a `new()` ou `default()` em testes ou instâncias
    /// genéricas é 100% silenciosa e não depende de inspeção de caminho de arquivo executável.
    pub fn new() -> Self {
        Self::with_sink(Arc::new(MockPlaybackSink::new()))
    }

    /// Construtor para ambiente de produção: conecta diretamente ao subsistema nativo do sistema operacional.
    ///
    /// No Windows com hardware de áudio detectado, inicializa com `WindowsAudioPlaybackSink::production()`.
    /// Se nenhum dispositivo físico for detectado, realiza fallback limpo para `MockPlaybackSink`.
    pub fn production() -> Self {
        #[cfg(windows)]
        {
            let win_sink = WindowsAudioPlaybackSink::production();
            if win_sink.is_device_available() {
                Self::with_sink(Arc::new(win_sink))
            } else {
                tracing::warn!(
                    "Nenhum dispositivo de áudio físico detectado. Inicializando MockPlaybackSink como fallback limpo."
                );
                Self::with_sink(Arc::new(MockPlaybackSink::new()))
            }
        }
        #[cfg(not(windows))]
        {
            tracing::warn!(
                "Sistema operacional não-Windows. Inicializando MockPlaybackSink como fallback limpo."
            );
            Self::with_sink(Arc::new(MockPlaybackSink::new()))
        }
    }

    /// Construtor para testes ao vivo (opt-in explícito via `YUKI_LIVE_AUDIO_TESTS=1`).
    pub fn live_test() -> Self {
        #[cfg(windows)]
        {
            Self::with_sink(Arc::new(WindowsAudioPlaybackSink::opt_in_test()))
        }
        #[cfg(not(windows))]
        {
            Self::with_sink(Arc::new(MockPlaybackSink::new()))
        }
    }

    /// Construtor parametrizado com sink explícito.
    pub fn with_sink(sink: Arc<dyn AudioPlaybackSink>) -> Self {
        Self {
            state: Arc::new(Mutex::new(PlaybackStateInner::default())),
            sink,
        }
    }

    /// Cria uma instância conectada a um `MockPlaybackSink`, retornando o controlador e o mock observável.
    pub fn mock() -> (Self, Arc<MockPlaybackSink>) {
        let mock_sink = Arc::new(MockPlaybackSink::new());
        let controller = Self::with_sink(mock_sink.clone());
        (controller, mock_sink)
    }

    /// Construtor conveniente para testes determinísticos.
    pub fn for_test() -> Self {
        Self::mock().0
    }

    /// Retorna referência ao sink de reprodução ativo.
    pub fn sink(&self) -> &Arc<dyn AudioPlaybackSink> {
        &self.sink
    }

    /// Define o estado de disponibilidade do dispositivo de áudio (para testes e diagnósticos).
    pub fn set_device_available(&self, available: bool) {
        let mut state = self.state.lock().unwrap();
        state.device_available_override = Some(available);
    }

    /// Verifica se há dispositivo de áudio disponível, respeitando sobrescrições manuais.
    pub fn is_device_available(&self) -> bool {
        let state = self.state.lock().unwrap();
        if let Some(forced) = state.device_available_override {
            return forced;
        }
        drop(state);
        self.sink.is_device_available()
    }

    /// Retorna o estado atual da reprodução, atualizando atomicamente para Completed se a duração do áudio tiver expirado.
    pub fn status(&self) -> PlaybackStatus {
        let mut state = self.state.lock().unwrap();
        if state.status == PlaybackStatus::Playing {
            if let Some((start_instant, duration_ms)) = state.playback_timing {
                if start_instant.elapsed().as_millis() as u64 >= duration_ms {
                    state.status = PlaybackStatus::Completed;
                    state.current_output = None;
                }
            }
        }
        state.status
    }

    /// Retorna cópia do áudio atualmente em reprodução, se houver.
    pub fn current_output(&self) -> Option<AudioOutput> {
        self.state.lock().unwrap().current_output.clone()
    }

    /// Declara explicitamente se a transição para `PlaybackStatus::Completed` é calculada com base
    /// na estimativa de duração dos quadros PCM e taxa de amostragem, em vez de callback físico do driver.
    pub fn is_completion_estimated(&self) -> bool {
        true
    }

    /// Envia um buffer de áudio sintetizado para reprodução.
    ///
    /// Invariantes de Segurança e Tratamento de Erro:
    /// - Se o dispositivo estiver indisponível, falha imediatamente com `DeviceUnavailable`.
    /// - Se houver reprodução ativa (`Playing`), tenta interrompê-la antes de iniciar a nova.
    /// - SE A INTERRUPÇÃO PRÉVIA FALHAR:
    ///   - O novo áudio NÃO é reproduzido sobre o áudio em falha.
    ///   - O estado do controlador transita para `PlaybackStatus::Failed`.
    ///   - Os recursos de timing e o buffer atual são limpos.
    ///   - O erro retornado pelo sink é registrado em `tracing::error!` e propagado ao chamador.
    /// - Se o envio para o sink falhar, transita para `Failed`, limpa buffers e retorna o erro.
    pub fn play(&self, output: &AudioOutput) -> Result<(), VoiceError> {
        if !self.is_device_available() {
            let mut state = self.state.lock().unwrap();
            state.status = PlaybackStatus::Failed;
            return Err(VoiceError::DeviceUnavailable(
                "Dispositivo de reprodução de áudio (alto-falante) não detectado ou inacessível"
                    .to_string(),
            ));
        }

        // Interrompe reprodução prévia no sink se já estiver tocando
        if self.status() == PlaybackStatus::Playing {
            if let Err(e) = self.sink.interrupt() {
                tracing::error!(
                    error = %e,
                    "Falha ao interromper reprodução ativa antes de iniciar novo áudio"
                );
                let mut state = self.state.lock().unwrap();
                state.status = PlaybackStatus::Failed;
                state.playback_timing = None;
                state.current_output = None;
                return Err(e);
            }
        }

        // Envia para o sink físico/mock
        if let Err(e) = self.sink.play(output) {
            tracing::error!(
                error = %e,
                "Falha ao enviar buffer para o sink de reprodução de áudio"
            );
            let mut state = self.state.lock().unwrap();
            state.status = PlaybackStatus::Failed;
            state.playback_timing = None;
            state.current_output = None;
            return Err(e);
        }

        let duration_ms = if output.duration_ms > 0 {
            output.duration_ms
        } else if output.format.sample_rate > 0 {
            ((output.payload.len() as u64) * 1000)
                / (output.format.sample_rate as u64 * output.format.channels as u64 * 2)
        } else {
            100
        };

        let mut state = self.state.lock().unwrap();
        state.playback_timing = Some((std::time::Instant::now(), duration_ms));
        state.current_output = Some(output.clone());
        state.status = PlaybackStatus::Playing;

        Ok(())
    }

    /// Interrompe imediatamente a reprodução em andamento (ex: barge-in do usuário).
    ///
    /// Se a interrupção no sink falhar:
    /// - O estado transita para `PlaybackStatus::Failed`.
    /// - Os recursos de timing e buffers são limpos.
    /// - O erro é registrado em `tracing::error!` e retornado formalmente.
    pub fn interrupt(&self) -> Result<(), VoiceError> {
        if let Err(e) = self.sink.interrupt() {
            tracing::error!(
                error = %e,
                "Falha ao interromper sink de reprodução de áudio"
            );
            let mut state = self.state.lock().unwrap();
            state.status = PlaybackStatus::Failed;
            state.playback_timing = None;
            state.current_output = None;
            return Err(e);
        }

        let mut state = self.state.lock().unwrap();
        state.status = PlaybackStatus::Interrupted;
        state.playback_timing = None;
        state.current_output = None;

        Ok(())
    }

    /// Encerra a reprodução e coloca o controlador no estado Stopped.
    ///
    /// Propaga falhas do sink de áudio, transitando para `PlaybackStatus::Failed` em caso de erro.
    pub fn stop(&self) -> Result<(), VoiceError> {
        if let Err(e) = self.sink.interrupt() {
            tracing::error!(
                error = %e,
                "Falha ao interromper sink de reprodução durante stop()"
            );
            let mut state = self.state.lock().unwrap();
            state.status = PlaybackStatus::Failed;
            state.playback_timing = None;
            state.current_output = None;
            return Err(e);
        }

        let mut state = self.state.lock().unwrap();
        state.status = PlaybackStatus::Stopped;
        state.playback_timing = None;
        state.current_output = None;

        Ok(())
    }

    /// Reseta o controlador para o estado inicial Idle.
    ///
    /// Propaga falhas do sink de áudio, transitando para `PlaybackStatus::Failed` em caso de erro.
    pub fn reset(&self) -> Result<(), VoiceError> {
        if let Err(e) = self.sink.interrupt() {
            tracing::error!(
                error = %e,
                "Falha ao interromper sink de reprodução durante reset()"
            );
            let mut state = self.state.lock().unwrap();
            state.status = PlaybackStatus::Failed;
            state.playback_timing = None;
            state.current_output = None;
            return Err(e);
        }

        let mut state = self.state.lock().unwrap();
        state.status = PlaybackStatus::Idle;
        state.playback_timing = None;
        state.current_output = None;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::voice::AudioFormat;

    #[test]
    fn test_mock_playback_sink_lifecycle_and_interrupt_failure() {
        let sink = MockPlaybackSink::new();
        assert!(sink.is_device_available());
        assert_eq!(sink.played_count(), 0);
        assert!(!sink.is_interrupt_failing());

        let output = AudioOutput::new(vec![0u8; 32], AudioFormat::default(), 1, "test");
        assert!(sink.play(&output).is_ok());
        assert_eq!(sink.played_count(), 1);

        assert!(sink.interrupt().is_ok());
        assert_eq!(sink.interrupted_count(), 1);

        // Simula falha na interrupção do sink
        sink.set_interrupt_fails(true);
        assert!(sink.is_interrupt_failing());
        let int_err = sink.interrupt();
        assert!(int_err.is_err());
        assert!(matches!(
            int_err.unwrap_err(),
            VoiceError::PlaybackFailed(_)
        ));
        assert_eq!(sink.interrupted_count(), 1); // Não incrementou por conta da falha

        // Recupera sucesso
        sink.set_interrupt_fails(false);
        assert!(sink.interrupt().is_ok());
        assert_eq!(sink.interrupted_count(), 2);

        sink.set_device_available(false);
        assert!(!sink.is_device_available());
        assert!(matches!(
            sink.play(&output),
            Err(VoiceError::DeviceUnavailable(_))
        ));
    }

    #[test]
    fn test_playback_controller_mock_mode() {
        let (controller, mock) = AudioPlaybackController::mock();
        assert_eq!(controller.status(), PlaybackStatus::Idle);

        let output = AudioOutput::new(vec![0u8; 100], AudioFormat::default(), 50, "test");
        assert!(controller.play(&output).is_ok());
        assert_eq!(controller.status(), PlaybackStatus::Playing);
        assert_eq!(mock.played_count(), 1);

        assert!(controller.interrupt().is_ok());
        assert_eq!(controller.status(), PlaybackStatus::Interrupted);
        assert_eq!(mock.interrupted_count(), 1);
    }

    #[test]
    fn test_playback_controller_interrupt_error_propagation_and_cleanup() {
        let (controller, mock) = AudioPlaybackController::mock();
        let output = AudioOutput::new(vec![0u8; 100], AudioFormat::default(), 1000, "test-err");

        assert!(controller.play(&output).is_ok());
        assert_eq!(controller.status(), PlaybackStatus::Playing);
        assert!(controller.current_output().is_some());

        // Força falha na interrupção
        mock.set_interrupt_fails(true);
        let int_res = controller.interrupt();
        assert!(int_res.is_err());
        assert!(matches!(
            int_res.unwrap_err(),
            VoiceError::PlaybackFailed(_)
        ));

        // Invariante: estado transiciona para Failed e buffers são limpos
        assert_eq!(controller.status(), PlaybackStatus::Failed);
        assert_eq!(controller.current_output(), None);
    }

    #[test]
    fn test_playback_controller_play_over_playing_fails_if_interrupt_fails() {
        let (controller, mock) = AudioPlaybackController::mock();
        let output1 = AudioOutput::new(vec![0u8; 100], AudioFormat::default(), 5000, "audio-1");
        let output2 = AudioOutput::new(vec![0u8; 100], AudioFormat::default(), 5000, "audio-2");

        // Inicia áudio 1
        assert!(controller.play(&output1).is_ok());
        assert_eq!(controller.status(), PlaybackStatus::Playing);
        assert_eq!(mock.played_count(), 1);

        // Simula que o sink de áudio falha ao tentar interromper o áudio 1
        mock.set_interrupt_fails(true);

        // Tentar tocar áudio 2 DEVE falhar imediatamente sem tocar o segundo áudio
        let play2_res = controller.play(&output2);
        assert!(play2_res.is_err());
        assert!(matches!(
            play2_res.unwrap_err(),
            VoiceError::PlaybackFailed(_)
        ));

        // Áudio 2 não foi enviado ao sink
        assert_eq!(mock.played_count(), 1);
        assert_eq!(controller.status(), PlaybackStatus::Failed);
        assert_eq!(controller.current_output(), None);
    }

    #[test]
    fn test_playback_controller_stop_and_reset_error_propagation() {
        let (controller, mock) = AudioPlaybackController::mock();
        let output = AudioOutput::new(vec![0u8; 100], AudioFormat::default(), 1000, "test-stop");

        assert!(controller.play(&output).is_ok());
        mock.set_interrupt_fails(true);

        // Stop com falha
        let stop_err = controller.stop();
        assert!(stop_err.is_err());
        assert_eq!(controller.status(), PlaybackStatus::Failed);
        assert_eq!(controller.current_output(), None);

        // Reset com falha
        let reset_err = controller.reset();
        assert!(reset_err.is_err());
        assert_eq!(controller.status(), PlaybackStatus::Failed);

        // Com falha desativada, stop e reset retornam Ok
        mock.set_interrupt_fails(false);
        assert!(controller.stop().is_ok());
        assert_eq!(controller.status(), PlaybackStatus::Stopped);
        assert!(controller.reset().is_ok());
        assert_eq!(controller.status(), PlaybackStatus::Idle);
    }

    #[test]
    fn test_playback_controller_silent_by_construction_default() {
        // Construtor padrão inicializa MockPlaybackSink por construção
        let controller = AudioPlaybackController::new();
        assert_eq!(controller.sink().name(), "MockPlaybackSink");
        assert_eq!(controller.status(), PlaybackStatus::Idle);

        let for_test_ctrl = AudioPlaybackController::for_test();
        assert_eq!(for_test_ctrl.sink().name(), "MockPlaybackSink");
    }

    #[test]
    fn test_playback_controller_completion_estimation_and_output() {
        let (controller, _) = AudioPlaybackController::mock();
        assert!(controller.is_completion_estimated());
        assert_eq!(controller.current_output(), None);

        let output = AudioOutput::new(vec![0u8; 100], AudioFormat::default(), 50, "test");
        assert!(controller.play(&output).is_ok());
        assert_eq!(controller.current_output(), Some(output));
        assert!(controller.stop().is_ok());
        assert_eq!(controller.current_output(), None);
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_sink_device_detection_and_safety_policies() {
        let sink = WindowsAudioPlaybackSink::opt_in_test();
        assert_eq!(sink.policy(), AudioOutputPolicy::OptInTest);
        let available = sink.is_device_available();
        println!("Windows audio sink device available: {}", available);

        // Sem a variável YUKI_LIVE_AUDIO_TESTS=1, a execução em teste é silenciada com segurança
        let output = AudioOutput::new(vec![0u8; 320], AudioFormat::default(), 10, "test-safety");
        let play_res = sink.play(&output);
        assert!(play_res.is_ok(), "Deve validar buffer sem erro");
        assert!(sink.interrupt().is_ok());

        // Sink silenciado por construção
        let silenced_sink = WindowsAudioPlaybackSink::silenced();
        assert_eq!(silenced_sink.policy(), AudioOutputPolicy::Silenced);
        assert!(silenced_sink.play(&output).is_ok());
        assert!(silenced_sink.interrupt().is_ok());
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "Requer autorização física explícita no hardware de áudio via YUKI_LIVE_AUDIO_TESTS=1"]
    fn test_windows_sink_live_opt_in_physical_playback() {
        if !is_live_audio_test_allowed() {
            println!("Pulando teste físico ao vivo: YUKI_LIVE_AUDIO_TESTS não configurada");
            return;
        }

        let sink = WindowsAudioPlaybackSink::production();
        if !sink.is_device_available() {
            println!("Nenhum dispositivo de áudio disponível para teste ao vivo");
            return;
        }

        let tone_samples: Vec<i16> = (0..8000)
            .map(|i| {
                let freq = 440.0;
                let sample_rate = 16000.0;
                ((i as f32 * freq * 2.0 * std::f32::consts::PI / sample_rate).sin() * 8000.0) as i16
            })
            .collect();
        let wav_data = AudioCodec::encode_wav(&tone_samples, &AudioFormat::default()).unwrap();
        let output = AudioOutput::new(wav_data, AudioFormat::default(), 500, "tone-live");

        assert!(sink.play(&output).is_ok());
        std::thread::sleep(std::time::Duration::from_millis(100));
        assert!(sink.interrupt().is_ok());
    }
}
