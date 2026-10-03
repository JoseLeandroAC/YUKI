use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::contracts::identifiers::{AttemptId, CapabilityId, EvidenceId, OperationId};
use crate::contracts::verification::{ObservedEffectState, VerificationState};

/// Default maximum size for serialized event payload JSON (64 KiB).
/// Operational limit to prevent storage abuse and accidental huge payloads.
pub const DEFAULT_MAX_PAYLOAD_BYTES: usize = 64 * 1024;

/// Durable verification record stored as an audit projection/index (ADR-009, ADR-019).
///
/// INVARIANTS:
/// - Distinct from operational workflow state.
/// - Uses a dedicated `verification_record_id` primary key, supporting multiple historical
///   evaluations for the same `operation_id` over time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationRecord {
    pub verification_record_id: String,
    pub operation_id: OperationId,
    pub attempt_id: Option<AttemptId>,
    pub capability_id: CapabilityId,
    pub strategy_id: String,
    pub verification_state: VerificationState,
    pub observed_effect_state: ObservedEffectState,
    pub evaluated_at: DateTime<Utc>,
    pub verification_basis: String,
    pub evidence_refs: Vec<EvidenceId>,
}

pub const SCHEMA_VERSION_TABLE_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS _yuki_schema_version (
    version INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    applied_at TEXT NOT NULL
);
"#;

pub const AUDIT_EVENTS_TABLE_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS audit_events (
    sequence_num INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id TEXT NOT NULL UNIQUE,
    timestamp TEXT NOT NULL,
    event_type TEXT NOT NULL,
    correlation_id TEXT NOT NULL,
    causation_id TEXT,
    operation_id TEXT,
    capability_id TEXT,
    payload_json TEXT NOT NULL,
    provenance TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_audit_correlation ON audit_events (correlation_id);
CREATE INDEX IF NOT EXISTS idx_audit_operation ON audit_events (operation_id);
CREATE INDEX IF NOT EXISTS idx_audit_timestamp ON audit_events (timestamp);
"#;

pub const VERIFICATION_RECORDS_TABLE_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS verification_records (
    verification_record_id TEXT PRIMARY KEY,
    operation_id TEXT NOT NULL,
    attempt_id TEXT,
    capability_id TEXT NOT NULL,
    strategy_id TEXT NOT NULL,
    verification_state TEXT NOT NULL,
    observed_effect_state TEXT NOT NULL,
    evaluated_at TEXT NOT NULL,
    verification_basis TEXT NOT NULL,
    evidence_refs_json TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_verification_operation ON verification_records (operation_id);
"#;

pub const PREVENT_MUTATION_TRIGGERS_SQL: &str = r#"
CREATE TRIGGER IF NOT EXISTS prevent_audit_events_update
BEFORE UPDATE ON audit_events
BEGIN
    SELECT RAISE(ABORT, 'Audit events are immutable; UPDATE is forbidden');
END;

CREATE TRIGGER IF NOT EXISTS prevent_audit_events_delete
BEFORE DELETE ON audit_events
BEGIN
    SELECT RAISE(ABORT, 'Audit events are immutable; DELETE is forbidden in runtime connection');
END;

CREATE TRIGGER IF NOT EXISTS prevent_verification_records_update
BEFORE UPDATE ON verification_records
BEGIN
    SELECT RAISE(ABORT, 'Verification records are immutable; UPDATE is forbidden');
END;

CREATE TRIGGER IF NOT EXISTS prevent_verification_records_delete
BEFORE DELETE ON verification_records
BEGIN
    SELECT RAISE(ABORT, 'Verification records are immutable; DELETE is forbidden in runtime connection');
END;
"#;
