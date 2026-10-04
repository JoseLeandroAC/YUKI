use crate::contracts::execution::OperationState;
use crate::contracts::identifiers::now_utc;
use crate::contracts::verification::{ObservedEffectState, VerificationResult, VerificationState};
use crate::verification::strategy::{VerificationContext, VerificationStrategy};
use std::collections::HashSet;

/// Trusted verification strategy for `system.info` (ADR-009, ADR-015, ADR-016).
///
/// Strategy ID: `"system_info_schema"`
///
/// Properties:
/// - Enforces strict ALLOWLIST schema validation: output keys MUST be exactly `os`, `arch`, `yuki_version`.
/// - Any unexpected or forbidden keys trigger `VerifiedFailure`.
/// - All allowed fields must be non-empty strings.
/// - Asserts `ObservedNoMutation`.
pub struct InfoVerificationStrategy;

impl InfoVerificationStrategy {
    pub const STRATEGY_ID: &'static str = "system_info_schema";

    pub fn new() -> Self {
        Self
    }
}

impl Default for InfoVerificationStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl VerificationStrategy for InfoVerificationStrategy {
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
                    "Evidências conflitantes encontradas durante a verificação de system.info."
                        .to_string(),
            };
        }

        // 2. Handle execution failure
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
                        "Execução de system.info falhou com evidência confirmada: {}",
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
                    verification_basis: "Saída ausente na execução de system.info.".to_string(),
                };
            }
        };

        let map = match output.as_object() {
            Some(m) => m,
            None => {
                return VerificationResult {
                    operation_id: context.operation_id.clone(),
                    verification_state: VerificationState::VerifiedFailure,
                    observed_effect_state: ObservedEffectState::NotObserved,
                    evidence_refs,
                    evaluated_at: now,
                    verification_basis: "Saída de system.info deve ser um objeto JSON.".to_string(),
                };
            }
        };

        // Strict allowlist validation
        let expected_keys: HashSet<&str> = ["os", "arch", "yuki_version"].into_iter().collect();
        let actual_keys: HashSet<&str> = map.keys().map(|k| k.as_str()).collect();

        // Any extra key not in allowlist triggers failure
        let unexpected_keys: Vec<&str> = actual_keys.difference(&expected_keys).copied().collect();
        if !unexpected_keys.is_empty() {
            return VerificationResult {
                operation_id: context.operation_id.clone(),
                verification_state: VerificationState::VerifiedFailure,
                observed_effect_state: ObservedEffectState::NotObserved,
                evidence_refs,
                evaluated_at: now,
                verification_basis: format!(
                    "Violação de allowlist em system.info: chaves não autorizadas detectadas: {:?}",
                    unexpected_keys
                ),
            };
        }

        // Any missing key triggers failure
        let missing_keys: Vec<&str> = expected_keys.difference(&actual_keys).copied().collect();
        if !missing_keys.is_empty() {
            return VerificationResult {
                operation_id: context.operation_id.clone(),
                verification_state: VerificationState::VerifiedFailure,
                observed_effect_state: ObservedEffectState::NotObserved,
                evidence_refs,
                evaluated_at: now,
                verification_basis: format!(
                    "Chaves obrigatórias ausentes na saída de system.info: {:?}",
                    missing_keys
                ),
            };
        }

        // Validate values are non-empty strings
        for key in &expected_keys {
            match map.get(*key).and_then(|v| v.as_str()) {
                Some(val) if !val.trim().is_empty() => {}
                _ => {
                    return VerificationResult {
                        operation_id: context.operation_id.clone(),
                        verification_state: VerificationState::VerifiedFailure,
                        observed_effect_state: ObservedEffectState::NotObserved,
                        evidence_refs,
                        evaluated_at: now,
                        verification_basis: format!(
                            "Campo '{}' deve ser uma string não vazia.",
                            key
                        ),
                    };
                }
            }
        }

        VerificationResult {
            operation_id: context.operation_id.clone(),
            verification_state: VerificationState::VerifiedSuccess,
            observed_effect_state: ObservedEffectState::ObservedNoMutation,
            evidence_refs,
            evaluated_at: now,
            verification_basis: "Saída de system.info validada com sucesso contra o allowlist restrito de minimização de dados."
                .to_string(),
        }
    }
}
