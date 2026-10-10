pub mod brave;
pub mod fetch;
pub mod manifest;
pub mod provider;
pub mod search;

pub use brave::*;
pub use fetch::*;
pub use manifest::*;
pub use provider::*;
pub use search::*;

use crate::security::credentials::CredentialBroker;
use std::sync::Arc;

/// Resolve o provedor de busca a partir de variáveis de ambiente com fallback seguro.
///
/// REGRAS DE GOVERNANÇA:
/// 1. Padrão incondicional: `MockSearchProvider` (zero rede, zero credenciais).
/// 2. `YUKI_RESEARCH_PROVIDER=brave`: instancia `BraveSearchProvider`.
/// 3. A presença de `YUKI_RESEARCH_PROVIDER=brave` NÃO ativa consultas reais sem `YUKI_RESEARCH_LIVE_ENABLED=true`.
pub fn resolve_search_provider_from_env(
    credential_broker: Arc<dyn CredentialBroker>,
) -> Arc<dyn SearchProvider> {
    let provider_name = std::env::var("YUKI_RESEARCH_PROVIDER")
        .unwrap_or_else(|_| "mock".to_string())
        .to_lowercase();

    if provider_name == "brave" {
        let config = BraveSearchConfig::from_env();
        match BraveSearchProvider::new(config, credential_broker) {
            Ok(brave) => Arc::new(brave),
            Err(e) => {
                tracing::warn!(
                    "Falha ao configurar BraveSearchProvider ({}). Recuando para MockSearchProvider.",
                    e
                );
                Arc::new(MockSearchProvider::new())
            }
        }
    } else {
        Arc::new(MockSearchProvider::new())
    }
}
