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
    Interrupted,
    Stopped,
}

// ---------------------------------------------------------------------------
// Backend Windows nativo via winmm.dll
// ---------------------------------------------------------------------------

#[cfg(windows)]
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

/// Sink concreto para Windows usando APIs nativas multimídia (winmm.dll).
///
/// Não requer dependências externas pesadas e garante controle direto de
/// hardware, interrupção imediata de reprodução (barge-in) e detecção de dispositivos.
#[derive(Debug)]
pub struct WindowsAudioPlaybackSink {
    current_wav_buffer: Arc<Mutex<Option<Vec<u8>>>>,
}

impl Default for WindowsAudioPlaybackSink {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowsAudioPlaybackSink {
    pub fn new() -> Self {
        Self {
            current_wav_buffer: Arc::new(Mutex::new(None)),
        }
    }

    /// Prepara um buffer de áudio WAV válido a partir do payload fornecido.
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
            if !self.is_device_available() {
                return Err(VoiceError::DeviceUnavailable(
                    "Nenhum dispositivo de reprodução de áudio detectado no sistema operacional (waveOut)"
                        .to_string(),
                ));
            }

            let wav_bytes = self.prepare_wav_bytes(output)?;

            let mut buf_lock = self.current_wav_buffer.lock().unwrap();
            *buf_lock = Some(wav_bytes);

