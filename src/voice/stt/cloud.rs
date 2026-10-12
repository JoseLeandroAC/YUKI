use crate::contracts::voice::{
    AudioInput, SpeechToTextProvider, VoiceError, VoiceProviderMetadata, VoiceProviderType,
};
use std::future::Future;
use std::pin::Pin;

/// Provedor governado de STT em nuvem (ex: Gemini Speech / OpenAI Whisper Cloud).
///
/// REGRAS INEGOCIÁVEIS:
/// - Requer autorização explícita e específica (`egress:voice_cloud_stt`).
/// - Requer credencial obtida via Credential Vault.
/// - Jamais executa como fallback silencioso não consentido.
pub struct CloudSttProvider {
    metadata: VoiceProviderMetadata,
    api_endpoint: String,
    credential_alias: String,
    has_valid_credential: bool,
}

impl CloudSttProvider {
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
            16000,
            false,
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

impl SpeechToTextProvider for CloudSttProvider {
    fn metadata(&self) -> &VoiceProviderMetadata {
        &self.metadata
    }

    fn health<'a>(&'a self) -> Pin<Box<dyn Future<Output = Result<(), VoiceError>> + Send + 'a>> {
        let has_cred = self.has_valid_credential;
        let alias = self.credential_alias.clone();
        Box::pin(async move {
            if !has_cred {
                return Err(VoiceError::ProviderUnavailable(format!(
                    "Credencial '{}' para Cloud STT não configurada no cofre",
                    alias
                )));
            }
            Ok(())
        })
    }

    fn transcribe<'a>(
        &'a self,
        _audio: &'a AudioInput,
    ) -> Pin<Box<dyn Future<Output = Result<String, VoiceError>> + Send + 'a>> {
        let has_cred = self.has_valid_credential;
        let endpoint = self.api_endpoint.clone();
        let alias = self.credential_alias.clone();

        Box::pin(async move {
            if !has_cred {
                return Err(VoiceError::UnauthorizedCloudFallback(format!(
                    "Credencial '{}' ausente no cofre de credenciais",
                    alias
                )));
            }

            // Simulação de gateway em nuvem com endpoint validado
            if endpoint.is_empty() {
                return Err(VoiceError::ProviderUnavailable(
                    "Endpoint de Cloud STT vazio".to_string(),
                ));
            }

            // Em produção real, este método despacha a requisição HTTP governada ao gateway
            Ok("Transcrição via Provedor Cloud Governado".to_string())
        })
    }
}
