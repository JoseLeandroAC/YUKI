use crate::contracts::identifiers::{AttemptId, EvidenceId, OperationId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObservedEffectState {
    NotObserved,
    Unknown,
    ObservedNoMutation,
    ObservedMutation,
    ObservedPartial,
    Conflicting,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationState {
    VerifiedSuccess,
    VerifiedFailure,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub evidence_id: EvidenceId,
    pub source: String,
    pub data: serde_json::Value,
    pub observed_at: DateTime<Utc>,
    pub confidence_basis: String,
    #[serde(default)]
    pub operation_id: Option<OperationId>,
    #[serde(default)]
    pub attempt_id: Option<AttemptId>,
}

impl Evidence {
    pub fn new(
        source: impl Into<String>,
        data: serde_json::Value,
        confidence_basis: impl Into<String>,
    ) -> Self {
        Self {
            evidence_id: EvidenceId::new(),
            source: source.into(),
            data,
            observed_at: crate::contracts::identifiers::now_utc(),
            confidence_basis: confidence_basis.into(),
            operation_id: None,
            attempt_id: None,
        }
    }

    pub fn with_operation(mut self, op_id: OperationId) -> Self {
        self.operation_id = Some(op_id);
        self
    }

    pub fn with_attempt(mut self, attempt_id: AttemptId) -> Self {
        self.attempt_id = Some(attempt_id);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub operation_id: OperationId,
    pub verification_state: VerificationState,
    pub observed_effect_state: ObservedEffectState,
    pub evidence_refs: Vec<EvidenceId>,
    pub evaluated_at: DateTime<Utc>,
    pub verification_basis: String,
}

impl VerificationResult {
    /// Invariant: UNKNOWN is never treated as SUCCESS
    pub fn is_success(&self) -> bool {
        self.verification_state == VerificationState::VerifiedSuccess
    }

    /// Invariant: UNKNOWN is never treated as FAILURE
    pub fn is_failure(&self) -> bool {
        self.verification_state == VerificationState::VerifiedFailure
    }

    /// UNKNOWN is a first-class state
    pub fn is_unknown(&self) -> bool {
        self.verification_state == VerificationState::Unknown
            || self.observed_effect_state == ObservedEffectState::Unknown
    }
}
