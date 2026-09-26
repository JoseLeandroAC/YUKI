use crate::contracts::identifiers::{now_utc, ContextId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EpistemicState {
    Declared,
    Confirmed,
    Verified,
    Validated,
    Unverified,
    Inferred,
    Hypothesis,
    Stale,
    Conflicting,
    Unknown,
    TaintedExternal,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DataClassification {
    Public,
    Personal,
    Sensitive,
    Restricted,
    Secret,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provenance {
    pub source: String,
    pub author: String,
    pub timestamp: DateTime<Utc>,
    pub transformations: Vec<String>,
    pub evidence_refs: Vec<String>,
}

impl Default for Provenance {
    fn default() -> Self {
        Self {
            source: "user_input".to_string(),
            author: "user".to_string(),
            timestamp: now_utc(),
            transformations: Vec::new(),
            evidence_refs: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextObject {
    pub context_id: ContextId,
    pub purpose: String,
    pub timestamp: DateTime<Utc>,
    pub source_refs: Vec<String>,
    pub user_input: String,
    pub relevant_data: serde_json::Value,
    pub provenance: Provenance,
    pub freshness: DateTime<Utc>,
    pub epistemic_state: EpistemicState,
    pub classification: DataClassification,
}

impl ContextObject {
    /// Validates that no raw credentials or forbidden secrets are present in context
    pub fn assert_no_secrets(&self) -> Result<(), String> {
        let serialized = serde_json::to_string(self)
            .unwrap_or_default()
            .to_lowercase();
        let forbidden_patterns = [
            "password",
            "api_key",
            "apikey",
            "private_key",
            "secret_key",
            "refresh_token",
            "bearer ",
            "access_token",
        ];

        for pattern in &forbidden_patterns {
            if serialized.contains(pattern) {
                return Err(format!(
                    "Context contains forbidden secret pattern: '{}'",
                    pattern
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextRequest {
    pub purpose: String,
    pub user_input: String,
    pub source: String,
}
