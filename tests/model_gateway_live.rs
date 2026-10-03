use std::sync::Arc;
use yuki::models::config::ModelGatewayConfig;
use yuki::models::gemini::GeminiProviderAdapter;
use yuki::models::provider::{ModelProvider, ModelRequest};
use yuki::security::credentials::{EnvSecretStore, SecretRef};

/// Teste de fumaça ao vivo contra o provedor Google Gemini (ADR-018).
///
/// REGRAS DE EXECUÇÃO:
/// 1. Marcado com `#[ignore]` — NUNCA executa em CI padrão ou em execuções de rotina de `cargo test`.
/// 2. Executável exclusivamente de forma manual via:
///    `cargo test --test model_gateway_live -- --ignored`
/// 3. Estritamente delimitado em custo: teto de tokens `max_output_tokens = 50`.
/// 4. Credenciais nunca são expostas em logs de teste.
/// 5. Se nenhuma chave de API estiver presente no ambiente (`GEMINI_API_KEY` ou `YUKI_GEMINI_API_KEY`),
///    o teste é ignorado graciosamente com aviso explícito.
#[tokio::test]
#[ignore]
async fn test_live_gemini_smoke_turn() {
    let env_var = if std::env::var("GEMINI_API_KEY").is_ok() {
        "GEMINI_API_KEY"
    } else if std::env::var("YUKI_GEMINI_API_KEY").is_ok() {
        "YUKI_GEMINI_API_KEY"
    } else {
        println!("[SKIPPED] Teste live ignorado: GEMINI_API_KEY ou YUKI_GEMINI_API_KEY não encontrada no ambiente.");
        return;
    };

    let secret_ref = SecretRef::new(format!("secret://env/{}", env_var), "gemini_live_key")
        .expect("valid secret ref");
    let broker = Arc::new(EnvSecretStore::new());

    let config = ModelGatewayConfig {
        max_output_tokens: 50, // Delimitação estrita de consumo
        ..Default::default()
    };

    let adapter = GeminiProviderAdapter::new(secret_ref, broker, config)
        .expect("GeminiProviderAdapter should initialize cleanly");

    let req = ModelRequest::from_prompt("Respond with the single word: YUKI");

    match adapter.generate(&req).await {
        Ok(resp) => {
            assert!(
                !resp.raw_content.trim().is_empty(),
                "Resposta do Gemini não deve ser vazia"
            );
            println!(
                "[LIVE OK] Provedor: {}, Modelo: {}, Tokens de saída delimitados.",
                resp.provider, resp.model
            );
        }
        Err(err) => {
            panic!(
                "Falha na execução live contra o provedor Gemini: {:?}. (Verifique validade da credencial ou quota)",
                err
            );
        }
    }
}
