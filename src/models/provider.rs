use crate::contracts::identifiers::{
    CapabilityId, ContextId, ModelRequestId, ProposalId, ProviderResponseId,
};
use crate::models::errors::ModelError;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;

/// Papel do emissor de uma mensagem no histórico conversacional.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageRole {
    User,
    Model,
    System,
    Tool,
}

/// Mensagem individual de histórico ou turno de ferramenta projetada para o modelo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelMessage {
    User {
        content: String,
    },
    Model {
        content: String,
    },
    System {
        content: String,
    },
    AssistantWithToolCall {
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<String>,
        capability_name: String,
        arguments: serde_json::Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        opaque_signature: Option<String>,
    },
    ToolResult {
        capability_name: String,
        content: serde_json::Value,
    },
}

impl ModelMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self::User {
            content: content.into(),
        }
    }

    pub fn model(content: impl Into<String>) -> Self {
        Self::Model {
            content: content.into(),
        }
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self::System {
            content: content.into(),
        }
    }

    pub fn assistant_tool_call(
        capability_name: impl Into<String>,
        arguments: serde_json::Value,
        opaque_signature: Option<String>,
    ) -> Self {
        Self::AssistantWithToolCall {
            content: None,
            capability_name: capability_name.into(),
            arguments,
            opaque_signature,
        }
    }

    pub fn tool_result(capability_name: impl Into<String>, content: serde_json::Value) -> Self {
        Self::ToolResult {
            capability_name: capability_name.into(),
            content,
        }
    }
}

/// Declaração tipada de capacidade registrada para projeção em esquemas de chamadas de ferramentas.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityDeclaration {
    pub name: String,
    pub description: String,
    pub parameters_schema: serde_json::Value,
}

/// Contrato canônico e soberano da Yuki para requisições a modelos.
///
/// INVARIANTE:
/// O `ModelRequest` NÃO contém credenciais, chaves de API ou segredos.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRequest {
    pub request_id: ModelRequestId,
    pub prompt: String,
    pub context_id: ContextId,
    pub purpose: String,
    pub system_instruction: Option<String>,
    pub messages: Vec<ModelMessage>,
    pub available_capabilities: Vec<CapabilityDeclaration>,
    pub temperature: Option<f32>,
    pub max_output_tokens: Option<u32>,
}

impl ModelRequest {
    /// Construtor compatível com a Foundation e testes clássicos.
    pub fn new(
        prompt: impl Into<String>,
        context_id: ContextId,
        purpose: impl Into<String>,
    ) -> Self {
        let p = prompt.into();
        Self {
            request_id: ModelRequestId::new(),
            messages: vec![ModelMessage::user(p.clone())],
            prompt: p,
            context_id,
            purpose: purpose.into(),
            system_instruction: None,
            available_capabilities: Vec::new(),
            temperature: None,
            max_output_tokens: None,
        }
    }

    /// Cria uma requisição básica apenas a partir do prompt do usuário.
    pub fn from_prompt(prompt: impl Into<String>) -> Self {
        Self::new(prompt, ContextId::new(), "general_turn")
    }
}

/// Candidato bruto a proposta de capacidade extraído da resposta do modelo externo.
///
/// INVARIANTES ARQUITETURAIS:
/// 1. `RawProposalCandidate` é DADO NÃO CONFIÁVEL (`UNTRUSTED DATA`).
/// 2. NÃO contém campo `reasoning` interno do provedor.
/// 3. Não é executável sem passar por validação de esquema no `ProposalParser`
///    e autorização expressa no `SecurityController`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawProposalCandidate {
    pub capability_name: String,
    pub arguments: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub opaque_signature: Option<String>,
}

impl RawProposalCandidate {
    pub fn new(capability_name: impl Into<String>, arguments: serde_json::Value) -> Self {
        Self {
            capability_name: capability_name.into(),
            arguments,
            opaque_signature: None,
        }
    }

    pub fn with_signature(
        capability_name: impl Into<String>,
        arguments: serde_json::Value,
        opaque_signature: Option<String>,
    ) -> Self {
        Self {
            capability_name: capability_name.into(),
            arguments,
            opaque_signature,
        }
    }
}

