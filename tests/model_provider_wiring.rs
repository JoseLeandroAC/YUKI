use std::sync::Mutex;
use yuki::contracts::identifiers::CapabilityId;
use yuki::core::yuki_core::YukiCore;
use yuki::models::errors::ModelError;
use yuki::models::provider::HealthStatus;
use yuki::models::resolve_model_provider_from_env;

// Serializa testes que manipulam variáveis de ambiente para evitar race conditions
static ENV_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn test_bootstrap_default_is_mock() {
    let _guard = ENV_LOCK.lock().unwrap();
    std::env::remove_var("YUKI_MODEL_PROVIDER");
    std::env::remove_var("YUKI_MODEL_ID");
    std::env::remove_var("YUKI_GEMINI_API_KEY");

    let provider =
        resolve_model_provider_from_env().expect("Default provider resolution must succeed");
    let meta = provider.metadata();

    assert_eq!(meta.provider_name, "MockProvider");
    assert_eq!(meta.model_name, "yuki-mock-reasoner-v0.1");
    assert_eq!(provider.health(), HealthStatus::Healthy);
}

#[test]
fn test_bootstrap_explicit_mock_even_with_keys() {
    let _guard = ENV_LOCK.lock().unwrap();
    std::env::set_var("YUKI_MODEL_PROVIDER", "mock");
    std::env::set_var("YUKI_GEMINI_API_KEY", "some_unused_key");

    let provider =
        resolve_model_provider_from_env().expect("Explicit mock provider resolution must succeed");
    let meta = provider.metadata();

    assert_eq!(meta.provider_name, "MockProvider");
    assert_eq!(meta.model_name, "yuki-mock-reasoner-v0.1");
    assert_eq!(provider.health(), HealthStatus::Healthy);

    std::env::remove_var("YUKI_MODEL_PROVIDER");
    std::env::remove_var("YUKI_GEMINI_API_KEY");
}

#[test]
fn test_bootstrap_explicit_gemini_wires_adapter() {
    let _guard = ENV_LOCK.lock().unwrap();
    std::env::set_var("YUKI_MODEL_PROVIDER", "gemini");
    std::env::set_var("YUKI_GEMINI_API_KEY", "mock_dummy_key_for_wiring_test");

    let provider =
        resolve_model_provider_from_env().expect("Gemini provider resolution must succeed");
    let meta = provider.metadata();

    assert_eq!(meta.provider_name, "GoogleGemini");
    assert_eq!(meta.model_name, "gemini-3.8-flash"); // default model id
    assert_eq!(meta.version, "v1beta");
    assert_eq!(provider.health(), HealthStatus::Healthy);

    std::env::remove_var("YUKI_MODEL_PROVIDER");
    std::env::remove_var("YUKI_GEMINI_API_KEY");
}

#[test]
fn test_bootstrap_gemini_without_credential_reports_degraded_health() {
    let _guard = ENV_LOCK.lock().unwrap();
    std::env::set_var("YUKI_MODEL_PROVIDER", "gemini");
    std::env::remove_var("YUKI_GEMINI_API_KEY");

    // O bootstrap constrói o adaptador e o SecretRef sem vazar segredos
    let provider = resolve_model_provider_from_env()
        .expect("Gemini provider adapter structure must build cleanly");
    let meta = provider.metadata();

    assert_eq!(meta.provider_name, "GoogleGemini");
    // Mas a saúde local reflete que a credencial não pôde ser adquirida no broker local
    match provider.health() {
        HealthStatus::Degraded(reason) => {
            assert!(
                reason.contains("cred_gemini_primary"),
                "Mensagem de degradação deve referenciar o alias sanitizado seguro"
            );
            assert!(!reason.contains("AIza"), "Mensagem não deve vazar segredos");
        }
        other => panic!("Esperado HealthStatus::Degraded, obtido: {:?}", other),
    }

    std::env::remove_var("YUKI_MODEL_PROVIDER");
}

#[test]
fn test_bootstrap_unknown_provider_fails_closed() {
    let _guard = ENV_LOCK.lock().unwrap();
    std::env::set_var("YUKI_MODEL_PROVIDER", "unsupported_provider_xyz");

    let result = resolve_model_provider_from_env();
    match result {
        Err(ModelError::Configuration(msg)) => {
            assert!(msg.contains("unsupported_provider_xyz"));
            assert!(msg.contains("mock"));
            assert!(msg.contains("gemini"));
        }
        Ok(_) => panic!("Provedor desconhecido deve falhar fechado com erro de configuração"),
        Err(other) => panic!("Esperado ModelError::Configuration, obtido: {:?}", other),
    }

    std::env::remove_var("YUKI_MODEL_PROVIDER");
}

