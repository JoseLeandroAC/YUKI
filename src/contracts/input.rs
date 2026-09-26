use crate::contracts::identifiers::{now_utc, RequestId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInput {
    pub request_id: RequestId,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub source: String,
}

impl UserInput {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            request_id: RequestId::new(),
            content: content.into(),
            timestamp: now_utc(),
            source: "user".to_string(),
        }
    }

    pub fn with_request_id(request_id: RequestId, content: impl Into<String>) -> Self {
        Self {
            request_id,
            content: content.into(),
            timestamp: now_utc(),
            source: "user".to_string(),
        }
    }
}
