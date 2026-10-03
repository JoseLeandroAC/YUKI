use crate::models::config::ModelGatewayConfig;
use crate::models::errors::ModelError;
use crate::models::provider::{
    HealthStatus, ModelMetadata, ModelProvider, ModelRequest, ModelResponse, ModelUsage,
    RawProposalCandidate,
};
use crate::security::credentials::{CredentialBroker, SecretRef};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Wire types específicos da API Google Gemini (REST v1beta).
///
/// INVARIANTE:
/// Estes tipos são estritamente privados ao adaptador e NUNCA vazam para o Yuki Core.
#[derive(Debug, Serialize)]
struct GeminiPart {
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    function_call: Option<GeminiFunctionCall>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GeminiFunctionCall {
    name: String,
    args: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct GeminiContent {
    role: String,
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize)]
struct GeminiFunctionDeclaration {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct GeminiTool {
    function_declarations: Vec<GeminiFunctionDeclaration>,
}

#[derive(Debug, Serialize)]
struct GeminiGenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<u32>,
}

#[derive(Debug, Serialize)]
struct GeminiGenerateRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<GeminiContent>,
    contents: Vec<GeminiContent>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<GeminiTool>,
    generation_config: GeminiGenerationConfig,
}

#[derive(Debug, Clone, Deserialize)]
struct GeminiCandidatePart {
    text: Option<String>,
    #[serde(rename = "functionCall")]
    function_call: Option<GeminiFunctionCall>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidateContent {
    parts: Option<Vec<GeminiCandidatePart>>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    content: Option<GeminiCandidateContent>,
    #[serde(rename = "finishReason")]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeminiUsageMetadata {
    #[serde(rename = "promptTokenCount")]
    prompt_token_count: Option<u32>,
    #[serde(rename = "candidatesTokenCount")]
    candidates_token_count: Option<u32>,
    #[serde(rename = "totalTokenCount")]
    total_token_count: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct GeminiGenerateResponse {
    candidates: Option<Vec<GeminiCandidate>>,
    #[serde(rename = "usageMetadata")]
    usage_metadata: Option<GeminiUsageMetadata>,
    #[serde(rename = "responseId")]
    response_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GeminiErrorDetail {
    code: Option<u16>,
    message: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeminiErrorEnvelope {
    error: Option<GeminiErrorDetail>,
}

/// Adaptador para o provedor Google Gemini via REST HTTPS (ADR-018).
pub struct GeminiProviderAdapter {
    client: reqwest::Client,
    config: ModelGatewayConfig,
    secret_ref: SecretRef,
    credential_broker: Arc<dyn CredentialBroker>,
    base_url: String,
}

impl GeminiProviderAdapter {
    pub fn new(
        secret_ref: SecretRef,
        credential_broker: Arc<dyn CredentialBroker>,
        config: ModelGatewayConfig,
    ) -> Result<Self, ModelError> {
        let client = reqwest::Client::builder()
            .connect_timeout(config.connect_timeout)
            .timeout(config.request_timeout)
            .build()
            .map_err(|e| {
                ModelError::Internal(format!("Falha ao inicializar cliente HTTP: {}", e))
            })?;

        Ok(Self {
            client,
            config,
            secret_ref,
            credential_broker,
            base_url: "https://generativelanguage.googleapis.com".to_string(),
        })
    }

    /// Permite substituir a URL base para testes e simulações com servidor HTTP mock.
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Executa uma única tentativa de requisição HTTP direta contra o endpoint Gemini.
    async fn execute_single_turn(
        &self,
        request: &ModelRequest,
    ) -> Result<ModelResponse, ModelError> {
        // 1. Obter lease temporário de credencial imediatamente antes do envio
        let lease = self
            .credential_broker
            .acquire(&self.secret_ref)
            .map_err(|e| {
                ModelError::Authentication(format!(
                    "Falha ao resolver credencial '{}': {}",
                    self.config.sanitized_credential_alias, e
                ))
            })?;

        // 2. Mapear mensagens do domínio para wire format
        let mut contents = Vec::new();
        for msg in &request.messages {
            let role_str = match msg.role {
                crate::models::provider::MessageRole::User => "user",
                crate::models::provider::MessageRole::Model => "model",
                crate::models::provider::MessageRole::System => "user", // Gemini v1beta mapeia system no system_instruction
            };
            contents.push(GeminiContent {
                role: role_str.to_string(),
                parts: vec![GeminiPart {
                    text: Some(msg.content.clone()),
                    function_call: None,
                }],
            });
        }

        // Se messages estava vazio mas prompt estava preenchido, assegura ao menos 1 turno
        if contents.is_empty() && !request.prompt.trim().is_empty() {
            contents.push(GeminiContent {
                role: "user".to_string(),
                parts: vec![GeminiPart {
                    text: Some(request.prompt.clone()),
                    function_call: None,
                }],
            });
        }

        // 3. Mapear declarações de capacidades para tools
        let mut tools = Vec::new();
        if !request.available_capabilities.is_empty() {
            let declarations = request
                .available_capabilities
                .iter()
                .map(|cap| GeminiFunctionDeclaration {
                    name: cap.name.clone(),
                    description: cap.description.clone(),
                    parameters: cap.parameters_schema.clone(),
                })
                .collect();
            tools.push(GeminiTool {
                function_declarations: declarations,
            });
        }

        // 4. Mapear system instruction
        let system_instruction = request
            .system_instruction
            .as_ref()
            .map(|inst| GeminiContent {
                role: "system".to_string(),
                parts: vec![GeminiPart {
                    text: Some(inst.clone()),
                    function_call: None,
                }],
            });

        let wire_request = GeminiGenerateRequest {
            system_instruction,
            contents,
            tools,
            generation_config: GeminiGenerationConfig {
                temperature: request.temperature,
                max_output_tokens: request
                    .max_output_tokens
                    .or(Some(self.config.max_output_tokens)),
            },
        };

        // 5. Validar limite de tamanho da requisição
        let body_bytes = serde_json::to_vec(&wire_request).map_err(|e| {
            ModelError::InvalidRequest(format!("Falha ao serializar payload: {}", e))
        })?;

        if body_bytes.len() > self.config.max_request_bytes {
            return Err(ModelError::InvalidRequest(format!(
                "Payload de requisição ({} bytes) excedeu o limite configurado ({} bytes)",
                body_bytes.len(),
                self.config.max_request_bytes
            )));
        }

        let endpoint = format!(
            "{}/v1beta/models/{}:generateContent",
            self.base_url.trim_end_matches('/'),
            self.config.model_id
        );

        // 6. Construir e disparar requisição HTTP
        let http_resp = self
            .client
            .post(&endpoint)
            .header("x-goog-api-key", lease.expose_secret())
            .header("content-type", "application/json")
            .body(body_bytes)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    ModelError::Timeout(self.config.request_timeout)
                } else if e.is_connect() {
                    ModelError::Network(format!("Erro de conexão com provedor: {}", e))
                } else {
                    ModelError::Network(format!("Erro de transporte HTTP: {}", e))
                }
            })?;

        // 7. Avaliar status HTTP da resposta
        let status = http_resp.status();
        let status_code = status.as_u16();

        // Limite de corpo da resposta
        let content_length = http_resp.content_length().unwrap_or(0);
        if content_length > self.config.max_response_bytes as u64 {
            return Err(ModelError::MalformedResponse(format!(
                "Tamanho da resposta ({} bytes) excedeu o teto configurado de {} bytes",
                content_length, self.config.max_response_bytes
            )));
        }

        let resp_bytes = http_resp.bytes().await.map_err(|e| {
            ModelError::Network(format!("Falha ao ler corpo da resposta HTTP: {}", e))
        })?;

        if resp_bytes.len() > self.config.max_response_bytes {
            return Err(ModelError::MalformedResponse(format!(
                "Corpo da resposta recebido ({} bytes) excedeu o teto configurado de {} bytes",
                resp_bytes.len(),
                self.config.max_response_bytes
            )));
        }

        // Tratamento de falhas HTTP
        if !status.is_success() {
            let error_msg =
                if let Ok(envelope) = serde_json::from_slice::<GeminiErrorEnvelope>(&resp_bytes) {
                    envelope
                        .error
                        .and_then(|d| d.message)
                        .unwrap_or_else(|| format!("HTTP {}", status_code))
                } else {
                    format!("HTTP {}", status_code)
                };

            return match status_code {
                400 => Err(ModelError::InvalidRequest(error_msg)),
                401 | 403 => Err(ModelError::Authentication(error_msg)),
                429 => Err(ModelError::RateLimited {
                    retry_after_secs: None,
                }),
                500..=599 => Err(ModelError::ProviderUnavailable {
                    status: status_code,
                    message: error_msg,
                }),
                _ => Err(ModelError::Internal(format!(
                    "Status HTTP inesperado: {}",
                    status_code
                ))),
            };
        }

        // 8. Decodificar resposta de sucesso
        let wire_resp: GeminiGenerateResponse =
            serde_json::from_slice(&resp_bytes).map_err(|e| {
                ModelError::MalformedResponse(format!("Falha ao decodificar JSON do Gemini: {}", e))
            })?;

        let candidate = wire_resp
            .candidates
            .as_ref()
            .and_then(|c| c.first())
            .ok_or_else(|| {
                ModelError::MalformedResponse("Resposta não contém candidatos".to_string())
            })?;

        let finish_reason = candidate
            .finish_reason
            .clone()
            .unwrap_or_else(|| "STOP".to_string());

        if finish_reason == "SAFETY" || finish_reason == "BLOCKLIST" {
            return Err(ModelError::ContentBlocked(format!(
                "Resposta bloqueada por política de segurança upstream: {}",
                finish_reason
            )));
        }

        let parts = candidate
            .content
            .as_ref()
            .and_then(|c| c.parts.as_ref())
            .cloned()
            .unwrap_or_default();

        let mut text_content = String::new();
        let mut raw_proposal = None;

        for part in parts {
            if let Some(txt) = part.text {
                if !text_content.is_empty() {
                    text_content.push('\n');
                }
                text_content.push_str(&txt);
            }
            if let Some(fc) = part.function_call {
                raw_proposal = Some(RawProposalCandidate {
                    capability_name: fc.name,
                    arguments: fc.args,
                });
            }
        }

        let usage = wire_resp.usage_metadata.map(|u| ModelUsage {
            prompt_tokens: u.prompt_token_count.unwrap_or(0),
            candidate_tokens: u.candidates_token_count.unwrap_or(0),
            total_tokens: u.total_token_count.unwrap_or(0),
        });

        let provider_response_id = wire_resp
            .response_id
            .map(crate::contracts::identifiers::ProviderResponseId::new);

        Ok(ModelResponse {
            request_id: request.request_id.clone(),
            provider: "GoogleGemini".to_string(),
            model: self.config.model_id.clone(),
            raw_content: text_content,
            capability_proposal: None,
            candidate_proposal: raw_proposal,
            usage,
            finish_reason,
            provider_response_id,
        })
    }
}

impl ModelProvider for GeminiProviderAdapter {
    fn generate<'a>(
        &'a self,
        request: &'a ModelRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, ModelError>> + Send + 'a>> {
        Box::pin(async move {
            let mut attempts = 0;
            let mut backoff = self.config.initial_backoff;

            loop {
                attempts += 1;
                match self.execute_single_turn(request).await {
                    Ok(resp) => return Ok(resp),
                    Err(err) => {
                        if !err.is_retryable() || attempts > self.config.max_retries {
                            return Err(err);
                        }

                        // Backoff exponencial limitado
                        tokio::time::sleep(backoff).await;
                        backoff = std::cmp::min(backoff * 2, self.config.max_backoff);
                    }
                }
            }
        })
    }

    fn metadata(&self) -> ModelMetadata {
        ModelMetadata {
            provider_name: "GoogleGemini".to_string(),
            model_name: self.config.model_id.clone(),
            version: "v1beta".to_string(),
        }
    }

    fn health(&self) -> HealthStatus {
        HealthStatus::Healthy
    }
}
