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
    #[serde(rename = "functionCall", skip_serializing_if = "Option::is_none")]
    function_call: Option<GeminiFunctionCall>,
    #[serde(rename = "functionResponse", skip_serializing_if = "Option::is_none")]
    function_response: Option<GeminiFunctionResponse>,
    #[serde(rename = "thoughtSignature", skip_serializing_if = "Option::is_none")]
    thought_signature: Option<String>,
}

#[derive(Debug, Serialize)]
struct GeminiFunctionResponse {
    name: String,
    response: serde_json::Value,
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
    #[serde(rename = "thoughtSignature", alias = "thought_signature")]
    thought_signature: Option<String>,
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
    code: Option<serde_json::Value>,
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
            match msg {
                crate::models::provider::ModelMessage::User { content } => {
                    contents.push(GeminiContent {
                        role: "user".to_string(),
                        parts: vec![GeminiPart {
                            text: Some(content.clone()),
                            function_call: None,
                            function_response: None,
                            thought_signature: None,
                        }],
                    });
                }
                crate::models::provider::ModelMessage::Model { content } => {
                    contents.push(GeminiContent {
                        role: "model".to_string(),
                        parts: vec![GeminiPart {
                            text: Some(content.clone()),
                            function_call: None,
                            function_response: None,
                            thought_signature: None,
                        }],
                    });
                }
                crate::models::provider::ModelMessage::System { content } => {
                    // System turn in history maps to user in Gemini content sequence
                    contents.push(GeminiContent {
                        role: "user".to_string(),
                        parts: vec![GeminiPart {
                            text: Some(content.clone()),
                            function_call: None,
                            function_response: None,
                            thought_signature: None,
                        }],
                    });
                }
                crate::models::provider::ModelMessage::AssistantWithToolCall {
                    content,
                    capability_name,
                    arguments,
                    opaque_signature,
                } => {
                    let mut parts = Vec::new();
                    if let Some(txt) = content {
                        if !txt.trim().is_empty() {
                            parts.push(GeminiPart {
                                text: Some(txt.clone()),
                                function_call: None,
                                function_response: None,
                                thought_signature: None,
                            });
                        }
                    }
                    parts.push(GeminiPart {
                        text: None,
                        function_call: Some(GeminiFunctionCall {
                            name: capability_name.clone(),
                            args: arguments.clone(),
                        }),
                        function_response: None,
                        thought_signature: opaque_signature.clone(),
                    });
                    contents.push(GeminiContent {
                        role: "model".to_string(),
                        parts,
                    });
                }
                crate::models::provider::ModelMessage::ToolResult {
                    capability_name,
                    content,
                } => {
                    let response_obj = serde_json::json!({ "output": content });
                    contents.push(GeminiContent {
                        role: "user".to_string(),
                        parts: vec![GeminiPart {
                            text: None,
                            function_call: None,
                            function_response: Some(GeminiFunctionResponse {
                                name: capability_name.clone(),
                                response: response_obj,
                            }),
                            thought_signature: None,
                        }],
                    });
                }
            }
        }

        // Se messages estava vazio mas prompt estava preenchido, assegura ao menos 1 turno
        if contents.is_empty() && !request.prompt.trim().is_empty() {
            contents.push(GeminiContent {
                role: "user".to_string(),
                parts: vec![GeminiPart {
                    text: Some(request.prompt.clone()),
                    function_call: None,
                    function_response: None,
                    thought_signature: None,
                }],
            });
        }

        // 3. Mapear declarações de capacidades para tools com projeção de esquema compatível
        let mut tools = Vec::new();
        if !request.available_capabilities.is_empty() {
            let mut declarations = Vec::new();
            for cap in &request.available_capabilities {
                let projected_parameters = project_schema_to_gemini(&cap.parameters_schema)?;
                declarations.push(GeminiFunctionDeclaration {
                    name: cap.name.clone(),
                    description: cap.description.clone(),
                    parameters: projected_parameters,
                });
            }
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
                    function_response: None,
                    thought_signature: None,
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

        let endpoint = build_gemini_endpoint(
            &self.base_url,
            &self.config.api_version,
            &self.config.model_id,
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

        let retry_after_secs = http_resp
            .headers()
            .get("retry-after")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok());

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

        // Tratamento de falhas HTTP com preservação de diagnóstico sanitizado (ADR-018)
        if !status.is_success() {
            let error_msg = parse_gemini_error_message(&resp_bytes, status_code);

            return match status_code {
                400 => Err(ModelError::InvalidRequest(error_msg)),
                401 => Err(ModelError::Authentication(error_msg)),
                403 => Err(ModelError::ProviderAuthorization(error_msg)),
                404 => Err(ModelError::InvalidRequest(format!(
                    "Recurso ou modelo não encontrado no provedor (HTTP 404): {}",
                    error_msg
                ))),
                429 => {
                    if error_msg.to_lowercase().contains("quota") {
                        Err(ModelError::QuotaExceeded(error_msg))
                    } else {
                        Err(ModelError::RateLimited { retry_after_secs })
                    }
                }
                500..=599 => Err(ModelError::ProviderUnavailable {
                    status: status_code,
                    message: error_msg,
                }),
                _ => Err(ModelError::Internal(format!(
                    "Status HTTP inesperado {}: {}",
                    status_code, error_msg
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
                    opaque_signature: part.thought_signature.clone(),
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
            model_name: sanitize_gemini_model_id(&self.config.model_id),
            version: sanitize_gemini_api_version(&self.config.api_version),
        }
    }

    fn health(&self) -> HealthStatus {
        match self.credential_broker.acquire(&self.secret_ref) {
            Ok(lease) => {
                drop(lease);
                HealthStatus::Healthy
            }
            Err(err) => HealthStatus::Degraded(format!(
                "Falha na resolução local da credencial para '{}': {}",
                self.config.sanitized_credential_alias, err
            )),
        }
    }
}

/// Projeta um esquema canônico de capacidade para o subconjunto OpenAPI 3.0 aceito pelo Google Gemini.
///
/// INVARIANTES ARQUITETURAIS:
/// 1. Não mutabilidade do manifesto canônico: O esquema canônico de entrada permanece intacto (`&serde_json::Value`).
/// 2. Fronteira de provedor: `Yuki Schema != Gemini Schema`. Restrições do Gemini pertencem exclusivamente a esta fronteira.
/// 3. Segurança de execução: A omissão de `additionalProperties` para compatibilidade com o Gemini NÃO afeta a validação
///    interna da Yuki (`validate_capability_input`), que continua executando contra o manifesto canônico estrito.
/// 4. Recursividade: Normaliza recursivamente subesquemas em `properties`, `items` e `anyOf`.
/// 5. Falha fechada (*fail-closed*): Rejeita esquemas não-objeto na raiz e construções não representáveis (ex.: `not`, `patternProperties`).
pub fn project_schema_to_gemini(
    schema: &serde_json::Value,
) -> Result<serde_json::Value, ModelError> {
    let obj = schema.as_object().ok_or_else(|| {
        ModelError::InvalidRequest(
            "Esquema de parâmetros para declaração de função do Gemini deve ser um objeto JSON"
                .to_string(),
        )
    })?;

    // O esquema raiz dos parâmetros de uma FunctionDeclaration DEVE ser type 'object'
    let root_type = obj.get("type").and_then(|v| v.as_str());
    if root_type != Some("object") {
        return Err(ModelError::InvalidRequest(format!(
            "Esquema raiz de parâmetros de função para Gemini deve possuir type 'object', encontrado: '{:?}'",
            root_type
        )));
    }

    project_schema_node(schema)
}

fn project_schema_node(schema: &serde_json::Value) -> Result<serde_json::Value, ModelError> {
    let obj = schema.as_object().ok_or_else(|| {
        ModelError::InvalidRequest("Nó de esquema inválido: esperado objeto JSON".to_string())
    })?;

    // Palavras-chave estritamente proibidas / não representáveis com fidelidade semântica
    const FORBIDDEN_KEYWORDS: &[&str] = &[
        "not",
        "patternProperties",
        "oneOf",
        "allOf",
        "dependentRequired",
        "dependentSchemas",
    ];

    for &forbidden in FORBIDDEN_KEYWORDS {
        if obj.contains_key(forbidden) {
            return Err(ModelError::InvalidRequest(format!(
                "Esquema contém construção não suportada para projeção do Gemini: '{}'",
                forbidden
            )));
        }
    }

    let mut projected = serde_json::Map::new();

    for (k, v) in obj {
        match k.as_str() {
            // Palavras-chave a serem explicitamente descartadas no wire format do Gemini
            "additionalProperties" | "$schema" | "$id" | "$comment" | "definitions" | "$defs" => {
                // Omitir no wire format do Gemini (incompatíveis com google.ai.generativelanguage.v1beta.Schema)
                continue;
            }

            // Normalização recursiva de propriedades de objeto
            "properties" => {
                let props_obj = v.as_object().ok_or_else(|| {
                    ModelError::InvalidRequest(
                        "Campo 'properties' deve ser um objeto JSON mapeando nomes a subesquemas"
                            .to_string(),
                    )
                })?;
                let mut projected_props = serde_json::Map::new();
                for (prop_name, prop_schema) in props_obj {
                    projected_props.insert(prop_name.clone(), project_schema_node(prop_schema)?);
                }
                projected.insert(
                    "properties".to_string(),
                    serde_json::Value::Object(projected_props),
                );
            }

            // Normalização recursiva de array items
            "items" => {
                let projected_items = project_schema_node(v)?;
                projected.insert("items".to_string(), projected_items);
            }

            // Normalização recursiva de anyOf
            "anyOf" => {
                let any_of_arr = v.as_array().ok_or_else(|| {
                    ModelError::InvalidRequest(
                        "Campo 'anyOf' deve ser uma lista de esquemas".to_string(),
                    )
                })?;
                let mut projected_any_of = Vec::new();
                for subschema in any_of_arr {
                    projected_any_of.push(project_schema_node(subschema)?);
                }
                projected.insert(
                    "anyOf".to_string(),
                    serde_json::Value::Array(projected_any_of),
                );
            }

            // Campos padrão do OpenAPI 3.0 Schema permitidos pelo Gemini
            "type" | "format" | "title" | "description" | "nullable" | "enum" | "required"
            | "minItems" | "maxItems" | "minLength" | "maxLength" | "pattern" | "example"
            | "propertyOrdering" | "default" | "minimum" | "maximum" | "minProperties"
            | "maxProperties" => {
                projected.insert(k.clone(), v.clone());
            }

            // Qualquer outra palavra-chave não reconhecida: fail-closed para evitar rejeições do Gemini
            other => {
                return Err(ModelError::InvalidRequest(format!(
                    "Palavra-chave não reconhecida ou incompatível com o esquema do Gemini: '{}'",
                    other
                )));
            }
        }
    }

    // Se o tipo for object e properties estiver ausente, garantir "properties": {}
    if projected.get("type").and_then(|v| v.as_str()) == Some("object")
        && !projected.contains_key("properties")
    {
        projected.insert("properties".to_string(), serde_json::json!({}));
    }

    Ok(serde_json::Value::Object(projected))
}

/// Sanitiza e normaliza o identificador do modelo Gemini.
///
/// Trata variações operacionais:
/// - Espaços em branco nas extremidades.
/// - Aspas envolventes (ex: `"gemini-2.5-flash"` ou `'gemini-2.5-flash'`).
/// - Barras iniciais (ex: `"/models/gemini-2.5-flash"`).
/// - Prefixo redundante `"models/"` (ex: `"models/gemini-2.5-flash"` -> `"gemini-2.5-flash"`).
pub fn sanitize_gemini_model_id(raw: &str) -> String {
    let mut s = raw.trim();
    if ((s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')))
        && s.len() >= 2
    {
        s = s[1..s.len() - 1].trim();
    }
    s = s.trim_start_matches('/');
    s = s.strip_prefix("models/").unwrap_or(s);
    s.trim().to_string()
}

/// Sanitiza e normaliza a versão da API REST do Gemini.
///
/// Remove barras e espaços (ex: `"/v1beta/"` -> `"v1beta"`).
pub fn sanitize_gemini_api_version(raw: &str) -> String {
    let trimmed = raw.trim().trim_matches('/');
    if trimmed.is_empty() {
        "v1beta".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Constrói o endpoint REST canônico para o método `generateContent`.
///
/// Garante que o caminho seja sempre `{base_url}/{api_version}/models/{model_name}:generateContent`,
/// sem duplicação de `models/models/` ou barras duplicadas.
pub fn build_gemini_endpoint(base_url: &str, api_version: &str, model_id: &str) -> String {
    let clean_base = base_url.trim_end_matches('/');
    let clean_version = sanitize_gemini_api_version(api_version);
    let clean_model = sanitize_gemini_model_id(model_id);
    format!(
        "{}/{}/models/{}:generateContent",
        clean_base, clean_version, clean_model
    )
}

/// Sanitiza qualquer indício de segredos (ex: chaves API padrão Google `AIza...`) de mensagens de erro.
pub fn scrub_potential_secrets(s: &str) -> String {
    let mut result = s.to_string();
    while let Some(idx) = result.find("AIza") {
        let end = result[idx..]
            .find(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
            .map(|i| idx + i)
            .unwrap_or(result.len());
        if end - idx >= 20 {
            result.replace_range(idx..end, "[REDACTED_KEY]");
        } else {
            break;
        }
    }
    result
}

/// Extrai e formata mensagens de erro retornadas pelo provedor Gemini com observabilidade sanitizada (ADR-018).
pub fn parse_gemini_error_message(resp_bytes: &[u8], status_code: u16) -> String {
    if let Ok(envelope) = serde_json::from_slice::<GeminiErrorEnvelope>(resp_bytes) {
        if let Some(detail) = envelope.error {
            let mut parts = Vec::new();
            if let Some(st) = detail.status {
                let st_trimmed = st.trim();
                if !st_trimmed.is_empty() {
                    parts.push(st_trimmed.to_string());
                }
            }
            if let Some(code) = detail.code {
                match code {
                    serde_json::Value::Number(n) => {
                        let code_str = n.to_string();
                        if !parts.contains(&code_str) && code_str != status_code.to_string() {
                            parts.push(format!("código {}", code_str));
                        }
                    }
                    serde_json::Value::String(s) => {
                        let s_trimmed = s.trim();
                        if !s_trimmed.is_empty() && !parts.contains(&s_trimmed.to_string()) {
                            parts.push(s_trimmed.to_string());
                        }
                    }
                    _ => {}
                }
            }
            if let Some(msg) = detail.message {
                let trimmed = msg.trim();
                if !trimmed.is_empty() {
                    parts.push(trimmed.to_string());
                }
            }

            if !parts.is_empty() {
                return scrub_potential_secrets(&parts.join(": "));
            }
        }
    }

    // Fallback para respostas não-JSON (ex: HTML/proxy de gateway)
    if let Ok(text) = std::str::from_utf8(resp_bytes) {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            let scrubbed = scrub_potential_secrets(trimmed);
            return if scrubbed.len() > 300 {
                format!("{}...", &scrubbed[..300])
            } else {
                scrubbed
            };
        }
    }

    format!("HTTP {}", status_code)
}
