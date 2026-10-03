use yuki::capabilities::registry::CapabilityRegistry;
use yuki::contracts::errors::YukiError;
use yuki::contracts::execution::ExecutionRequest;
use yuki::contracts::identifiers::{AttemptId, CapabilityId, CapabilityToken, OperationId};
use yuki::execution::executor::Executor;
use yuki::security::authorization::SecurityController;

#[test]
fn test_h_execution_requires_valid_authorization() {
    let executor = Executor::new();
    let registry = CapabilityRegistry::new();
    let security = SecurityController::new();

    let op_id = OperationId::new();
    let cap_id = CapabilityId::new("system.echo");

    // Attempt to execute with a forged/unauthorized token
    let forged_token = CapabilityToken::new("forged_fake_token_12345");
    let request = ExecutionRequest {
        operation_id: op_id.clone(),
        attempt_id: AttemptId::new(),
        capability_id: cap_id.clone(),
        authorization_token: forged_token,
        input: serde_json::json!({ "message": "tentativa sem autorização" }),
    };

    let result = executor.execute(&request, &registry, &security);

    match result {
        Err(YukiError::UnauthorizedExecution { op_id: failed_op }) => {
            assert_eq!(failed_op, op_id.to_string());
        }
        other => panic!("Expected UnauthorizedExecution error, got: {:?}", other),
    }
}

#[test]
fn test_expired_token_is_rejected_by_executor() {
    let executor = Executor::new();
    let registry = CapabilityRegistry::new();
    let security = SecurityController::new();

    let op_id = OperationId::new();
    let cap_id = CapabilityId::new("system.echo");

    let auth_req = yuki::contracts::authorization::AuthorizationRequest {
        operation_id: op_id.clone(),
        capability_id: cap_id.clone(),
        context_id: yuki::contracts::identifiers::ContextId::new(),
        caller_id: "test_runner".to_string(),
        input_summary: serde_json::json!({ "message": "teste" }),
        risk_class: yuki::contracts::capability::RiskClass::Low,
    };

    // Issue token with negative TTL (already expired)
    let decision = security
        .authorize_with_ttl(&auth_req, &registry, -10)
        .expect("authorize");
    let expired_token = decision.get_token().expect("token").clone();

    let request = ExecutionRequest {
        operation_id: op_id.clone(),
        attempt_id: AttemptId::new(),
        capability_id: cap_id,
        authorization_token: expired_token,
        input: serde_json::json!({ "message": "execução expirada" }),
    };

    let result = executor.execute(&request, &registry, &security);
    assert!(
        matches!(result, Err(YukiError::UnauthorizedExecution { .. })),
        "Expired token must be rejected with UnauthorizedExecution"
    );
}

#[test]
fn test_token_for_different_operation_is_rejected_by_executor() {
    let executor = Executor::new();
    let registry = CapabilityRegistry::new();
    let security = SecurityController::new();

    let op_legit = OperationId::new();
    let op_attacker = OperationId::new();
    let cap_id = CapabilityId::new("system.echo");

    let auth_req = yuki::contracts::authorization::AuthorizationRequest {
        operation_id: op_legit.clone(),
        capability_id: cap_id.clone(),
        context_id: yuki::contracts::identifiers::ContextId::new(),
        caller_id: "test_runner".to_string(),
        input_summary: serde_json::json!({ "message": "teste" }),
        risk_class: yuki::contracts::capability::RiskClass::Low,
    };

    let decision = security.authorize(&auth_req, &registry).expect("authorize");
    let token = decision.get_token().expect("token").clone();

    // Attacker attempts to use op_legit's token for op_attacker
    let request = ExecutionRequest {
        operation_id: op_attacker.clone(),
        attempt_id: AttemptId::new(),
        capability_id: cap_id,
        authorization_token: token,
        input: serde_json::json!({ "message": "tentativa com token de outra operação" }),
    };

    let result = executor.execute(&request, &registry, &security);
    assert!(
        matches!(result, Err(YukiError::UnauthorizedExecution { .. })),
        "Token bound to another operation must be rejected"
    );
}

