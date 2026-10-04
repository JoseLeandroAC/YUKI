use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum YukiError {
    #[error("Security violation: {0}")]
    SecurityViolation(String),

    #[error("Authorization denied: {0}")]
    AuthorizationDenied(String),

    #[error("Capability not found: {0}")]
    CapabilityNotFound(String),

    #[error("Unauthorized execution: operation {op_id} requires valid token")]
    UnauthorizedExecution { op_id: String },

    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Verification error: {0}")]
    VerificationError(String),

    #[error("Context error: {0}")]
    ContextError(String),

    #[error("Model error: {0}")]
    ModelError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),
}

impl From<crate::models::errors::ModelError> for YukiError {
    fn from(err: crate::models::errors::ModelError) -> Self {
        YukiError::ModelError(err.to_string())
    }
}
