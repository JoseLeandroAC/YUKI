use crate::contracts::identifiers::generate_unique_id;
use crate::contracts::voice::{AudioFormat, AudioInput, VoiceError};
use crate::voice::audio::AudioCodec;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Trait abstrata para captura de áudio a partir do microfone físico ou simulado.
pub trait AudioCaptureBackend: Send + Sync + std::fmt::Debug {
    /// Inicia a captura de áudio no backend.
    fn start_capture(&self) -> Result<(), VoiceError>;

    /// Conclui a captura de áudio e retorna os dados capturados como `AudioInput` canônico.
    fn stop_capture(&self) -> Result<AudioInput, VoiceError>;

    /// Cancela a captura em andamento, descartando quaisquer amostras pendentes.
    fn cancel_capture(&self) -> Result<(), VoiceError>;

    /// Verifica se há dispositivo de captura (microfone) disponível e acessível no sistema.
    fn is_device_available(&self) -> bool;

    /// Nome identificador da implementação de captura.
    fn name(&self) -> &str;
}

// ---------------------------------------------------------------------------
// Backend de Captura Windows Nativo via MCI / WinMM
// ---------------------------------------------------------------------------

#[cfg(windows)]
#[allow(clippy::upper_case_acronyms)]
mod winmm {
    use std::os::raw::{c_char, c_void};
    pub type UINT = u32;
    pub type LPCSTR = *const c_char;
    pub type LPSTR = *mut c_char;
    pub type HWND = *mut c_void;

    #[link(name = "winmm")]
    extern "system" {
        pub fn mciSendStringA(
            lpstrCommand: LPCSTR,
            lpstrReturnString: LPSTR,
            uReturnLength: UINT,
            hwndCallback: HWND,
        ) -> u32;
        pub fn waveInGetNumDevs() -> UINT;
    }
}

/// Backend nativo de captura de áudio para Windows usando a interface MCI (Media Control Interface).
///
/// Não requer crates externas nem drivers C++ complexos, operando de forma soberana
/// e gerando contêineres WAV canônicos mono PCM 16-bit a 16 kHz.
#[derive(Debug)]
pub struct WindowsAudioCaptureBackend {
    alias: Arc<Mutex<Option<String>>>,
    temp_path: Arc<Mutex<Option<PathBuf>>>,
    is_recording: Arc<AtomicBool>,
}

impl Default for WindowsAudioCaptureBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowsAudioCaptureBackend {
    pub fn new() -> Self {
        Self {
            alias: Arc::new(Mutex::new(None)),
            temp_path: Arc::new(Mutex::new(None)),
            is_recording: Arc::new(AtomicBool::new(false)),
        }
    }

    #[cfg(windows)]
    fn send_mci(cmd: &str) -> Result<(), VoiceError> {
        use std::ffi::CString;
        let c_cmd = CString::new(cmd).map_err(|e| {
            VoiceError::CaptureFailed(format!("String de comando MCI inválida: {}", e))
        })?;
        let code = unsafe {
            winmm::mciSendStringA(
                c_cmd.as_ptr(),
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
            )
        };
        if code != 0 {
            Err(VoiceError::CaptureFailed(format!(
                "Falha na API MCI do Windows (código {}): comando '{}'",
                code, cmd
            )))
        } else {
            Ok(())
        }
    }
}

impl AudioCaptureBackend for WindowsAudioCaptureBackend {
    fn start_capture(&self) -> Result<(), VoiceError> {
        #[cfg(windows)]
        {
            if !self.is_device_available() {
                return Err(VoiceError::DeviceUnavailable(
                    "Nenhum microfone detectado ou acessível no sistema operacional (waveIn)"
                        .to_string(),
                ));
            }

            if self.is_recording.swap(true, Ordering::SeqCst) {
                return Err(VoiceError::SessionConflict(
                    "Captura de áudio já em andamento".to_string(),
                ));
            }

            let session_id = generate_unique_id("rec");
            let alias = format!("yuki_{}", session_id);
            let temp_file = std::env::temp_dir().join(format!("yuki_{}.wav", session_id));

            // Garante que o alias esteja fechado previamente
            let _ = Self::send_mci(&format!("close {}", alias));

            let setup_result = (|| {
                Self::send_mci(&format!("open new type waveaudio alias {}", alias))?;
                Self::send_mci(&format!("set {} time format ms", alias))?;
                Self::send_mci(&format!("set {} format tag pcm", alias))?;
                Self::send_mci(&format!("set {} channels 1", alias))?;
                Self::send_mci(&format!("set {} samplespersec 16000", alias))?;
                Self::send_mci(&format!("set {} bitspersample 16", alias))?;
                Self::send_mci(&format!("record {}", alias))?;
                Ok::<(), VoiceError>(())
            })();

            if let Err(e) = setup_result {
                let _ = Self::send_mci(&format!("close {}", alias));
                self.is_recording.store(false, Ordering::SeqCst);
                return Err(e);
            }

            *self.alias.lock().unwrap() = Some(alias);
            *self.temp_path.lock().unwrap() = Some(temp_file);

            Ok(())
        }
        #[cfg(not(windows))]
        {
            Err(VoiceError::CaptureFailed(
                "WindowsAudioCaptureBackend é suportado apenas no Windows".to_string(),
            ))
        }
    }