#[test]
fn test_token_for_different_capability_is_rejected_by_executor() {
    let executor = Executor::new();
    let mut registry = CapabilityRegistry::new();
    let security = SecurityController::new();

    let op_id = OperationId::new();
    let cap_echo = CapabilityId::new("system.echo");
    let cap_other = CapabilityId::new("system.other");

    // Register dummy capability
    struct OtherCapability;
    impl yuki::capabilities::registry::CapabilityHandler for OtherCapability {
        fn manifest(&self) -> yuki::contracts::capability::CapabilityManifest {
            yuki::contracts::capability::CapabilityManifest {
                id: CapabilityId::new("system.other"),
                version: "0.1.0".to_string(),
                description: "Other".to_string(),
                input_schema: serde_json::json!({}),
                output_schema: serde_json::json!({}),
                required_permissions: vec!["capability:system.echo".to_string()],
                risk_class: yuki::contracts::capability::RiskClass::Low,
                side_effects: yuki::contracts::capability::SideEffects::None,
                network_required: false,
                filesystem_required: false,
                secrets_required: false,
            }
        }
        fn execute(&self, _input: &serde_json::Value) -> Result<serde_json::Value, YukiError> {
            Ok(serde_json::json!({ "ok": true }))
        }
    }
    registry.register(Box::new(OtherCapability));

    // Authorize token for system.echo
    let auth_req = yuki::contracts::authorization::AuthorizationRequest {
        operation_id: op_id.clone(),
        capability_id: cap_echo,
        context_id: yuki::contracts::identifiers::ContextId::new(),
        caller_id: "test_runner".to_string(),
        input_summary: serde_json::json!({ "message": "teste" }),
        risk_class: yuki::contracts::capability::RiskClass::Low,
    };

    let decision = security.authorize(&auth_req, &registry).expect("authorize");
    let token = decision.get_token().expect("token").clone();

    // Attempt to use token authorized for echo to execute system.other
    let request = ExecutionRequest {
        operation_id: op_id.clone(),
        attempt_id: AttemptId::new(),
        capability_id: cap_other,
        authorization_token: token,
        input: serde_json::json!({}),
    };

    let result = executor.execute(&request, &registry, &security);
    assert!(
        matches!(result, Err(YukiError::UnauthorizedExecution { .. })),
        "Token bound to capability A must not authorize capability B"
    );
}

#[test]
fn test_single_use_token_replay_attack_is_rejected() {
    let executor = Executor::new();
    let registry = CapabilityRegistry::new();
    let security = SecurityController::new();

    let op_id = OperationId::new();
    let cap_id = CapabilityId::new("system.echo");

    let auth_req = yuki::contracts::authorization::AuthorizationRequest {
        operation_id: op_id.clone(),
        capability_id: cap_id.clone(),
        context_id: yuki::contracts::identifiers::ContextId::new(),
        caller_id: "test_runner".to_string(),
        input_summary: serde_json::json!({ "message": "teste replay" }),
        risk_class: yuki::contracts::capability::RiskClass::Low,
    };

    let decision = security.authorize(&auth_req, &registry).expect("authorize");
    let token = decision.get_token().expect("token").clone();

    let request_1 = ExecutionRequest {
        operation_id: op_id.clone(),
        attempt_id: AttemptId::new(),
        capability_id: cap_id.clone(),
        authorization_token: token.clone(),
        input: serde_json::json!({ "message": "primeira execução" }),
    };

    // First execution: should succeed and consume token
    let result_1 = executor.execute(&request_1, &registry, &security);
    assert!(result_1.is_ok(), "First execution must succeed");

    // Second execution with SAME token (replay attempt): must fail!
    let request_2 = ExecutionRequest {
        operation_id: op_id.clone(),
        attempt_id: AttemptId::new(),
        capability_id: cap_id,
        authorization_token: token,
        input: serde_json::json!({ "message": "segunda execução indevida" }),
    };

    let result_2 = executor.execute(&request_2, &registry, &security);
    assert!(
        matches!(result_2, Err(YukiError::UnauthorizedExecution { .. })),
        "Replay attack with consumed single-use token must be rejected"
    );
}
