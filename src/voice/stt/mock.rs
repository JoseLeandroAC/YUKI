use crate::contracts::voice::{
    AudioInput, SpeechToTextProvider, VoiceError, VoiceProviderMetadata, VoiceProviderType,
};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Provedor mock de reconhecimento de fala para testes unitários e de integração determinísticos.
#[derive(Clone)]
pub struct MockSttProvider {
    metadata: VoiceProviderMetadata,
    configured_response: Arc<Mutex<Option<String>>>,
    should_fail: Arc<Mutex<bool>>,
    delay: Duration,
    received_inputs: Arc<Mutex<Vec<AudioInput>>>,
}

impl Default for MockSttProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl MockSttProvider {
    pub fn new() -> Self {
        let metadata = VoiceProviderMetadata::new(
            "Mock-STT",
            VoiceProviderType::Mock,
            vec!["pt-BR".to_string(), "en".to_string()],
            16000,
            true,
            "1.0.0",
        );
        Self {
            metadata,
            configured_response: Arc::new(Mutex::new(Some("Olá Yuki".to_string()))),
            should_fail: Arc::new(Mutex::new(false)),
            delay: Duration::from_millis(0),
            received_inputs: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn with_response(self, text: impl Into<String>) -> Self {
        *self.configured_response.lock().unwrap() = Some(text.into());
        self
    }

    pub fn with_failure(self, fail: bool) -> Self {
        *self.should_fail.lock().unwrap() = fail;
        self
    }

    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }

    pub fn set_response(&self, text: impl Into<String>) {
        *self.configured_response.lock().unwrap() = Some(text.into());
    }

    pub fn set_failure(&self, fail: bool) {
        *self.should_fail.lock().unwrap() = fail;
    }

    pub fn get_received_inputs(&self) -> Vec<AudioInput> {
        self.received_inputs.lock().unwrap().clone()
    }
}

impl SpeechToTextProvider for MockSttProvider {
    fn metadata(&self) -> &VoiceProviderMetadata {
        &self.metadata
    }

    fn health<'a>(&'a self) -> Pin<Box<dyn Future<Output = Result<(), VoiceError>> + Send + 'a>> {
        let fail = *self.should_fail.lock().unwrap();
        Box::pin(async move {
            if fail {
                Err(VoiceError::ProviderUnavailable("Mock STT offline".into()))
            } else {
                Ok(())
            }
        })
    }

    fn transcribe<'a>(
        &'a self,
        audio: &'a AudioInput,
    ) -> Pin<Box<dyn Future<Output = Result<String, VoiceError>> + Send + 'a>> {
        let fail = *self.should_fail.lock().unwrap();
        let resp = self.configured_response.lock().unwrap().clone();
        let delay = self.delay;
        self.received_inputs.lock().unwrap().push(audio.clone());

        Box::pin(async move {
            if delay > Duration::ZERO {
                tokio::time::sleep(delay).await;
            }

            if fail {
                return Err(VoiceError::TranscriptionFailed(
                    "Falha simulada de transcrição no Mock STT".into(),
                ));
            }

            Ok(resp.unwrap_or_default())
        })
    }
}
