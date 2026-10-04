use yuki::contracts::execution::{ExecutionResult, OperationState};
use yuki::contracts::identifiers::{now_utc, AttemptId, CapabilityId, EvidenceId, OperationId};
use yuki::contracts::verification::{Evidence, ObservedEffectState, VerificationState};
use yuki::verification::strategy::VerificationContext;
use yuki::verification::Verifier;

#[test]
fn test_echo_strategy_success() {
    let verifier = Verifier::new();
    let op_id = OperationId::new();
    let cap_id = CapabilityId::new("system.echo");
    let attempt_id = AttemptId::new();

    let execution = ExecutionResult {
        operation_id: op_id.clone(),
        attempt_id: attempt_id.clone(),
        state: OperationState::Completed,
        output: Some(serde_json::json!({ "echoed_message": "sucesso dinâmico" })),
        error: None,
        executed_at: now_utc(),
    };

    let evidence = Evidence::new(
        "echo_output",
        serde_json::json!({ "valid_echo": true, "echoed_message": "sucesso dinâmico" }),
        "exact_match",
    )
    .with_operation(op_id.clone())
    .with_attempt(attempt_id);

    let evidences = [evidence];
    let context = VerificationContext::new(&op_id, &cap_id, &execution, &evidences);
    let result = verifier.verify_operation(&context);

    assert_eq!(
        result.verification_state,
        VerificationState::VerifiedSuccess
    );
    assert_eq!(
        result.observed_effect_state,
        ObservedEffectState::ObservedNoMutation
    );
    assert!(result.is_success());
}

#[test]
fn test_echo_strategy_failure_with_evidence() {
    let verifier = Verifier::new();
    let op_id = OperationId::new();
    let cap_id = CapabilityId::new("system.echo");
    let attempt_id = AttemptId::new();

    let execution = ExecutionResult {
        operation_id: op_id.clone(),
        attempt_id: attempt_id.clone(),
        state: OperationState::Failed,
        output: None,
        error: Some("Erro simulado de IO".to_string()),
        executed_at: now_utc(),
    };

    let evidence = Evidence::new(
        "failure_probe",
        serde_json::json!({ "confirmed_failure": true }),
        "probe_report",
    )
    .with_operation(op_id.clone())
    .with_attempt(attempt_id);

    let evidences = [evidence];
    let context = VerificationContext::new(&op_id, &cap_id, &execution, &evidences);
    let result = verifier.verify_operation(&context);

    assert_eq!(
        result.verification_state,
        VerificationState::VerifiedFailure
    );
    assert_eq!(
        result.observed_effect_state,
        ObservedEffectState::NotObserved
    );
    assert!(result.is_failure());
}

#[test]
fn test_echo_strategy_failure_without_evidence_preserves_unknown() {
    let verifier = Verifier::new();
    let op_id = OperationId::new();
    let cap_id = CapabilityId::new("system.echo");

    let execution = ExecutionResult {
        operation_id: op_id.clone(),
        attempt_id: AttemptId::new(),
        state: OperationState::Failed,
        output: None,
        error: Some("Timeout no subsistema".to_string()),
        executed_at: now_utc(),
    };

    let context = VerificationContext::new(&op_id, &cap_id, &execution, &[]);
    let result = verifier.verify_operation(&context);

    // EPISTEMIC INVARIANT: Execution failure alone MUST NOT establish absence of external effect!
    assert_eq!(result.verification_state, VerificationState::Unknown);
    assert_eq!(result.observed_effect_state, ObservedEffectState::Unknown);
    assert!(result.is_unknown());
    assert!(!result.is_failure());
    assert!(!result.is_success());
}

#[test]
fn test_echo_strategy_unknown_conflicting_evidence() {
    let verifier = Verifier::new();
    let op_id = OperationId::new();
    let cap_id = CapabilityId::new("system.echo");
    let attempt_id = AttemptId::new();

    let execution = ExecutionResult {
        operation_id: op_id.clone(),
        attempt_id: attempt_id.clone(),
        state: OperationState::Completed,
        output: Some(serde_json::json!({ "echoed_message": "sucesso" })),
        error: None,
        executed_at: now_utc(),
    };

    let evidence = Evidence::new(
        "sensor",
        serde_json::json!({ "conflict": true }),
        "divergent_reading",
    )
    .with_operation(op_id.clone())
    .with_attempt(attempt_id);

    let evidences = [evidence];
    let context = VerificationContext::new(&op_id, &cap_id, &execution, &evidences);
    let result = verifier.verify_operation(&context);

    assert_eq!(result.verification_state, VerificationState::Unknown);
    assert_eq!(
        result.observed_effect_state,
        ObservedEffectState::Conflicting
    );
    assert!(result.is_unknown());
}

