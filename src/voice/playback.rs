use crate::contracts::voice::{AudioOutput, VoiceError};
use std::sync::{Arc, Mutex};

/// Estado atual da reprodução de áudio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackStatus {
    Idle,
    Playing,
    Interrupted,
    Stopped,
}

/// Controlador seguro de saída e reprodução de áudio da Yuki.
///
/// Garante:
/// - Prevenção estrita de sobreposição acidental de fala.
/// - Interrupção instantânea na detecção de fala do operador.
/// - Tratamento explícito de dispositivo de áudio indisponível ou inacessível.
/// - Limpeza imediata de buffers e recursos na finalização ou cancelamento.
#[derive(Debug, Clone)]
pub struct AudioPlaybackController {
    status: Arc<Mutex<PlaybackStatus>>,
    current_output: Arc<Mutex<Option<AudioOutput>>>,
    device_available: Arc<Mutex<bool>>,
}

impl Default for AudioPlaybackController {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioPlaybackController {
    pub fn new() -> Self {
        Self {
            status: Arc::new(Mutex::new(PlaybackStatus::Idle)),
            current_output: Arc::new(Mutex::new(None)),
            device_available: Arc::new(Mutex::new(true)),
        }
    }

    /// Define o estado de disponibilidade do dispositivo de áudio (para testes e diagnósticos).
    pub fn set_device_available(&self, available: bool) {
        *self.device_available.lock().unwrap() = available;
    }

    pub fn status(&self) -> PlaybackStatus {
        *self.status.lock().unwrap()
    }

    /// Envia um buffer de áudio sintetizado para reprodução.
    /// Rejeita se o dispositivo estiver indisponível ou interrompe reprodução prévia em curso.
    pub fn play(&self, output: &AudioOutput) -> Result<(), VoiceError> {
        let is_avail = *self.device_available.lock().unwrap();
        if !is_avail {
            return Err(VoiceError::DeviceUnavailable(
                "Dispositivo de reprodução de áudio (alto-falante) não detectado ou inacessível"
                    .to_string(),
            ));
        }

        let mut status_lock = self.status.lock().unwrap();
        let mut out_lock = self.current_output.lock().unwrap();

        // Se já estava reproduzindo, substitui de forma ordenada sem sobreposição descontrolada
        *out_lock = Some(output.clone());
        *status_lock = PlaybackStatus::Playing;

        Ok(())
    }

    /// Interrompe imediatamente a reprodução em andamento (ex: barge-in do usuário).
    pub fn interrupt(&self) -> Result<(), VoiceError> {
        let mut status_lock = self.status.lock().unwrap();
        let mut out_lock = self.current_output.lock().unwrap();

        *status_lock = PlaybackStatus::Interrupted;
        *out_lock = None; // Limpeza imediata do buffer

        Ok(())
    }

    /// Encerra normalmente a reprodução e retorna o estado para ocioso.
    pub fn stop(&self) {
        let mut status_lock = self.status.lock().unwrap();
        let mut out_lock = self.current_output.lock().unwrap();

        *status_lock = PlaybackStatus::Stopped;
        *out_lock = None;
    }

    /// Reseta o controlador para Idle.
    pub fn reset(&self) {
        let mut status_lock = self.status.lock().unwrap();
        let mut out_lock = self.current_output.lock().unwrap();

        *status_lock = PlaybackStatus::Idle;
        *out_lock = None;
    }
}
