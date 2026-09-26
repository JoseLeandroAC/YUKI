use crate::contracts::identifiers::{CapabilityId, OperationId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RiskClass {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SideEffects {
    None,
    StateMutation,
    ExternalMutation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityManifest {
    pub id: CapabilityId,
    pub version: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub output_schema: serde_json::Value,
    pub required_permissions: Vec<String>,
    pub risk_class: RiskClass,
    pub side_effects: SideEffects,
    pub network_required: bool,
    pub filesystem_required: bool,
    pub secrets_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityRequest {
    pub operation_id: OperationId,
    pub capability_id: CapabilityId,
    pub input: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityResponse {
    pub operation_id: OperationId,
    pub output: serde_json::Value,
}