#[test]
fn test_unknown_strategy_produces_unknown_state() {
    let verifier = Verifier::new();
    let op_id = OperationId::new();
    let cap_id = CapabilityId::new("custom.unregistered");

    let execution = ExecutionResult {
        operation_id: op_id.clone(),
        attempt_id: AttemptId::new(),
        state: OperationState::Completed,
        output: Some(serde_json::json!({ "some": "data" })),
        error: None,
        executed_at: now_utc(),
    };

    let context = VerificationContext::new(&op_id, &cap_id, &execution, &[]);
    let result = verifier.verify_operation(&context);

    // INVARIANT: Unknown strategy != false success or false failure
    assert_eq!(result.verification_state, VerificationState::Unknown);
    assert_eq!(result.observed_effect_state, ObservedEffectState::Unknown);
    assert!(result.is_unknown());
    assert!(result.verification_basis.contains("desconhecida"));
}

#[test]
fn test_operation_bound_evidence_mismatch_rejected() {
    let verifier = Verifier::new();
    let op_a = OperationId::new();
    let op_b = OperationId::new();
    let cap_id = CapabilityId::new("system.echo");
    let attempt_id = AttemptId::new();

    let execution = ExecutionResult {
        operation_id: op_a.clone(),
        attempt_id: attempt_id.clone(),
        state: OperationState::Completed,
        output: Some(serde_json::json!({ "echoed_message": "msg" })),
        error: None,
        executed_at: now_utc(),
    };

    // Evidence bound to Operation B!
    let evidence_for_b = Evidence::new(
        "echo_output",
        serde_json::json!({ "valid_echo": true, "echoed_message": "msg" }),
        "exact_match",
    )
    .with_operation(op_b)
    .with_attempt(attempt_id);

    let evidences = [evidence_for_b];
    let context = VerificationContext::new(&op_a, &cap_id, &execution, &evidences);
    let result = verifier.verify_operation(&context);

    // Mismatched evidence is filtered out; resulting in empty evidence -> UNKNOWN
    assert_eq!(result.verification_state, VerificationState::Unknown);
    assert_eq!(result.observed_effect_state, ObservedEffectState::Unknown);
    assert!(result.is_unknown());
    assert!(result.evidence_refs.is_empty());
}

#[test]
fn test_attempt_bound_evidence_mismatch_rejected() {
    let verifier = Verifier::new();
    let op_id = OperationId::new();
    let cap_id = CapabilityId::new("system.echo");
    let attempt_1 = AttemptId::new();
    let attempt_2 = AttemptId::new();

    let execution = ExecutionResult {
        operation_id: op_id.clone(),
        attempt_id: attempt_1,
        state: OperationState::Completed,
        output: Some(serde_json::json!({ "echoed_message": "msg" })),
        error: None,
        executed_at: now_utc(),
    };

    // Evidence explicitly bound to Attempt 2!
    let evidence_for_attempt_2 = Evidence::new(
        "echo_output",
        serde_json::json!({ "valid_echo": true, "echoed_message": "msg" }),
        "exact_match",
    )
    .with_operation(op_id.clone())
    .with_attempt(attempt_2);

    let evidences = [evidence_for_attempt_2];
    let context = VerificationContext::new(&op_id, &cap_id, &execution, &evidences);
    let result = verifier.verify_operation(&context);

    assert_eq!(result.verification_state, VerificationState::Unknown);
    assert_eq!(result.observed_effect_state, ObservedEffectState::Unknown);
    assert!(result.is_unknown());
    assert!(result.evidence_refs.is_empty());
}

