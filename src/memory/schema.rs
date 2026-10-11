use rusqlite::Connection;

/// Esquema relacional e FTS5 para armazenamento persistente de memórias da Yuki (ADR-016).
pub const INIT_MEMORY_SCHEMA_SQL: &str = r#"
-- Tabela principal de memórias normalizadas
CREATE TABLE IF NOT EXISTS memories (
    id TEXT PRIMARY KEY,
    owner_id TEXT NOT NULL DEFAULT 'operator:primary',
    memory_type TEXT NOT NULL,
    source TEXT NOT NULL,
    confidence TEXT NOT NULL,
    status TEXT NOT NULL,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    tags TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    source_session_id TEXT,
    source_event_id TEXT,
    origin_context TEXT,
    superseded_by TEXT,
    FOREIGN KEY (superseded_by) REFERENCES memories(id)
);

-- Índices operacionais para consultas bounded
CREATE INDEX IF NOT EXISTS idx_memories_owner ON memories(owner_id);
CREATE INDEX IF NOT EXISTS idx_memories_owner_status ON memories(owner_id, status);
CREATE INDEX IF NOT EXISTS idx_memories_status ON memories(status);
CREATE INDEX IF NOT EXISTS idx_memories_type ON memories(memory_type);
CREATE INDEX IF NOT EXISTS idx_memories_source ON memories(source);
CREATE INDEX IF NOT EXISTS idx_memories_updated_at ON memories(updated_at DESC);

-- Tabela virtual FTS5 para busca textual de alta performance com BM25
CREATE VIRTUAL TABLE IF NOT EXISTS memories_fts USING fts5(
    id UNINDEXED,
    title,
    content,
    tags,
    tokenize = 'unicode61 remove_diacritics 2'
);

-- Gatilhos atômicos para sincronização contínua com FTS5
CREATE TRIGGER IF NOT EXISTS trg_memories_ai AFTER INSERT ON memories BEGIN
    INSERT INTO memories_fts(id, title, content, tags)
    VALUES (new.id, new.title, new.content, new.tags);
END;

CREATE TRIGGER IF NOT EXISTS trg_memories_au AFTER UPDATE ON memories BEGIN
    DELETE FROM memories_fts WHERE id = old.id;
    INSERT INTO memories_fts(id, title, content, tags)
    VALUES (new.id, new.title, new.content, new.tags);
END;

CREATE TRIGGER IF NOT EXISTS trg_memories_ad AFTER DELETE ON memories BEGIN
    DELETE FROM memories_fts WHERE id = old.id;
END;
"#;

/// Inicializa o esquema de memórias na conexão SQLite com suporte a migração automática.
pub fn initialize_memory_schema(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(
        r#"
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;
        PRAGMA foreign_keys = ON;
        "#,
    )?;
    conn.execute_batch(INIT_MEMORY_SCHEMA_SQL)?;

    // Migração transparente de bancos existentes sem owner_id
    let has_owner: bool = conn
        .prepare("SELECT COUNT(*) FROM pragma_table_info('memories') WHERE name = 'owner_id';")?
        .query_row([], |row| row.get::<_, i64>(0))
        .map(|cnt| cnt > 0)
        .unwrap_or(false);

    if !has_owner {
        let _ = conn.execute(
            "ALTER TABLE memories ADD COLUMN owner_id TEXT NOT NULL DEFAULT 'operator:primary';",
            [],
        );
    }

    conn.execute_batch(
        r#"
        CREATE INDEX IF NOT EXISTS idx_memories_owner ON memories(owner_id);
        CREATE INDEX IF NOT EXISTS idx_memories_owner_status ON memories(owner_id, status);
        "#,
    )?;

    Ok(())
}
