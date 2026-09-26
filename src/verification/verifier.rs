use crate::contracts::execution::{ExecutionResult, OperationState};
use crate::contracts::identifiers::{now_utc, OperationId};
use crate::contracts::verification::{
    Evidence, ObservedEffectState, VerificationResult, VerificationState,
};

pub struct Verifier;

impl Verifier {
    pub fn new() -> Self {
        Self
    }

    /// Evaluates evidence for an operation and determines verification outcome.
    /// Preserves UNKNOWN when evidence is absent, insufficient, or inconclusive.
    pub fn verify(
        &self,
        operation_id: &OperationId,
        execution: &ExecutionResult,
        evidences: &[Evidence],
    ) -> VerificationResult {
        let evidence_refs = evidences.iter().map(|e| e.evidence_id.clone()).collect();
        let now = now_utc();

        // If execution itself failed, verification confirms failure
        if execution.state == OperationState::Failed {
            return VerificationResult {
                operation_id: operation_id.clone(),
                verification_state: VerificationState::VerifiedFailure,
                observed_effect_state: ObservedEffectState::NotObserved,
                evidence_refs,
                evaluated_at: now,
                verification_basis: format!(
                    "Execução da operação falhou: {}",
                    execution.error.as_deref().unwrap_or("erro desconhecido")
                ),
            };
        }

        // If there is NO evidence at all, we CANNOT assume success!
        // INV-FND-015: UNKNOWN != SUCCESS
        if evidences.is_empty() {
            return VerificationResult {
                operation_id: operation_id.clone(),
                verification_state: VerificationState::Unknown,
                observed_effect_state: ObservedEffectState::Unknown,
                evidence_refs,
                evaluated_at: now,
                verification_basis:
                    "Ausência de evidências verificáveis; o estado permanece UNKNOWN.".to_string(),
            };
        }

        // Check if any evidence explicitly indicates failure or conflict
        let has_conflict = evidences.iter().any(|e| {
            e.data
                .get("conflict")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
        });

        if has_conflict {
            return VerificationResult {
                operation_id: operation_id.clone(),
                verification_state: VerificationState::Unknown,
                observed_effect_state: ObservedEffectState::Conflicting,
                evidence_refs,
                evaluated_at: now,
                verification_basis: "Evidências conflitantes encontradas durante a verificação."
                    .to_string(),
            };
        }

        // Check for positive evidence
        let has_confirmed_output = evidences.iter().any(|e| {
            e.data
                .get("valid_echo")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
                || e.data.get("echoed_message").is_some()
        });

        if has_confirmed_output {
            VerificationResult {
                operation_id: operation_id.clone(),
                verification_state: VerificationState::VerifiedSuccess,
                observed_effect_state: ObservedEffectState::ObservedNoMutation,
                evidence_refs,
                evaluated_at: now,
                verification_basis: "Evidência confirmou retorno de echo determinístico sem mutação de estado externo.".to_string(),
            }
        } else {
            VerificationResult {
                operation_id: operation_id.clone(),
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

impl Default for Verifier {
    fn default() -> Self {
        Self::new()
    }
}
