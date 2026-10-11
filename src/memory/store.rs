use crate::contracts::identifiers::now_utc;
use crate::contracts::memory::{
    MemoryConfidence, MemoryError, MemoryId, MemoryProvenance, MemoryRecord, MemoryRepository,
    MemorySource, MemoryStatus, MemoryType,
};
use crate::contracts::MemoryQuery;
use crate::memory::schema::initialize_memory_schema;
use chrono::{DateTime, Utc};
use rusqlite::types::ToSql;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Implementação persistente do repositório de memórias sobre SQLite e FTS5.
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

    /// Busca memória ativa existente com o mesmo título e tipo para evitar duplicações no escopo do proprietário.
    pub fn find_active_duplicate(
        &self,
        owner_id: &str,
        title: &str,
        mem_type: MemoryType,
    ) -> Result<Option<MemoryRecord>, MemoryError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| MemoryError::Storage("Lock de banco de dados envenenado".to_string()))?;

        let sql = r#"
            SELECT id, owner_id, memory_type, source, confidence, status, title, content, tags,
                   created_at, updated_at, source_session_id, source_event_id, origin_context, superseded_by
            FROM memories
            WHERE status = 'active'
              AND owner_id = ?
              AND memory_type = ?
              AND LOWER(TRIM(title)) = LOWER(TRIM(?))
            LIMIT 1;
        "#;

        let mut stmt = conn
            .prepare(sql)
            .map_err(|e| MemoryError::Storage(e.to_string()))?;

        let res = stmt
            .query_row(params![owner_id, mem_type.as_str(), title], |row| {
                row_to_memory_record(row)
            })
            .optional()
            .map_err(|e| MemoryError::Storage(e.to_string()))?;

        Ok(res)
    }

    /// Sanitiza uma string de consulta para sintaxe segura do FTS5.
    pub fn sanitize_fts5_query(query: &str) -> String {
        let cleaned_words: Vec<String> = query
            .split_whitespace()
            .map(|word| {
                word.chars()
                    .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
                    .collect()
            })
            .filter(|w: &String| !w.is_empty())
            .collect();

        // Se houver termos com 3+ caracteres, ignora preposições/partículas curtas de 1-2 caracteres
        let filtered_words: Vec<&String> = if cleaned_words.iter().any(|w| w.chars().count() >= 3) {
            cleaned_words
                .iter()
                .filter(|w| w.chars().count() >= 3)
                .collect()
        } else {
            cleaned_words.iter().collect()
        };

        let terms: Vec<String> = filtered_words
            .into_iter()
            .map(|w| {
                if w.chars().count() >= 3 {
                    format!("\"{}\"*", w)
                } else {
                    format!("\"{}\"", w)
                }
            })
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
                id, owner_id, memory_type, source, confidence, status, title, content, tags,
                created_at, updated_at, source_session_id, source_event_id, origin_context, superseded_by
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                owner_id = excluded.owner_id,
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
                &record.owner_id,
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

        // Detecção de duplicata ativa no escopo do proprietário
        if record.status == MemoryStatus::Active {
            let check_sql = r#"
                SELECT id FROM memories
                WHERE status = 'active'
                  AND owner_id = ?
                  AND memory_type = ?
                  AND LOWER(TRIM(title)) = LOWER(TRIM(?))
                  AND id != ?
                LIMIT 1;
            "#;

            let prev_id: Option<String> = tx
                .query_row(
                    check_sql,
                    params![
                        &record.owner_id,
                        record.memory_type.as_str(),
                        &record.title,
                        &record.id.0
                    ],
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
            SELECT id, owner_id, memory_type, source, confidence, status, title, content, tags,
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

        // Se houver busca textual, realizar consulta via FTS5 garantindo que todos os filtros
        // sejam avaliados no SQL ANTES de LIMIT/OFFSET.
        if let Some(ref text) = query.text_query {
            let fts_pattern = Self::sanitize_fts5_query(text);
            if fts_pattern.is_empty() {
                // Termos vazios ou inválidos na busca textual retornam lista vazia; nunca despejam memórias aleatórias.
                return Ok(Vec::new());
            }

            let mut conditions = vec!["memories_fts MATCH ?".to_string()];
            let mut params_vec: Vec<Box<dyn ToSql>> = vec![Box::new(fts_pattern)];

            append_sql_filters(query, "m", &mut conditions, &mut params_vec);

            let where_clause = conditions.join(" AND ");
            let fts_sql = format!(
                r#"
                SELECT m.id, m.owner_id, m.memory_type, m.source, m.confidence, m.status, m.title, m.content, m.tags,
                       m.created_at, m.updated_at, m.source_session_id, m.source_event_id, m.origin_context, m.superseded_by
                FROM memories_fts fts
                JOIN memories m ON m.id = fts.id
                WHERE {}
                ORDER BY fts.rank
                LIMIT ? OFFSET ?;
                "#,
                where_clause
            );

            params_vec.push(Box::new(limit as i64));
            params_vec.push(Box::new(offset as i64));

            let mut stmt = conn.prepare(&fts_sql).map_err(|e| {
                MemoryError::Storage(format!("Falha ao preparar busca FTS5: {}", e))
            })?;

            let param_refs: Vec<&dyn ToSql> = params_vec.iter().map(|b| b.as_ref()).collect();
            let rows = stmt
                .query_map(param_refs.as_slice(), row_to_memory_record)
                .map_err(|e| MemoryError::Storage(format!("Erro ao executar FTS5: {}", e)))?;

            let mut records = Vec::new();
            for r in rows {
                records.push(r.map_err(|e| MemoryError::Storage(e.to_string()))?);
            }

            // A busca textual deliberadamente NÃO faz fallback para retornar memórias não relacionadas.
            return Ok(records);
        }

        // Consulta relacional padrão quando não há busca textual
        let mut conditions = Vec::new();
        let mut params_vec: Vec<Box<dyn ToSql>> = Vec::new();

        append_sql_filters(query, "memories", &mut conditions, &mut params_vec);

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let sql = format!(
            r#"
            SELECT id, owner_id, memory_type, source, confidence, status, title, content, tags,
                   created_at, updated_at, source_session_id, source_event_id, origin_context, superseded_by
            FROM memories
            {}
            ORDER BY updated_at DESC
            LIMIT ? OFFSET ?;
            "#,
            where_clause
        );

        params_vec.push(Box::new(limit as i64));
        params_vec.push(Box::new(offset as i64));

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| MemoryError::Storage(format!("Falha ao preparar consulta: {}", e)))?;

        let param_refs: Vec<&dyn ToSql> = params_vec.iter().map(|b| b.as_ref()).collect();
        let rows = stmt
            .query_map(param_refs.as_slice(), row_to_memory_record)
            .map_err(|e| MemoryError::Storage(format!("Erro ao executar consulta: {}", e)))?;

        let mut records = Vec::new();
        for r in rows {
            records.push(r.map_err(|e| MemoryError::Storage(e.to_string()))?);
        }

        Ok(records)
    }

    fn count(&self, query: &MemoryQuery) -> Result<usize, MemoryError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| MemoryError::Storage("Lock de banco de dados envenenado".to_string()))?;

        if let Some(ref text) = query.text_query {
            let fts_pattern = Self::sanitize_fts5_query(text);
            if fts_pattern.is_empty() {
                return Ok(0);
            }

            let mut conditions = vec!["memories_fts MATCH ?".to_string()];
            let mut params_vec: Vec<Box<dyn ToSql>> = vec![Box::new(fts_pattern)];

            append_sql_filters(query, "m", &mut conditions, &mut params_vec);

            let where_clause = conditions.join(" AND ");
            let fts_count_sql = format!(
                r#"
                SELECT COUNT(*)
                FROM memories_fts fts
                JOIN memories m ON m.id = fts.id
                WHERE {};
                "#,
                where_clause
            );

            let mut stmt = conn
                .prepare(&fts_count_sql)
                .map_err(|e| MemoryError::Storage(e.to_string()))?;

            let param_refs: Vec<&dyn ToSql> = params_vec.iter().map(|b| b.as_ref()).collect();
            let total: i64 = stmt
                .query_row(param_refs.as_slice(), |row| row.get(0))
                .map_err(|e| MemoryError::Storage(e.to_string()))?;

            return Ok(total as usize);
        }

        let mut conditions = Vec::new();
        let mut params_vec: Vec<Box<dyn ToSql>> = Vec::new();

        append_sql_filters(query, "memories", &mut conditions, &mut params_vec);

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let count_sql = format!("SELECT COUNT(*) FROM memories {};", where_clause);
        let mut stmt = conn
            .prepare(&count_sql)
            .map_err(|e| MemoryError::Storage(e.to_string()))?;

        let param_refs: Vec<&dyn ToSql> = params_vec.iter().map(|b| b.as_ref()).collect();
        let total: i64 = stmt
            .query_row(param_refs.as_slice(), |row| row.get(0))
            .map_err(|e| MemoryError::Storage(e.to_string()))?;

        Ok(total as usize)
    }
}