#[test]
fn test_legacy_unbound_foundation_evidence_compatibility() {
    let verifier = Verifier::new();
    let op_id = OperationId::new();
    let cap_id = CapabilityId::new("system.echo");

    let execution = ExecutionResult {
        operation_id: op_id.clone(),
        attempt_id: AttemptId::new(),
        state: OperationState::Completed,
        output: Some(serde_json::json!({ "echoed_message": "legado" })),
        error: None,
        executed_at: now_utc(),
    };

    // Unbound evidence (legacy Foundation format)
    let legacy_evidence = Evidence {
        evidence_id: EvidenceId::new(),
        source: "output_validator".to_string(),
        data: serde_json::json!({ "valid_echo": true, "echoed_message": "legado" }),
        observed_at: now_utc(),
        confidence_basis: "confirmed_match".to_string(),
        operation_id: None,
        attempt_id: None,
    };

    let evidences = [legacy_evidence];
    let context = VerificationContext::new(&op_id, &cap_id, &execution, &evidences);
    let result = verifier.verify_operation(&context);

    // Unbound evidence accepted for echo backwards compatibility
    assert_eq!(
        result.verification_state,
        VerificationState::VerifiedSuccess
    );
    assert!(result.is_success());
}

#[test]
fn test_unknown_state_never_success_or_failure() {
    let verifier = Verifier::new();
    let op_id = OperationId::new();
    let cap_id = CapabilityId::new("system.echo");

    let execution = ExecutionResult {
        operation_id: op_id.clone(),
        attempt_id: AttemptId::new(),
        state: OperationState::Completed,
        output: Some(serde_json::json!({ "echoed_message": "inconclusivo" })),
        error: None,
        executed_at: now_utc(),
    };

    // No evidence at all
    let context = VerificationContext::new(&op_id, &cap_id, &execution, &[]);
    let result = verifier.verify_operation(&context);

    // Invariant: UNKNOWN is never success and never failure
    assert_eq!(result.verification_state, VerificationState::Unknown);
    assert!(result.is_unknown());
    assert!(!result.is_success());
    assert!(!result.is_failure());
}

#[test]
fn test_execution_timeout_does_not_imply_no_mutation() {
    let verifier = Verifier::new();
    let op_id = OperationId::new();
    let cap_id = CapabilityId::new("system.echo");
    let attempt_id = AttemptId::new();

    let execution = ExecutionResult {
        operation_id: op_id.clone(),
        attempt_id: attempt_id.clone(),
        state: OperationState::Failed,
        output: None,
        error: Some("Timeout no subsistema".to_string()),
        executed_at: now_utc(),
    };

    // Sensor observes conflicting/uncertain state after timeout
    let evidence = Evidence::new(
        "external_sensor",
        serde_json::json!({ "conflict": true }),
        "sensor_report",
    )
    .with_operation(op_id.clone())
    .with_attempt(attempt_id);

    let evidences = [evidence];
    let context = VerificationContext::new(&op_id, &cap_id, &execution, &evidences);
    let result = verifier.verify_operation(&context);

    // EPISTEMIC INVARIANT: Conflicting evidence prevents assuming failure without mutation!
    // Execution Failure != Proof Of No External Effect
    assert_eq!(result.verification_state, VerificationState::Unknown);
    assert_eq!(
        result.observed_effect_state,
        ObservedEffectState::Conflicting
    );
    assert!(result.is_unknown());
}

#[test]
fn test_partially_bound_attempt_without_operation_rejected() {
    let verifier = Verifier::new();
    let op_id = OperationId::new();
    let cap_id = CapabilityId::new("system.echo");
    let attempt_id = AttemptId::new();

    let execution = ExecutionResult {
        operation_id: op_id.clone(),
        attempt_id: attempt_id.clone(),
        state: OperationState::Completed,
        output: Some(serde_json::json!({ "message": "hello" })),
        error: None,
        executed_at: now_utc(),
    };

    // Evidence bound to attempt but missing operation_id (semantically invalid)
    let evidence = Evidence::new(
        "output_sensor",
        serde_json::json!({ "valid_echo": true, "message": "hello" }),
        "sensor_report",
    )
    .with_attempt(attempt_id);

    let evidences = [evidence];
    let context = VerificationContext::new(&op_id, &cap_id, &execution, &evidences);

    // Partially bound attempt without operation must be rejected
    assert_eq!(context.valid_evidences().len(), 0);

    let result = verifier.verify_operation(&context);
    assert_eq!(result.verification_state, VerificationState::Unknown);
    assert!(result.is_unknown());
}
