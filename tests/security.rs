use yuki::audit::event_store::{EventStore, InMemoryEventStore};
use yuki::context::builder::ContextBuilder;
use yuki::contracts::context::ContextRequest;
use yuki::contracts::errors::YukiError;
use yuki::contracts::events::{AuditEvent, EventType};
use yuki::contracts::identifiers::{CausationId, CorrelationId};
use yuki::models::mock::{MockBehavior, MockModelProvider};
use yuki::models::provider::{CapabilityProposal, ModelProvider, ModelRequest};

#[test]
fn test_d_model_output_cannot_execute_directly() {
    let mock = MockModelProvider::with_behavior(MockBehavior::ForceProposal(CapabilityProposal {
        capability_id: yuki::contracts::identifiers::CapabilityId::new("malicious.shell_execute"),
        parameters: serde_json::json!({ "cmd": "rm -rf /" }),
        reasoning: "Attempting direct execution".to_string(),
    }));

    let req = ModelRequest {
        prompt: "Run shell command".to_string(),
        context_id: yuki::contracts::identifiers::ContextId::new(),
        purpose: "malicious_attempt".to_string(),
    };

    let resp = mock.generate(&req).expect("model generates proposal");

    // Invariant: The model's response is ONLY a proposal
    assert!(resp.capability_proposal.is_some());
    let proposal = resp.capability_proposal.unwrap();
    assert_eq!(proposal.capability_id.0, "malicious.shell_execute");

    // The model has NO execution method, NO authorization capability, and NO direct access to OS or executor.
    // The proposal MUST pass to SecurityController, where it is rejected because it is not registered.
    let registry = yuki::capabilities::registry::CapabilityRegistry::new();
    let security = yuki::security::authorization::SecurityController::new();

    let auth_req = yuki::contracts::authorization::AuthorizationRequest {
        operation_id: yuki::contracts::identifiers::OperationId::new(),
        capability_id: proposal.capability_id.clone(),
        context_id: req.context_id,
        caller_id: "test".to_string(),
        input_summary: proposal.parameters,
        risk_class: yuki::contracts::capability::RiskClass::High,
    };

    let decision = security.authorize(&auth_req, &registry).expect("authorize");
    assert!(
        !decision.is_allowed(),
        "Model proposal was not and cannot be executed directly"
    );
}

#[test]
fn test_e_context_rejects_and_prevents_secrets() {
    let req_with_secret = ContextRequest {
        purpose: "user_task".to_string(),
        user_input: "Aqui está minha chave: api_key=sk-123456789abcdef".to_string(),
        source: "cli".to_string(),
    };

    let result = ContextBuilder::build(&req_with_secret);
    match result {
        Err(YukiError::ContextError(msg)) => {
            assert!(msg.contains("api_key"));
        }
        Ok(_) => panic!("ContextBuilder must not allow secrets into ContextObject"),
        Err(other) => panic!("Expected ContextError, got: {:?}", other),
    }

    let req_with_password = ContextRequest {
        purpose: "user_task".to_string(),
        user_input: "Minha password secreta é 123456".to_string(),
        source: "cli".to_string(),
    };

    let result_pwd = ContextBuilder::build(&req_with_password);
    assert!(matches!(result_pwd, Err(YukiError::ContextError(_))));
}

#[test]
fn test_f_logs_reject_secrets() {
    let store = InMemoryEventStore::new();

    let event_with_secret = AuditEvent::new(
        EventType::InputReceived,
        CorrelationId::new(),
        CausationId::new("user"),
        serde_json::json!({
            "prompt": "Here is bearer secret_token_xyz"
        }),
        "test",
    );

    let record_result = store.record(event_with_secret);
    match record_result {
        Err(YukiError::SecurityViolation(msg)) => {
            assert!(msg.contains("forbidden secret pattern"));
        }
        Ok(_) => panic!("EventStore must reject events containing secrets"),
        Err(other) => panic!("Expected SecurityViolation, got: {:?}", other),
    }
}
