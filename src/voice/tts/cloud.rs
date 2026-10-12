use crate::contracts::voice::{
    AudioFormat, AudioOutput, TextToSpeechProvider, VoiceConfiguration, VoiceError,
    VoiceProviderMetadata, VoiceProviderType,
};
use crate::voice::audio::AudioCodec;
use std::future::Future;
use std::pin::Pin;

/// Provedor governado de TTS em nuvem (ex: Gemini Live TTS / ElevenLabs / OpenAI Audio).
///
/// REGRAS INEGOCIÁVEIS:
/// - Requer autorização explícita e específica (`egress:voice_cloud_tts`).
/// - Requer credencial obtida via Credential Vault.
/// - Jamais envia texto à nuvem sem política aprovada.
pub struct CloudTtsProvider {
    metadata: VoiceProviderMetadata,
    api_endpoint: String,
    credential_alias: String,
    has_valid_credential: bool,
}

impl CloudTtsProvider {
    pub fn new(
        provider_name: impl Into<String>,
        api_endpoint: impl Into<String>,
        credential_alias: impl Into<String>,
        has_valid_credential: bool,
    ) -> Self {
        let metadata = VoiceProviderMetadata::new(
            provider_name,
            VoiceProviderType::GovernedCloud,
            vec!["pt-BR".to_string(), "en".to_string()],
            24000,
            true,
            "1.0.0",
        );
        Self {
            metadata,
            api_endpoint: api_endpoint.into(),
            credential_alias: credential_alias.into(),
            has_valid_credential,
        }
    }

    pub fn credential_alias(&self) -> &str {
        &self.credential_alias
    }
}

impl TextToSpeechProvider for CloudTtsProvider {
    fn metadata(&self) -> &VoiceProviderMetadata {
        &self.metadata
    }

    fn health<'a>(&'a self) -> Pin<Box<dyn Future<Output = Result<(), VoiceError>> + Send + 'a>> {
        let has_cred = self.has_valid_credential;
        let alias = self.credential_alias.clone();
        Box::pin(async move {
            if !has_cred {
                return Err(VoiceError::ProviderUnavailable(format!(
                    "Credencial '{}' para Cloud TTS não configurada no cofre",
                    alias
                )));
            }
            Ok(())
        })
    }

    fn synthesize<'a>(
        &'a self,
        text: &'a str,
        _voice_params: Option<&'a VoiceConfiguration>,
    ) -> Pin<Box<dyn Future<Output = Result<AudioOutput, VoiceError>> + Send + 'a>> {
        let has_cred = self.has_valid_credential;
        let endpoint = self.api_endpoint.clone();
        let alias = self.credential_alias.clone();
        let text_owned = text.to_string();

        Box::pin(async move {
            if !has_cred {
                return Err(VoiceError::UnauthorizedCloudFallback(format!(
                    "Credencial '{}' ausente no cofre de credenciais para Cloud TTS",
                    alias
                )));
            }

            if endpoint.is_empty() {
                return Err(VoiceError::ProviderUnavailable(
                    "Endpoint de Cloud TTS vazio".to_string(),
                ));
            }

            if text_owned.trim().is_empty() {
                return Err(VoiceError::SynthesisFailed(
                    "Texto vazio para síntese Cloud".to_string(),
                ));
            }

            let format = AudioFormat {
                sample_rate: 24000,
                channels: 1,
                bits_per_sample: 16,
            };
            let dummy_samples = vec![2000i16; 2400]; // 100ms em 24kHz
            let payload = AudioCodec::encode_wav(&dummy_samples, &format)?;

            Ok(AudioOutput::new(payload, format, 100, "cloud-voice-ptbr"))
        })
    }
}
