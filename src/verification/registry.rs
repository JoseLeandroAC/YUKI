use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::contracts::identifiers::CapabilityId;
use crate::verification::strategies::echo::EchoVerificationStrategy;
use crate::verification::strategy::VerificationStrategy;

/// Trusted registry of verification strategies owned by Yuki's control plane (ADR-009).
///
/// INVARIANTS:
/// - Manifest Field != Automatic Trust: Plugins/models cannot inject arbitrary verification logic.
/// - Only trusted strategies registered in this registry can evaluate operation outcomes.
/// - Unregistered strategies produce `VerificationState::Unknown`, never false success or false failure.
pub struct VerificationStrategyRegistry {
    strategies: RwLock<HashMap<String, Arc<dyn VerificationStrategy>>>,
    capability_bindings: RwLock<HashMap<CapabilityId, String>>,
}

impl VerificationStrategyRegistry {
    /// Creates a new registry with default trusted strategies and system bindings.
    pub fn new() -> Self {
        let registry = Self {
            strategies: RwLock::new(HashMap::new()),
            capability_bindings: RwLock::new(HashMap::new()),
        };

        // 1. Register default trusted strategies
        registry.register_strategy(Arc::new(EchoVerificationStrategy::new()));

        // 2. Register trusted capability-to-strategy bindings
        registry.bind_capability(
            CapabilityId::new("system.echo"),
            EchoVerificationStrategy::STRATEGY_ID,
        );

        registry
    }

    /// Registers a trusted strategy implementation.
    pub fn register_strategy(&self, strategy: Arc<dyn VerificationStrategy>) {
        let mut lock = self
            .strategies
            .write()
            .expect("lock poisoned in VerificationStrategyRegistry");
        lock.insert(strategy.strategy_id().to_string(), strategy);
    }

    /// Binds a capability to a trusted strategy identifier.
    ///
    /// Control-plane invariant:
    /// Only Yuki-controlled registration can bind capabilities to verification strategies.
    pub fn bind_capability(&self, capability_id: CapabilityId, strategy_id: impl Into<String>) {
        let mut lock = self
            .capability_bindings
            .write()
            .expect("lock poisoned in VerificationStrategyRegistry");
        lock.insert(capability_id, strategy_id.into());
    }

    /// Resolves a strategy by its unique identifier.
    pub fn get(&self, strategy_id: &str) -> Option<Arc<dyn VerificationStrategy>> {
        let lock = self
            .strategies
            .read()
            .expect("lock poisoned in VerificationStrategyRegistry");
        lock.get(strategy_id).cloned()
    }

    /// Resolves the trusted verification strategy for a capability.
    pub fn get_strategy_for_capability(
        &self,
        capability_id: &CapabilityId,
    ) -> Option<Arc<dyn VerificationStrategy>> {
        let strategy_id = {
            let bindings = self
                .capability_bindings
                .read()
                .expect("lock poisoned in VerificationStrategyRegistry");
            bindings.get(capability_id).cloned()?
        };
        self.get(&strategy_id)
    }

    /// Returns the strategy ID bound to a capability, if any.
    pub fn get_strategy_id_for_capability(&self, capability_id: &CapabilityId) -> Option<String> {
        let bindings = self
            .capability_bindings
            .read()
            .expect("lock poisoned in VerificationStrategyRegistry");
        bindings.get(capability_id).cloned()
    }
}

impl Default for VerificationStrategyRegistry {
    fn default() -> Self {
        Self::new()
    }
}
