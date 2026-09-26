use crate::capabilities::echo::EchoCapability;
use crate::contracts::capability::CapabilityManifest;
use crate::contracts::errors::YukiError;
use crate::contracts::identifiers::CapabilityId;
use std::collections::HashMap;

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
        // Register default initial capabilities for Foundation v0.1
        registry.register(Box::new(EchoCapability::new()));
        registry
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
