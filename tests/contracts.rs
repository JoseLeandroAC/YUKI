use yuki::contracts::authorization::*;
use yuki::contracts::capability::*;
use yuki::contracts::identifiers::*;
use yuki::contracts::input::UserInput;
use yuki::contracts::output::{ResultStatus, YukiResult};
use yuki::contracts::verification::*;

#[test]
fn test_identifiers_uniqueness() {
    let req1 = RequestId::new();
    let req2 = RequestId::new();
    assert_ne!(req1, req2);
    assert!(req1.0.starts_with("req_"));

    let op1 = OperationId::new();
    let op2 = OperationId::new();
    assert_ne!(op1, op2);
    assert!(op1.0.starts_with("op_"));

    let att1 = AttemptId::new();
    let att2 = AttemptId::new();
    assert_ne!(att1, att2);
    assert!(att1.0.starts_with("att_"));

    let corr1 = CorrelationId::new();
    let corr2 = CorrelationId::new();
    assert_ne!(corr1, corr2);

    let ctx1 = ContextId::new();
    let ctx2 = ContextId::new();
    assert_ne!(ctx1, ctx2);

    let evt1 = EventId::new();
    let evt2 = EventId::new();
    assert_ne!(evt1, evt2);
}

#[test]
fn test_contract_invariants() {
    // INV-FND-001: Capability != Permission
    let cap_id = CapabilityId::new("system.echo");
    let perm = PermissionRule {
        capability_id: cap_id.clone(),
        allowed_callers: vec!["yuki_core".to_string()],
        max_risk: RiskClass::Low,
    };
    assert_eq!(perm.capability_id, cap_id);

    // INV-FND-015 & INV-FND-016: UNKNOWN != SUCCESS and UNKNOWN != FAILURE
    let v_unknown = VerificationResult {
        operation_id: OperationId::new(),
        verification_state: VerificationState::Unknown,
        observed_effect_state: ObservedEffectState::Unknown,
        evidence_refs: vec![],
        evaluated_at: now_utc(),
        verification_basis: "Insufficient evidence".to_string(),
    };
    assert!(!v_unknown.is_success());
    assert!(!v_unknown.is_failure());
    assert!(v_unknown.is_unknown());

    let v_success = VerificationResult {
        operation_id: OperationId::new(),
        verification_state: VerificationState::VerifiedSuccess,
        observed_effect_state: ObservedEffectState::ObservedNoMutation,
        evidence_refs: vec![],
        evaluated_at: now_utc(),
        verification_basis: "Confirmed".to_string(),
    };
    assert!(v_success.is_success());
    assert!(!v_success.is_failure());
    assert!(!v_success.is_unknown());
}

#[test]
fn test_input_and_output_serialization() {
    let input = UserInput::new("Teste de serialização");
    let json = serde_json::to_string(&input).expect("serialize input");
    let deserialized: UserInput = serde_json::from_str(&json).expect("deserialize input");
    assert_eq!(input.content, deserialized.content);
    assert_eq!(input.request_id, deserialized.request_id);

    let res = YukiResult::success(
        input.request_id.clone(),
        CorrelationId::from_request(&input.request_id),
        Some(OperationId::new()),
        "Resposta",
        None,
    );
    assert_eq!(res.status, ResultStatus::Success);
}
