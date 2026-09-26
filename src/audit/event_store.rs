use crate::contracts::errors::YukiError;
use crate::contracts::events::AuditEvent;
use crate::contracts::identifiers::CorrelationId;
use std::sync::RwLock;

pub trait EventStore: Send + Sync {
    fn record(&self, event: AuditEvent) -> Result<(), YukiError>;
    fn get_events(&self, correlation_id: &CorrelationId) -> Vec<AuditEvent>;
    fn all_events(&self) -> Vec<AuditEvent>;
}

pub struct InMemoryEventStore {
    events: RwLock<Vec<AuditEvent>>,
}

impl InMemoryEventStore {
    pub fn new() -> Self {
        Self {
            events: RwLock::new(Vec::new()),
        }
    }
}

impl Default for InMemoryEventStore {
    fn default() -> Self {
        Self::new()
    }
}

impl EventStore for InMemoryEventStore {
    fn record(&self, event: AuditEvent) -> Result<(), YukiError> {
        // Enforce security invariant: no secrets in audit logs
        if let Err(err) = event.assert_no_secrets() {
            return Err(YukiError::SecurityViolation(err));
        }

        let mut lock = self.events.write().map_err(|e| {
            YukiError::ExecutionFailed(format!(
                "Failed to acquire write lock on event store: {}",
                e
            ))
        })?;
        lock.push(event);
        Ok(())
    }

    fn get_events(&self, correlation_id: &CorrelationId) -> Vec<AuditEvent> {
        let lock = self.events.read().unwrap();
        lock.iter()
            .filter(|e| e.correlation_id == *correlation_id)
            .cloned()
            .collect()
    }

    fn all_events(&self) -> Vec<AuditEvent> {
        let lock = self.events.read().unwrap();
        lock.clone()
    }
}
