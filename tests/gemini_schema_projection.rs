use yuki::capabilities::echo::EchoCapability;
use yuki::capabilities::info::InfoCapability;
use yuki::capabilities::registry::CapabilityHandler;
use yuki::capabilities::time::TimeCapability;
use yuki::capabilities::validation::validate_capability_input;
use yuki::contracts::errors::YukiError;
use yuki::models::gemini::project_schema_to_gemini;
use yuki::models::mock::MockModelProvider;
use yuki::models::provider::{ModelProvider, ModelRequest};

#[test]
fn test_gemini_projection_echo_manifest() {
    let cap = EchoCapability::new();
    let manifest = cap.manifest();

    // 1. O manifesto canônico DEVE conter additionalProperties: false
    assert_eq!(
        manifest.input_schema.get("additionalProperties"),
        Some(&serde_json::Value::Bool(false)),
        "Manifesto canônico deve conter additionalProperties: false"
    );

    // 2. Projeção para o Gemini
    let projected = project_schema_to_gemini(&manifest.input_schema)
        .expect("Projeção de system.echo para Gemini deve ter sucesso");

    // 3. O esquema projetado NÃO deve conter additionalProperties
    assert!(
        projected.get("additionalProperties").is_none(),
        "Esquema projetado para o Gemini NÃO deve conter additionalProperties"
    );

    // 4. Propriedades e campos obrigatórios devem ser preservados
    assert_eq!(
        projected.get("type").and_then(|v| v.as_str()),
        Some("object")
    );
    assert_eq!(
        projected["properties"]["message"]["type"].as_str(),
        Some("string")
    );
    assert_eq!(
        projected["required"].as_array(),
        Some(&vec![serde_json::Value::String("message".to_string())])
    );

    // 5. Garantir que o manifesto canônico não foi alterado na memória
    assert_eq!(
        manifest.input_schema.get("additionalProperties"),
        Some(&serde_json::Value::Bool(false))
    );
}

#[test]
fn test_gemini_projection_time_manifest() {
    let cap = TimeCapability::new();
    let manifest = cap.manifest();

    // 1. O manifesto canônico DEVE conter additionalProperties: false
    assert_eq!(
        manifest.input_schema.get("additionalProperties"),
        Some(&serde_json::Value::Bool(false))
    );

    // 2. Projeção para o Gemini
    let projected = project_schema_to_gemini(&manifest.input_schema)
        .expect("Projeção de system.time para Gemini deve ter sucesso");

    // 3. Sem additionalProperties, com properties: {}
    assert!(projected.get("additionalProperties").is_none());
    assert_eq!(
        projected.get("type").and_then(|v| v.as_str()),
        Some("object")
    );
    assert!(projected.get("properties").is_some());
    assert!(projected["properties"].as_object().unwrap().is_empty());

    // 4. Manifesto inalterado
    assert_eq!(
        manifest.input_schema.get("additionalProperties"),
        Some(&serde_json::Value::Bool(false))
    );
}

#[test]
fn test_gemini_projection_info_manifest() {
    let cap = InfoCapability::new();
    let manifest = cap.manifest();

    assert_eq!(
        manifest.input_schema.get("additionalProperties"),
        Some(&serde_json::Value::Bool(false))
    );

    let projected = project_schema_to_gemini(&manifest.input_schema)
        .expect("Projeção de system.info para Gemini deve ter sucesso");

    assert!(projected.get("additionalProperties").is_none());
    assert_eq!(
        projected.get("type").and_then(|v| v.as_str()),
        Some("object")
    );
    assert!(projected.get("properties").is_some());
    assert!(projected["properties"].as_object().unwrap().is_empty());

    assert_eq!(
        manifest.input_schema.get("additionalProperties"),
        Some(&serde_json::Value::Bool(false))
    );
}

#[test]
fn test_gemini_projection_nested_object_recursive() {
    let canonical = serde_json::json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "type": "object",
        "properties": {
            "user": {
                "type": "object",
                "properties": {
                    "name": { "type": "string" },
                    "details": {
                        "type": "object",
                        "properties": {
                            "age": { "type": "integer" }
                        },
                        "additionalProperties": false
                    }
                },
                "required": ["name"],
                "additionalProperties": false
            },
            "tags": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string" }
                    },
                    "additionalProperties": false
                }
            }
        },
        "required": ["user"],
        "additionalProperties": false
    });

    let projected =
        project_schema_to_gemini(&canonical).expect("Projeção recursiva deve ter sucesso");

    // Verificar que additionalProperties e $schema foram removidos em todos os níveis
    let serialized = serde_json::to_string(&projected).unwrap();
    assert!(!serialized.contains("additionalProperties"));
    assert!(!serialized.contains("$schema"));

    // Verificar estrutura aninhada preservada
    assert_eq!(projected["properties"]["user"]["type"], "object");
    assert_eq!(
        projected["properties"]["user"]["properties"]["name"]["type"],
        "string"
    );
    assert_eq!(
        projected["properties"]["user"]["properties"]["details"]["properties"]["age"]["type"],
        "integer"
    );
    assert_eq!(projected["properties"]["tags"]["type"], "array");
    assert_eq!(
        projected["properties"]["tags"]["items"]["properties"]["id"]["type"],
        "string"
    );
}