    fn stop_capture(&self) -> Result<AudioInput, VoiceError> {
        #[cfg(windows)]
        {
            if !self.is_recording.swap(false, Ordering::SeqCst) {
                return Err(VoiceError::SessionConflict(
                    "Nenhuma gravação de áudio em andamento para encerrar".to_string(),
                ));
            }

            let alias =
                self.alias.lock().unwrap().take().ok_or_else(|| {
                    VoiceError::CaptureFailed("Alias MCI não encontrado".to_string())
                })?;
            let temp_path = self.temp_path.lock().unwrap().take().ok_or_else(|| {
                VoiceError::CaptureFailed("Caminho temporário de gravação ausente".to_string())
            })?;

            let stop_result = (|| {
                Self::send_mci(&format!("stop {}", alias))?;
                let path_str = temp_path.to_string_lossy().replace('\\', "/");
                Self::send_mci(&format!("save {} \"{}\"", alias, path_str))?;
                Self::send_mci(&format!("close {}", alias))?;
                Ok::<(), VoiceError>(())
            })();

            if let Err(e) = stop_result {
                let _ = Self::send_mci(&format!("close {}", alias));
                let _ = std::fs::remove_file(&temp_path);
                return Err(e);
            }

            let wav_bytes = std::fs::read(&temp_path).map_err(|e| {
                VoiceError::Io(format!("Falha ao ler áudio gravado do microfone: {}", e))
            })?;
            let _ = std::fs::remove_file(&temp_path);

            let (format, samples) = AudioCodec::decode_wav(&wav_bytes)?;
            let duration_ms = if format.sample_rate > 0 && format.channels > 0 {
                ((samples.len() as u64) * 1000)
                    / (format.sample_rate as u64 * format.channels as u64)
            } else {
                0
            };

            Ok(AudioInput::new(
                wav_bytes,
                format,
                duration_ms,
                "microphone-windows",
            ))
        }
        #[cfg(not(windows))]
        {
            Err(VoiceError::CaptureFailed(
                "WindowsAudioCaptureBackend é suportado apenas no Windows".to_string(),
            ))
        }
    }

    fn cancel_capture(&self) -> Result<(), VoiceError> {
        #[cfg(windows)]
        {
            if self.is_recording.swap(false, Ordering::SeqCst) {
                if let Some(alias) = self.alias.lock().unwrap().take() {
                    let _ = Self::send_mci(&format!("stop {}", alias));
                    let _ = Self::send_mci(&format!("close {}", alias));
                }
                if let Some(temp_path) = self.temp_path.lock().unwrap().take() {
                    let _ = std::fs::remove_file(&temp_path);
                }
            }
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
            let devs = unsafe { winmm::waveInGetNumDevs() };
            devs > 0
        }
        #[cfg(not(windows))]
        {
            false
        }
    }

    fn name(&self) -> &str {
        "WindowsAudioCaptureBackend (MCI/waveIn)"
    }
}

// ---------------------------------------------------------------------------
// Backend Mock Observável para Testes e CI Headless
// ---------------------------------------------------------------------------

/// Backend mock determinístico para testes e validação sem microfone físico.
#[derive(Debug)]
pub struct MockCaptureBackend {
    device_available: Arc<AtomicBool>,
    is_recording: Arc<AtomicBool>,
    configured_input: Arc<Mutex<Option<AudioInput>>>,
    start_count: Arc<AtomicUsize>,
    stop_count: Arc<AtomicUsize>,
    cancel_count: Arc<AtomicUsize>,
}

