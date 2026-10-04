use std::path::Path;
use std::sync::{Arc, Mutex};

use chrono::DateTime;
use rusqlite::{params, Connection};

use crate::contracts::events::{AuditEvent, EventType};
use crate::contracts::identifiers::{CausationId, CorrelationId, EventId, OperationId};
use crate::persistence::errors::PersistenceError;
use crate::persistence::migrations::MigrationManager;
use crate::persistence::schema::{VerificationRecord, DEFAULT_MAX_PAYLOAD_BYTES};

/// Filter criteria for bounded audit queries (ADR-019).
///
/// INVARIANTS:
/// - All persistent queries must be bounded by a maximum limit to prevent unbounded memory growth.
/// - Pagination is deterministic via `cursor_sequence` (based on local `sequence_num`).
#[derive(Debug, Clone)]
pub struct AuditFilter {
    pub correlation_id: Option<CorrelationId>,
    pub operation_id: Option<OperationId>,
    pub event_type: Option<EventType>,
    pub cursor_sequence: Option<i64>,
    pub limit: usize,
}

impl Default for AuditFilter {
    fn default() -> Self {
        Self {
            correlation_id: None,
            operation_id: None,
            event_type: None,
            cursor_sequence: None,
            limit: 100,
        }
    }
}

/// Fallible persistent write interface for audit events.
pub trait PersistentAuditWriter: Send + Sync {
    fn record_event(&self, event: &AuditEvent) -> Result<(), PersistenceError>;
}

/// Fallible, bounded query interface for persistent audit storage.
pub trait AuditQueryStore: Send + Sync {
    /// Executes a bounded query against the persistent audit store.
    fn query_events(&self, filter: &AuditFilter) -> Result<Vec<AuditEvent>, PersistenceError>;

    /// Queries historical verification records for an operation (bounded).
    fn get_verification_records(
        &self,
        operation_id: &OperationId,
        limit: usize,
    ) -> Result<Vec<VerificationRecord>, PersistenceError>;
}

/// SQLite adapter for Yuki's local audit storage (ADR-019).
///
/// OPERATIONAL DEFAULTS:
/// - WAL journal mode (`PRAGMA journal_mode = WAL;`)
/// - NORMAL synchronization (`PRAGMA synchronous = NORMAL;`)
/// - Maximum serialized payload limit (default: 64 KiB)
///
/// INVARIANTS:
/// - CapabilityTokens are NEVER persisted.
/// - SecretMaterial is NEVER serialized or persisted.
/// - Database does not own authorization policy.
pub struct SqliteAuditStore {
    conn: Mutex<Connection>,
    max_payload_bytes: usize,
}

