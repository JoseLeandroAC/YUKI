use crate::models::config::ModelGatewayConfig;
use crate::models::errors::ModelError;
use crate::models::gemini::{
    sanitize_gemini_api_version, sanitize_gemini_model_id, GeminiProviderAdapter,
};
use crate::models::mock::MockModelProvider;
use crate::models::provider::ModelProvider;
use crate::security::credentials::{
    CompositeCredentialBroker, CredentialBroker, SecretMaterial, SecretRef,
};
use crate::security::vault::{CredentialVault, VaultConfig, VaultSecretStore};
use std::sync::Arc;
use std::time::Duration;

fn build_gemini_gateway_config_from_env() -> ModelGatewayConfig {
    let mut config = ModelGatewayConfig::default();

    if let Ok(model_id) = std::env::var("YUKI_MODEL_ID") {
        let trimmed = model_id.trim();
        if !trimmed.is_empty() {
            config.model_id = sanitize_gemini_model_id(trimmed);
        }
    }

    if let Ok(api_version) = std::env::var("YUKI_API_VERSION") {
        let trimmed = api_version.trim();
        if !trimmed.is_empty() {
            config.api_version = sanitize_gemini_api_version(trimmed);
        }
    }

    if let Ok(timeout_ms_str) = std::env::var("YUKI_REQUEST_TIMEOUT_MS") {
        if let Ok(timeout_ms) = timeout_ms_str.trim().parse::<u64>() {
            config.request_timeout = Duration::from_millis(timeout_ms);
        }
    }

    if let Ok(iterations_str) = std::env::var("YUKI_MAX_TOOL_ITERATIONS") {
        if let Ok(iterations) = iterations_str.trim().parse::<u32>() {
            config.max_tool_iterations = iterations;
        }
    }

    config
}

fn build_composite_broker_from_env() -> Result<(Arc<dyn CredentialBroker>, bool), ModelError> {
    let mut broker = CompositeCredentialBroker::new();
    let mut vault_loaded = false;

    let vault_cfg = VaultConfig::from_env();
    if let Ok(pass) = std::env::var("YUKI_VAULT_PASSPHRASE") {
        if !pass.trim().is_empty() && vault_cfg.vault_path.exists() {
            let vault =
                CredentialVault::open_or_create(&vault_cfg.vault_path, SecretMaterial::new(pass))
                    .map_err(|e| {
                    ModelError::Authentication(format!(
                        "Falha ao desbloquear CredentialVault em '{}': {}",
                        vault_cfg.vault_path.display(),
                        e
                    ))
                })?;
            broker.set_vault(Arc::new(VaultSecretStore::new(Arc::new(vault))));
            vault_loaded = true;
        }
    }

    Ok((Arc::new(broker), vault_loaded))
}

fn resolve_gemini_secret_ref(
    config: &ModelGatewayConfig,
    vault_loaded: bool,
) -> Result<SecretRef, ModelError> {
    let uri = if let Ok(explicit_ref) = std::env::var("YUKI_GEMINI_SECRET_REF") {
        let trimmed = explicit_ref.trim();
        if !trimmed.is_empty() {
            trimmed.to_string()
        } else {
            "secret://env/YUKI_GEMINI_API_KEY".to_string()
        }
    } else if std::env::var("YUKI_GEMINI_API_KEY")
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false)
    {
        "secret://env/YUKI_GEMINI_API_KEY".to_string()
    } else if std::env::var("GEMINI_API_KEY")
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false)
    {
        "secret://env/GEMINI_API_KEY".to_string()
    } else if vault_loaded {
        "secret://vault/gemini_api_key".to_string()
    } else {
        "secret://env/YUKI_GEMINI_API_KEY".to_string()
    };

    SecretRef::new(uri, &config.sanitized_credential_alias)
        .map_err(|e| ModelError::Configuration(format!("SecretRef inválida: {}", e)))
}

/// Formata exclusivamente os metadados públicos do provedor de modelo ativo para exibição/diagnóstico.
/// NUNCA inspeciona nem exibe segredos, chaves ou URIs internas de credenciais.
pub fn format_active_model_provider_banner(provider: &dyn ModelProvider) -> String {
    let meta = provider.metadata();
    if meta.provider_name.eq_ignore_ascii_case("MockProvider") {
        format!(
            "{} (modelo: {}) [MODO DIAGNÓSTICO OFFLINE / MOCK]",
            meta.provider_name, meta.model_name
        )
    } else {
        format!("{} (modelo: {})", meta.provider_name, meta.model_name)
    }
}

