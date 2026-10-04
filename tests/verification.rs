use yuki::contracts::execution::{ExecutionResult, OperationState};
use yuki::contracts::identifiers::{now_utc, AttemptId, EvidenceId, OperationId};
use yuki::contracts::verification::{Evidence, ObservedEffectState, VerificationState};
use yuki::verification::verifier::Verifier;

#[test]
fn test_g_unknown_remains_unknown_when_evidence_is_insufficient() {
    let verifier = Verifier::new();
    let op_id = OperationId::new();

    let execution = ExecutionResult {
        operation_id: op_id.clone(),
        attempt_id: AttemptId::new(),
        state: OperationState::Completed,
        output: Some(serde_json::json!({ "message": "done" })),
        error: None,
        executed_at: now_utc(),
    };

    // No evidence provided at all
    let empty_evidences = vec![];
    let result = verifier.verify(&op_id, &execution, &empty_evidences);

    assert_eq!(result.verification_state, VerificationState::Unknown);
    assert_eq!(result.observed_effect_state, ObservedEffectState::Unknown);
    assert!(result.is_unknown());
    assert!(!result.is_success());
    assert!(!result.is_failure());
}

#[test]
fn test_verification_confirms_success_with_valid_evidence() {
    let verifier = Verifier::new();
    let op_id = OperationId::new();

    let execution = ExecutionResult {
        operation_id: op_id.clone(),
        attempt_id: AttemptId::new(),
        state: OperationState::Completed,
        output: Some(serde_json::json!({ "echoed_message": "sucesso" })),
        error: None,
        executed_at: now_utc(),
    };

    let evidences = vec![Evidence {
        evidence_id: EvidenceId::new(),
        source: "output_validator".to_string(),
        data: serde_json::json!({ "valid_echo": true, "echoed_message": "sucesso" }),
        observed_at: now_utc(),
        confidence_basis: "confirmed_match".to_string(),
        operation_id: None,
        attempt_id: None,
    }];

    let result = verifier.verify(&op_id, &execution, &evidences);

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
fn test_verification_confirms_failure_when_execution_fails() {
    let verifier = Verifier::new();
    let op_id = OperationId::new();

    let execution = ExecutionResult {
        operation_id: op_id.clone(),
        attempt_id: AttemptId::new(),
        state: OperationState::Failed,
        output: None,
        error: Some("Falha comprovada na capacidade".to_string()),
        executed_at: now_utc(),
    };

    let evidences = vec![Evidence {
        evidence_id: EvidenceId::new(),
        source: "failure_probe".to_string(),
        data: serde_json::json!({ "confirmed_failure": true }),
        observed_at: now_utc(),
        confidence_basis: "confirmed_failure".to_string(),
        operation_id: None,
        attempt_id: None,
    }];

    let result = verifier.verify(&op_id, &execution, &evidences);

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
fn test_verification_failure_without_evidence_preserves_unknown() {
    let verifier = Verifier::new();
    let op_id = OperationId::new();

    let execution = ExecutionResult {
        operation_id: op_id.clone(),
        attempt_id: AttemptId::new(),
        state: OperationState::Failed,
        output: None,
        error: Some("Timeout no subsistema".to_string()),
        executed_at: now_utc(),
    };

    let result = verifier.verify(&op_id, &execution, &[]);

    // Epistemic invariant (ADR-009): Execution Failure != Proof Of No External Effect
    assert_eq!(result.verification_state, VerificationState::Unknown);
    assert_eq!(result.observed_effect_state, ObservedEffectState::Unknown);
    assert!(result.is_unknown());
    assert!(!result.is_failure());
    assert!(!result.is_success());
}

#[test]
fn test_conflicting_evidence_produces_unknown_conflicting() {
    let verifier = Verifier::new();
    let op_id = OperationId::new();

    let execution = ExecutionResult {
        operation_id: op_id.clone(),
        attempt_id: AttemptId::new(),
        state: OperationState::Completed,
        output: Some(serde_json::json!({ "echoed_message": "sucesso" })),
        error: None,
        executed_at: now_utc(),
    };

    let evidences = vec![Evidence {
        evidence_id: EvidenceId::new(),
        source: "external_sensor".to_string(),
        data: serde_json::json!({ "conflict": true, "details": "sensor reports conflicting state" }),
        observed_at: now_utc(),
        confidence_basis: "sensor_discordance".to_string(),
        operation_id: None,
        attempt_id: None,
    }];

    let result = verifier.verify(&op_id, &execution, &evidences);

    assert_eq!(result.verification_state, VerificationState::Unknown);
    assert_eq!(
        result.observed_effect_state,
        ObservedEffectState::Conflicting
    );
    assert!(result.is_unknown());
    assert!(!result.is_success());
    assert!(!result.is_failure());
}
