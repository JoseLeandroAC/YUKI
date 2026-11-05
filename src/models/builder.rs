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
                     Nota: A assinatura do Antigravity IDE não injeta automaticamente chave de API no binário autônomo da Yuki.\n\
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

/// Origem pública sanitizada da credencial Gemini resolvida pelo `CredentialBroker`.
/// NUNCA contém valores de segredos, passphrases ou URIs privadas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GeminiCredentialSource {
    /// Variável de ambiente `YUKI_GEMINI_API_KEY` via `EnvSecretStore`.
    EnvYukiGeminiApiKey,
    /// Variável de ambiente `GEMINI_API_KEY` via `EnvSecretStore`.
    EnvGeminiApiKey,
    /// Cofre cifrado local `CredentialVault` (`VaultSecretStore`).
    VaultEncrypted,
    /// Referência explícita via `YUKI_GEMINI_SECRET_REF` resolvida pelo `CredentialBroker`.
    ExplicitSecretRef,
    /// Cofre cifrado existe no disco, porém `YUKI_VAULT_PASSPHRASE` não foi definida na sessão.
    VaultPresentLocked,
    /// Nenhuma credencial configurada no ambiente nem no cofre local.
    Missing,
}

impl GeminiCredentialSource {
    pub fn public_label(&self) -> &'static str {
        match self {
            Self::EnvYukiGeminiApiKey => "EnvSecretStore (YUKI_GEMINI_API_KEY)",
            Self::EnvGeminiApiKey => "EnvSecretStore (GEMINI_API_KEY)",
            Self::VaultEncrypted => "VaultSecretStore (CredentialVault cifrado)",
            Self::ExplicitSecretRef => "CredentialBroker (YUKI_GEMINI_SECRET_REF)",
            Self::VaultPresentLocked => {
                "CredentialVault detectado em disco (bloqueado — defina YUKI_VAULT_PASSPHRASE)"
            }
            Self::Missing => "Não configurada",
        }
    }
}

/// Relatório público e sanitizado de prontidão do provedor Gemini via `CredentialBroker`.
#[derive(Debug, Clone)]
pub struct GeminiReadinessReport {
    pub provider_selected: String,
    pub model_id: String,
    pub api_version: String,
    pub credential_source: GeminiCredentialSource,
    pub credential_resolvable: bool,
    pub ready_for_live_conversation: bool,
    pub operator_guidance: String,
}

/// Inspeciona com segurança a prontidão do provedor Gemini e do `CredentialBroker`
/// sem exibir, registrar ou reter segredos em memória (`SecretLease` zeroizado imediatamente).
pub fn diagnose_gemini_provider_readiness() -> GeminiReadinessReport {
    let config = build_gemini_gateway_config_from_env();
    let provider_selected = std::env::var("YUKI_MODEL_PROVIDER")
        .ok()
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "<não definido>".to_string());

    let vault_cfg = VaultConfig::from_env();
    let vault_file_exists = vault_cfg.vault_path.exists();
    let passphrase_present = std::env::var("YUKI_VAULT_PASSPHRASE")
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false);

    let has_explicit_ref = std::env::var("YUKI_GEMINI_SECRET_REF")
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false);
    let has_yuki_env = std::env::var("YUKI_GEMINI_API_KEY")
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false);
    let has_std_env = std::env::var("GEMINI_API_KEY")
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false);

    let mut credential_resolvable = false;
    let credential_source = if let Ok((broker, vault_loaded)) = build_composite_broker_from_env() {
        let candidate_source = if has_explicit_ref {
            GeminiCredentialSource::ExplicitSecretRef
        } else if has_yuki_env {
            GeminiCredentialSource::EnvYukiGeminiApiKey
        } else if has_std_env {
            GeminiCredentialSource::EnvGeminiApiKey
        } else if vault_loaded {
            GeminiCredentialSource::VaultEncrypted
        } else if vault_file_exists && !passphrase_present {
            GeminiCredentialSource::VaultPresentLocked
        } else {
            GeminiCredentialSource::Missing
        };

        if !matches!(
            candidate_source,
            GeminiCredentialSource::Missing | GeminiCredentialSource::VaultPresentLocked
        ) {
            if let Ok(secret_ref) = resolve_gemini_secret_ref(&config, vault_loaded) {
                if let Ok(lease) = broker.acquire(&secret_ref) {
                    drop(lease);
                    credential_resolvable = true;
                }
            }
        }
        candidate_source
    } else if vault_file_exists {
        GeminiCredentialSource::VaultPresentLocked
    } else {
        GeminiCredentialSource::Missing
    };

    let ready_for_live_conversation = provider_selected == "gemini" && credential_resolvable;

    let operator_guidance = if ready_for_live_conversation {
        format!(
            "Provedor Gemini pronto via {} (modelo: {}).",
            credential_source.public_label(),
            config.model_id
        )
    } else {
        "Nota: A assinatura do Antigravity IDE é independente e não disponibiliza automaticamente API ao aplicativo Yuki.\n\
         Opção A (Sessão PowerShell rápida — sem persistir em texto puro no disco):\n\
           $env:YUKI_MODEL_PROVIDER = \"gemini\"\n\
           $env:YUKI_GEMINI_API_KEY = Read-Host \"Informe sua Gemini API Key\" -MaskInput\n\
         Opção B (Cofre cifrado persistente via CredentialVault):\n\
           $env:YUKI_VAULT_PASSPHRASE = Read-Host \"Defina/informe a senha do cofre\" -MaskInput\n\
           .\\target\\release\\yuki.exe vault init\n\
           .\\target\\release\\yuki.exe vault store gemini_api_key --provider gemini\n\
           $env:YUKI_MODEL_PROVIDER = \"gemini\"\n\
         (Para testar o pipeline acústico offline sem API, utilize '--mock' em 'yuki voice live --mock')."
            .to_string()
    };

    GeminiReadinessReport {
        provider_selected,
        model_id: config.model_id,
        api_version: config.api_version,
        credential_source,
        credential_resolvable,
        ready_for_live_conversation,
        operator_guidance,
    }
}
