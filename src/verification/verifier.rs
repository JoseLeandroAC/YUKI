use std::sync::Arc;

use crate::contracts::execution::ExecutionResult;
use crate::contracts::identifiers::{now_utc, CapabilityId, OperationId};
use crate::contracts::verification::{
    Evidence, ObservedEffectState, VerificationResult, VerificationState,
};
use crate::verification::registry::VerificationStrategyRegistry;
use crate::verification::strategies::echo::EchoVerificationStrategy;
use crate::verification::strategy::VerificationContext;

/// Facade for the dynamic verification subsystem (ADR-009).
///
/// INVARIANTS:
/// - Verification != Truth: A verification result confirms whether evidence satisfies
///   the operation's defined confirmation requirements; it does not claim metaphysical reality.
/// - Verifier does not authorize execution or decide retries.
/// - Preserves Foundation v0.1 compatibility while delegating capability verification
///   to trusted strategies in `VerificationStrategyRegistry`.
pub struct Verifier {
    registry: Arc<VerificationStrategyRegistry>,
}

impl Verifier {
    /// Creates a new `Verifier` with the default trusted strategy registry.
    pub fn new() -> Self {
        Self {
            registry: Arc::new(VerificationStrategyRegistry::new()),
        }
    }

    /// Creates a `Verifier` with a custom trusted strategy registry.
    pub fn with_registry(registry: Arc<VerificationStrategyRegistry>) -> Self {
        Self { registry }
    }

    /// Access to the underlying strategy registry.
    pub fn registry(&self) -> &Arc<VerificationStrategyRegistry> {
        &self.registry
    }

    /// Legacy Foundation facade method.
    ///
    /// Preserves exact Foundation v0.1 signature and semantics for backward compatibility.
    /// Delegates internally to the trusted `EchoVerificationStrategy`.
    pub fn verify(
        &self,
        operation_id: &OperationId,
        execution: &ExecutionResult,
        evidences: &[Evidence],
    ) -> VerificationResult {
        let default_capability = CapabilityId::new("system.echo");
        let context =
            VerificationContext::new(operation_id, &default_capability, execution, evidences);
        self.verify_operation_with_strategy(&context, EchoVerificationStrategy::STRATEGY_ID)
    }

    /// Dynamic verification method resolving strategy from the capability binding.
    pub fn verify_operation(&self, context: &VerificationContext) -> VerificationResult {
        if let Some(strategy) = self
            .registry
            .get_strategy_for_capability(context.capability_id)
        {
            strategy.verify(context)
        } else {
            let strategy_name = self
                .registry
                .get_strategy_id_for_capability(context.capability_id)
                .unwrap_or_else(|| "<nenhuma estratégia vinculada>".to_string());
            let evidence_refs = context
                .valid_evidences()
                .iter()
                .map(|e| e.evidence_id.clone())
                .collect();
            VerificationResult {
                operation_id: context.operation_id.clone(),
                verification_state: VerificationState::Unknown,
                observed_effect_state: ObservedEffectState::Unknown,
                evidence_refs,
                evaluated_at: now_utc(),
                verification_basis: format!(
                    "Estratégia de verificação desconhecida ou não registrada para a capacidade '{}': '{}'",
                    context.capability_id.0, strategy_name
                ),
            }
        }
    }

    /// Dynamic verification method with explicit strategy identifier.
    pub fn verify_operation_with_strategy(
        &self,
        context: &VerificationContext,
        strategy_id: &str,
    ) -> VerificationResult {
        if let Some(strategy) = self.registry.get(strategy_id) {
            strategy.verify(context)
        } else {
            let evidence_refs = context
                .valid_evidences()
                .iter()
                .map(|e| e.evidence_id.clone())
                .collect();
            VerificationResult {
                operation_id: context.operation_id.clone(),
                verification_state: VerificationState::Unknown,
                observed_effect_state: ObservedEffectState::Unknown,
                evidence_refs,
                evaluated_at: now_utc(),
                verification_basis: format!(
                    "Estratégia de verificação desconhecida ou não registrada: '{}'",
                    strategy_id
                ),
            }
        }
    }
}

impl Default for Verifier {
    fn default() -> Self {
        Self::new()
    }
}
