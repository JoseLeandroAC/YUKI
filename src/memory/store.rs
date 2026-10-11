use crate::contracts::identifiers::now_utc;
use crate::contracts::memory::{
    MemoryConfidence, MemoryError, MemoryId, MemoryProvenance, MemoryQuery, MemoryRecord,
    MemoryRepository, MemorySource, MemoryStatus, MemoryType,
};
use crate::memory::schema::initialize_memory_schema;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Implementação persistente do repositório de memória em SQLite com FTS5 (ADR-016).
#[derive(Clone)]
pub struct SqliteMemoryStore {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteMemoryStore {
    /// Abre ou cria o banco de dados de memórias no caminho especificado.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, MemoryError> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    MemoryError::Storage(format!("Falha ao criar diretório de memória: {}", e))
                })?;
            }
        }

        let conn = Connection::open(path).map_err(|e| {
            MemoryError::Storage(format!("Falha ao abrir SQLite de memória: {}", e))
        })?;

        initialize_memory_schema(&conn).map_err(|e| {
            MemoryError::Storage(format!("Falha ao inicializar esquema de memória: {}", e))
        })?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Abre uma instância em memória volátil (ideal para testes).
    pub fn open_in_memory() -> Result<Self, MemoryError> {
        let conn = Connection::open_in_memory().map_err(|e| {
            MemoryError::Storage(format!("Falha ao abrir SQLite em memória: {}", e))
        })?;

        initialize_memory_schema(&conn).map_err(|e| {
            MemoryError::Storage(format!("Falha ao inicializar esquema de memória: {}", e))
        })?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Busca memória ativa existente com o mesmo título e tipo para evitar duplicações.
    pub fn find_active_duplicate(
        &self,
        title: &str,
        mem_type: MemoryType,
    ) -> Result<Option<MemoryRecord>, MemoryError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| MemoryError::Storage("Lock de banco de dados envenenado".to_string()))?;

        let sql = r#"
            SELECT id, memory_type, source, confidence, status, title, content, tags,
                   created_at, updated_at, source_session_id, source_event_id, origin_context, superseded_by
            FROM memories
            WHERE status = 'active'
              AND memory_type = ?
              AND LOWER(TRIM(title)) = LOWER(TRIM(?))
            LIMIT 1;
        "#;

        let mut stmt = conn
            .prepare(sql)
            .map_err(|e| MemoryError::Storage(e.to_string()))?;

        let res = stmt
            .query_row(params![mem_type.as_str(), title], |row| {
                row_to_memory_record(row)
            })
            .optional()
            .map_err(|e| MemoryError::Storage(e.to_string()))?;

        Ok(res)
    }

    /// Sanitiza uma string de consulta para sintaxe segura do FTS5.
    fn sanitize_fts5_query(query: &str) -> String {
        let terms: Vec<String> = query
            .split_whitespace()
            .map(|word| {
                let clean: String = word
                    .chars()
                    .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
                    .collect();
                clean
            })
            .filter(|w| !w.is_empty())
            .map(|w| format!("\"{}\"*", w))
            .collect();

        if terms.is_empty() {
            String::new()
        } else {
            terms.join(" OR ")
        }
    }
}

