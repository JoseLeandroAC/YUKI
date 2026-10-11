use crate::contracts::errors::YukiError;
use rusqlite::{params, Connection};
use std::sync::{Arc, Mutex};

/// Item retornado por uma busca local no índice FTS5.
#[derive(Debug, Clone)]
pub struct LocalSearchResultItem {
    pub chunk_id: String,
    pub doc_id: String,
    pub source_uri: String,
    pub title: String,
    pub heading_context: Option<String>,
    pub content_text: String,
    pub snippet: String,
    pub rank: f64,
}

/// Motor de busca FTS5 local operando sobre a base de conhecimento SQLite.
pub struct LocalSearchEngine {
    conn: Arc<Mutex<Connection>>,
}

impl LocalSearchEngine {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// Executa uma consulta BM25 sobre a tabela virtual FTS5.
    ///
    /// INVARIANTES:
    /// 1. Sanitiza a query do usuário, prevenindo injeções de sintaxe FTS5.
    /// 2. Suporta termos acentuados em português e termos técnicos.
    /// 3. Retorna os resultados ordenados por relevância BM25 (menor score = maior relevância).
    pub fn search(
        &self,
        query: &str,
        max_results: usize,
    ) -> Result<Vec<LocalSearchResultItem>, YukiError> {
        let sanitized_query = Self::sanitize_fts5_query(query);
        if sanitized_query.is_empty() {
            return Ok(Vec::new());
        }

        let conn = self.conn.lock().map_err(|_| {
            YukiError::ExecutionFailed("Mutex de conexão SQLite envenenado".to_string())
        })?;

        let mut stmt = conn
            .prepare(
                r#"
                SELECT
                    chunks_fts.chunk_id,
                    chunks_fts.doc_id,
                    d.source_uri,
                    d.title,
                    chunks_fts.heading_context,
                    chunks_fts.content_text,
                    snippet(chunks_fts, 2, '...', '...', '...', 32),
                    bm25(chunks_fts) as score
                FROM chunks_fts
                JOIN indexed_documents d ON d.doc_id = chunks_fts.doc_id
                WHERE chunks_fts MATCH ?
                ORDER BY score ASC
                LIMIT ?
                "#,
            )
            .map_err(|e| {
                YukiError::ExecutionFailed(format!("Erro ao preparar busca FTS5: {}", e))
            })?;

        let rows = stmt
            .query_map(params![sanitized_query, max_results as i64], |row| {
                let chunk_id: String = row.get(0)?;
                let doc_id: String = row.get(1)?;
                let source_uri: String = row.get(2)?;
                let title: String = row.get(3)?;
                let heading_context: Option<String> = row.get(4)?;
                let content_text: String = row.get(5)?;
                let snippet_raw: Option<String> = row.get(6)?;
                let rank: f64 = row.get(7)?;

                let snippet = snippet_raw
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or_else(|| {
                        if content_text.chars().count() > 200 {
                            format!("{}...", &content_text[..200])
                        } else {
                            content_text.clone()
                        }
                    });

                Ok(LocalSearchResultItem {
                    chunk_id,
                    doc_id,
                    source_uri,
                    title,
                    heading_context,
                    content_text,
                    snippet,
                    rank,
                })
            })
            .map_err(|e| {
                YukiError::ExecutionFailed(format!("Erro ao executar busca FTS5: {}", e))
            })?;

        let mut results = Vec::new();
        for item in rows.flatten() {
            results.push(item);
        }

        Ok(results)
    }

    /// Sanitiza a consulta para FTS5, extraindo palavras e encapsulando em aspas para busca booleana segura.
    pub fn sanitize_fts5_query(raw_query: &str) -> String {
        let words: Vec<&str> = raw_query
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .filter(|w| !w.trim().is_empty())
            .collect();

        if words.is_empty() {
            return String::new();
        }

        // Formata como busca booleana OR entre termos citados: "termo1" OR "termo2"
        words
            .iter()
            .map(|w| format!("\"{}\"", w))
            .collect::<Vec<String>>()
            .join(" OR ")
    }
}
