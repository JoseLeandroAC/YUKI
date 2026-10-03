use crate::contracts::execution::ExecutionResult;
use crate::contracts::identifiers::{CapabilityId, OperationId};
use crate::contracts::verification::{Evidence, VerificationResult};

/// Context provided to a verification strategy to evaluate an operation's outcome.
///
/// EPISTEMIC RULES (ADR-009):
/// - Attempt identity is derived authoritatively from `execution.attempt_id`.
/// - Evidence is untrusted until bound and verified.
pub struct VerificationContext<'a> {
    pub operation_id: &'a OperationId,
    pub capability_id: &'a CapabilityId,
    pub execution: &'a ExecutionResult,
    pub evidences: &'a [Evidence],
}

impl<'a> VerificationContext<'a> {
    pub fn new(
        operation_id: &'a OperationId,
        capability_id: &'a CapabilityId,
        execution: &'a ExecutionResult,
        evidences: &'a [Evidence],
    ) -> Self {
        Self {
            operation_id,
            capability_id,
            execution,
            evidences,
        }
    }

    /// Filters evidences that are validly bound to this operation and attempt.
    ///
    /// BINDING INVARIANTS:
    /// - (Some(op), Some(att)): Both operation and attempt must match context.
    /// - (Some(op), None): Operation matches, attempt unstated.
    /// - (None, Some(_)): REJECTED. Attempt without operation is semantically invalid.
    /// - (None, None): Retained for backward compatibility with legacy Foundation v0.1 tests.
    pub fn valid_evidences(&self) -> Vec<&'a Evidence> {
        self.evidences
            .iter()
            .filter(|e| match (&e.operation_id, &e.attempt_id) {
                (Some(op), Some(att)) => {
                    op == self.operation_id && att == &self.execution.attempt_id
                }
                (Some(op), None) => op == self.operation_id,
                (None, Some(_)) => false,
                (None, None) => true,
            })
            .collect()
    }
}

/// Abstract verification strategy for capability effect verification.
///
/// Invariant:
/// Strategies are pure, deterministic in-memory evaluations of evidence against execution expectations.
pub trait VerificationStrategy: Send + Sync {
    /// Unique identifier for this strategy (e.g. "echo_exact_match").
    fn strategy_id(&self) -> &'static str;

    /// Evaluates evidence for an operation and produces a typed verification result.
    fn verify(&self, context: &VerificationContext) -> VerificationResult;
}
