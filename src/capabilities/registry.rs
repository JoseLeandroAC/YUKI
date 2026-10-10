use crate::capabilities::echo::EchoCapability;
use crate::capabilities::info::InfoCapability;
use crate::capabilities::research::fetch::FetchCapability;
use crate::capabilities::research::search::SearchCapability;
use crate::capabilities::time::TimeCapability;
use crate::contracts::capability::CapabilityManifest;
use crate::contracts::errors::YukiError;
use crate::contracts::identifiers::CapabilityId;
use std::collections::HashMap;
use std::sync::Arc;

pub trait CapabilityHandler: Send + Sync {
    fn manifest(&self) -> CapabilityManifest;
    fn execute(&self, input: &serde_json::Value) -> Result<serde_json::Value, YukiError>;
}

pub struct CapabilityRegistry {
    handlers: HashMap<String, Box<dyn CapabilityHandler>>,
}

impl CapabilityRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            handlers: HashMap::new(),
        };
        // Register default capabilities for Yuki (MVP-1 + Research v1 Marco 1 Mocks)
        registry.register(Box::new(EchoCapability::new()));
        registry.register(Box::new(TimeCapability::new()));
        registry.register(Box::new(InfoCapability::new()));
        registry.register(Box::new(SearchCapability::default()));
        registry.register(Box::new(FetchCapability::default()));
        registry
    }

    pub fn with_search_provider(
        mut self,
        provider: Arc<dyn crate::capabilities::research::SearchProvider>,
    ) -> Self {
        self.register(Box::new(SearchCapability::new(provider)));
        self
    }

    pub fn with_fetch_provider(
        mut self,
        provider: Arc<dyn crate::capabilities::research::ContentFetchProvider>,
    ) -> Self {
        self.register(Box::new(FetchCapability::new(provider)));
        self
    }

    pub fn register(&mut self, handler: Box<dyn CapabilityHandler>) {
        let id = handler.manifest().id.0;
        self.handlers.insert(id, handler);
    }

    pub fn get_manifest(&self, id: &CapabilityId) -> Option<CapabilityManifest> {
        self.handlers.get(&id.0).map(|h| h.manifest())
    }

    pub fn get_handler(&self, id: &CapabilityId) -> Option<&dyn CapabilityHandler> {
        self.handlers.get(&id.0).map(|h| h.as_ref())
    }

    pub fn has_capability(&self, id: &CapabilityId) -> bool {
        self.handlers.contains_key(&id.0)
    }

    pub fn list_capabilities(&self) -> Vec<CapabilityManifest> {
        self.handlers.values().map(|h| h.manifest()).collect()
    }
}

impl Default for CapabilityRegistry {
    fn default() -> Self {
        Self::new()
    }
}