#[test]
fn test_gemini_projection_execution_validation_remains_strict() {
    let echo_manifest = EchoCapability::new().manifest();

    // 1. Validação com parâmetros válidos
    let valid_input = serde_json::json!({ "message": "hello" });
    assert!(validate_capability_input(&echo_manifest.input_schema, &valid_input).is_ok());

    // 2. Validação contra payload com propriedade inesperada/adversarial
    let invalid_input = serde_json::json!({
        "message": "hello",
        "unexpected_extra_field": "injected"
    });
    let err = validate_capability_input(&echo_manifest.input_schema, &invalid_input)
        .expect_err("Manifesto canônico deve continuar rejeitando propriedades adicionais");

    match err {
        YukiError::InvalidRequest(msg) => {
            assert!(msg.contains("unexpected_extra_field"));
        }
        _ => panic!("Esperado erro de InvalidRequest"),
    }
}

#[test]
fn test_gemini_projection_fail_closed_on_unrepresentable_schemas() {
    // 1. Raiz não é objeto
    let non_object_root = serde_json::json!({ "type": "string" });
    assert!(project_schema_to_gemini(&non_object_root).is_err());

    // 2. Construção 'not' não suportada
    let not_schema = serde_json::json!({
        "type": "object",
        "not": { "type": "string" }
    });
    assert!(project_schema_to_gemini(&not_schema).is_err());

    // 3. Construção 'patternProperties' não suportada
    let pattern_props_schema = serde_json::json!({
        "type": "object",
        "patternProperties": {
            "^s_": { "type": "string" }
        }
    });
    assert!(project_schema_to_gemini(&pattern_props_schema).is_err());

    // 4. Palavra-chave desconhecida arbitrária
    let unknown_keyword_schema = serde_json::json!({
        "type": "object",
        "customUnknownAnnotation": 123
    });
    assert!(project_schema_to_gemini(&unknown_keyword_schema).is_err());
}

#[test]
fn test_regression_reproduce_additional_properties_contract_defect() {
    // Demonstra a causa raiz exata da rejeição do Gemini:
    // A serialização direta do manifesto canônico contém 'additionalProperties'
    let echo_manifest = EchoCapability::new().manifest();
    let time_manifest = TimeCapability::new().manifest();
    let info_manifest = InfoCapability::new().manifest();

    let raw_payload = serde_json::json!({
        "tools": [{
            "function_declarations": [
                {
                    "name": echo_manifest.id.0,
                    "description": echo_manifest.description,
                    "parameters": echo_manifest.input_schema
                },
                {
                    "name": time_manifest.id.0,
                    "description": time_manifest.description,
                    "parameters": time_manifest.input_schema
                },
                {
                    "name": info_manifest.id.0,
                    "description": info_manifest.description,
                    "parameters": info_manifest.input_schema
                }
            ]
        }]
    });

    let raw_serialized = serde_json::to_string(&raw_payload).unwrap();
    // Confirma que a serialização ingênua prévia continha exatamente 3 ocorrências de 'additionalProperties'
    assert_eq!(
        raw_serialized
            .matches("\"additionalProperties\":false")
            .count(),
        3,
        "Payload prévio não normalizado continha exatamente as 3 ocorrências rejeitadas pelo Gemini"
    );

    // Agora constrói com a projeção arquitetural corrigida
    let fixed_payload = serde_json::json!({
        "tools": [{
            "function_declarations": [
                {
                    "name": echo_manifest.id.0,
                    "description": echo_manifest.description,
                    "parameters": project_schema_to_gemini(&echo_manifest.input_schema).unwrap()
                },
                {
                    "name": time_manifest.id.0,
                    "description": time_manifest.description,
                    "parameters": project_schema_to_gemini(&time_manifest.input_schema).unwrap()
                },
                {
                    "name": info_manifest.id.0,
                    "description": info_manifest.description,
                    "parameters": project_schema_to_gemini(&info_manifest.input_schema).unwrap()
                }
            ]
        }]
    });

    let fixed_serialized = serde_json::to_string(&fixed_payload).unwrap();
    assert!(
        !fixed_serialized.contains("additionalProperties"),
        "Payload corrigido não deve conter nenhuma menção a additionalProperties"
    );
}

#[tokio::test]
async fn test_mock_behavior_unaffected() {
    let provider = MockModelProvider::new();
    let req = ModelRequest::from_prompt("Mensagem de teste");
    let resp = provider
        .generate(&req)
        .await
        .expect("Mock deve continuar funcionando perfeitamente");
    assert_eq!(resp.provider, "MockProvider");
    assert_eq!(resp.model, "yuki-mock-reasoner-v0.1");
    assert!(resp.candidate_proposal.is_some());
}
