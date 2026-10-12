use crate::contracts::voice::{
    AudioFormat, AudioOutput, TextToSpeechProvider, VoiceConfiguration, VoiceError,
    VoiceProviderMetadata, VoiceProviderType,
};
use crate::voice::audio::AudioCodec;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Provedor mock de síntese de fala (TTS) para testes determinísticos.
#[derive(Clone)]
pub struct MockTtsProvider {
    metadata: VoiceProviderMetadata,
    should_fail: Arc<Mutex<bool>>,
    delay: Duration,
    synthesized_texts: Arc<Mutex<Vec<String>>>,
}

impl Default for MockTtsProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl MockTtsProvider {
    pub fn new() -> Self {
        let metadata = VoiceProviderMetadata::new(
            "Mock-TTS",
            VoiceProviderType::Mock,
            vec!["pt-BR".to_string(), "en".to_string()],
            16000,
            true,
            "1.0.0",
        );
        Self {
            metadata,
            should_fail: Arc::new(Mutex::new(false)),
            delay: Duration::from_millis(0),
            synthesized_texts: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn with_failure(self, fail: bool) -> Self {
        *self.should_fail.lock().unwrap() = fail;
        self
    }

    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }

    pub fn set_failure(&self, fail: bool) {
        *self.should_fail.lock().unwrap() = fail;
    }

    pub fn get_synthesized_texts(&self) -> Vec<String> {
        self.synthesized_texts.lock().unwrap().clone()
    }
}

impl TextToSpeechProvider for MockTtsProvider {
    fn metadata(&self) -> &VoiceProviderMetadata {
        &self.metadata
    }

    fn health<'a>(&'a self) -> Pin<Box<dyn Future<Output = Result<(), VoiceError>> + Send + 'a>> {
        let fail = *self.should_fail.lock().unwrap();
        Box::pin(async move {
            if fail {
                Err(VoiceError::ProviderUnavailable("Mock TTS offline".into()))
            } else {
                Ok(())
            }
        })
    }

    fn synthesize<'a>(
        &'a self,
        text: &'a str,
        _voice_params: Option<&'a VoiceConfiguration>,
    ) -> Pin<Box<dyn Future<Output = Result<AudioOutput, VoiceError>> + Send + 'a>> {
        let fail = *self.should_fail.lock().unwrap();
        let delay = self.delay;
        let text_owned = text.to_string();
        self.synthesized_texts
            .lock()
            .unwrap()
            .push(text_owned.clone());

        Box::pin(async move {
            if delay > Duration::ZERO {
                tokio::time::sleep(delay).await;
            }

            if fail {
                return Err(VoiceError::SynthesisFailed(
                    "Falha simulada de síntese no Mock TTS".into(),
                ));
            }

            let format = AudioFormat {
                sample_rate: 16000,
                channels: 1,
                bits_per_sample: 16,
            };
            let dummy_samples = vec![1000i16; 1600]; // 100ms
            let payload = AudioCodec::encode_wav(&dummy_samples, &format)?;

            Ok(AudioOutput::new(payload, format, 100, "mock-voice"))
        })
    }
}
