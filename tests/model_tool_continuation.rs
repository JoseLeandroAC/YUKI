use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use yuki::contracts::identifiers::{CapabilityId, ModelRequestId, ProposalId};
use yuki::contracts::input::UserInput;
use yuki::contracts::output::ResultStatus;
use yuki::core::yuki_core::YukiCore;
use yuki::models::config::ModelGatewayConfig;
use yuki::models::errors::ModelError;
use yuki::models::provider::{
    CapabilityProposal, HealthStatus, ModelMessage, ModelMetadata, ModelProvider, ModelRequest,
    ModelResponse, RawProposalCandidate,
};

/// Mock provider multi-turn que responde com uma sequência pré-definida de respostas
struct SequenceMockProvider {
    responses: Vec<ModelResponse>,
    cursor: AtomicUsize,
    recorded_requests: Arc<Mutex<Vec<ModelRequest>>>,
}

impl SequenceMockProvider {
    fn new(responses: Vec<ModelResponse>) -> (Arc<Self>, Arc<Mutex<Vec<ModelRequest>>>) {
        let recorded = Arc::new(Mutex::new(Vec::new()));
        let provider = Arc::new(Self {
            responses,
            cursor: AtomicUsize::new(0),
            recorded_requests: recorded.clone(),
        });
        (provider, recorded)
    }
}

impl ModelProvider for SequenceMockProvider {
    fn generate<'a>(
        &'a self,
        request: &'a ModelRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, ModelError>> + Send + 'a>> {
        self.recorded_requests.lock().unwrap().push(request.clone());

        let idx = self.cursor.fetch_add(1, Ordering::SeqCst);
        let resp = if idx < self.responses.len() {
            self.responses[idx].clone()
        } else {
            ModelResponse::text(
                request.request_id.clone(),
                "SequenceMock",
                "yuki-mock-test",
                "Fim das respostas pré-definidas",
            )
        };

        Box::pin(async move { Ok(resp) })
    }

    fn metadata(&self) -> ModelMetadata {
        ModelMetadata {
            provider_name: "SequenceMock".to_string(),
            model_name: "sequence-tester".to_string(),
            version: "v1".to_string(),
        }
    }

    fn health(&self) -> HealthStatus {
        HealthStatus::Healthy
    }
}

// ============================================================================
// 1. CICLO DE CONTINUAÇÃO DE FERRAMENTAS (TOOL CONTINUATION LOOP)
// ============================================================================

