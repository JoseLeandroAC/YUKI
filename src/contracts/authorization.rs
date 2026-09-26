use crate::contracts::capability::RiskClass;
use crate::contracts::identifiers::{CapabilityId, CapabilityToken, ContextId, OperationId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationRequest {
    pub operation_id: OperationId,
    pub capability_id: CapabilityId,
    pub context_id: ContextId,
    pub caller_id: String,
    pub input_summary: serde_json::Value,
    pub risk_class: RiskClass,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthorizationDecision {
    Allow {
        token: CapabilityToken,
        authorized_at: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    },
    Deny {
        reason: String,
    },
    RequiresApproval {
        reason: String,
    },
}

impl AuthorizationDecision {
    pub fn is_allowed(&self) -> bool {
        matches!(self, AuthorizationDecision::Allow { .. })
    }

    pub fn get_token(&self) -> Option<&CapabilityToken> {
        match self {
            AuthorizationDecision::Allow { token, .. } => Some(token),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRule {
    pub capability_id: CapabilityId,
    pub allowed_callers: Vec<String>,
    pub max_risk: RiskClass,
}
