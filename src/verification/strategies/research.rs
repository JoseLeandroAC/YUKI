use crate::contracts::execution::OperationState;
use crate::contracts::identifiers::now_utc;
use crate::contracts::research::compute_sha256;
use crate::contracts::verification::{ObservedEffectState, VerificationResult, VerificationState};
use crate::verification::strategy::{VerificationContext, VerificationStrategy};

// ============================================================================
// Search Verification Strategy (ADR-009, ADR-020)
// ============================================================================

/// Estratégia confiável de verificação estrutural para `research.search`.
///
/// INVARIANTES EPISTÊMICOS (ADR-009, ADR-020):
/// - Conformidade Estrutural != Verdade Factual da Internet.
/// - A estratégia verifica se a execução ocorreu dentro dos limites contratuais,
///   sem atestar veracidade das informações externas ou snippets.
/// - Ausência ou conflito de evidências preserva estritamente `UNKNOWN`.
pub struct SearchVerificationStrategy;

impl SearchVerificationStrategy {
    pub const STRATEGY_ID: &'static str = "research_search_structural";

    pub fn new() -> Self {
        Self
    }
}

impl Default for SearchVerificationStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl VerificationStrategy for SearchVerificationStrategy {
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

        // 1. Evidências conflitantes produzem UNKNOWN/Conflicting
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
                    "Evidências conflitantes encontradas durante a verificação de pesquisa."
                        .to_string(),
            };
        }

        // 2. Tratamento de falha de execução
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
                        "Execução da busca falhou e evidência confirmou ausência de efeito colateral: {}",
                        context.execution.error.as_deref().unwrap_or("falha confirmada")
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
                        "Execução da busca falhou ({}), incerteza preservada como UNKNOWN (ADR-009).",
                        context.execution.error.as_deref().unwrap_or("erro desconhecido")
                    ),
                };
            }
        }

        // 3. Ausência de evidências válidas vinculadas
        if valid_evidences.is_empty() {
            return VerificationResult {
                operation_id: context.operation_id.clone(),
                verification_state: VerificationState::Unknown,
                observed_effect_state: ObservedEffectState::Unknown,
                evidence_refs,
                evaluated_at: now,
                verification_basis:
                    "Ausência de evidências verificáveis de busca; preservado UNKNOWN.".to_string(),
            };
        }

        // 4. Verificação de conformidade estrutural da saída
        let mut structural_valid = false;
        for evidence in &valid_evidences {
            if let Some(results) = evidence.data.get("results").and_then(|v| v.as_array()) {
                let has_query = evidence
                    .data
                    .get("query")
                    .and_then(|v| v.as_str())
                    .is_some();
                let has_provider = evidence
                    .data
                    .get("provider")
                    .and_then(|v| v.as_str())
                    .is_some();
                let has_count = evidence
                    .data
                    .get("results_count")
                    .and_then(|v| v.as_u64())
                    .is_some();

                if has_query && has_provider && has_count {
                    // Validar itens estruturados
                    let all_items_valid = results.iter().all(|item| {
                        let has_cite = item.get("cite_id").and_then(|v| v.as_str()).is_some();
                        let has_url = item.get("url").and_then(|v| v.as_str()).is_some();
                        let has_title = item.get("title").and_then(|v| v.as_str()).is_some();
                        has_cite && has_url && has_title
                    });

                    if all_items_valid {
                        structural_valid = true;
                        break;
                    }
                }
            }
        }

        if structural_valid {
            VerificationResult {
                operation_id: context.operation_id.clone(),
                verification_state: VerificationState::VerifiedSuccess,
                observed_effect_state: ObservedEffectState::ObservedNoMutation,
                evidence_refs,
                evaluated_at: now,
                verification_basis:
                    "Evidência confirmou execução estrutural da busca sem mutação de estado externo (ADR-020)."
                        .to_string(),
            }
        } else {
            VerificationResult {
                operation_id: context.operation_id.clone(),
                verification_state: VerificationState::Unknown,
                observed_effect_state: ObservedEffectState::Unknown,
                evidence_refs,
                evaluated_at: now,
                verification_basis:
                    "Evidência estruturalmente incompleta ou não conforme com o contrato de pesquisa."
                        .to_string(),
            }
        }
    }
}

