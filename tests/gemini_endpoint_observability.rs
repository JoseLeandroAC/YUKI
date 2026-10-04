use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use yuki::models::config::ModelGatewayConfig;
use yuki::models::errors::ModelError;
use yuki::models::gemini::{
    build_gemini_endpoint, parse_gemini_error_message, sanitize_gemini_api_version,
    sanitize_gemini_model_id, scrub_potential_secrets, GeminiProviderAdapter,
};
use yuki::models::provider::{ModelProvider, ModelRequest};
use yuki::security::credentials::{EnvSecretStore, SecretRef};

#[test]
fn test_sanitize_gemini_model_id() {
    assert_eq!(
        sanitize_gemini_model_id("gemini-2.5-flash"),
        "gemini-2.5-flash"
    );
    assert_eq!(
        sanitize_gemini_model_id("models/gemini-2.5-flash"),
        "gemini-2.5-flash"
    );
    assert_eq!(
        sanitize_gemini_model_id("/models/gemini-2.5-flash"),
        "gemini-2.5-flash"
    );
    assert_eq!(
        sanitize_gemini_model_id(" models/gemini-2.5-flash "),
        "gemini-2.5-flash"
    );
    assert_eq!(
        sanitize_gemini_model_id("\"models/gemini-2.5-flash\""),
        "gemini-2.5-flash"
    );
    assert_eq!(
        sanitize_gemini_model_id("'gemini-3.8-flash'"),
        "gemini-3.8-flash"
    );
    assert_eq!(
        sanitize_gemini_model_id("gemini-custom-enterprise"),
        "gemini-custom-enterprise"
    );
}

#[test]
fn test_sanitize_gemini_api_version() {
    assert_eq!(sanitize_gemini_api_version("v1beta"), "v1beta");
    assert_eq!(sanitize_gemini_api_version("/v1beta/"), "v1beta");
    assert_eq!(sanitize_gemini_api_version(" v1 "), "v1");
    assert_eq!(sanitize_gemini_api_version(""), "v1beta");
}

#[test]
fn test_build_gemini_endpoint() {
    let ep1 = build_gemini_endpoint(
        "https://generativelanguage.googleapis.com",
        "v1beta",
        "gemini-2.5-flash",
    );
    assert_eq!(
        ep1,
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent"
    );

    // Com prefixo models/ redundantemente fornecido pelo operador
    let ep2 = build_gemini_endpoint(
        "https://generativelanguage.googleapis.com/",
        "/v1beta/",
        "models/gemini-2.5-flash",
    );
    assert_eq!(
        ep2,
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent"
    );

    // Com aspas e espaços
    let ep3 = build_gemini_endpoint(
        "https://generativelanguage.googleapis.com",
        "v1",
        " \"models/gemini-3.8-flash\" ",
    );
    assert_eq!(
        ep3,
        "https://generativelanguage.googleapis.com/v1/models/gemini-3.8-flash:generateContent"
    );
}

#[test]
fn test_scrub_potential_secrets() {
    let raw = "Error occurred with key AIzaSy_EXAMPLE_SECRET_KEY_1234567890 in request";
    let scrubbed = scrub_potential_secrets(raw);
    assert!(!scrubbed.contains("AIzaSy_EXAMPLE_SECRET_KEY_1234567890"));
    assert!(scrubbed.contains("[REDACTED_KEY]"));

    let clean = "models/gemini-2.5-flash is not found";
    assert_eq!(scrub_potential_secrets(clean), clean);
}

#[test]
fn test_parse_gemini_error_message_standard_json_404() {
    let json = serde_json::json!({
        "error": {
            "code": 404,
            "message": "models/gemini-2.5-flash is not found for API version v1beta, or is not supported for generateContent.",
            "status": "NOT_FOUND"
        }
    });
    let bytes = serde_json::to_vec(&json).unwrap();
    let msg = parse_gemini_error_message(&bytes, 404);

    assert!(msg.contains("NOT_FOUND"));
    assert!(msg.contains("models/gemini-2.5-flash is not found"));
}

#[test]
fn test_parse_gemini_error_message_string_code() {
    let json = serde_json::json!({
        "error": {
            "code": "invalid_request",
            "message": "Unsupported argument passed."
        }
    });
    let bytes = serde_json::to_vec(&json).unwrap();
    let msg = parse_gemini_error_message(&bytes, 400);

    assert!(msg.contains("invalid_request"));
    assert!(msg.contains("Unsupported argument passed."));
}

#[test]
fn test_parse_gemini_error_message_plaintext_proxy_fallback() {
    let text = b"502 Bad Gateway: upstream server disconnected";
    let msg = parse_gemini_error_message(text, 502);

    assert_eq!(msg, "502 Bad Gateway: upstream server disconnected");
}