#[test]
fn test_bootstrap_model_id_override() {
    let _guard = ENV_LOCK.lock().unwrap();
    std::env::set_var("YUKI_MODEL_PROVIDER", "gemini");
    std::env::set_var("YUKI_MODEL_ID", "gemini-custom-enterprise-v1");

    let provider = resolve_model_provider_from_env()
        .expect("Gemini provider resolution with model override must succeed");
    let meta = provider.metadata();

    assert_eq!(meta.provider_name, "GoogleGemini");
    assert_eq!(meta.model_name, "gemini-custom-enterprise-v1");

    std::env::remove_var("YUKI_MODEL_PROVIDER");
    std::env::remove_var("YUKI_MODEL_ID");
}

#[test]
fn test_bootstrap_model_id_with_models_prefix_sanitized() {
    let _guard = ENV_LOCK.lock().unwrap();
    std::env::set_var("YUKI_MODEL_PROVIDER", "gemini");
    std::env::set_var("YUKI_MODEL_ID", "models/gemini-2.5-flash");

    let provider = resolve_model_provider_from_env()
        .expect("Gemini provider resolution with models/ prefix must succeed");
    let meta = provider.metadata();

    assert_eq!(meta.provider_name, "GoogleGemini");
    assert_eq!(
        meta.model_name, "gemini-2.5-flash",
        "models/ prefix must be sanitized out from model_name"
    );

    std::env::remove_var("YUKI_MODEL_PROVIDER");
    std::env::remove_var("YUKI_MODEL_ID");
}

#[test]
fn test_bootstrap_api_version_override() {
    let _guard = ENV_LOCK.lock().unwrap();
    std::env::set_var("YUKI_MODEL_PROVIDER", "gemini");
    std::env::set_var("YUKI_API_VERSION", "v1");

    let provider = resolve_model_provider_from_env()
        .expect("Gemini provider resolution with api_version override must succeed");
    let meta = provider.metadata();

    assert_eq!(meta.provider_name, "GoogleGemini");
    assert_eq!(meta.version, "v1", "API version override must be reflected");

    std::env::remove_var("YUKI_MODEL_PROVIDER");
    std::env::remove_var("YUKI_API_VERSION");
}

#[test]
fn test_core_integration_reflects_selected_provider_metadata_and_health() {
    let _guard = ENV_LOCK.lock().unwrap();
    std::env::set_var("YUKI_MODEL_PROVIDER", "gemini");
    std::env::set_var("YUKI_GEMINI_API_KEY", "test_key_for_core_integration");

    let provider = resolve_model_provider_from_env().expect("Provider must resolve cleanly");
    let core = YukiCore::new().with_model_provider(provider);

    // O Core deve refletir exatamente o provedor injetado
    assert_eq!(core.model_provider.metadata().provider_name, "GoogleGemini");
    let health = core.health();
    assert!(
        health.model_ok,
        "Model provider health must be true when credentials resolve"
    );
    assert!(core
        .capability_registry
        .has_capability(&CapabilityId::new("system.echo")));

    std::env::remove_var("YUKI_MODEL_PROVIDER");
    std::env::remove_var("YUKI_GEMINI_API_KEY");
}

#[test]
fn test_bootstrap_preserves_secret_confidentiality() {
    let _guard = ENV_LOCK.lock().unwrap();
    let sensitive_key = "AIzaSy_SENSITIVE_SECRET_TOKEN_DO_NOT_LEAK";
    std::env::set_var("YUKI_MODEL_PROVIDER", "gemini");
    std::env::set_var("YUKI_GEMINI_API_KEY", sensitive_key);

    let provider = resolve_model_provider_from_env().expect("Provider resolution must succeed");
    let meta = provider.metadata();

    // Metadados não devem conter nenhum fragmento da chave
    assert!(!meta.provider_name.contains("SENSITIVE"));
    assert!(!meta.model_name.contains("SENSITIVE"));
    assert!(!meta.version.contains("SENSITIVE"));

    std::env::remove_var("YUKI_MODEL_PROVIDER");
    std::env::remove_var("YUKI_GEMINI_API_KEY");
}
