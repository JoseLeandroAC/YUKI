use rusqlite::Connection;

use crate::contracts::identifiers::now_utc;
use crate::persistence::errors::PersistenceError;
use crate::persistence::schema::{
    AUDIT_EVENTS_TABLE_SQL, PREVENT_MUTATION_TRIGGERS_SQL, SCHEMA_VERSION_TABLE_SQL,
    VERIFICATION_RECORDS_TABLE_SQL,
};

/// Highest schema version supported by this binary.
pub const CURRENT_SCHEMA_VERSION: i64 = 1;

/// Deterministic schema migration manager.
pub struct MigrationManager;

impl MigrationManager {
    /// Inspects and applies pending migrations inside a transaction.
    /// Refuses to proceed if the database schema is newer than supported.
    pub fn apply_migrations(conn: &mut Connection) -> Result<i64, PersistenceError> {
        let tx = conn.transaction().map_err(|e| {
            PersistenceError::Migration(format!("Falha ao iniciar transação: {}", e))
        })?;

        // 1. Ensure version tracking table exists
        tx.execute_batch(SCHEMA_VERSION_TABLE_SQL).map_err(|e| {
            PersistenceError::Migration(format!("Falha ao criar tabela de versão: {}", e))
        })?;

        // 2. Query current applied version
        let current_version: Option<i64> = tx
            .query_row("SELECT MAX(version) FROM _yuki_schema_version", [], |row| {
                row.get(0)
            })
            .map_err(|e| {
                PersistenceError::Migration(format!("Falha ao consultar versão: {}", e))
            })?;

        let current = current_version.unwrap_or(0);

        // 3. Safety check: reject databases created by newer software versions
        if current > CURRENT_SCHEMA_VERSION {
            return Err(PersistenceError::SchemaVersionUnsupported {
                found: current,
                supported: CURRENT_SCHEMA_VERSION,
            });
        }

        // 4. Apply initial migration 001 if needed
        if current < 1 {
            tx.execute_batch(AUDIT_EVENTS_TABLE_SQL).map_err(|e| {
                PersistenceError::Migration(format!("Falha ao criar audit_events: {}", e))
            })?;

            tx.execute_batch(VERIFICATION_RECORDS_TABLE_SQL)
                .map_err(|e| {
                    PersistenceError::Migration(format!(
                        "Falha ao criar verification_records: {}",
                        e
                    ))
                })?;

            tx.execute_batch(PREVENT_MUTATION_TRIGGERS_SQL)
                .map_err(|e| {
                    PersistenceError::Migration(format!(
                        "Falha ao criar triggers de imutabilidade: {}",
                        e
                    ))
                })?;

            let now = now_utc().to_rfc3339();
            tx.execute(
                "INSERT INTO _yuki_schema_version (version, name, applied_at) VALUES (?1, ?2, ?3)",
                rusqlite::params![1, "001_initial_schema", now],
            )
            .map_err(|e| {
                PersistenceError::Migration(format!("Falha ao registrar versão 1: {}", e))
            })?;
        }

        tx.commit().map_err(|e| {
            PersistenceError::Migration(format!("Falha ao confirmar transação de migração: {}", e))
        })?;

        Ok(CURRENT_SCHEMA_VERSION)
    }
}