/// Resolve e instancia o provedor de modelo ativo a partir das configurações de ambiente.
///
/// REGRAS ARQUITETURAIS (ADR-008, ADR-018):
/// 1. Padrão estrito para CLI geral / testes: na ausência de `YUKI_MODEL_PROVIDER`, adota `MockModelProvider`.
/// 2. Gemini só é ativado se explicitamente solicitado via `YUKI_MODEL_PROVIDER=gemini`.
/// 3. O bootstrap NÃO inspeciona ou lê os bytes da chave de API (resolução lazy via `CredentialBroker`).
/// 4. Nenhum fallback silencioso Gemini -> Mock.
pub fn resolve_model_provider_from_env() -> Result<Arc<dyn ModelProvider>, ModelError> {
    let provider_name = std::env::var("YUKI_MODEL_PROVIDER")
        .unwrap_or_else(|_| "mock".to_string())
        .trim()
        .to_lowercase();

    match provider_name.as_str() {
        "mock" => Ok(Arc::new(MockModelProvider::new())),
        "gemini" => {
            let config = build_gemini_gateway_config_from_env();
            let (broker, vault_loaded) = build_composite_broker_from_env()?;
            let secret_ref = resolve_gemini_secret_ref(&config, vault_loaded)?;
            let adapter = GeminiProviderAdapter::new(secret_ref, broker, config)?;
            Ok(Arc::new(adapter))
        }
        unknown => Err(ModelError::Configuration(format!(
            "Provedor de modelo desconhecido: '{}'. Valores suportados: 'mock', 'gemini'.",
            unknown
        ))),
    }
}

/// Resolve e valida estritamente o provedor de modelo para modos conversacionais reais (`voice live` / `voice turn`).
///
/// INVARIANTES DE HOMOLOGAÇÃO (Marco 8.1.3):
/// 1. NUNCA faz fallback silencioso para `MockModelProvider`.
/// 2. Se `allow_mock == false`, exige um provedor real (`YUKI_MODEL_PROVIDER=gemini`) com credencial válida
///    resolvível pelo `CredentialBroker` (`EnvSecretStore` ou `VaultSecretStore`), falhando fechado imediatamente
///    com instruções claras caso ausente.
/// 3. Nunca registra, imprime ou retém o material secreto em texto puro; o `SecretLease` de verificação
///    é descartado e zeroizado imediatamente.
pub fn resolve_conversational_model_provider_from_env(
    allow_mock: bool,
) -> Result<Arc<dyn ModelProvider>, ModelError> {
    let raw_provider = std::env::var("YUKI_MODEL_PROVIDER")
        .ok()
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty());

    match raw_provider.as_deref() {
        None | Some("mock") => {
            if allow_mock {
                Ok(Arc::new(MockModelProvider::new()))
            } else {
                let current = raw_provider.as_deref().unwrap_or("<não definido>");
                Err(ModelError::Configuration(format!(
                    "Modo de conversação real bloqueou o uso silencioso do MockModelProvider (YUKI_MODEL_PROVIDER={}).\n\
                     Para conversar com inteligência real via Gemini, configure no PowerShell:\n\
                       $env:YUKI_MODEL_PROVIDER = \"gemini\"\n\
                       $env:YUKI_GEMINI_API_KEY = \"<sua-chave-gemini>\"\n\
                     (Alternativa com cofre cifrado: 'yuki vault init' -> 'yuki vault store gemini_api_key --provider gemini' + $env:YUKI_VAULT_PASSPHRASE)\n\
                     Ou passe explicitamente '--mock' apenas para diagnóstico acústico offline.",
                    current
                )))
            }
        }
        Some("gemini") => {
            let config = build_gemini_gateway_config_from_env();
            let (broker, vault_loaded) = build_composite_broker_from_env()?;
            let secret_ref = resolve_gemini_secret_ref(&config, vault_loaded)?;

            // Validação pré-voo da credencial sem expor nem logar o segredo (zeroize imediato no Drop do lease)
            let lease = broker.acquire(&secret_ref).map_err(|e| {
                ModelError::Authentication(format!(
                    "Provedor 'gemini' selecionado, mas a credencial '{}' não pôde ser adquirida via CredentialBroker ({}).\n\
                     Defina $env:YUKI_GEMINI_API_KEY (ou armazene 'gemini_api_key' no CredentialVault e defina $env:YUKI_VAULT_PASSPHRASE).",
                    secret_ref.sanitized_alias(),
                    e
                ))
            })?;
            drop(lease);

            let adapter = GeminiProviderAdapter::new(secret_ref, broker, config)?;
            Ok(Arc::new(adapter))
        }
        Some(unknown) => Err(ModelError::Configuration(format!(
            "Provedor de modelo desconhecido: '{}'. Valores suportados: 'gemini' (ou 'mock' apenas com '--mock').",
            unknown
        ))),
    }
}
