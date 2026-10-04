use crate::capabilities::registry::CapabilityRegistry;
use crate::contracts::identifiers::{CapabilityId, ProposalId, ProviderResponseId};
use crate::models::config::ModelGatewayConfig;
use crate::models::errors::ModelError;
use crate::models::provider::{CapabilityProposal, ModelRequest, RawProposalCandidate};

/// Parser sintático e validador estrutural de propostas de modelos externos (ADR-018).
///
/// INVARIANTE ABSOLUTO:
/// `ProposalParser != SecurityController`
/// O ProposalParser valida estrutura, esquema e limites técnicos de segurança.
/// Ele NUNCA autoriza, NUNCA emite `CapabilityToken`, NUNCA gera `OperationId` ou `AttemptId`
/// e NUNCA executa qualquer ação.
pub struct ProposalParser;

impl ProposalParser {
    /// Converte um `RawProposalCandidate` não confiável em uma `CapabilityProposal` canônica.
    pub fn parse(
        candidate: RawProposalCandidate,
        request: &ModelRequest,
        registry: &CapabilityRegistry,
        config: &ModelGatewayConfig,
        provider_response_id: Option<ProviderResponseId>,
    ) -> Result<CapabilityProposal, ModelError> {
        let cap_id = CapabilityId::new(&candidate.capability_name);

        // 1. Validar se a capacidade existe no registro da Yuki
        if !registry.has_capability(&cap_id) {
            return Err(ModelError::UnsupportedCapability(candidate.capability_name));
        }

        // 2. Validar que os parâmetros são um objeto JSON
        if !candidate.arguments.is_object() {
            return Err(ModelError::InvalidArguments {
                capability: candidate.capability_name,
                details: "Os parâmetros da capacidade devem constituir um objeto JSON ({...})"
                    .to_string(),
            });
        }

        // 3. Validar limite de profundidade (prevenção contra DoS via recursão profunda)
        let depth = Self::calculate_json_depth(&candidate.arguments);
        if depth > config.max_proposal_depth {
            return Err(ModelError::SchemaViolation(format!(
                "Profundidade dos parâmetros JSON ({}) excedeu o limite configurado ({})",
                depth, config.max_proposal_depth
            )));
        }

        // 4. Validar limite de tamanho em bytes dos parâmetros
        let payload_str = serde_json::to_string(&candidate.arguments).map_err(|e| {
            ModelError::MalformedResponse(format!("Falha ao serializar argumentos: {}", e))
        })?;

        if payload_str.len() > config.max_proposal_size_bytes {
            return Err(ModelError::SchemaViolation(format!(
                "Tamanho dos parâmetros JSON ({} bytes) excedeu o limite configurado ({} bytes)",
                payload_str.len(),
                config.max_proposal_size_bytes
            )));
        }

        // 5. Emitir CapabilityProposal canônica com procedência estrita
        Ok(CapabilityProposal {
            proposal_id: ProposalId::new(),
            model_request_id: request.request_id.clone(),
            provider_response_id,
            capability_id: cap_id,
            parameters: candidate.arguments,
            reasoning: format!(
                "Proposta validada estruturalmente para {}",
                candidate.capability_name
            ),
        })
    }

    /// Calcula a profundidade máxima de aninhamento de um valor JSON.
    fn calculate_json_depth(value: &serde_json::Value) -> usize {
        match value {
            serde_json::Value::Object(map) => {
                1 + map
                    .values()
                    .map(Self::calculate_json_depth)
                    .max()
                    .unwrap_or(0)
            }
            serde_json::Value::Array(arr) => {
                1 + arr
                    .iter()
                    .map(Self::calculate_json_depth)
                    .max()
                    .unwrap_or(0)
            }
            _ => 1,
        }
    }
}
