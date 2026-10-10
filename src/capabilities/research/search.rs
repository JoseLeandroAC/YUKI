use crate::capabilities::registry::CapabilityHandler;
use crate::capabilities::research::manifest::search_manifest;
use crate::capabilities::research::provider::{MockSearchProvider, SearchProvider};
use crate::contracts::capability::CapabilityManifest;
use crate::contracts::errors::YukiError;
use crate::contracts::research::ResearchSearchInput;
use std::sync::Arc;

/// Capability de pesquisa textual governada da Yuki (`research.search`).
pub struct SearchCapability {
    provider: Arc<dyn SearchProvider>,
}

impl SearchCapability {
    pub fn new(provider: Arc<dyn SearchProvider>) -> Self {
        Self { provider }
    }
}

impl Default for SearchCapability {
    fn default() -> Self {
        Self::new(Arc::new(MockSearchProvider::new()))
    }
}

impl CapabilityHandler for SearchCapability {
    fn manifest(&self) -> CapabilityManifest {
        search_manifest()
    }

    fn execute(&self, input: &serde_json::Value) -> Result<serde_json::Value, YukiError> {
        let search_input: ResearchSearchInput =
            serde_json::from_value(input.clone()).map_err(|e| {
                YukiError::InvalidRequest(format!(
                    "Falha ao desserializar parâmetros de 'research.search': {}",
                    e
                ))
            })?;

        search_input.validate()?;

        let result = self.provider.search(&search_input)?;

        serde_json::to_value(result).map_err(|e| {
            YukiError::ExecutionFailed(format!(
                "Falha ao serializar resultado de 'research.search': {}",
                e
            ))
        })
    }
}
