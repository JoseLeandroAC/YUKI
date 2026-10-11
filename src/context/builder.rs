use crate::context::personal_context::PersonalContext;
use crate::contracts::context::{
    ContextObject, ContextRequest, DataClassification, EpistemicState, Provenance,
};
use crate::contracts::errors::YukiError;
use crate::contracts::identifiers::{now_utc, ContextId};

pub struct ContextBuilder;

impl ContextBuilder {
    pub fn build(req: &ContextRequest) -> Result<ContextObject, YukiError> {
        Self::build_with_personal_context(req, None)
    }

    pub fn build_with_personal_context(
        req: &ContextRequest,
        personal_context: Option<PersonalContext>,
    ) -> Result<ContextObject, YukiError> {
        let now = now_utc();
        let relevant_data = if let Some(ref pc) = personal_context {
            serde_json::json!({
                "personal_context": pc,
                "personal_prompt_section": pc.to_prompt_section(),
                "referenced_memory_ids": pc.referenced_memory_ids,
            })
        } else {
            serde_json::json!({})
        };

        let mut transformations = vec!["purpose_bounded_projection".to_string()];
        if personal_context.is_some() {
            transformations.push("personal_context_enrichment".to_string());
        }

        let ctx = ContextObject {
            context_id: ContextId::new(),
            purpose: req.purpose.clone(),
            timestamp: now,
            source_refs: vec![req.source.clone()],
            user_input: req.user_input.clone(),
            relevant_data,
            provenance: Provenance {
                source: req.source.clone(),
                author: "user".to_string(),
                timestamp: now,
                transformations,
                evidence_refs: Vec::new(),
            },
            freshness: now,
            epistemic_state: EpistemicState::Declared,
            classification: DataClassification::Personal,
        };

        // Enforce INV-FND-005 & ADR-015: No credentials or secrets in Context
        if let Err(err) = ctx.assert_no_secrets() {
            return Err(YukiError::ContextError(err));
        }

        Ok(ctx)
    }
}