impl Default for MockCaptureBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl MockCaptureBackend {
    pub fn new() -> Self {
        Self {
            device_available: Arc::new(AtomicBool::new(true)),
            is_recording: Arc::new(AtomicBool::new(false)),
            configured_input: Arc::new(Mutex::new(None)),
            start_count: Arc::new(AtomicUsize::new(0)),
            stop_count: Arc::new(AtomicUsize::new(0)),
            cancel_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn set_device_available(&self, available: bool) {
        self.device_available.store(available, Ordering::SeqCst);
    }

    pub fn set_mock_input(&self, input: AudioInput) {
        *self.configured_input.lock().unwrap() = Some(input);
    }

    pub fn start_count(&self) -> usize {
        self.start_count.load(Ordering::SeqCst)
    }

    pub fn stop_count(&self) -> usize {
        self.stop_count.load(Ordering::SeqCst)
    }

    pub fn cancel_count(&self) -> usize {
        self.cancel_count.load(Ordering::SeqCst)
    }

    pub fn is_recording(&self) -> bool {
        self.is_recording.load(Ordering::SeqCst)
    }
}

impl AudioCaptureBackend for MockCaptureBackend {
    fn start_capture(&self) -> Result<(), VoiceError> {
        if !self.is_device_available() {
            return Err(VoiceError::DeviceUnavailable(
                "Microfone mock configurado como indisponível".to_string(),
            ));
        }
        if self.is_recording.swap(true, Ordering::SeqCst) {
            return Err(VoiceError::SessionConflict(
                "Captura mock já em andamento".to_string(),
            ));
        }
        self.start_count.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn stop_capture(&self) -> Result<AudioInput, VoiceError> {
        if !self.is_recording.swap(false, Ordering::SeqCst) {
            return Err(VoiceError::SessionConflict(
                "Nenhuma captura mock ativa para encerrar".to_string(),
            ));
        }
        self.stop_count.fetch_add(1, Ordering::SeqCst);

        if let Some(input) = self.configured_input.lock().unwrap().clone() {
            Ok(input)
        } else {
            // Gera 1 segundo de silêncio codificado em WAV canônico
            let samples = vec![0i16; 16000];
            let format = AudioFormat::default();
            let wav_bytes = AudioCodec::encode_wav(&samples, &format)?;
            Ok(AudioInput::new(wav_bytes, format, 1000, "mock-microphone"))
        }
    }

    fn cancel_capture(&self) -> Result<(), VoiceError> {
        self.is_recording.store(false, Ordering::SeqCst);
        self.cancel_count.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn is_device_available(&self) -> bool {
        self.device_available.load(Ordering::SeqCst)
    }

    fn name(&self) -> &str {
        "MockCaptureBackend"
    }
}

// ---------------------------------------------------------------------------
// Controlador de Push-to-Talk Seguro
// ---------------------------------------------------------------------------

/// Controlador de ciclo de vida para operação Push-to-Talk (PTT) com garantias de segurança:
/// - Limite estrito de tempo de fala contínua (padrão 30s) para evitar estouro de memória.
/// - Limite máximo de tamanho de payload em bytes.
/// - Validação de integridade do buffer capturado antes da entrega ao subsistema STT.
#[derive(Debug)]
pub struct PushToTalkController {
    backend: Arc<dyn AudioCaptureBackend>,
    max_duration: Duration,
    max_bytes: usize,
    started_at: Arc<Mutex<Option<Instant>>>,
    is_active: Arc<AtomicBool>,
}

impl PushToTalkController {
    pub const DEFAULT_MAX_DURATION_SECS: u64 = 30;
    /// 30 segundos * 16000 amostras/s * 2 bytes/amostra = 960.000 bytes PCM + header WAV (~1MB)
    pub const DEFAULT_MAX_BYTES: usize = 1_000_000;

    pub fn new(backend: Arc<dyn AudioCaptureBackend>) -> Self {
        Self {
            backend,
            max_duration: Duration::from_secs(Self::DEFAULT_MAX_DURATION_SECS),
            max_bytes: Self::DEFAULT_MAX_BYTES,
            started_at: Arc::new(Mutex::new(None)),
            is_active: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn with_limits(mut self, max_duration: Duration, max_bytes: usize) -> Self {
        self.max_duration = max_duration;
        self.max_bytes = max_bytes;
        self
    }

    pub fn backend(&self) -> &Arc<dyn AudioCaptureBackend> {
        &self.backend
    }

    pub fn is_active(&self) -> bool {
        self.is_active.load(Ordering::SeqCst)
    }

    pub fn elapsed(&self) -> Option<Duration> {
        self.started_at.lock().unwrap().map(|t| t.elapsed())
    }

    /// Pressionar o botão / iniciar captura Push-to-Talk.
    pub fn press(&self) -> Result<(), VoiceError> {
        if !self.backend.is_device_available() {
            return Err(VoiceError::DeviceUnavailable(
                "Microfone indisponível ou desconectado para Push-to-Talk".to_string(),
            ));
        }

        if self.is_active.swap(true, Ordering::SeqCst) {
            return Err(VoiceError::SessionConflict(
                "Sessão Push-to-Talk já está ativa".to_string(),
            ));
        }

        if let Err(e) = self.backend.start_capture() {
            self.is_active.store(false, Ordering::SeqCst);
            return Err(e);
        }

        *self.started_at.lock().unwrap() = Some(Instant::now());
        Ok(())
    }

    /// Soltar o botão / finalizar captura Push-to-Talk e validar integridade e limites.
    pub fn release(&self) -> Result<AudioInput, VoiceError> {
        if !self.is_active.swap(false, Ordering::SeqCst) {
            return Err(VoiceError::SessionConflict(
                "Nenhuma sessão Push-to-Talk ativa para finalizar".to_string(),
            ));
        }

        let started = self.started_at.lock().unwrap().take();
        let elapsed = started.map(|t| t.elapsed()).unwrap_or_default();

        let audio_input = self.backend.stop_capture()?;

        // Validação de limite estrito de tamanho
        if audio_input.payload.len() > self.max_bytes {
            return Err(VoiceError::AudioTooLarge {
                bytes: audio_input.payload.len(),
                max_bytes: self.max_bytes,
            });
        }

        // Validação de duração máxima
        if elapsed > self.max_duration {
            return Err(VoiceError::AudioTooLong {
                duration_ms: elapsed.as_millis() as u64,
                max_ms: self.max_duration.as_millis() as u64,
            });
        }

        // Validação de payload não-vazio
        if audio_input.payload.is_empty() {
            return Err(VoiceError::InvalidAudioFormat(
                "Nenhum áudio foi gravado durante o período Push-to-Talk".to_string(),
            ));
        }

        Ok(audio_input)
    }

    /// Cancela a captura imediatamente sem retornar dados.
    pub fn cancel(&self) -> Result<(), VoiceError> {
        self.is_active.store(false, Ordering::SeqCst);
        *self.started_at.lock().unwrap() = None;
        self.backend.cancel_capture()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_capture_lifecycle() {
        let backend = MockCaptureBackend::new();
        assert!(backend.is_device_available());
        assert!(!backend.is_recording());

        assert!(backend.start_capture().is_ok());
        assert!(backend.is_recording());
        assert_eq!(backend.start_count(), 1);

        // Tentativa de iniciar em andamento deve falhar
        assert!(matches!(
            backend.start_capture(),
            Err(VoiceError::SessionConflict(_))
        ));

        let audio = backend.stop_capture().expect("deve parar com sucesso");
        assert_eq!(backend.stop_count(), 1);
        assert!(!backend.is_recording());
        assert!(!audio.payload.is_empty());
        assert_eq!(audio.format.sample_rate, 16000);
    }

    #[test]
    fn test_ptt_controller_lifecycle_and_guards() {
        let mock_backend = Arc::new(MockCaptureBackend::new());
        let ptt = PushToTalkController::new(mock_backend.clone());

        assert!(!ptt.is_active());
        assert!(ptt.press().is_ok());
        assert!(ptt.is_active());

        // Press duplo rejeitado
        assert!(matches!(ptt.press(), Err(VoiceError::SessionConflict(_))));

        let audio = ptt.release().expect("deve liberar com sucesso");
        assert!(!ptt.is_active());
        assert_eq!(audio.duration_ms, 1000);

        // Release sem press rejeitado
        assert!(matches!(ptt.release(), Err(VoiceError::SessionConflict(_))));
    }

    #[test]
    fn test_ptt_payload_limit_enforcement() {
        let mock_backend = Arc::new(MockCaptureBackend::new());
        // Limite restrito de 100 bytes
        let ptt = PushToTalkController::new(mock_backend.clone())
            .with_limits(Duration::from_secs(30), 100);

        // Configura payload mock com 500 bytes (> 100)
        let large_input =
            AudioInput::new(vec![0u8; 500], AudioFormat::default(), 100, "mock-large");
        mock_backend.set_mock_input(large_input);

        ptt.press().unwrap();
        let res = ptt.release();
        assert!(matches!(res, Err(VoiceError::AudioTooLarge { .. })));
    }

    #[test]
    fn test_ptt_device_unavailable() {
        let mock_backend = Arc::new(MockCaptureBackend::new());
        mock_backend.set_device_available(false);
        let ptt = PushToTalkController::new(mock_backend);

        assert!(matches!(ptt.press(), Err(VoiceError::DeviceUnavailable(_))));
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_capture_device_detection() {
        let backend = WindowsAudioCaptureBackend::new();
        let available = backend.is_device_available();
        println!("Windows audio capture device available: {}", available);
    }
}