impl MemoryRepository for SqliteMemoryStore {
    fn store(&self, record: &MemoryRecord) -> Result<(), MemoryError> {
        // Validação sanitária preventiva contra segredos
        record.assert_no_secrets()?;

        let mut conn = self
            .conn
            .lock()
            .map_err(|_| MemoryError::Storage("Lock de banco de dados envenenado".to_string()))?;

        let tx = conn
            .transaction()
            .map_err(|e| MemoryError::Storage(format!("Falha ao iniciar transação: {}", e)))?;

        let tags_json = serde_json::to_string(&record.tags)
            .map_err(|e| MemoryError::Serialization(e.to_string()))?;

        let insert_sql = r#"
            INSERT INTO memories (
                id, memory_type, source, confidence, status, title, content, tags,
                created_at, updated_at, source_session_id, source_event_id, origin_context, superseded_by
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                memory_type = excluded.memory_type,
                source = excluded.source,
                confidence = excluded.confidence,
                status = excluded.status,
                title = excluded.title,
                content = excluded.content,
                tags = excluded.tags,
                updated_at = excluded.updated_at,
                source_session_id = excluded.source_session_id,
                source_event_id = excluded.source_event_id,
                origin_context = excluded.origin_context,
                superseded_by = excluded.superseded_by;
        "#;

        tx.execute(
            insert_sql,
            params![
                &record.id.0,
                record.memory_type.as_str(),
                record.source.as_str(),
                record.confidence.as_str(),
                record.status.as_str(),
                &record.title,
                &record.content,
                &tags_json,
                &record.provenance.created_at.to_rfc3339(),
                &record.provenance.updated_at.to_rfc3339(),
                &record.provenance.source_session_id,
                &record.provenance.source_event_id,
                &record.provenance.origin_context,
                &record.superseded_by.as_ref().map(|s| &s.0),
            ],
        )
        .map_err(|e| MemoryError::Storage(format!("Falha ao inserir memória: {}", e)))?;

        // Detecção de duplicata ativa: agora que record.id existe no banco,
        // pode ser referenciado como superseded_by sem violar a FK.
        if record.status == MemoryStatus::Active {
            let check_sql = r#"
                SELECT id FROM memories
                WHERE status = 'active'
                  AND memory_type = ?
                  AND LOWER(TRIM(title)) = LOWER(TRIM(?))
                  AND id != ?
                LIMIT 1;
            "#;

            let prev_id: Option<String> = tx
                .query_row(
                    check_sql,
                    params![record.memory_type.as_str(), &record.title, &record.id.0],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|e| MemoryError::Storage(e.to_string()))?;

            if let Some(prev) = prev_id {
                let now_str = now_utc().to_rfc3339();
                let supersede_sql = r#"
                    UPDATE memories
                    SET status = 'superseded', superseded_by = ?, updated_at = ?
                    WHERE id = ?;
                "#;
                tx.execute(supersede_sql, params![&record.id.0, &now_str, &prev])
                    .map_err(|e| MemoryError::Storage(e.to_string()))?;
            }
        }

        tx.commit()
            .map_err(|e| MemoryError::Storage(format!("Falha ao comitar memória: {}", e)))?;

        Ok(())
    }

    fn get_by_id(&self, id: &MemoryId) -> Result<Option<MemoryRecord>, MemoryError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| MemoryError::Storage("Lock de banco de dados envenenado".to_string()))?;

        let sql = r#"
            SELECT id, memory_type, source, confidence, status, title, content, tags,
                   created_at, updated_at, source_session_id, source_event_id, origin_context, superseded_by
            FROM memories
            WHERE id = ?;
        "#;

        let mut stmt = conn
            .prepare(sql)
            .map_err(|e| MemoryError::Storage(e.to_string()))?;

        let res = stmt
            .query_row(params![&id.0], row_to_memory_record)
            .optional()
            .map_err(|e| MemoryError::Storage(e.to_string()))?;

        Ok(res)
    }

    fn revoke(&self, id: &MemoryId) -> Result<(), MemoryError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| MemoryError::Storage("Lock de banco de dados envenenado".to_string()))?;

        let sql = r#"
            UPDATE memories
            SET status = 'revoked', updated_at = ?
            WHERE id = ?;
        "#;

        let now_str = now_utc().to_rfc3339();
        let rows = conn
            .execute(sql, params![&now_str, &id.0])
            .map_err(|e| MemoryError::Storage(e.to_string()))?;

        if rows == 0 {
            return Err(MemoryError::NotFound(id.0.clone()));
        }

        Ok(())
    }

    fn archive(&self, id: &MemoryId) -> Result<(), MemoryError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| MemoryError::Storage("Lock de banco de dados envenenado".to_string()))?;

        let sql = r#"
            UPDATE memories
            SET status = 'archived', updated_at = ?
            WHERE id = ?;
        "#;

        let now_str = now_utc().to_rfc3339();
        let rows = conn
            .execute(sql, params![&now_str, &id.0])
            .map_err(|e| MemoryError::Storage(e.to_string()))?;

        if rows == 0 {
            return Err(MemoryError::NotFound(id.0.clone()));
        }

        Ok(())
    }

    fn delete_permanently(&self, id: &MemoryId) -> Result<(), MemoryError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| MemoryError::Storage("Lock de banco de dados envenenado".to_string()))?;

        let sql = "DELETE FROM memories WHERE id = ?;";
        let rows = conn
            .execute(sql, params![&id.0])
            .map_err(|e| MemoryError::Storage(e.to_string()))?;

        if rows == 0 {
            return Err(MemoryError::NotFound(id.0.clone()));
        }

        Ok(())
    }

    fn query(&self, query: &MemoryQuery) -> Result<Vec<MemoryRecord>, MemoryError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| MemoryError::Storage("Lock de banco de dados envenenado".to_string()))?;

        let limit = if query.limit == 0 { 20 } else { query.limit };
        let offset = query.offset;

        // Se houver busca textual, tentar consulta FTS5
        if let Some(ref text) = query.text_query {
            let fts_pattern = Self::sanitize_fts5_query(text);
            if !fts_pattern.is_empty() {
                let fts_sql = r#"
                    SELECT m.id, m.memory_type, m.source, m.confidence, m.status, m.title, m.content, m.tags,
                           m.created_at, m.updated_at, m.source_session_id, m.source_event_id, m.origin_context, m.superseded_by
                    FROM memories_fts fts
                    JOIN memories m ON m.id = fts.id
                    WHERE memories_fts MATCH ?
                    ORDER BY fts.rank
                    LIMIT ? OFFSET ?;
                "#;

                if let Ok(mut stmt) = conn.prepare(fts_sql) {
                    let rows = stmt
                        .query_map(params![&fts_pattern, limit as i64, offset as i64], |row| {
                            row_to_memory_record(row)
                        });

                    if let Ok(mapped) = rows {
                        let mut records = Vec::new();
                        for r in mapped.flatten() {
                            if matches_filters(&r, query) {
                                records.push(r);
                            }
                        }
                        if !records.is_empty() {
                            return Ok(records);
                        }
                    }
                }
            }
        }

        // Consulta padrão relacional com filtros
        let sql = r#"
            SELECT id, memory_type, source, confidence, status, title, content, tags,
                   created_at, updated_at, source_session_id, source_event_id, origin_context, superseded_by
            FROM memories
            ORDER BY updated_at DESC
            LIMIT ? OFFSET ?;
        "#;

        let mut stmt = conn
            .prepare(sql)
            .map_err(|e| MemoryError::Storage(e.to_string()))?;

        let rows = stmt
            .query_map(params![limit as i64, offset as i64], |row| {
                row_to_memory_record(row)
            })
            .map_err(|e| MemoryError::Storage(e.to_string()))?;

        let mut records = Vec::new();
        for r in rows.flatten() {
            if matches_filters(&r, query) {
                records.push(r);
            }
        }

        Ok(records)
    }

    fn count(&self, query: &MemoryQuery) -> Result<usize, MemoryError> {
        let records = self.query(query)?;
        Ok(records.len())
    }
}