#[tokio::test]
async fn test_adapter_offline_404_error_preserves_sanitized_message() {
    // 1. Inicia um listener TCP local simulando o endpoint Gemini
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        if let Ok((mut socket, _)) = listener.accept().await {
            let mut buf = [0u8; 4096];
            let _ = socket.read(&mut buf).await;

            let response_body = serde_json::json!({
                "error": {
                    "code": 404,
                    "message": "models/gemini-2.5-flash is not found for API version v1beta, or is not supported for generateContent. Call ListModels to see the list of available models and their supported methods.",
                    "status": "NOT_FOUND"
                }
            }).to_string();

            let http_response = format!(
                "HTTP/1.1 404 Not Found\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                response_body.len(),
                response_body
            );
            let _ = socket.write_all(http_response.as_bytes()).await;
        }
    });

    std::env::set_var("YUKI_TEST_GEMINI_KEY", "test_key_dummy_val");
    let secret_ref =
        SecretRef::new("secret://env/YUKI_TEST_GEMINI_KEY", "cred_test_alias").unwrap();
    let broker = Arc::new(EnvSecretStore::new());

    let config = ModelGatewayConfig {
        model_id: "models/gemini-2.5-flash".to_string(), // prefixo redundante
        ..Default::default()
    };

    let adapter = GeminiProviderAdapter::new(secret_ref, broker, config)
        .unwrap()
        .with_base_url(format!("http://127.0.0.1:{}", port));

    let req = ModelRequest::from_prompt("Olá");
    let result = adapter.generate(&req).await;

    assert!(result.is_err(), "Adapter deve retornar erro para HTTP 404");
    match result.unwrap_err() {
        ModelError::InvalidRequest(msg) => {
            // Prova fundamental: a mensagem de erro do Google NÃO foi descartada!
            assert!(
                msg.contains("HTTP 404"),
                "Mensagem deve classificar explicitamente como HTTP 404: {}",
                msg
            );
            assert!(
                msg.contains("NOT_FOUND"),
                "Mensagem deve conter o status retornado pelo Google: {}",
                msg
            );
            assert!(
                msg.contains("models/gemini-2.5-flash is not found"),
                "Mensagem deve conter a descrição detalhada enviada pelo provedor: {}",
                msg
            );
        }
        other => panic!(
            "Esperava ModelError::InvalidRequest com diagnóstico 404, obteve: {:?}",
            other
        ),
    }

    std::env::remove_var("YUKI_TEST_GEMINI_KEY");
}

#[tokio::test]
async fn test_adapter_offline_strips_models_prefix_in_wire_url() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let (tx, rx) = tokio::sync::oneshot::channel::<String>();

    tokio::spawn(async move {
        if let Ok((mut socket, _)) = listener.accept().await {
            let mut buf = [0u8; 4096];
            let n = socket.read(&mut buf).await.unwrap_or(0);
            let req_str = String::from_utf8_lossy(&buf[..n]).to_string();
            let first_line = req_str.lines().next().unwrap_or("").to_string();
            let _ = tx.send(first_line);

            // Responde 200 OK com candidate simulado
            let resp_body = serde_json::json!({
                "candidates": [{
                    "content": {
                        "parts": [{ "text": "Wire test OK" }]
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

    std::env::set_var("YUKI_TEST_GEMINI_KEY_2", "test_key_dummy_val_2");
    let secret_ref =
        SecretRef::new("secret://env/YUKI_TEST_GEMINI_KEY_2", "cred_test_alias_2").unwrap();
    let broker = Arc::new(EnvSecretStore::new());

    let config = ModelGatewayConfig {
        model_id: "models/gemini-2.5-flash".to_string(), // prefixo models/
        api_version: "/v1beta/".to_string(),             // barras extras
        ..Default::default()
    };

    let adapter = GeminiProviderAdapter::new(secret_ref, broker, config)
        .unwrap()
        .with_base_url(format!("http://127.0.0.1:{}", port));

    let req = ModelRequest::from_prompt("Ping");
    let resp = adapter.generate(&req).await.unwrap();
    assert_eq!(resp.raw_content, "Wire test OK");

    let first_line = rx.await.unwrap();
    // Prova que a URL na linha HTTP é /v1beta/models/gemini-2.5-flash:generateContent
    // e NÃO /v1beta/models/models/...
    assert!(
        first_line.contains("/v1beta/models/gemini-2.5-flash:generateContent"),
        "A requisição HTTP na linha de comando deve ter o endpoint canônico limpo: {}",
        first_line
    );
    assert!(
        !first_line.contains("/models/models/"),
        "A requisição HTTP NUNCA deve conter models/ duplicado: {}",
        first_line
    );

    std::env::remove_var("YUKI_TEST_GEMINI_KEY_2");
}

#[tokio::test]
async fn test_adapter_offline_unexpected_status_preserves_error_details() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        if let Ok((mut socket, _)) = listener.accept().await {
            let mut buf = [0u8; 4096];
            let _ = socket.read(&mut buf).await;

            let response_body = serde_json::json!({
                "error": {
                    "code": 418,
                    "message": "I'm a teapot: customized gateway rejection.",
                    "status": "TEAPOT"
                }
            })
            .to_string();

            let http_response = format!(
                "HTTP/1.1 418 I'm a teapot\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                response_body.len(),
                response_body
            );
            let _ = socket.write_all(http_response.as_bytes()).await;
        }
    });

    std::env::set_var("YUKI_TEST_GEMINI_KEY_3", "test_key_dummy_val_3");
    let secret_ref =
        SecretRef::new("secret://env/YUKI_TEST_GEMINI_KEY_3", "cred_test_alias_3").unwrap();
    let broker = Arc::new(EnvSecretStore::new());

    let config = ModelGatewayConfig::default();
    let adapter = GeminiProviderAdapter::new(secret_ref, broker, config)
        .unwrap()
        .with_base_url(format!("http://127.0.0.1:{}", port));

    let req = ModelRequest::from_prompt("Teapot test");
    let result = adapter.generate(&req).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        ModelError::Internal(msg) => {
            // Prova: o status inesperado 418 NÃO descarta o corpo do erro
            assert!(msg.contains("418"), "Deve conter o status HTTP: {}", msg);
            assert!(
                msg.contains("TEAPOT"),
                "Deve conter o status string: {}",
                msg
            );
            assert!(
                msg.contains("customized gateway rejection"),
                "Deve conter a mensagem: {}",
                msg
            );
        }
        other => panic!("Esperava ModelError::Internal, obteve: {:?}", other),
    }

    std::env::remove_var("YUKI_TEST_GEMINI_KEY_3");
}