/// Anexa dinamicamente as condições de filtro SQL garantindo que sejam aplicadas antes de LIMIT/OFFSET.
fn append_sql_filters(
    query: &MemoryQuery,
    prefix: &str,
    conditions: &mut Vec<String>,
    params_vec: &mut Vec<Box<dyn ToSql>>,
) {
    if let Some(ref owner) = query.owner_id {
        conditions.push(format!("{}.owner_id = ?", prefix));
        params_vec.push(Box::new(owner.clone()));
    }

    if let Some(ref statuses) = query.statuses {
        if !statuses.is_empty() {
            let placeholders = vec!["?"; statuses.len()].join(", ");
            conditions.push(format!("{}.status IN ({})", prefix, placeholders));
            for s in statuses {
                params_vec.push(Box::new(s.as_str().to_string()));
            }
        }
    }

    if let Some(ref types) = query.memory_types {
        if !types.is_empty() {
            let placeholders = vec!["?"; types.len()].join(", ");
            conditions.push(format!("{}.memory_type IN ({})", prefix, placeholders));
            for t in types {
                params_vec.push(Box::new(t.as_str().to_string()));
            }
        }
    }

    if let Some(ref sources) = query.sources {
        if !sources.is_empty() {
            let placeholders = vec!["?"; sources.len()].join(", ");
            conditions.push(format!("{}.source IN ({})", prefix, placeholders));
            for s in sources {
                params_vec.push(Box::new(s.as_str().to_string()));
            }
        }
    }

    if let Some(ref min_conf) = query.min_confidence {
        match min_conf {
            MemoryConfidence::Verified => {
                conditions.push(format!("{}.confidence = 'verified'", prefix));
            }
            MemoryConfidence::High => {
                conditions.push(format!("{}.confidence IN ('verified', 'high')", prefix));
            }
            MemoryConfidence::Medium => {
                conditions.push(format!(
                    "{}.confidence IN ('verified', 'high', 'medium')",
                    prefix
                ));
            }
            MemoryConfidence::Low => {}
        }
    }

    if let Some(ref tags) = query.tags {
        if !tags.is_empty() {
            let mut tag_conditions = Vec::new();
            for tag in tags {
                tag_conditions.push(format!("{}.tags LIKE ?", prefix));
                params_vec.push(Box::new(format!("%\"{}\"%", tag)));
            }
            conditions.push(format!("({})", tag_conditions.join(" OR ")));
        }
    }
}

fn row_to_memory_record(row: &rusqlite::Row) -> rusqlite::Result<MemoryRecord> {
    let id_str: String = row.get(0)?;
    let owner_id: String = row.get(1)?;
    let type_str: String = row.get(2)?;
    let source_str: String = row.get(3)?;
    let conf_str: String = row.get(4)?;
    let status_str: String = row.get(5)?;
    let title: String = row.get(6)?;
    let content: String = row.get(7)?;
    let tags_json: String = row.get(8)?;
    let created_str: String = row.get(9)?;
    let updated_str: String = row.get(10)?;
    let session_id: Option<String> = row.get(11)?;
    let event_id: Option<String> = row.get(12)?;
    let origin_context: Option<String> = row.get(13)?;
    let superseded_str: Option<String> = row.get(14)?;

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
        owner_id,
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
