use std::time::Duration;
use yuki::contracts::input::UserInput;
use yuki::contracts::output::ResultStatus;
use yuki::core::yuki_core::YukiCore;

#[tokio::test]
async fn test_async_runtime_process_input() {
    let core = YukiCore::new();
    let input = UserInput::new("Olá Yuki, teste assíncrono");

    let result = core
        .process_input_async(input)
        .await
        .expect("O processamento assíncrono deve ser bem-sucedido");

    assert_eq!(result.status, ResultStatus::Success);
    assert!(!result.content.is_empty());
}

#[tokio::test]
async fn test_async_runtime_with_timeout_succeeds_under_limit() {
    let core = YukiCore::new();
    let input = UserInput::new("system.echo: Olá mundo com timeout");

    let result = core
        .process_input_with_timeout(input, Duration::from_secs(5))
        .await
        .expect("Operação rápida não deve estourar o timeout");

    assert_eq!(result.status, ResultStatus::Success);
    assert!(result.verification_result.is_some());
}

/// Proves cooperative branch selection and future dropping behavior via tokio::select!.
/// NOTE: Dropping a future locally cancels the waiting task, but does NOT represent a general
/// distributed cancellation or rollback guarantee for external side effects.
#[tokio::test]
async fn test_async_runtime_cancellation_select() {
    let core = YukiCore::new();
    let input = UserInput::new("teste de select");

    tokio::select! {
        res = core.process_input_async(input) => {
            let res = res.expect("Pipeline executou com sucesso");
            assert_eq!(res.status, ResultStatus::Success);
        }
        _ = tokio::time::sleep(Duration::from_secs(10)) => {
            panic!("Timeout inesperado");
        }
    }
}
