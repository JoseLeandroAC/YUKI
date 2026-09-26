use yuki::capabilities::registry::CapabilityRegistry;
use yuki::contracts::authorization::*;
use yuki::contracts::capability::RiskClass;
use yuki::contracts::identifiers::{CapabilityId, ContextId, OperationId};
use yuki::security::authorization::SecurityController;

#[test]
fn test_b_nonexistent_capability_is_rejected() {
    let security = SecurityController::new();
    let registry = CapabilityRegistry::new();

    let request = AuthorizationRequest {
        operation_id: OperationId::new(),
        capability_id: CapabilityId::new("nonexistent.arbitrary_tool"),
        context_id: ContextId::new(),
        caller_id: "test_runner".to_string(),
        input_summary: serde_json::json!({}),
        risk_class: RiskClass::Low,
    };

    let decision = security
        .authorize(&request, &registry)
        .expect("authorization check should succeed in returning decision");

    match decision {
        AuthorizationDecision::Deny { reason } => {
            assert!(reason.contains("não existe no registro"));
        }
        _ => panic!(
            "Expected Deny for nonexistent capability, got: {:?}",
            decision
        ),
    }
}

#[test]
fn test_c_unauthorized_high_risk_capability_is_not_allowed() {
    let security = SecurityController::new();
    let mut registry = CapabilityRegistry::new();

    // Register a dummy critical capability
    struct CriticalCapability;
    impl yuki::capabilities::registry::CapabilityHandler for CriticalCapability {
        fn manifest(&self) -> yuki::contracts::capability::CapabilityManifest {
            yuki::contracts::capability::CapabilityManifest {
                id: CapabilityId::new("critical.operation"),
                version: "0.1.0".to_string(),
                description: "Critical action".to_string(),
                input_schema: serde_json::json!({}),
                output_schema: serde_json::json!({}),
                required_permissions: vec!["critical".to_string()],
                risk_class: RiskClass::Critical,
                side_effects: yuki::contracts::capability::SideEffects::ExternalMutation,
                network_required: true,
                filesystem_required: true,
                secrets_required: false,
            }
        }
        fn execute(
            &self,
            _input: &serde_json::Value,
        ) -> Result<serde_json::Value, yuki::contracts::errors::YukiError> {
            Ok(serde_json::json!({"status": "executed"}))
        }
    }

    registry.register(Box::new(CriticalCapability));

    let request = AuthorizationRequest {
        operation_id: OperationId::new(),
        capability_id: CapabilityId::new("critical.operation"),
        context_id: ContextId::new(),
        caller_id: "test_runner".to_string(),
        input_summary: serde_json::json!({}),
        risk_class: RiskClass::Critical,
    };

    let decision = security
        .authorize(&request, &registry)
        .expect("authorization check");

    assert!(
        !decision.is_allowed(),
        "Critical capability must not be ALLOW in MVP-0"
    );
    match decision {
        AuthorizationDecision::Deny { reason } => {
            assert!(reason.contains("Crítico"));
        }
        _ => panic!("Expected Deny for critical risk, got: {:?}", decision),
    }
}

#[test]
fn test_valid_authorization_issues_verifiable_token() {
    let security = SecurityController::new();
    let registry = CapabilityRegistry::new();
    let op_id = OperationId::new();
    let cap_id = CapabilityId::new("system.echo");

    let request = AuthorizationRequest {
        operation_id: op_id.clone(),
        capability_id: cap_id.clone(),
        context_id: ContextId::new(),
        caller_id: "test_runner".to_string(),
        input_summary: serde_json::json!({"message": "test"}),
        risk_class: RiskClass::Low,
    };

    let decision = security.authorize(&request, &registry).expect("authorize");
    assert!(decision.is_allowed());

    let token = decision.get_token().expect("token must be present");
    assert!(security.validate_token(&op_id, &cap_id, token));

    // Token for a different operation must fail
    let different_op = OperationId::new();
    assert!(!security.validate_token(&different_op, &cap_id, token));

    // Token for a different capability must fail
    let different_cap = CapabilityId::new("other.cap");
    assert!(!security.validate_token(&op_id, &different_cap, token));
}
