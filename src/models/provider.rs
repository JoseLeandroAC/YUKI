use crate::contracts::errors::YukiError;
use crate::contracts::identifiers::{CapabilityId, ContextId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRequest {
    pub prompt: String,
    pub context_id: ContextId,
    pub purpose: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityProposal {
    pub capability_id: CapabilityId,
    pub parameters: serde_json::Value,
    pub reasoning: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelResponse {
    pub raw_content: String,
    pub capability_proposal: Option<CapabilityProposal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetadata {
    pub provider_name: String,
    pub model_name: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded(String),
    Unhealthy(String),
}

pub trait ModelProvider: Send + Sync {
    fn generate(&self, request: &ModelRequest) -> Result<ModelResponse, YukiError>;
    fn metadata(&self) -> ModelMetadata;
    fn health(&self) -> HealthStatus;
}
