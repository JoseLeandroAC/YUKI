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

#[test]
fn test_untrusted_caller_is_denied() {
    let security = SecurityController::new();
    let registry = CapabilityRegistry::new();

    let request = AuthorizationRequest {
        operation_id: OperationId::new(),
        capability_id: CapabilityId::new("system.echo"),
        context_id: ContextId::new(),
        caller_id: "untrusted_external_infiltrator".to_string(),
        input_summary: serde_json::json!({"message": "teste"}),
        risk_class: RiskClass::Low,
    };

    let decision = security.authorize(&request, &registry).expect("authorize");
    match decision {
        AuthorizationDecision::Deny { reason } => {
            assert!(reason.contains("Chamador não autorizado"));
        }
        _ => panic!("Untrusted caller must be denied, got: {:?}", decision),
    }
}

#[test]
fn test_missing_required_permission_is_denied() {
    let security = SecurityController::new();
    let mut registry = CapabilityRegistry::new();

    struct ProtectedCapability;
    impl yuki::capabilities::registry::CapabilityHandler for ProtectedCapability {
        fn manifest(&self) -> yuki::contracts::capability::CapabilityManifest {
            yuki::contracts::capability::CapabilityManifest {
                id: CapabilityId::new("protected.capability"),
                version: "0.1.0".to_string(),
                description: "Protected".to_string(),
                input_schema: serde_json::json!({}),
                output_schema: serde_json::json!({}),
                required_permissions: vec!["permission:special_admin".to_string()],
                risk_class: RiskClass::Low,
                side_effects: yuki::contracts::capability::SideEffects::None,
                network_required: false,
                filesystem_required: false,
                secrets_required: false,
            }
        }
        fn execute(
            &self,
            _input: &serde_json::Value,
        ) -> Result<serde_json::Value, yuki::contracts::errors::YukiError> {
            Ok(serde_json::json!({"ok": true}))
        }
    }

    registry.register(Box::new(ProtectedCapability));

    let request = AuthorizationRequest {
        operation_id: OperationId::new(),
        capability_id: CapabilityId::new("protected.capability"),
        context_id: ContextId::new(),
        caller_id: "test_runner".to_string(),
        input_summary: serde_json::json!({}),
        risk_class: RiskClass::Low,
    };

    let decision = security.authorize(&request, &registry).expect("authorize");
    match decision {
        AuthorizationDecision::Deny { reason } => {
            assert!(reason.contains("Permissão ausente"));
        }
        _ => panic!("Missing permission must be denied, got: {:?}", decision),
    }
}

#[test]
fn test_external_mutation_side_effect_requires_approval() {
    let security = SecurityController::new();
    let mut registry = CapabilityRegistry::new();

    struct MutatingCapability;
    impl yuki::capabilities::registry::CapabilityHandler for MutatingCapability {
        fn manifest(&self) -> yuki::contracts::capability::CapabilityManifest {
            yuki::contracts::capability::CapabilityManifest {
                id: CapabilityId::new("mutating.capability"),
                version: "0.1.0".to_string(),
                description: "Mutates state".to_string(),
                input_schema: serde_json::json!({}),
                output_schema: serde_json::json!({}),
                required_permissions: vec!["capability:system.echo".to_string()],
                risk_class: RiskClass::Low,
                side_effects: yuki::contracts::capability::SideEffects::ExternalMutation,
                network_required: false,
                filesystem_required: false,
                secrets_required: false,
            }
        }
        fn execute(
            &self,
            _input: &serde_json::Value,
        ) -> Result<serde_json::Value, yuki::contracts::errors::YukiError> {
            Ok(serde_json::json!({"mutated": true}))
        }
    }

    registry.register(Box::new(MutatingCapability));

    let request = AuthorizationRequest {
        operation_id: OperationId::new(),
        capability_id: CapabilityId::new("mutating.capability"),
        context_id: ContextId::new(),
        caller_id: "test_runner".to_string(),
        input_summary: serde_json::json!({}),
        risk_class: RiskClass::Low,
    };

    let decision = security.authorize(&request, &registry).expect("authorize");
    match decision {
        AuthorizationDecision::RequiresApproval { reason } => {
            assert!(reason.contains("mutação externa"));
        }
        _ => panic!(
            "External mutation must require approval, got: {:?}",
            decision
        ),
    }
}
