use rusqlite::{Connection, Result as SqlResult};

/// Inicializa o esquema de tabelas relacionais e a tabela virtual FTS5 para o índice local.
///
/// ESQUEMA (ADR-021):
/// - `indexed_documents`: metadados de proveniência e hash SHA-256 do documento.
/// - `document_chunks`: blocos textuais para citação granular.
/// - `chunks_fts`: tabela virtual SQLite FTS5 com tokenizador `unicode61 remove_diacritics 2`.
pub fn initialize_search_schema(conn: &Connection) -> SqlResult<()> {
    conn.execute_batch(
        r#"
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS indexed_documents (
            doc_id TEXT PRIMARY KEY,
            source_uri TEXT NOT NULL UNIQUE,
            domain TEXT NOT NULL,
            source_kind TEXT NOT NULL,
            title TEXT NOT NULL,
            raw_hash_sha256 TEXT NOT NULL,
            extracted_hash_sha256 TEXT NOT NULL,
            byte_size INTEGER NOT NULL,
            char_count INTEGER NOT NULL,
            created_at TEXT NOT NULL,
            indexed_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS document_chunks (
            chunk_id TEXT PRIMARY KEY,
            doc_id TEXT NOT NULL REFERENCES indexed_documents(doc_id) ON DELETE CASCADE,
            chunk_index INTEGER NOT NULL,
            heading_context TEXT,
            content_text TEXT NOT NULL,
            content_hash_sha256 TEXT NOT NULL,
            token_count INTEGER NOT NULL
        );

        CREATE VIRTUAL TABLE IF NOT EXISTS chunks_fts USING fts5(
            chunk_id UNINDEXED,
            doc_id UNINDEXED,
            content_text,
            heading_context,
            title,
            tokenize = 'unicode61 remove_diacritics 2'
        );
        "#,
    )?;

    Ok(())
}
