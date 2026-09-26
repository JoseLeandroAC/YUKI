use crate::contracts::identifiers::{EvidenceId, OperationId};
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