/// Proposta canônica de invocação de capacidade validada sintaticamente pelo `ProposalParser`.
///
/// INVARIANTE:
/// `CapabilityProposal != CapabilityToken != Authorization != Execution`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityProposal {
    pub proposal_id: ProposalId,
    pub model_request_id: ModelRequestId,
    pub provider_response_id: Option<ProviderResponseId>,
    pub capability_id: CapabilityId,
    pub parameters: serde_json::Value,
    /// Procedência auditável gerada internamente pela Yuki.
    ///
    /// INVARIANTE:
    /// Este campo NÃO contém tokens de 'thought' ou cadeia oculta de raciocínio (hidden CoT)
    /// do provedor externo. Ele documenta a validação sintática e procedência para auditoria.
    pub reasoning: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub opaque_signature: Option<String>,
}

impl CapabilityProposal {
    pub fn new(
        capability_id: CapabilityId,
        parameters: serde_json::Value,
        reasoning: impl Into<String>,
    ) -> Self {
        Self {
            proposal_id: ProposalId::new(),
            model_request_id: ModelRequestId::new(),
            provider_response_id: None,
            capability_id,
            parameters,
            reasoning: reasoning.into(),
            opaque_signature: None,
        }
    }

    pub fn with_signature(
        capability_id: CapabilityId,
        parameters: serde_json::Value,
        reasoning: impl Into<String>,
        opaque_signature: Option<String>,
    ) -> Self {
        Self {
            proposal_id: ProposalId::new(),
            model_request_id: ModelRequestId::new(),
            provider_response_id: None,
            capability_id,
            parameters,
            reasoning: reasoning.into(),
            opaque_signature,
        }
    }
}

/// Estatísticas de consumo e uso de tokens informadas pelo provedor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelUsage {
    pub prompt_tokens: u32,
    pub candidate_tokens: u32,
    pub total_tokens: u32,
}

/// Resposta canônica e neutra emitida pelo gateway de modelos.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelResponse {
    pub request_id: ModelRequestId,
    pub provider: String,
    pub model: String,
    pub raw_content: String,
    pub capability_proposal: Option<CapabilityProposal>,
    pub candidate_proposal: Option<RawProposalCandidate>,
    pub usage: Option<ModelUsage>,
    pub finish_reason: String,
    pub provider_response_id: Option<ProviderResponseId>,
}

impl ModelResponse {
    /// Constrói uma resposta puramente conversacional de texto.
    pub fn text(
        request_id: ModelRequestId,
        provider: impl Into<String>,
        model: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        let text = content.into();
        Self {
            request_id,
            provider: provider.into(),
            model: model.into(),
            raw_content: text,
            capability_proposal: None,
            candidate_proposal: None,
            usage: None,
            finish_reason: "STOP".to_string(),
            provider_response_id: None,
        }
    }

    /// Constrói uma resposta contendo um candidato bruto a proposta.
    pub fn with_candidate(
        request_id: ModelRequestId,
        provider: impl Into<String>,
        model: impl Into<String>,
        raw_content: impl Into<String>,
        candidate: RawProposalCandidate,
        provider_response_id: Option<ProviderResponseId>,
    ) -> Self {
        Self {
            request_id,
            provider: provider.into(),
            model: model.into(),
            raw_content: raw_content.into(),
            capability_proposal: None,
            candidate_proposal: Some(candidate),
            usage: None,
            finish_reason: "TOOL_CALL".to_string(),
            provider_response_id,
        }
    }
}

/// Metadados de identificação do provedor de modelo ativo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetadata {
    pub provider_name: String,
    pub model_name: String,
    pub version: String,
}

/// Estado de saúde operacional do provedor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded(String),
    Unhealthy(String),
}

/// Trait assíncrona soberana para provedores de modelos (ADR-018 / Marco 2).
///
/// Mantém segurança de objetos (`dyn ModelProvider`) através de pinned boxed future
/// padrão da biblioteca Rust, sem dependência de macros procedurais como `async-trait`.
pub trait ModelProvider: Send + Sync {
    fn generate<'a>(
        &'a self,
        request: &'a ModelRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, ModelError>> + Send + 'a>>;

    fn metadata(&self) -> ModelMetadata;
    fn health(&self) -> HealthStatus;
}