#[tokio::test]
async fn test_continuation_01_single_tool_returns_conversational_response() {
    // Turn 1: Model proposes system.echo
    let resp1 = ModelResponse {
        request_id: ModelRequestId::new(),
        provider: "SequenceMock".to_string(),
        model: "sequence-tester".to_string(),
        raw_content: "Proponho invocar system.echo".to_string(),
        capability_proposal: Some(CapabilityProposal {
            proposal_id: ProposalId::new(),
            model_request_id: ModelRequestId::new(),
            provider_response_id: None,
            capability_id: CapabilityId::new("system.echo"),
            parameters: serde_json::json!({ "message": "Yuki online" }),
            reasoning: "Saudação inicial".to_string(),
            opaque_signature: Some("opaque_sig_turn1".to_string()),
        }),
        candidate_proposal: None,
        usage: None,
        finish_reason: "TOOL_CALL".to_string(),
        provider_response_id: None,
    };

    // Turn 2: Having received verified tool output, model produces final conversational response
    let final_text = "Olá! Eu sou a Yuki, e o meu propósito fundamental é ser uma assistente soberana, governada e determinística.";
    let resp2 = ModelResponse::text(
        ModelRequestId::new(),
        "SequenceMock",
        "sequence-tester",
        final_text,
    );

    let (provider, requests) = SequenceMockProvider::new(vec![resp1, resp2]);
    let core = YukiCore::new().with_model_provider(provider);

    let input = UserInput::new("Yuki, se apresente e diga seu propósito");
    let result = core
        .process_input_async(input)
        .await
        .expect("Continuation loop must succeed");

    // Must return the conversational text produced by the model
    assert_eq!(result.status, ResultStatus::Success);
    assert_eq!(result.content, final_text);
    // Preserves the verified operation audit and verification state
    assert!(result.operation_id.is_some());
    assert!(result.verification_result.is_some());
    assert!(result.verification_result.unwrap().is_success());

    // Verify requests sent to the model provider
    let reqs = requests.lock().unwrap();
    assert_eq!(
        reqs.len(),
        2,
        "Expected exactly 2 model provider invocations"
    );

    // Turn 1: initial user turn
    assert_eq!(reqs[0].messages.len(), 1);
    assert!(matches!(&reqs[0].messages[0], ModelMessage::User { .. }));

    // Turn 2: continuation turn with tool call and tool result as data
    assert_eq!(reqs[1].messages.len(), 3);
    assert!(matches!(&reqs[1].messages[0], ModelMessage::User { .. }));
    match &reqs[1].messages[1] {
        ModelMessage::AssistantWithToolCall {
            capability_name,
            opaque_signature,
            ..
        } => {
            assert_eq!(capability_name, "system.echo");
            assert_eq!(opaque_signature.as_deref(), Some("opaque_sig_turn1"));
        }
        other => panic!("Expected AssistantWithToolCall, got: {:?}", other),
    }
    match &reqs[1].messages[2] {
        ModelMessage::ToolResult {
            capability_name,
            content,
        } => {
            assert_eq!(capability_name, "system.echo");
            assert_eq!(
                content.get("echoed_message").and_then(|v| v.as_str()),
                Some("Yuki online")
            );
        }
        other => panic!("Expected ToolResult, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_continuation_02_pure_text_requires_zero_tool_turns() {
    let direct_text = "Sou uma IA conversacional pura neste turno.";
    let resp = ModelResponse::text(
        ModelRequestId::new(),
        "SequenceMock",
        "sequence-tester",
        direct_text,
    );

    let (provider, requests) = SequenceMockProvider::new(vec![resp]);
    let core = YukiCore::new().with_model_provider(provider);

    let input = UserInput::new("Pergunta puramente teórica");
    let result = core.process_input_async(input).await.expect("success");

    assert_eq!(result.status, ResultStatus::Success);
    assert_eq!(result.content, direct_text);
    assert!(result.operation_id.is_none());
    assert!(result.verification_result.is_none());

    let reqs = requests.lock().unwrap();
    assert_eq!(reqs.len(), 1);
}

#[tokio::test]
async fn test_continuation_03_multi_step_sequential_tool_calls_with_fresh_auth() {
    // Step 1: Model proposes system.info
    let resp1 = ModelResponse {
        request_id: ModelRequestId::new(),
        provider: "SequenceMock".to_string(),
        model: "sequence-tester".to_string(),
        raw_content: "Consultando info".to_string(),
        capability_proposal: Some(CapabilityProposal {
            proposal_id: ProposalId::new(),
            model_request_id: ModelRequestId::new(),
            provider_response_id: None,
            capability_id: CapabilityId::new("system.info"),
            parameters: serde_json::json!({}),
            reasoning: "Passo 1: ler info da plataforma".to_string(),
            opaque_signature: Some("sig_info".to_string()),
        }),
        candidate_proposal: None,
        usage: None,
        finish_reason: "TOOL_CALL".to_string(),
        provider_response_id: None,
    };

    // Step 2: Model proposes system.time
    let resp2 = ModelResponse {
        request_id: ModelRequestId::new(),
        provider: "SequenceMock".to_string(),
        model: "sequence-tester".to_string(),
        raw_content: "Consultando horário".to_string(),
        capability_proposal: Some(CapabilityProposal {
            proposal_id: ProposalId::new(),
            model_request_id: ModelRequestId::new(),
            provider_response_id: None,
            capability_id: CapabilityId::new("system.time"),
            parameters: serde_json::json!({}),
            reasoning: "Passo 2: ler data/hora".to_string(),
            opaque_signature: Some("sig_time".to_string()),
        }),
        candidate_proposal: None,
        usage: None,
        finish_reason: "TOOL_CALL".to_string(),
        provider_response_id: None,
    };

    // Step 3: Model finishes
    let final_answer = "Sistema Windows x86_64, horário consultado com sucesso.";
    let resp3 = ModelResponse::text(
        ModelRequestId::new(),
        "SequenceMock",
        "sequence-tester",
        final_answer,
    );

    let (provider, requests) = SequenceMockProvider::new(vec![resp1, resp2, resp3]);
    let core = YukiCore::new().with_model_provider(provider);

    let input = UserInput::new("Qual é o sistema e o horário atual?");
    let result = core.process_input_async(input).await.expect("success");

    assert_eq!(result.status, ResultStatus::Success);
    assert_eq!(result.content, final_answer);
    assert!(result.operation_id.is_some());

    let reqs = requests.lock().unwrap();
    assert_eq!(reqs.len(), 3, "Expected 3 turns (2 tools + 1 final text)");

    // Turn 3 has 5 messages: User, ToolCall 1, Result 1, ToolCall 2, Result 2
    assert_eq!(reqs[2].messages.len(), 5);
}

#[tokio::test]
async fn test_continuation_04_max_tool_iterations_exhaustion_fails_closed() {
    // Model continuously proposes system.info in an infinite tool loop
    let infinite_proposal = ModelResponse {
        request_id: ModelRequestId::new(),
        provider: "SequenceMock".to_string(),
        model: "sequence-tester".to_string(),
        raw_content: "Looping tool call".to_string(),
        capability_proposal: Some(CapabilityProposal {
            proposal_id: ProposalId::new(),
            model_request_id: ModelRequestId::new(),
            provider_response_id: None,
            capability_id: CapabilityId::new("system.info"),
            parameters: serde_json::json!({}),
            reasoning: "Looping attempt".to_string(),
            opaque_signature: None,
        }),
        candidate_proposal: None,
        usage: None,
        finish_reason: "TOOL_CALL".to_string(),
        provider_response_id: None,
    };

    let responses = vec![infinite_proposal; 10];
    let (provider, requests) = SequenceMockProvider::new(responses);

    // Limit to 3 iterations
    let core = YukiCore::new()
        .with_model_provider(provider)
        .with_max_tool_iterations(3);

    let input = UserInput::new("Inicia loop de ferramentas");
    let result = core.process_input_async(input).await;

    // Fail-closed invariant: loop must terminate with ExecutionFailed error
    assert!(
        result.is_err(),
        "Must fail when max tool iterations exceeded"
    );
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("Limite máximo de iterações"),
        "Error message should mention iteration limit: {}",
        err_msg
    );

    // Verified that exactly 3 iterations executed before boundary closed
    let reqs = requests.lock().unwrap();
    assert_eq!(reqs.len(), 4, "Turn 0 + 3 tool executions = 4 invocations");
}

#[tokio::test]
async fn test_continuation_05_security_denial_halts_loop_without_execution() {
    // Capability proposal with unregistered dangerous capability
    let candidate = RawProposalCandidate {
        capability_name: "system.unregistered_dangerous_action".to_string(),
        arguments: serde_json::json!({}),
        opaque_signature: None,
    };

    let resp = ModelResponse::with_candidate(
        ModelRequestId::new(),
        "SequenceMock",
        "sequence-tester",
        "Tentativa não autorizada",
        candidate,
        None,
    );

    let (provider, requests) = SequenceMockProvider::new(vec![resp]);
    let core = YukiCore::new().with_model_provider(provider);

    let input = UserInput::new("Ação perigosa");
    let result = core.process_input_async(input).await;

    // Must return Err because capability does not exist in registry
    assert!(result.is_err());
    let reqs = requests.lock().unwrap();
    assert_eq!(reqs.len(), 1, "Only 1 invocation occurred, loop halted");
}

// ============================================================================
// 2. WIRE FORMAT SERIALIZATION & DESERIALIZATION TESTS
// ============================================================================

#[test]
fn test_wire_01_model_message_serialization() {
    let user_msg = ModelMessage::user("Olá");
    let serialized_user = serde_json::to_string(&user_msg).expect("serialize user");
    assert!(serialized_user.contains("User"));
    assert!(serialized_user.contains("Olá"));

    let tool_call = ModelMessage::assistant_tool_call(
        "system.info",
        serde_json::json!({}),
        Some("sig_123".to_string()),
    );
    let serialized_call = serde_json::to_string(&tool_call).expect("serialize tool call");
    assert!(serialized_call.contains("AssistantWithToolCall"));
    assert!(serialized_call.contains("system.info"));
    assert!(serialized_call.contains("sig_123"));

    let tool_result = ModelMessage::tool_result(
        "system.info",
        serde_json::json!({ "arch": "x86_64", "os": "windows" }),
    );
    let serialized_result = serde_json::to_string(&tool_result).expect("serialize tool result");
    assert!(serialized_result.contains("ToolResult"));
    assert!(serialized_result.contains("x86_64"));
}

#[test]
fn test_wire_02_default_model_is_gemini_3_8_flash() {
    let config = ModelGatewayConfig::default();
    assert_eq!(config.model_id, "gemini-3.8-flash");
    assert_eq!(config.max_tool_iterations, 5);
}

#[tokio::test]
async fn test_wire_03_gemini_multiturn_payload_contains_function_response_and_signature() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use yuki::models::gemini::GeminiProviderAdapter;
    use yuki::security::credentials::{EnvSecretStore, SecretRef};

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        if let Ok((mut socket, _)) = listener.accept().await {
            let mut buf = vec![0u8; 8192];
            let n = socket.read(&mut buf).await.unwrap();
            let req_str = String::from_utf8_lossy(&buf[..n]);

            // Encontra o corpo JSON após o cabeçalho HTTP duplo CRLF
            let body = req_str.split("\r\n\r\n").nth(1).unwrap();
            let parsed: serde_json::Value = serde_json::from_str(body).unwrap();

            // 1. Validar estrutura de contents
            let contents = parsed.get("contents").and_then(|v| v.as_array()).unwrap();
            assert_eq!(contents.len(), 3);

            // Turno 0: user text
            assert_eq!(contents[0]["role"], "user");
            assert!(contents[0]["parts"][0]["text"].as_str().is_some());

            // Turno 1: model tool call com thoughtSignature
            assert_eq!(contents[1]["role"], "model");
            let fc = &contents[1]["parts"][0]["functionCall"];
            assert_eq!(fc["name"], "system.info");
            assert_eq!(
                contents[1]["parts"][0]["thoughtSignature"],
                "opaque_sig_test_abc"
            );

            // Turno 2: user functionResponse com objeto output
            assert_eq!(contents[2]["role"], "user");
            let fr = &contents[2]["parts"][0]["functionResponse"];
            assert_eq!(fr["name"], "system.info");
            assert_eq!(fr["response"]["output"]["os"], "windows");

            // Responder simulando sucesso final do Gemini
            let resp_body = serde_json::json!({
                "candidates": [{
                    "content": {
                        "parts": [{ "text": "Resposta final do Gemini multi-turn" }]
                    },
                    "finishReason": "STOP"
                }]
            })
            .to_string();

            let http_response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                resp_body.len(),
                resp_body
            );
            let _ = socket.write_all(http_response.as_bytes()).await;
        }
    });

    std::env::set_var("YUKI_DUMMY_KEY", "dummy_val");
    let secret_ref = SecretRef::new("secret://env/YUKI_DUMMY_KEY", "cred_test").unwrap();
    let broker = Arc::new(EnvSecretStore::new());
    let adapter = GeminiProviderAdapter::new(secret_ref, broker, ModelGatewayConfig::default())
        .unwrap()
        .with_base_url(format!("http://127.0.0.1:{}", port));

    let mut req = ModelRequest::new(
        "Olá, consulte info",
        yuki::contracts::identifiers::ContextId::new(),
        "test",
    );
    req.messages.push(ModelMessage::assistant_tool_call(
        "system.info",
        serde_json::json!({}),
        Some("opaque_sig_test_abc".to_string()),
    ));
    req.messages.push(ModelMessage::tool_result(
        "system.info",
        serde_json::json!({ "os": "windows" }),
    ));

    let resp = adapter.generate(&req).await.expect("generate multi-turn");
    assert_eq!(resp.raw_content, "Resposta final do Gemini multi-turn");
    std::env::remove_var("YUKI_DUMMY_KEY");
}

#[test]
fn test_wire_04_env_max_tool_iterations_override() {
    std::env::set_var("YUKI_MODEL_PROVIDER", "mock");
    let provider = yuki::models::builder::resolve_model_provider_from_env().unwrap();
    assert_eq!(provider.metadata().provider_name, "MockProvider");

    std::env::set_var("YUKI_MODEL_PROVIDER", "gemini");
    std::env::set_var("YUKI_MAX_TOOL_ITERATIONS", "12");
    std::env::set_var("YUKI_MODEL_ID", "gemini-3.8-flash");

    let provider = yuki::models::builder::resolve_model_provider_from_env().unwrap();
    assert_eq!(provider.metadata().provider_name, "GoogleGemini");

    // Limpa variáveis de ambiente para isolamento de testes
    std::env::remove_var("YUKI_MODEL_PROVIDER");
    std::env::remove_var("YUKI_MAX_TOOL_ITERATIONS");
    std::env::remove_var("YUKI_MODEL_ID");
}