            if let Some(ref buf) = *buf_lock {
                let ptr = buf.as_ptr() as winmm::LPCSTR;
                let flags = winmm::SND_MEMORY | winmm::SND_ASYNC | winmm::SND_NODEFAULT;
                let success = unsafe { winmm::PlaySoundA(ptr, std::ptr::null_mut(), flags) };
                if success == 0 {
                    return Err(VoiceError::PlaybackFailed(
                        "Falha ao acionar winmm::PlaySoundA no subsistema de áudio do Windows"
                            .to_string(),
                    ));
                }
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

/// Controlador seguro de saída e reprodução de áudio da Yuki.
///
/// Garante:
/// - Acoplamento direto com o subsistema de áudio via `AudioPlaybackSink`.
/// - Prevenção estrita de sobreposição acidental de fala.
/// - Interrupção física instantânea na detecção de fala do operador (barge-in).
/// - Tratamento explícito de dispositivo de áudio indisponível ou inacessível.
/// - Limpeza imediata de buffers e recursos na finalização ou cancelamento.
#[derive(Debug, Clone)]
pub struct AudioPlaybackController {
    status: Arc<Mutex<PlaybackStatus>>,
    current_output: Arc<Mutex<Option<AudioOutput>>>,
    device_available_override: Arc<Mutex<Option<bool>>>,
    sink: Arc<dyn AudioPlaybackSink>,
}

impl Default for AudioPlaybackController {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioPlaybackController {
    /// Construtor padrão da plataforma.
    ///
    /// No Windows com hardware de áudio detectado, inicializa com `WindowsAudioPlaybackSink`.
    /// Em ambientes CI ou sem dispositivos de saída, faz fallback limpo para `MockPlaybackSink`.
    pub fn new() -> Self {
        #[cfg(windows)]
        {
            if std::env::var("YUKI_MOCK_AUDIO").is_ok() {
                Self::with_sink(Arc::new(MockPlaybackSink::new()))
            } else {
                let win_sink = WindowsAudioPlaybackSink::new();
                if win_sink.is_device_available() {
                    Self::with_sink(Arc::new(win_sink))
                } else {
                    tracing::warn!(
                        "Nenhum dispositivo de áudio físico detectado. Inicializando MockPlaybackSink como fallback limpo."
                    );
                    Self::with_sink(Arc::new(MockPlaybackSink::new()))
                }
            }
        }
        #[cfg(not(windows))]
        {
            Self::with_sink(Arc::new(MockPlaybackSink::new()))
        }
    }

    /// Construtor parametrizado com sink explícito.
    pub fn with_sink(sink: Arc<dyn AudioPlaybackSink>) -> Self {
        Self {
            status: Arc::new(Mutex::new(PlaybackStatus::Idle)),
            current_output: Arc::new(Mutex::new(None)),
            device_available_override: Arc::new(Mutex::new(None)),
            sink,
        }
    }

    /// Cria uma instância conectada a um `MockPlaybackSink`, retornando o controlador e o mock.
    pub fn mock() -> (Self, Arc<MockPlaybackSink>) {
        let mock_sink = Arc::new(MockPlaybackSink::new());
        let controller = Self::with_sink(mock_sink.clone());
        (controller, mock_sink)
    }

    /// Retorna referência ao sink de reprodução ativo.
    pub fn sink(&self) -> &Arc<dyn AudioPlaybackSink> {
        &self.sink
    }

    /// Define o estado de disponibilidade do dispositivo de áudio (para testes e diagnósticos).
    pub fn set_device_available(&self, available: bool) {
        *self.device_available_override.lock().unwrap() = Some(available);
    }

    /// Verifica se há dispositivo de áudio disponível, respeitando sobrescrições manuais.
    pub fn is_device_available(&self) -> bool {
        if let Some(forced) = *self.device_available_override.lock().unwrap() {
            return forced;
        }
        self.sink.is_device_available()
    }

    /// Retorna o estado atual da reprodução.
    pub fn status(&self) -> PlaybackStatus {
        *self.status.lock().unwrap()
    }

    /// Envia um buffer de áudio sintetizado para reprodução física.
    /// Rejeita se o dispositivo estiver indisponível ou interrompe reprodução prévia em curso.
    pub fn play(&self, output: &AudioOutput) -> Result<(), VoiceError> {
        if !self.is_device_available() {
            return Err(VoiceError::DeviceUnavailable(
                "Dispositivo de reprodução de áudio (alto-falante) não detectado ou inacessível"
                    .to_string(),
            ));
        }

        // Interrompe reprodução prévia no sink se já estiver tocando
        if self.status() == PlaybackStatus::Playing {
            let _ = self.sink.interrupt();
        }

        // Envia para o sink físico/mock
        self.sink.play(output)?;

        let mut status_lock = self.status.lock().unwrap();
        let mut out_lock = self.current_output.lock().unwrap();

        *out_lock = Some(output.clone());
        *status_lock = PlaybackStatus::Playing;

        Ok(())
    }

    /// Interrompe imediatamente a reprodução em andamento (ex: barge-in do usuário).
    pub fn interrupt(&self) -> Result<(), VoiceError> {
        let _ = self.sink.interrupt();

        let mut status_lock = self.status.lock().unwrap();
        let mut out_lock = self.current_output.lock().unwrap();

        *status_lock = PlaybackStatus::Interrupted;
        *out_lock = None; // Limpeza imediata do buffer

        Ok(())
    }

    /// Encerra normalmente a reprodução e retorna o estado para parado.
    pub fn stop(&self) {
        let _ = self.sink.interrupt();

        let mut status_lock = self.status.lock().unwrap();
        let mut out_lock = self.current_output.lock().unwrap();

        *status_lock = PlaybackStatus::Stopped;
        *out_lock = None;
    }

    /// Reseta o controlador para Idle.
    pub fn reset(&self) {
        let _ = self.sink.interrupt();

        let mut status_lock = self.status.lock().unwrap();
        let mut out_lock = self.current_output.lock().unwrap();

        *status_lock = PlaybackStatus::Idle;
        *out_lock = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::voice::AudioFormat;

    #[test]
    fn test_mock_playback_sink_lifecycle() {
        let sink = MockPlaybackSink::new();
        assert!(sink.is_device_available());
        assert_eq!(sink.played_count(), 0);

        let output = AudioOutput::new(vec![0u8; 32], AudioFormat::default(), 1, "test");
        assert!(sink.play(&output).is_ok());
        assert_eq!(sink.played_count(), 1);

        assert!(sink.interrupt().is_ok());
        assert_eq!(sink.interrupted_count(), 1);

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

    #[cfg(windows)]
    #[test]
    fn test_windows_sink_device_detection() {
        let sink = WindowsAudioPlaybackSink::new();
        // Em máquina com placa de som, deve detectar dispositivo
        let available = sink.is_device_available();
        println!("Windows audio sink device available: {}", available);
    }
}
