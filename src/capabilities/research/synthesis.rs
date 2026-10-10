// ============================================================================
// Yuki Personal AI Platform — Research v1 (Marco 4)
// Governed Synthesis & Verifiable Citations Validator
// ============================================================================

use crate::capabilities::research::registry::ObservedSourceRegistry;
use crate::contracts::research::{ResearchSynthesis, SynthesisStatus, VerifiedCitation};
use std::collections::BTreeSet;

/// Validador governado de síntese e resolução de citações (ADR-020).
///
/// Implementa a separação ontológica:
/// 1. `Model Output != Command != Authorization`: Propostas do modelo com fontes são candidatas não confiáveis.
/// 2. `Verification != Truth`: Validar que uma citação existe e tem hash íntegro atesta proveniência,
///    mas NÃO constitui comprovação de veracidade factual no mundo real.
pub struct SynthesisValidator;

impl SynthesisValidator {
    pub const VERIFICATION_DISCLAIMER: &'static str =
        "Aviso Ontológico (ADR-020): Validação de citações atesta existência e integridade de transporte criptográfico da fonte observada no registro deste turno. Não constitui, isoladamente, atestado de veracidade factual no mundo real (Verification != Truth).";

    /// Extrai identificadores de citação no formato `[src:N]` de forma determinística e sem dependências externas.
    pub fn extract_citations(text: &str) -> BTreeSet<String> {
        let mut citations = BTreeSet::new();
        let mut rest = text;
        while let Some(start_idx) = rest.find("[src:") {
            let after_bracket = &rest[start_idx..];
            if let Some(end_idx) = after_bracket.find(']') {
                let candidate = &after_bracket[..=end_idx];
                let inner = &candidate[5..candidate.len() - 1]; // Após "[src:" e antes de "]"
                if !inner.is_empty()
                    && inner
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
                {
                    citations.insert(candidate.to_string());
                }
                rest = &after_bracket[end_idx + 1..];
            } else {
                break;
            }
        }
        citations
    }

    /// Valida e resolve as citações contidas no texto gerado pelo modelo contra o registro governado.
    pub fn validate(raw_answer: &str, registry: &ObservedSourceRegistry) -> ResearchSynthesis {
        let cited_ids = Self::extract_citations(raw_answer);

        let mut verified_citations = Vec::new();
        let mut unresolved_citations = Vec::new();
        let mut limitations = Vec::new();

        for cite_tag in cited_ids {
            let bare_id = cite_tag.trim_start_matches('[').trim_end_matches(']');
            if let Some(obs) = registry.get_observation_by_cite_id(bare_id) {
                let is_full_page =
                    obs.source_kind == crate::contracts::research::SourceKind::DirectSource;
                verified_citations.push(VerifiedCitation {
                    cite_id: cite_tag.clone(),
                    observation_id: obs.observation_id.clone(),
                    canonical_url: obs.final_url.clone(),
                    title: obs.title.clone(),
                    source_kind: obs.source_kind,
                    is_full_page,
                    content_hash_sha256: obs.content_hash_sha256.clone(),
                });
            } else {
                unresolved_citations.push(cite_tag.clone());
            }
        }

        let total_available = registry.observations().len();
        let status = if !unresolved_citations.is_empty() {
            if !verified_citations.is_empty() {
                limitations.push(format!(
                    "Citações não resolvidas identificadas: {}. As referências não existem no registro de evidências deste turno.",
                    unresolved_citations.join(", ")
                ));
                SynthesisStatus::PartiallyVerified
            } else {
                limitations.push(format!(
                    "Todas as citações fornecidas ({}) são inexistentes ou pertencem a escopo não autorizado.",
                    unresolved_citations.join(", ")
                ));
                SynthesisStatus::UnverifiedClaims
            }
        } else if !verified_citations.is_empty() {
            SynthesisStatus::FullyVerified
        } else if total_available > 0 {
            // Havia fontes disponíveis, mas nenhuma citação foi incluída no texto
            limitations.push(
                "Resposta formulada sem vincular citações explícitas às fontes observadas disponíveis no turno."
                    .to_string(),
            );
            SynthesisStatus::UnverifiedClaims
        } else {
            // Nenhuma evidência de pesquisa foi requisitada no turno (conversa padrão)
            SynthesisStatus::FullyVerified
        };

        let mut final_answer = raw_answer.to_string();
        if !unresolved_citations.is_empty() {
            final_answer.push_str(&format!(
                "\n\n[Limitação de Verificação: Citação(ões) {} não puderam ser resolvidas no registro governado deste turno.]",
                unresolved_citations.join(", ")
            ));
        }

        ResearchSynthesis {
            answer_text: final_answer,
            citations: verified_citations,
            unresolved_citations,
            limitations,
            status,
            verification_disclaimer: Self::VERIFICATION_DISCLAIMER.to_string(),
        }
    }
}
