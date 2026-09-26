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
