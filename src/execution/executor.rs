use crate::capabilities::registry::CapabilityRegistry;
use crate::contracts::errors::YukiError;
use crate::contracts::execution::{ExecutionRequest, ExecutionResult, OperationState};
use crate::contracts::identifiers::now_utc;
use crate::security::authorization::SecurityController;

pub struct Executor;

impl Executor {
    pub fn new() -> Self {
        Self
    }

    /// Executes an authorized operation.
    /// Invariant: Executor strictly rejects any operation lacking valid authorization.
    pub fn execute(
        &self,
        request: &ExecutionRequest,
        registry: &CapabilityRegistry,
        security: &SecurityController,
    ) -> Result<ExecutionResult, YukiError> {
        // Enforce INV-FND-002 & INV-FND-004: Execution requires explicit authorization
        if !security.validate_token(
            &request.operation_id,
            &request.capability_id,
            &request.authorization_token,
        ) {
            return Err(YukiError::UnauthorizedExecution {
                op_id: request.operation_id.to_string(),
            });
        }

        let handler = registry
            .get_handler(&request.capability_id)
            .ok_or_else(|| YukiError::CapabilityNotFound(request.capability_id.to_string()))?;

        match handler.execute(&request.input) {
            Ok(output) => Ok(ExecutionResult {
                operation_id: request.operation_id.clone(),
                attempt_id: request.attempt_id.clone(),
                state: OperationState::Completed,
                output: Some(output),
                error: None,
                executed_at: now_utc(),
            }),
            Err(e) => Ok(ExecutionResult {
                operation_id: request.operation_id.clone(),
                attempt_id: request.attempt_id.clone(),
                state: OperationState::Failed,
                output: None,
                error: Some(e.to_string()),
                executed_at: now_utc(),
            }),
        }
    }
}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}
