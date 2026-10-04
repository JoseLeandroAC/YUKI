use std::sync::Arc;
use yuki::contracts::input::UserInput;
use yuki::contracts::output::ResultStatus;
use yuki::core::yuki_core::YukiCore;
use yuki::models::config::ModelGatewayConfig;
use yuki::models::gemini::GeminiProviderAdapter;
use yuki::models::provider::{ModelProvider, ModelRequest};
use yuki::models::resolve_model_provider_from_env;
use yuki::security::credentials::{EnvSecretStore, SecretRef};

/// Teste de fumaça ao vivo direto contra o adaptador Google Gemini (ADR-018).
///
/// REGRAS DE EXECUÇÃO:
/// 1. Marcado com `#[ignore]` — NUNCA executa em CI padrão ou em execuções de rotina de `cargo test`.
/// 2. Executável exclusivamente de forma manual e opt-in via:
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
                "[LIVE ADAPTER OK] Provedor: {}, Modelo: {}, Tokens de saída delimitados.",
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

/// Teste end-to-end de integração live do runtime:
/// resolve_model_provider_from_env() -> YukiCore -> ModelProvider -> GeminiProviderAdapter -> Google Gemini API.
///
/// Prova explicitamente:
/// 1. resolve_model_provider_from_env() selecionou GoogleGemini;
/// 2. metadata do provider confirma GoogleGemini;
/// 3. o provider foi injetado no YukiCore;
/// 4. um turno completo percorreu o caminho real do Gemini;
/// 5. a resposta é incompatível com o Mock determinístico padrão;
/// 6. nenhum secret aparece no output.
#[tokio::test]
#[ignore]
async fn test_live_gemini_core_turn() {
    // 1. Verifica presença de credencial para o teste manual opt-in
    let api_key = if let Ok(k) = std::env::var("YUKI_GEMINI_API_KEY") {
        k
    } else if let Ok(k) = std::env::var("GEMINI_API_KEY") {
        k
    } else {
        println!("[SKIPPED] Teste live end-to-end ignorado: YUKI_GEMINI_API_KEY não encontrada no ambiente.");
        return;
    };

    // Assegura que YUKI_GEMINI_API_KEY e YUKI_MODEL_PROVIDER estão configuradas
    std::env::set_var("YUKI_GEMINI_API_KEY", &api_key);
    std::env::set_var("YUKI_MODEL_PROVIDER", "gemini");

    // 2. Resolve provedor através da função oficial de bootstrap
    let provider = resolve_model_provider_from_env()
        .expect("resolve_model_provider_from_env deve resolver para Gemini com sucesso");

    // 3. Valida metadados e saúde local
    let meta = provider.metadata();
    assert_eq!(
        meta.provider_name, "GoogleGemini",
        "O provedor resolvido deve ser GoogleGemini"
    );
    assert_eq!(
        provider.health(),
        yuki::models::HealthStatus::Healthy,
        "A saúde local deve ser Healthy quando a credencial está presente"
    );

    // 4. Injeta provedor no YukiCore de produção
    let core = YukiCore::new().with_model_provider(provider);
    assert_eq!(core.model_provider.metadata().provider_name, "GoogleGemini");

    // 5. Executa turno conversacional com pergunta não trivial para o mock
    // (Não utiliza system.echo para evitar confusão com echo determinístico)
    let prompt = "What is 2 + 2? Answer with only the digit 4.";
    let user_input = UserInput::new(prompt);

    let result = core
        .process_input_async(user_input)
        .await
        .expect("Processamento pelo YukiCore com Gemini deve retornar Ok");

    assert_eq!(
        result.status,
        ResultStatus::Success,
        "Turno deve ter status Success"
    );

    // 6. Prova que o resultado veio do Gemini e NÃO do MockModelProvider
    // MockModelProvider::Standard responderia: "Entendido. Processando sua solicitação..."
    assert!(
        !result
            .content
            .contains("Como posso ajudar a avançar com segurança"),
        "A resposta não deve conter o texto padrão do MockModelProvider"
    );
    assert!(
        result.content.contains('4'),
        "A resposta do modelo deve conter o dígito '4'"
    );

    // 7. Prova ausência de vazamento de segredos no conteúdo de resposta
    assert!(
        !result.content.contains(&api_key),
        "A resposta do modelo não deve vazar a chave de API"
    );

    println!(
        "[LIVE CORE OK] Resposta real produzida pelo Gemini através do YukiCore: '{}'",
        result.content.trim()
    );

    std::env::remove_var("YUKI_MODEL_PROVIDER");
}
