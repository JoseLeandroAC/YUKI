use crate::contracts::identifiers::{CapabilityId, ProposalId};
use crate::models::errors::ModelError;
use crate::models::provider::{
    CapabilityProposal, HealthStatus, ModelMetadata, ModelProvider, ModelRequest, ModelResponse,
    RawProposalCandidate,
};
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

/// Comportamentos simulados pelo `MockModelProvider` para testes determinísticos sem rede.
#[derive(Debug, Clone)]
pub enum MockBehavior {
    /// Comportamento automático padrão baseado no conteúdo do prompt (compatibilidade Foundation).
    Standard,
    /// Força uma proposta canônica específica.
    ForceProposal(CapabilityProposal),
    /// Emite um candidato bruto a proposta (simulação de function call externo).
    ProposeCandidate(RawProposalCandidate),
    /// Retorna texto direto sem propostas de capacidades.
    DirectText(String),
    /// Simula um erro específico tipado do provedor.
    SimulateError(ModelError),
    /// Simula resposta malformada do provedor.
    MalformedResponse,
    /// Simula saída hostil/adversa tentando injeção de comandos.
    HostileText(String),
    /// Simula erro de limite de taxa (HTTP 429).
    RateLimit { retry_after_secs: Option<u64> },
    /// Simula estouro de tempo limite (Timeout).
    Timeout(Duration),
    /// Simula indisponibilidade do provedor upstream (HTTP 5xx).
    ProviderUnavailable { status: u16, message: String },
    /// Simula bloqueio de conteúdo por filtro de segurança.
    ContentBlocked(String),
}

/// Implementação determinística e offline de `ModelProvider` para testes e CI.
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

    /// Lógica síncrona de geração de resposta determinística.
    pub fn generate_sync(&self, request: &ModelRequest) -> Result<ModelResponse, ModelError> {
        let req_id = request.request_id.clone();
        match &self.behavior {
            MockBehavior::SimulateError(err) => Err(err.clone()),
            MockBehavior::MalformedResponse => Err(ModelError::MalformedResponse(
                "Resposta JSON do provedor malformada ou truncada".to_string(),
            )),
            MockBehavior::RateLimit { retry_after_secs } => Err(ModelError::RateLimited {
                retry_after_secs: *retry_after_secs,
            }),
            MockBehavior::Timeout(dur) => Err(ModelError::Timeout(*dur)),
            MockBehavior::ProviderUnavailable { status, message } => {
                Err(ModelError::ProviderUnavailable {
                    status: *status,
                    message: message.clone(),
                })
            }
            MockBehavior::ContentBlocked(msg) => Err(ModelError::ContentBlocked(msg.clone())),
            MockBehavior::HostileText(text) => Ok(ModelResponse::text(
                req_id,
                "MockProvider",
                "yuki-mock-reasoner-v0.1",
                text.clone(),
            )),
            MockBehavior::DirectText(text) => Ok(ModelResponse::text(
                req_id,
                "MockProvider",
                "yuki-mock-reasoner-v0.1",
                text.clone(),
            )),
            MockBehavior::ProposeCandidate(candidate) => Ok(ModelResponse::with_candidate(
                req_id,
                "MockProvider",
                "yuki-mock-reasoner-v0.1",
                format!("Candidate proposed: {}", candidate.capability_name),
                candidate.clone(),
                None,
            )),
            MockBehavior::ForceProposal(proposal) => Ok(ModelResponse {
                request_id: req_id,
                provider: "MockProvider".to_string(),
                model: "yuki-mock-reasoner-v0.1".to_string(),
                raw_content: format!("Proposing capability: {}", proposal.capability_id),
                capability_proposal: Some(proposal.clone()),
                candidate_proposal: Some(RawProposalCandidate {
                    capability_name: proposal.capability_id.0.clone(),
                    arguments: proposal.parameters.clone(),
                }),
                usage: None,
                finish_reason: "TOOL_CALL".to_string(),
                provider_response_id: None,
            }),
            MockBehavior::Standard => {
                let trimmed = request.prompt.trim();
                let lower = trimmed.to_lowercase();

                if lower.contains("olá") || lower.contains("ola") {
                    let proposal = CapabilityProposal {
                        proposal_id: ProposalId::new(),
                        model_request_id: req_id.clone(),
                        provider_response_id: None,
                        capability_id: CapabilityId::new("system.echo"),
                        parameters: serde_json::json!({
                            "message": "Olá! Estou funcionando."
                        }),
                        reasoning: "Saudação do usuário recebida; respondendo com confirmação de funcionamento.".to_string(),
                    };
                    Ok(ModelResponse {
                        request_id: req_id,
                        provider: "MockProvider".to_string(),
                        model: "yuki-mock-reasoner-v0.1".to_string(),
                        raw_content: "Reconheci saudação. Proponho echo com resposta amigável."
                            .to_string(),
                        capability_proposal: Some(proposal.clone()),
                        candidate_proposal: Some(RawProposalCandidate {
                            capability_name: proposal.capability_id.0.clone(),
                            arguments: proposal.parameters.clone(),
                        }),
                        usage: None,
                        finish_reason: "TOOL_CALL".to_string(),
                        provider_response_id: None,
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

                    let proposal = CapabilityProposal {
                        proposal_id: ProposalId::new(),
                        model_request_id: req_id.clone(),
                        provider_response_id: None,
                        capability_id: CapabilityId::new("system.echo"),
                        parameters: serde_json::json!({
                            "message": msg
                        }),
                        reasoning: "Usuário solicitou repetição de mensagem.".to_string(),
                    };

                    Ok(ModelResponse {
                        request_id: req_id,
                        provider: "MockProvider".to_string(),
                        model: "yuki-mock-reasoner-v0.1".to_string(),
                        raw_content: format!("Proponho repetir: {}", msg),
                        capability_proposal: Some(proposal.clone()),
                        candidate_proposal: Some(RawProposalCandidate {
                            capability_name: proposal.capability_id.0.clone(),
                            arguments: proposal.parameters.clone(),
                        }),
                        usage: None,
                        finish_reason: "TOOL_CALL".to_string(),
                        provider_response_id: None,
                    })
                } else {
                    let proposal = CapabilityProposal {
                        proposal_id: ProposalId::new(),
                        model_request_id: req_id.clone(),
                        provider_response_id: None,
                        capability_id: CapabilityId::new("system.echo"),
                        parameters: serde_json::json!({
                            "message": trimmed
                        }),
                        reasoning: "Entrada genérica em MVP-0 roteada deterministicamente para system.echo.".to_string(),
                    };

                    Ok(ModelResponse {
                        request_id: req_id,
                        provider: "MockProvider".to_string(),
                        model: "yuki-mock-reasoner-v0.1".to_string(),
                        raw_content: format!("Processando entrada: {}", trimmed),
                        capability_proposal: Some(proposal.clone()),
                        candidate_proposal: Some(RawProposalCandidate {
                            capability_name: proposal.capability_id.0.clone(),
                            arguments: proposal.parameters.clone(),
                        }),
                        usage: None,
                        finish_reason: "TOOL_CALL".to_string(),
                        provider_response_id: None,
                    })
                }
            }
        }
    }
}

impl Default for MockModelProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelProvider for MockModelProvider {
    fn generate<'a>(
        &'a self,
        request: &'a ModelRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, ModelError>> + Send + 'a>> {
        let res = self.generate_sync(request);
        Box::pin(std::future::ready(res))
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
