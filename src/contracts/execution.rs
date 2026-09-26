use crate::contracts::identifiers::{AttemptId, CapabilityId, CapabilityToken, OperationId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationState {
    Created,
    Dispatched,
    Acknowledged,
    Processing,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRequest {
    pub operation_id: OperationId,
    pub attempt_id: AttemptId,
    pub capability_id: CapabilityId,
    pub authorization_token: CapabilityToken,
    pub input: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub operation_id: OperationId,
    pub attempt_id: AttemptId,
    pub state: OperationState,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
    pub executed_at: DateTime<Utc>,
}
