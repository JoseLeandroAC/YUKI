pub mod brave;
pub mod fetch;
pub mod html_extract;
pub mod http_fetch;
pub mod manifest;
pub mod provider;
pub mod registry;
pub mod search;
pub mod ssrf;
pub mod synthesis;

pub use brave::*;
pub use fetch::*;
pub use html_extract::*;
pub use http_fetch::*;
pub use manifest::*;
pub use provider::*;
pub use registry::*;
pub use search::*;
pub use ssrf::*;
pub use synthesis::*;

use crate::contracts::research::ResearchBudgetTracker;
use crate::security::credentials::CredentialBroker;
use std::sync::Arc;

/// Resolve o provedor de busca a partir de variáveis de ambiente com fallback seguro.
///
/// REGRAS DE GOVERNANÇA:
/// 1. Padrão incondicional: `MockSearchProvider` (zero rede, zero credenciais).
/// 2. `YUKI_RESEARCH_PROVIDER=brave`: instancia `BraveSearchProvider`.
/// 3. A presença de `YUKI_RESEARCH_PROVIDER=brave` NÃO ativa consultas reais sem `YUKI_RESEARCH_LIVE_ENABLED=true`.
pub fn resolve_search_provider_from_env_with_tracker(
    credential_broker: Arc<dyn CredentialBroker>,
    tracker: Option<Arc<ResearchBudgetTracker>>,
) -> Arc<dyn SearchProvider> {
    let provider_name = std::env::var("YUKI_RESEARCH_PROVIDER")
        .unwrap_or_else(|_| "mock".to_string())
        .to_lowercase();

    if provider_name == "brave" {
        let mut config = BraveSearchConfig::from_env();
        if let Some(t) = tracker {
            config = config.with_budget_tracker(t);
        }
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

pub fn resolve_search_provider_from_env(
    credential_broker: Arc<dyn CredentialBroker>,
) -> Arc<dyn SearchProvider> {
    resolve_search_provider_from_env_with_tracker(credential_broker, None)
}

/// Resolve o provedor de leitura de páginas (`research.fetch`) a partir de variáveis de ambiente.
///
/// REGRAS DE GOVERNANÇA:
/// 1. Padrão incondicional: `MockFetchProvider` (zero rede).
/// 2. `YUKI_RESEARCH_FETCH_PROVIDER=http`: instancia `HttpContentFetchProvider`.
/// 3. O modo live exige incondicionalmente `YUKI_RESEARCH_LIVE_ENABLED=true` e autorização `egress:web_fetch`.
pub fn resolve_fetch_provider_from_env(
    tracker: Option<Arc<ResearchBudgetTracker>>,
) -> Arc<dyn ContentFetchProvider> {
    let provider_name = std::env::var("YUKI_RESEARCH_FETCH_PROVIDER")
        .unwrap_or_else(|_| "mock".to_string())
        .to_lowercase();

    if provider_name == "http" {
        let t = tracker.unwrap_or_else(|| Arc::new(ResearchBudgetTracker::default()));
        let config = HttpContentFetchConfig::from_env(t);
        Arc::new(HttpContentFetchProvider::new(config))
    } else {
        Arc::new(MockFetchProvider::new())
    }
}
