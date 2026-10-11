pub mod brave;
pub mod encoding;
pub mod fetch;
pub mod html_extract;
pub mod http_fetch;
pub mod langsearch;
pub mod manifest;
pub mod provider;
pub mod registry;
pub mod search;
pub mod ssrf;
pub mod synthesis;

pub use brave::*;
pub use encoding::*;
pub use fetch::*;
pub use html_extract::*;
pub use http_fetch::*;
pub use langsearch::*;
pub use manifest::*;
pub use provider::*;
pub use registry::*;
pub use search::*;
pub use ssrf::*;
pub use synthesis::*;

use crate::contracts::research::ResearchBudgetTracker;
use crate::security::credentials::CredentialBroker;
use std::sync::Arc;

/// Enumeração canônica dos tipos de provedores de busca textual configuráveis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchProviderChoice {
    Mock,
    Brave,
    LangSearch,
}

impl SearchProviderChoice {
    /// Determina o provedor a partir de variáveis de ambiente com precedência estrita:
    /// 1. `YUKI_RESEARCH_SEARCH_PROVIDER`
    /// 2. `YUKI_RESEARCH_PROVIDER`
    /// 3. Padrão: `Mock`
    pub fn from_env() -> Self {
        let name = std::env::var("YUKI_RESEARCH_SEARCH_PROVIDER")
            .or_else(|_| std::env::var("YUKI_RESEARCH_PROVIDER"))
            .unwrap_or_else(|_| "mock".to_string())
            .trim()
            .to_lowercase();

        match name.as_str() {
            "brave" => Self::Brave,
            "langsearch" => Self::LangSearch,
            _ => Self::Mock,
        }
    }
}

/// Resolve o provedor de busca a partir de variáveis de ambiente com fallback seguro.
///
/// REGRAS DE GOVERNANÇA:
/// 1. Padrão incondicional: `MockSearchProvider` (zero rede, zero credenciais).
/// 2. `YUKI_RESEARCH_SEARCH_PROVIDER=brave` ou `YUKI_RESEARCH_PROVIDER=brave`: instancia `BraveSearchProvider`.
/// 3. `YUKI_RESEARCH_SEARCH_PROVIDER=langsearch` ou `YUKI_RESEARCH_PROVIDER=langsearch`: instancia `LangSearchProvider`.
/// 4. A configuração de provedor real NÃO ativa consultas de rede sem `YUKI_RESEARCH_LIVE_ENABLED=true`.
pub fn resolve_search_provider_from_env_with_tracker(
    credential_broker: Arc<dyn CredentialBroker>,
    tracker: Option<Arc<ResearchBudgetTracker>>,
) -> Arc<dyn SearchProvider> {
    match SearchProviderChoice::from_env() {
        SearchProviderChoice::Brave => {
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
        }
        SearchProviderChoice::LangSearch => {
            let mut config = LangSearchConfig::from_env();
            if let Some(t) = tracker {
                config = config.with_budget_tracker(t);
            }
            match LangSearchProvider::new(config, credential_broker) {
                Ok(langsearch) => Arc::new(langsearch),
                Err(e) => {
                    tracing::warn!(
                        "Falha ao configurar LangSearchProvider ({}). Recuando para MockSearchProvider.",
                        e
                    );
                    Arc::new(MockSearchProvider::new())
                }
            }
        }
        SearchProviderChoice::Mock => Arc::new(MockSearchProvider::new()),
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
