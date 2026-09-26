use crate::contracts::context::{
    ContextObject, ContextRequest, DataClassification, EpistemicState, Provenance,
};
use crate::contracts::errors::YukiError;
use crate::contracts::identifiers::{now_utc, ContextId};

pub struct ContextBuilder;

impl ContextBuilder {
    pub fn build(req: &ContextRequest) -> Result<ContextObject, YukiError> {
        let now = now_utc();
        let ctx = ContextObject {
            context_id: ContextId::new(),
            purpose: req.purpose.clone(),
            timestamp: now,
            source_refs: vec![req.source.clone()],
            user_input: req.user_input.clone(),
            relevant_data: serde_json::json!({}),
            provenance: Provenance {
                source: req.source.clone(),
                author: "user".to_string(),
                timestamp: now,
                transformations: vec!["purpose_bounded_projection".to_string()],
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
