use crate::contracts::execution::OperationState;
use crate::contracts::identifiers::now_utc;
use crate::contracts::verification::{ObservedEffectState, VerificationResult, VerificationState};
use crate::verification::strategy::{VerificationContext, VerificationStrategy};

/// Trusted verification strategy for `system.echo` (ADR-009).
///
/// Strategy ID: `"echo_exact_match"`
///
/// Properties:
/// - Pure synchronous in-memory evaluation.
/// - Validates binding of evidence to operation and attempt.
/// - Preserves UNKNOWN when evidence is absent, insufficient, or conflicting.
pub struct EchoVerificationStrategy;

impl EchoVerificationStrategy {
    pub const STRATEGY_ID: &'static str = "echo_exact_match";

    pub fn new() -> Self {
        Self
    }
}

impl Default for EchoVerificationStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl VerificationStrategy for EchoVerificationStrategy {
    fn strategy_id(&self) -> &'static str {
        Self::STRATEGY_ID
    }

    fn verify(&self, context: &VerificationContext) -> VerificationResult {
        let valid_evidences = context.valid_evidences();
        let evidence_refs = valid_evidences
            .iter()
            .map(|e| e.evidence_id.clone())
            .collect();
        let now = now_utc();

        // 1. Check for conflicting evidence FIRST
        // EPISTEMIC RULE (ADR-009): Evidence != Truth; conflicting evidence must produce UNKNOWN/Conflicting
        let has_conflict = valid_evidences.iter().any(|e| {
            e.data
                .get("conflict")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
        });

        if has_conflict {
            return VerificationResult {
                operation_id: context.operation_id.clone(),
                verification_state: VerificationState::Unknown,
                observed_effect_state: ObservedEffectState::Conflicting,
                evidence_refs,
                evaluated_at: now,
                verification_basis: "Evidências conflitantes encontradas durante a verificação."
                    .to_string(),
            };
        }

        // 2. If execution itself failed and no conflicting evidence exists
        // INVARIANT: Execution Failure != Proof Of No External Effect
        if context.execution.state == OperationState::Failed {
            return VerificationResult {
                operation_id: context.operation_id.clone(),
                verification_state: VerificationState::VerifiedFailure,
                observed_effect_state: ObservedEffectState::NotObserved,
                evidence_refs,
                evaluated_at: now,
                verification_basis: format!(
                    "Execução da operação falhou: {}",
                    context
                        .execution
                        .error
                        .as_deref()
                        .unwrap_or("erro desconhecido")
                ),
            };
        }

        // 3. If there are NO validly bound evidences, state is UNKNOWN
        // INV-FND-015: UNKNOWN != SUCCESS and UNKNOWN != FAILURE
        if valid_evidences.is_empty() {
            return VerificationResult {
                operation_id: context.operation_id.clone(),
                verification_state: VerificationState::Unknown,
                observed_effect_state: ObservedEffectState::Unknown,
                evidence_refs,
                evaluated_at: now,
                verification_basis:
                    "Ausência de evidências verificáveis; o estado permanece UNKNOWN.".to_string(),
            };
        }

        // 4. Check for positive echo evidence
        let has_confirmed_output = valid_evidences.iter().any(|e| {
            e.data
                .get("valid_echo")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
                || e.data.get("echoed_message").is_some()
        });

        if has_confirmed_output {
            VerificationResult {
                operation_id: context.operation_id.clone(),
                verification_state: VerificationState::VerifiedSuccess,
                observed_effect_state: ObservedEffectState::ObservedNoMutation,
                evidence_refs,
                evaluated_at: now,
                verification_basis:
                    "Evidência confirmou retorno de echo determinístico sem mutação de estado externo."
                        .to_string(),
            }
        } else {
            VerificationResult {
                operation_id: context.operation_id.clone(),
                verification_state: VerificationState::Unknown,
                observed_effect_state: ObservedEffectState::Unknown,
                evidence_refs,
                evaluated_at: now,
                verification_basis: "Evidências incompletas para atestar sucesso ou falha."
                    .to_string(),
            }
        }
    }
}
