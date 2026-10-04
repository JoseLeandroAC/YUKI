use crate::contracts::execution::OperationState;
use crate::contracts::identifiers::now_utc;
use crate::contracts::verification::{ObservedEffectState, VerificationResult, VerificationState};
use crate::verification::strategy::{VerificationContext, VerificationStrategy};
use chrono::DateTime;

/// Trusted verification strategy for `system.time` (ADR-009).
///
/// Strategy ID: `"system_time_window"`
///
/// Properties:
/// - Verifies that the produced `utc_timestamp` parses as RFC 3339.
/// - Verifies that the timestamp falls within a bounded window relative to execution time.
/// - Asserts `ObservedNoMutation` since reading time causes no mutations.
/// - Epistemic rule: `Verification != Truth`.
pub struct TimeVerificationStrategy {
    tolerance_ms: i64,
}

impl TimeVerificationStrategy {
    pub const STRATEGY_ID: &'static str = "system_time_window";
    pub const DEFAULT_TOLERANCE_MS: i64 = 5000;

    pub fn new() -> Self {
        Self {
            tolerance_ms: Self::DEFAULT_TOLERANCE_MS,
        }
    }

    pub fn with_tolerance_ms(tolerance_ms: i64) -> Self {
        Self { tolerance_ms }
    }
}

impl Default for TimeVerificationStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl VerificationStrategy for TimeVerificationStrategy {
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

        // 1. Check for conflicting evidence
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
                verification_basis:
                    "Evidências conflitantes encontradas durante a verificação de system.time."
                        .to_string(),
            };
        }

        // 2. Handle execution failure
        // Invariant: Execution Failure != Proof Of No External Effect
        if context.execution.state == OperationState::Failed {
            let has_confirmed_failure = valid_evidences.iter().any(|e| {
                e.data
                    .get("confirmed_failure")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
                    || e.data
                        .get("effect_absent")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false)
            });

            if has_confirmed_failure {
                return VerificationResult {
                    operation_id: context.operation_id.clone(),
                    verification_state: VerificationState::VerifiedFailure,
                    observed_effect_state: ObservedEffectState::NotObserved,
                    evidence_refs,
                    evaluated_at: now,
                    verification_basis: format!(
                        "Execução de system.time falhou com evidência confirmada: {}",
                        context.execution.error.as_deref().unwrap_or("falha")
                    ),
                };
            } else {
                return VerificationResult {
                    operation_id: context.operation_id.clone(),
                    verification_state: VerificationState::Unknown,
                    observed_effect_state: ObservedEffectState::Unknown,
                    evidence_refs,
                    evaluated_at: now,
                    verification_basis: format!(
                        "Execução falhou sem evidência independente da ausência de efeito: {}",
                        context
                            .execution
                            .error
                            .as_deref()
                            .unwrap_or("erro desconhecido")
                    ),
                };
            }
        }

        // 3. Handle completed execution
        let output = match &context.execution.output {
            Some(out) => out,
            None => {
                return VerificationResult {
                    operation_id: context.operation_id.clone(),
                    verification_state: VerificationState::VerifiedFailure,
                    observed_effect_state: ObservedEffectState::NotObserved,
                    evidence_refs,
                    evaluated_at: now,
                    verification_basis: "Saída ausente na execução concluída de system.time."
                        .to_string(),
                };
            }
        };

        let timestamp_str = match output.get("utc_timestamp").and_then(|v| v.as_str()) {
            Some(ts) => ts,
            None => {
                return VerificationResult {
                    operation_id: context.operation_id.clone(),
                    verification_state: VerificationState::VerifiedFailure,
                    observed_effect_state: ObservedEffectState::NotObserved,
                    evidence_refs,
                    evaluated_at: now,
                    verification_basis:
                        "Campo 'utc_timestamp' ausente ou inválido na saída de system.time."
                            .to_string(),
                };
            }
        };

        // Parse RFC 3339 timestamp
        let parsed_time = match DateTime::parse_from_rfc3339(timestamp_str) {
            Ok(dt) => dt.with_timezone(&chrono::Utc),
            Err(e) => {
                return VerificationResult {
                    operation_id: context.operation_id.clone(),
                    verification_state: VerificationState::VerifiedFailure,
                    observed_effect_state: ObservedEffectState::NotObserved,
                    evidence_refs,
                    evaluated_at: now,
                    verification_basis: format!("Falha ao analisar utc_timestamp RFC 3339: {}", e),
                };
            }
        };

        // Verify window tolerance against execution timestamp
        let delta_ms = (parsed_time - context.execution.executed_at)
            .num_milliseconds()
            .abs();

        if delta_ms > self.tolerance_ms {
            return VerificationResult {
                operation_id: context.operation_id.clone(),
                verification_state: VerificationState::VerifiedFailure,
                observed_effect_state: ObservedEffectState::NotObserved,
                evidence_refs,
                evaluated_at: now,
                verification_basis: format!(
                    "Diferença temporal ({} ms) excedeu a tolerância permitida ({} ms)",
                    delta_ms, self.tolerance_ms
                ),
            };
        }

        VerificationResult {
            operation_id: context.operation_id.clone(),
            verification_state: VerificationState::VerifiedSuccess,
            observed_effect_state: ObservedEffectState::ObservedNoMutation,
            evidence_refs,
            evaluated_at: now,
            verification_basis: format!(
                "Timestamp UTC validado dentro da janela de tolerância de {} ms (delta: {} ms).",
                self.tolerance_ms, delta_ms
            ),
        }
    }
}
