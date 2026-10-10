use crate::contracts::identifiers::{CorrelationId, OperationId, RequestId};
use crate::contracts::research::ResearchSynthesis;
use crate::contracts::verification::VerificationResult;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResultStatus {
    Success,
    Denied,
    Failed,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YukiResult {
    pub request_id: RequestId,
    pub correlation_id: CorrelationId,
    pub operation_id: Option<OperationId>,
    pub content: String,
    pub status: ResultStatus,
    pub verification_result: Option<VerificationResult>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub synthesis: Option<ResearchSynthesis>,
}

impl YukiResult {
    pub fn success(
        request_id: RequestId,
        correlation_id: CorrelationId,
        operation_id: Option<OperationId>,
        content: impl Into<String>,
        verification: Option<VerificationResult>,
    ) -> Self {
        Self {
            request_id,
            correlation_id,
            operation_id,
            content: content.into(),
            status: ResultStatus::Success,
            verification_result: verification,
            synthesis: None,
        }
    }

    pub fn with_synthesis(mut self, synthesis: ResearchSynthesis) -> Self {
        self.synthesis = Some(synthesis);
        self
    }

    pub fn denied(
        request_id: RequestId,
        correlation_id: CorrelationId,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            request_id,
            correlation_id,
            operation_id: None,
            content: reason.into(),
            status: ResultStatus::Denied,
            verification_result: None,
            synthesis: None,
        }
    }

    pub fn failed(
        request_id: RequestId,
        correlation_id: CorrelationId,
        error: impl Into<String>,
    ) -> Self {
        Self {
            request_id,
            correlation_id,
            operation_id: None,
            content: error.into(),
            status: ResultStatus::Failed,
            verification_result: None,
            synthesis: None,
        }
    }

    pub fn unknown(
        request_id: RequestId,
        correlation_id: CorrelationId,
        operation_id: Option<OperationId>,
        message: impl Into<String>,
        verification: Option<VerificationResult>,
    ) -> Self {
        Self {
            request_id,
            correlation_id,
            operation_id,
            content: message.into(),
            status: ResultStatus::Unknown,
            verification_result: verification,
            synthesis: None,
        }
    }
}