// ============================================================================
// Fetch Verification Strategy (ADR-009, ADR-020)
// ============================================================================

/// Estratégia confiável de verificação estrutural e integridade de hash para `research.fetch`.
pub struct FetchVerificationStrategy;

impl FetchVerificationStrategy {
    pub const STRATEGY_ID: &'static str = "research_fetch_structural";

    pub fn new() -> Self {
        Self
    }
}

impl Default for FetchVerificationStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl VerificationStrategy for FetchVerificationStrategy {
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

        // 1. Evidências conflitantes
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
                verification_basis: "Evidências conflitantes na recuperação de página.".to_string(),
            };
        }

        // 2. Falha de execução
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
                        "Execução do fetch falhou e evidência confirmou ausência de efeito: {}",
                        context
                            .execution
                            .error
                            .as_deref()
                            .unwrap_or("falha confirmada")
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
                        "Execução do fetch falhou ({}), preservado UNKNOWN (ADR-009).",
                        context
                            .execution
                            .error
                            .as_deref()
                            .unwrap_or("erro desconhecido")
                    ),
                };
            }
        }

        // 3. Ausência de evidências
        if valid_evidences.is_empty() {
            return VerificationResult {
                operation_id: context.operation_id.clone(),
                verification_state: VerificationState::Unknown,
                observed_effect_state: ObservedEffectState::Unknown,
                evidence_refs,
                evaluated_at: now,
                verification_basis:
                    "Ausência de evidências verificáveis de fetch; preservado UNKNOWN.".to_string(),
            };
        }

        // 4. Verificação de conformidade estrutural e integridade de hash SHA-256
        let mut integrity_verified = false;
        for evidence in &valid_evidences {
            let has_url = evidence.data.get("url").and_then(|v| v.as_str()).is_some();
            let has_status = evidence
                .data
                .get("http_status")
                .and_then(|v| v.as_u64())
                .is_some();
            let text_opt = evidence.data.get("extracted_text").and_then(|v| v.as_str());
            let hash_opt = evidence
                .data
                .get("content_hash_sha256")
                .and_then(|v| v.as_str());

            if has_url && has_status {
                if let (Some(text), Some(reported_hash)) = (text_opt, hash_opt) {
                    // Validar se o hash reportado é um SHA-256 hex válido (64 caracteres)
                    let valid_hex_len = reported_hash.len() == 64
                        && reported_hash.chars().all(|c| c.is_ascii_hexdigit());

                    // Verificar correspondência exata do hash SHA-256 computado
                    let computed_hash = compute_sha256(text.as_bytes());
                    if valid_hex_len && computed_hash == reported_hash {
                        integrity_verified = true;
                        break;
                    }
                }
            }
        }

        if integrity_verified {
            VerificationResult {
                operation_id: context.operation_id.clone(),
                verification_state: VerificationState::VerifiedSuccess,
                observed_effect_state: ObservedEffectState::ObservedNoMutation,
                evidence_refs,
                evaluated_at: now,
                verification_basis:
                    "Evidência confirmou integridade estrutural e hash do conteúdo recuperado sem mutação de estado externo (ADR-020)."
                        .to_string(),
            }
        } else {
            VerificationResult {
                operation_id: context.operation_id.clone(),
                verification_state: VerificationState::Unknown,
                observed_effect_state: ObservedEffectState::Unknown,
                evidence_refs,
                evaluated_at: now,
                verification_basis:
                    "Inconsistência na integridade de hash ou estrutura de fetch; preservado UNKNOWN."
                        .to_string(),
            }
        }
    }
}