fn matches_filters(record: &MemoryRecord, query: &MemoryQuery) -> bool {
    if let Some(ref statuses) = query.statuses {
        if !statuses.contains(&record.status) {
            return false;
        }
    }
    if let Some(ref types) = query.memory_types {
        if !types.contains(&record.memory_type) {
            return false;
        }
    }
    if let Some(ref sources) = query.sources {
        if !sources.contains(&record.source) {
            return false;
        }
    }
    if let Some(ref min_conf) = query.min_confidence {
        if record.confidence < *min_conf {
            return false;
        }
    }
    if let Some(ref tags) = query.tags {
        if !tags.iter().any(|t| record.tags.contains(t)) {
            return false;
        }
    }
    true
}

fn row_to_memory_record(row: &rusqlite::Row) -> rusqlite::Result<MemoryRecord> {
    let id_str: String = row.get(0)?;
    let type_str: String = row.get(1)?;
    let source_str: String = row.get(2)?;
    let conf_str: String = row.get(3)?;
    let status_str: String = row.get(4)?;
    let title: String = row.get(5)?;
    let content: String = row.get(6)?;
    let tags_json: String = row.get(7)?;
    let created_str: String = row.get(8)?;
    let updated_str: String = row.get(9)?;
    let session_id: Option<String> = row.get(10)?;
    let event_id: Option<String> = row.get(11)?;
    let origin_context: Option<String> = row.get(12)?;
    let superseded_str: Option<String> = row.get(13)?;

    let memory_type = MemoryType::from_str_canonical(&type_str).unwrap_or(MemoryType::Fact);
    let source =
        MemorySource::from_str_canonical(&source_str).unwrap_or(MemorySource::UserDeclared);
    let confidence =
        MemoryConfidence::from_str_canonical(&conf_str).unwrap_or(MemoryConfidence::Medium);
    let status = MemoryStatus::from_str_canonical(&status_str).unwrap_or(MemoryStatus::Active);

    let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
    let created_at = DateTime::parse_from_rfc3339(&created_str)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| now_utc());
    let updated_at = DateTime::parse_from_rfc3339(&updated_str)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| now_utc());

    Ok(MemoryRecord {
        id: MemoryId(id_str),
        memory_type,
        source,
        confidence,
        status,
        title,
        content,
        tags,
        provenance: MemoryProvenance {
            created_at,
            updated_at,
            source_session_id: session_id,
            source_event_id: event_id,
            origin_context,
        },
        superseded_by: superseded_str.map(MemoryId),
    })
}
