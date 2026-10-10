use crate::contracts::identifiers::{now_utc, CausationId, CorrelationId, EventId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventType {
    InputReceived,
    ContextBuilt,
    ModelInvoked,
    CapabilityProposed,
    AuthorizationRequested,
    AuthorizationGranted,
    AuthorizationDenied,
    OperationDispatched,
    EffectObserved,
    VerificationCompleted,
    ResponseProduced,
    SourceObserved,
    EvidenceRegistered,
    CitationResolved,
    CitationRejected,
    SynthesisCompleted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub event_id: EventId,
    pub timestamp: DateTime<Utc>,
    pub event_type: EventType,
    pub correlation_id: CorrelationId,
    pub causation_id: CausationId,
    pub payload: serde_json::Value,
    pub provenance: String,
}

impl AuditEvent {
    pub fn new(
        event_type: EventType,
        correlation_id: CorrelationId,
        causation_id: CausationId,
        payload: serde_json::Value,
        provenance: impl Into<String>,
    ) -> Self {
        Self {
            event_id: EventId::new(),
            timestamp: now_utc(),
            event_type,
            correlation_id,
            causation_id,
            payload,
            provenance: provenance.into(),
        }
    }

    /// Salvaguarda básica e preventiva da Foundation v0.1 para rejeitar eventos
    /// de auditoria contendo segredos ou credenciais em texto claro.
    ///
    /// NOTA ARQUITETURAL: Salvaguarda mínima local da Foundation; o isolamento
    /// criptográfico integral de segredos pertence ao futuro Credential Broker (ADR-008).
    pub fn assert_no_secrets(&self) -> Result<(), String> {
        let serialized = serde_json::to_string(&self.payload)
            .unwrap_or_default()
            .to_lowercase();
        let forbidden = [
            "password",
            "api_key",
            "apikey",
            "private_key",
            "secret_key",
            "refresh_token",
            "bearer ",
            "access_token",
        ];

        for item in &forbidden {
            if serialized.contains(item) {
                return Err(format!(
                    "AuditEvent payload contains forbidden secret pattern: '{}'",
                    item
                ));
            }
        }
        Ok(())
    }
}