impl SqliteAuditStore {
    /// Opens or creates an audit database at the specified path.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, PersistenceError> {
        let path_ref = path.as_ref();
        if let Some(parent) = path_ref.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    PersistenceError::Initialization(format!(
                        "Falha ao criar diretório para banco SQLite: {}",
                        e
                    ))
                })?;
            }
        }

        let mut conn = Connection::open(path_ref).map_err(|e| {
            PersistenceError::Initialization(format!("Falha ao abrir banco SQLite: {}", e))
        })?;

        // 1. Operational pragmas
        conn.busy_timeout(std::time::Duration::from_secs(5))
            .map_err(|e| {
                PersistenceError::Initialization(format!("Falha ao configurar busy_timeout: {}", e))
            })?;

        conn.pragma_update(None, "journal_mode", "WAL")
            .map_err(|e| {
                PersistenceError::Initialization(format!("Falha ao configurar WAL: {}", e))
            })?;

        conn.pragma_update(None, "synchronous", "NORMAL")
            .map_err(|e| {
                PersistenceError::Initialization(format!("Falha ao configurar synchronous: {}", e))
            })?;

        // 2. Preliminary integrity check signal
        let integrity: String = conn
            .query_row("PRAGMA quick_check(1)", [], |r| r.get(0))
            .map_err(|e| {
                PersistenceError::IntegrityCheck(format!("Falha ao executar quick_check: {}", e))
            })?;

        if integrity != "ok" {
            return Err(PersistenceError::IntegrityCheck(format!(
                "Integridade comprometida: {}",
                integrity
            )));
        }

        // 3. Schema migrations
        MigrationManager::apply_migrations(&mut conn)?;

        Ok(Self {
            conn: Mutex::new(conn),
            max_payload_bytes: DEFAULT_MAX_PAYLOAD_BYTES,
        })
    }

    /// Opens an in-memory SQLite store (for fast isolated tests).
    pub fn open_in_memory() -> Result<Self, PersistenceError> {
        let mut conn = Connection::open_in_memory().map_err(|e| {
            PersistenceError::Initialization(format!("Falha ao abrir SQLite in-memory: {}", e))
        })?;

        MigrationManager::apply_migrations(&mut conn)?;

        Ok(Self {
            conn: Mutex::new(conn),
            max_payload_bytes: DEFAULT_MAX_PAYLOAD_BYTES,
        })
    }

    /// Sets custom maximum payload size limit.
    pub fn with_max_payload_bytes(mut self, limit: usize) -> Self {
        self.max_payload_bytes = limit;
        self
    }

    /// Records an immutable audit event to persistent storage.
    pub fn record_event(&self, event: &AuditEvent) -> Result<(), PersistenceError> {
        // Enforce defense-in-depth secret check
        if let Err(err) = event.assert_no_secrets() {
            return Err(PersistenceError::Write(format!(
                "Evento rejeitado por conter padrão sensível: {}",
                err
            )));
        }

        // Enforce maximum payload size bound
        let payload_json = serde_json::to_string(&event.payload)
            .map_err(|e| PersistenceError::Write(format!("Falha ao serializar payload: {}", e)))?;

        if payload_json.len() > self.max_payload_bytes {
            return Err(PersistenceError::PayloadTooLarge {
                size: payload_json.len(),
                limit: self.max_payload_bytes,
            });
        }

        // Extract optional indexing fields safely
        let operation_id = event
            .payload
            .get("operation_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let capability_id = event
            .payload
            .get("capability_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let event_type_str = serde_json::to_string(&event.event_type)
            .map_err(|e| PersistenceError::Write(format!("Falha ao serializar event_type: {}", e)))?
            .trim_matches('"')
            .to_string();

        let causation_str = event.causation_id.0.as_str();
        let causation_val = if causation_str.is_empty() {
            None
        } else {
            Some(causation_str)
        };

        let conn = self
            .conn
            .lock()
            .map_err(|e| PersistenceError::Write(format!("Lock poisoned no SQLite: {}", e)))?;

        conn.execute(
            "INSERT INTO audit_events (
                event_id, timestamp, event_type, correlation_id, causation_id,
                operation_id, capability_id, payload_json, provenance
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                event.event_id.0,
                event.timestamp.to_rfc3339(),
                event_type_str,
                event.correlation_id.0,
                causation_val,
                operation_id,
                capability_id,
                payload_json,
                event.provenance,
            ],
        )
        .map_err(|e| {
            PersistenceError::Write(format!("Falha ao inserir evento de auditoria: {}", e))
        })?;

        Ok(())
    }

    /// Records a durable verification record (historical audit projection).
    pub fn record_verification(&self, record: &VerificationRecord) -> Result<(), PersistenceError> {
        let evidence_refs_json = serde_json::to_string(&record.evidence_refs).map_err(|e| {
            PersistenceError::Write(format!("Falha ao serializar evidence_refs: {}", e))
        })?;

        let verification_state_str = serde_json::to_string(&record.verification_state)
            .map_err(|e| {
                PersistenceError::Write(format!("Falha ao serializar verification_state: {}", e))
            })?
            .trim_matches('"')
            .to_string();

        let effect_state_str = serde_json::to_string(&record.observed_effect_state)
            .map_err(|e| {
                PersistenceError::Write(format!("Falha ao serializar observed_effect_state: {}", e))
            })?
            .trim_matches('"')
            .to_string();

        let conn = self
            .conn
            .lock()
            .map_err(|e| PersistenceError::Write(format!("Lock poisoned no SQLite: {}", e)))?;

        conn.execute(
            "INSERT INTO verification_records (
                verification_record_id, operation_id, attempt_id, capability_id,
                strategy_id, verification_state, observed_effect_state,
                evaluated_at, verification_basis, evidence_refs_json
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                record.verification_record_id,
                record.operation_id.0,
                record.attempt_id.as_ref().map(|a| a.0.as_str()),
                record.capability_id.0,
                record.strategy_id,
                verification_state_str,
                effect_state_str,
                record.evaluated_at.to_rfc3339(),
                record.verification_basis,
                evidence_refs_json,
            ],
        )
        .map_err(|e| {
            PersistenceError::Write(format!("Falha ao inserir registro de verificação: {}", e))
        })?;

        Ok(())
    }

    /// Asynchronous non-blocking wrapper using `tokio::task::spawn_blocking` (Resolution 2.2).
    pub async fn record_event_async(
        self: Arc<Self>,
        event: AuditEvent,
    ) -> Result<(), PersistenceError> {
        tokio::task::spawn_blocking(move || self.record_event(&event))
            .await
            .map_err(|e| PersistenceError::Write(format!("Falha de thread blocking: {}", e)))?
    }

    /// Requests a passive WAL checkpoint on clean shutdown.
    pub fn wal_checkpoint_passive(&self) -> Result<(), PersistenceError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| PersistenceError::DatabaseUnavailable(e.to_string()))?;
        conn.execute_batch("PRAGMA wal_checkpoint(PASSIVE);")
            .map_err(|e| PersistenceError::Write(format!("Falha no checkpoint WAL: {}", e)))?;
        Ok(())
    }
}

