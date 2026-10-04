use crate::models::config::ModelGatewayConfig;
use crate::models::errors::ModelError;
use crate::models::gemini::{
    sanitize_gemini_api_version, sanitize_gemini_model_id, GeminiProviderAdapter,
};
use crate::models::mock::MockModelProvider;
use crate::models::provider::ModelProvider;
use crate::security::credentials::{EnvSecretStore, SecretRef};
use std::sync::Arc;
use std::time::Duration;

/// Resolve e instancia o provedor de modelo ativo a partir das configurações de ambiente.
///
/// REGRAS ARQUITETURAIS (ADR-008, ADR-018):
/// 1. Padrão estrito: na ausência de `YUKI_MODEL_PROVIDER`, adota `MockModelProvider`.
/// 2. Gemini só é ativado se explicitamente solicitado via `YUKI_MODEL_PROVIDER=gemini`.
/// 3. O bootstrap NÃO inspeciona ou lê os bytes de `YUKI_GEMINI_API_KEY`.
///    Ele apenas constrói `SecretRef` e injeta `EnvSecretStore` (`CredentialBroker`).
/// 4. Nenhum fallback silencioso Gemini -> Mock.
pub fn resolve_model_provider_from_env() -> Result<Arc<dyn ModelProvider>, ModelError> {
    let provider_name = std::env::var("YUKI_MODEL_PROVIDER")
        .unwrap_or_else(|_| "mock".to_string())
        .trim()
        .to_lowercase();

    match provider_name.as_str() {
        "mock" => Ok(Arc::new(MockModelProvider::new())),
        "gemini" => {
            let mut config = ModelGatewayConfig::default();

            // Override opcional de model_id via ambiente com sanitização
            if let Ok(model_id) = std::env::var("YUKI_MODEL_ID") {
                let trimmed = model_id.trim();
                if !trimmed.is_empty() {
                    config.model_id = sanitize_gemini_model_id(trimmed);
                }
            }

            // Override opcional de api_version via ambiente com sanitização
            if let Ok(api_version) = std::env::var("YUKI_API_VERSION") {
                let trimmed = api_version.trim();
                if !trimmed.is_empty() {
                    config.api_version = sanitize_gemini_api_version(trimmed);
                }
            }

            // Override opcional de timeout HTTP em ms via ambiente
            if let Ok(timeout_ms_str) = std::env::var("YUKI_REQUEST_TIMEOUT_MS") {
                if let Ok(timeout_ms) = timeout_ms_str.trim().parse::<u64>() {
                    config.request_timeout = Duration::from_millis(timeout_ms);
                }
            }

            // Constrói SecretRef de forma agnóstica sem manipular SecretMaterial
            let secret_ref = SecretRef::new(
                "secret://env/YUKI_GEMINI_API_KEY",
                &config.sanitized_credential_alias,
            )
            .map_err(|e| ModelError::Configuration(format!("SecretRef inválida: {}", e)))?;

            let broker = Arc::new(EnvSecretStore::new());

            let adapter = GeminiProviderAdapter::new(secret_ref, broker, config)?;
            Ok(Arc::new(adapter))
        }
        unknown => Err(ModelError::Configuration(format!(
            "Provedor de modelo desconhecido: '{}'. Valores suportados: 'mock', 'gemini'.",
            unknown
        ))),
    }
}
