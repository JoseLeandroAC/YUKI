use crate::contracts::errors::YukiError;
use crate::contracts::identifiers::CapabilityId;
use crate::models::provider::{
    CapabilityProposal, HealthStatus, ModelMetadata, ModelProvider, ModelRequest, ModelResponse,
};

#[derive(Debug, Clone)]
pub enum MockBehavior {
    /// Default automatic behavior based on prompt content
    Standard,
    /// Force propose a specific capability
    ForceProposal(CapabilityProposal),
    /// Return raw text response without proposing any capability
    DirectText(String),
    /// Return an error from the model provider
    SimulateError(String),
}

pub struct MockModelProvider {
    behavior: MockBehavior,
}

impl MockModelProvider {
    pub fn new() -> Self {
        Self {
            behavior: MockBehavior::Standard,
        }
    }

    pub fn with_behavior(behavior: MockBehavior) -> Self {
        Self { behavior }
    }
}

impl Default for MockModelProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelProvider for MockModelProvider {
    fn generate(&self, request: &ModelRequest) -> Result<ModelResponse, YukiError> {
        match &self.behavior {
            MockBehavior::SimulateError(err) => Err(YukiError::ModelError(err.clone())),
            MockBehavior::DirectText(text) => Ok(ModelResponse {
                raw_content: text.clone(),
                capability_proposal: None,
            }),
            MockBehavior::ForceProposal(proposal) => Ok(ModelResponse {
                raw_content: format!("Proposing capability: {}", proposal.capability_id),
                capability_proposal: Some(proposal.clone()),
            }),
            MockBehavior::Standard => {
                let trimmed = request.prompt.trim();
                let lower = trimmed.to_lowercase();

                if lower.contains("olá") || lower.contains("ola") {
                    Ok(ModelResponse {
                        raw_content: "Reconheci saudação. Proponho echo com resposta amigável.".to_string(),
                        capability_proposal: Some(CapabilityProposal {
                            capability_id: CapabilityId::new("system.echo"),
                            parameters: serde_json::json!({
                                "message": "Olá! Estou funcionando."
                            }),
                            reasoning: "Saudação do usuário recebida; respondendo com confirmação de funcionamento.".to_string(),
                        }),
                    })
                } else if lower.starts_with("repita:") || lower.starts_with("echo:") {
                    let msg = if let Some(stripped) = trimmed.strip_prefix("Repita:") {
                        stripped.trim()
                    } else if let Some(stripped) = trimmed.strip_prefix("repita:") {
                        stripped.trim()
                    } else if let Some(stripped) = trimmed.strip_prefix("Echo:") {
                        stripped.trim()
                    } else {
                        trimmed.strip_prefix("echo:").unwrap_or(trimmed).trim()
                    };

                    Ok(ModelResponse {
                        raw_content: format!("Proponho repetir: {}", msg),
                        capability_proposal: Some(CapabilityProposal {
                            capability_id: CapabilityId::new("system.echo"),
                            parameters: serde_json::json!({
                                "message": msg
                            }),
                            reasoning: "Usuário solicitou repetição de mensagem.".to_string(),
                        }),
                    })
                } else {
                    // For any generic prompt in v0.1, propose echo with the prompt as message
                    Ok(ModelResponse {
                        raw_content: format!("Processando entrada: {}", trimmed),
                        capability_proposal: Some(CapabilityProposal {
                            capability_id: CapabilityId::new("system.echo"),
                            parameters: serde_json::json!({
                                "message": trimmed
                            }),
                            reasoning: "Entrada genérica em MVP-0 roteada deterministicamente para system.echo.".to_string(),
                        }),
                    })
                }
            }
        }
    }

    fn metadata(&self) -> ModelMetadata {
        ModelMetadata {
            provider_name: "MockProvider".to_string(),
            model_name: "yuki-mock-reasoner-v0.1".to_string(),
            version: "0.1.0".to_string(),
        }
    }

    fn health(&self) -> HealthStatus {
        HealthStatus::Healthy
    }
}