impl AuditQueryStore for SqliteAuditStore {
    fn query_events(&self, filter: &AuditFilter) -> Result<Vec<AuditEvent>, PersistenceError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| PersistenceError::Read(format!("Lock poisoned: {}", e)))?;

        let mut query = String::from(
            "SELECT event_id, timestamp, event_type, correlation_id, causation_id, payload_json, provenance
             FROM audit_events WHERE 1=1",
        );

        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(corr) = &filter.correlation_id {
            query.push_str(" AND correlation_id = ?");
            params.push(Box::new(corr.0.clone()));
        }

        if let Some(op) = &filter.operation_id {
            query.push_str(" AND operation_id = ?");
            params.push(Box::new(op.0.clone()));
        }

        if let Some(ev_type) = &filter.event_type {
            let s = serde_json::to_string(ev_type)
                .map_err(|e| {
                    PersistenceError::Read(format!("Falha ao serializar filtro event_type: {}", e))
                })?
                .trim_matches('"')
                .to_string();
            query.push_str(" AND event_type = ?");
            params.push(Box::new(s));
        }

        if let Some(cursor) = filter.cursor_sequence {
            query.push_str(" AND sequence_num > ?");
            params.push(Box::new(cursor));
        }

        query.push_str(" ORDER BY sequence_num ASC LIMIT ?");
        let safe_limit = filter.limit.clamp(1, 1000);
        params.push(Box::new(safe_limit as i64));

        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| PersistenceError::Read(format!("Falha ao preparar query: {}", e)))?;

        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

        let rows = stmt
            .query_map(&param_refs[..], |row| {
                let event_id: String = row.get(0)?;
                let ts_str: String = row.get(1)?;
                let ev_type_str: String = row.get(2)?;
                let correlation_id: String = row.get(3)?;
                let causation_id: Option<String> = row.get(4)?;
                let payload_json: String = row.get(5)?;
                let provenance: String = row.get(6)?;

                Ok((
                    event_id,
                    ts_str,
                    ev_type_str,
                    correlation_id,
                    causation_id,
                    payload_json,
                    provenance,
                ))
            })
            .map_err(|e| PersistenceError::Read(format!("Falha na execução da query: {}", e)))?;

        let mut events = Vec::new();
        for row in rows {
            let (id, ts, ev_type, corr, caus, payload_str, prov) =
                row.map_err(|e| PersistenceError::Read(format!("Erro ao ler linha: {}", e)))?;

            let timestamp = DateTime::parse_from_rfc3339(&ts)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| crate::contracts::identifiers::now_utc());

            let parsed_event_type: EventType = serde_json::from_str(&format!("\"{}\"", ev_type))
                .unwrap_or(EventType::InputReceived);

            let payload: serde_json::Value =
                serde_json::from_str(&payload_str).unwrap_or(serde_json::Value::Null);

            events.push(AuditEvent {
                event_id: EventId(id),
                timestamp,
                event_type: parsed_event_type,
                correlation_id: CorrelationId(corr),
                causation_id: CausationId(caus.unwrap_or_default()),
                payload,
                provenance: prov,
            });
        }

        Ok(events)
    }

    fn get_verification_records(
        &self,
        operation_id: &OperationId,
        limit: usize,
    ) -> Result<Vec<VerificationRecord>, PersistenceError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| PersistenceError::Read(format!("Lock poisoned: {}", e)))?;

        let safe_limit = limit.clamp(1, 500) as i64;
        let mut stmt = conn
            .prepare(
                "SELECT verification_record_id, operation_id, attempt_id, capability_id, strategy_id,
                        verification_state, observed_effect_state, evaluated_at, verification_basis,
                        evidence_refs_json
                 FROM verification_records
                 WHERE operation_id = ?1
                 ORDER BY rowid ASC
                 LIMIT ?2",
            )
            .map_err(|e| PersistenceError::Read(format!("Falha ao preparar query: {}", e)))?;

        let rows = stmt
            .query_map(params![operation_id.0, safe_limit], |row| {
                let v_id: String = row.get(0)?;
                let op_id: String = row.get(1)?;
                let att_id: Option<String> = row.get(2)?;
                let cap_id: String = row.get(3)?;
                let strat_id: String = row.get(4)?;
                let v_state_str: String = row.get(5)?;
                let eff_state_str: String = row.get(6)?;
                let eval_at_str: String = row.get(7)?;
                let basis: String = row.get(8)?;
                let refs_json: String = row.get(9)?;

                Ok((
                    v_id,
                    op_id,
                    att_id,
                    cap_id,
                    strat_id,
                    v_state_str,
                    eff_state_str,
                    eval_at_str,
                    basis,
                    refs_json,
                ))
            })
            .map_err(|e| PersistenceError::Read(format!("Falha na execução: {}", e)))?;

        let mut records = Vec::new();
        for row in rows {
            let (v_id, op_id, att_id, cap_id, strat_id, v_state, eff_state, eval_at, basis, refs) =
                row.map_err(|e| PersistenceError::Read(format!("Erro ao ler linha: {}", e)))?;

            let evaluated_at = DateTime::parse_from_rfc3339(&eval_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .map_err(|e| {
                    PersistenceError::Read(format!(
                        "Timestamp inválido no banco ('{}'): {}",
                        eval_at, e
                    ))
                })?;

            let verification_state =
                serde_json::from_str(&format!("\"{}\"", v_state)).map_err(|e| {
                    PersistenceError::Read(format!(
                        "Valor inválido de verification_state no banco ('{}'): {}",
                        v_state, e
                    ))
                })?;

            let observed_effect_state = serde_json::from_str(&format!("\"{}\"", eff_state))
                .map_err(|e| {
                    PersistenceError::Read(format!(
                        "Valor inválido de observed_effect_state no banco ('{}'): {}",
                        eff_state, e
                    ))
                })?;

            let evidence_refs = serde_json::from_str(&refs).map_err(|e| {
                PersistenceError::Read(format!("Falha ao desserializar evidence_refs: {}", e))
            })?;

            records.push(VerificationRecord {
                verification_record_id: v_id,
                operation_id: OperationId(op_id),
                attempt_id: att_id.map(crate::contracts::identifiers::AttemptId),
                capability_id: crate::contracts::identifiers::CapabilityId::new(cap_id),
                strategy_id: strat_id,
                verification_state,
                observed_effect_state,
                evaluated_at,
                verification_basis: basis,
                evidence_refs,
            });
        }

        Ok(records)
    }
}

impl PersistentAuditWriter for SqliteAuditStore {
    fn record_event(&self, event: &AuditEvent) -> Result<(), PersistenceError> {
        self.record_event(event)
    }
}
