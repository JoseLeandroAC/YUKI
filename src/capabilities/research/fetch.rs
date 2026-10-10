use crate::capabilities::registry::CapabilityHandler;
use crate::capabilities::research::manifest::fetch_manifest_for_provider;
use crate::capabilities::research::provider::{ContentFetchProvider, MockFetchProvider};
use crate::contracts::capability::CapabilityManifest;
use crate::contracts::errors::YukiError;
use crate::contracts::research::ResearchFetchInput;
use std::sync::Arc;

/// Capability de recuperação controlada de páginas da Yuki (`research.fetch`).
pub struct FetchCapability {
    provider: Arc<dyn ContentFetchProvider>,
}

impl FetchCapability {
    pub fn new(provider: Arc<dyn ContentFetchProvider>) -> Self {
        Self { provider }
    }

    pub fn provider(&self) -> &Arc<dyn ContentFetchProvider> {
        &self.provider
    }
}

impl Default for FetchCapability {
    fn default() -> Self {
        Self::new(Arc::new(MockFetchProvider::new()))
    }
}

impl CapabilityHandler for FetchCapability {
    fn manifest(&self) -> CapabilityManifest {
        fetch_manifest_for_provider(self.provider.is_live())
    }

    fn execute(&self, input: &serde_json::Value) -> Result<serde_json::Value, YukiError> {
        let fetch_input: ResearchFetchInput =
            serde_json::from_value(input.clone()).map_err(|e| {
                YukiError::InvalidRequest(format!(
                    "Falha ao desserializar parâmetros de 'research.fetch': {}",
                    e
                ))
            })?;

        fetch_input.validate()?;

        let result = self.provider.fetch(&fetch_input)?;

        serde_json::to_value(result).map_err(|e| {
            YukiError::ExecutionFailed(format!(
                "Falha ao serializar resultado de 'research.fetch': {}",
                e
            ))
        })
    }
}
