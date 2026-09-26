use yuki::contracts::events::EventType;
use yuki::contracts::input::UserInput;
use yuki::contracts::output::ResultStatus;
use yuki::core::yuki_core::YukiCore;

#[test]
fn test_i_vertical_slice_echo() {
    let core = YukiCore::new();
    let input = UserInput::new("Olá Yuki");
    let request_id = input.request_id.clone();

    let result = core.process_input(input).expect("vertical slice execution");

    // 1. Verify result status and content
    assert_eq!(result.status, ResultStatus::Success);
    assert_eq!(result.content, "Olá! Estou funcionando.");
    assert_eq!(result.request_id, request_id);
    assert!(result.operation_id.is_some());

    // 2. Verify verification result
    assert!(result.verification_result.is_some());
    let verification = result.verification_result.unwrap();
    assert!(verification.is_success());
    assert_eq!(
        verification.observed_effect_state,
        yuki::contracts::verification::ObservedEffectState::ObservedNoMutation
    );

    // 3. Verify event audit trail across all architecture boundaries
    let correlation_id = result.correlation_id;
    let events = core.event_store.get_events(&correlation_id);

    let event_types: Vec<EventType> = events.iter().map(|e| e.event_type.clone()).collect();

    assert!(
        event_types.contains(&EventType::InputReceived),
        "Must emit InputReceived"
    );
    assert!(
        event_types.contains(&EventType::ContextBuilt),
        "Must emit ContextBuilt"
    );
    assert!(
        event_types.contains(&EventType::ModelInvoked),
        "Must emit ModelInvoked"
    );
    assert!(
        event_types.contains(&EventType::CapabilityProposed),
        "Must emit CapabilityProposed"
    );
    assert!(
        event_types.contains(&EventType::AuthorizationRequested),
        "Must emit AuthorizationRequested"
    );
    assert!(
        event_types.contains(&EventType::AuthorizationGranted),
        "Must emit AuthorizationGranted"
    );
    assert!(
        event_types.contains(&EventType::OperationDispatched),
        "Must emit OperationDispatched"
    );
    assert!(
        event_types.contains(&EventType::EffectObserved),
        "Must emit EffectObserved"
    );
    assert!(
        event_types.contains(&EventType::VerificationCompleted),
        "Must emit VerificationCompleted"
    );
    assert!(
        event_types.contains(&EventType::ResponseProduced),
        "Must emit ResponseProduced"
    );

    // Check correlation and causation
    for event in &events {
        assert_eq!(event.correlation_id, correlation_id);
        assert!(!event.causation_id.0.is_empty());
    }
}

#[test]
fn test_vertical_slice_custom_repeat() {
    let core = YukiCore::new();
    let input = UserInput::new("Repita: teste da Yuki Foundation v0.1");

    let result = core.process_input(input).expect("process repeat command");
    assert_eq!(result.status, ResultStatus::Success);
    assert_eq!(result.content, "teste da Yuki Foundation v0.1");
    assert!(result.verification_result.unwrap().is_success());
}
