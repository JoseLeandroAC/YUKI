use crate::contracts::errors::YukiError;
use crate::contracts::identifiers::{generate_unique_id, now_utc};
use crate::contracts::research::compute_sha256;
use rusqlite::{params, Connection};
use std::fs;
use std::path::{Path, PathBuf};

/// Relatório de execução da indexação incremental local.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexingReport {
    pub indexed_count: usize,
    pub skipped_count: usize,
    pub deleted_count: usize,
    pub total_chunks: usize,
}

/// Bloco textual estruturado pronto para inserção no índice.
#[derive(Debug, Clone)]
pub struct DocumentChunk {
    pub chunk_index: usize,
    pub heading_context: Option<String>,
    pub content_text: String,
    pub content_hash: String,
    pub token_count: usize,
}

/// Indexador governado de arquivos locais para o espaço soberano de busca.
///
/// INVARIANTES DE SEGURANÇA:
/// 1. Path traversal defense: resolve caminhos canônicos e restringe ao escopo raiz autorizado.
/// 2. Filtro estrito de arquivos sensíveis: ignora arquivos ocultos, chaves, segredos e tokens.
/// 3. Delta updates: skip inteligente com base no hash SHA-256 do arquivo original.
/// 4. Sincronização de remoções: limpa do índice arquivos deletados do disco.
pub struct LocalDocumentIndexer {
    root_dir: PathBuf,
}

impl LocalDocumentIndexer {
    /// Inicializa o indexador validando e canonicizando a raiz autorizada.
    pub fn new(root_dir: impl Into<PathBuf>) -> Result<Self, YukiError> {
        let path = root_dir.into();
        let canonical = fs::canonicalize(&path).map_err(|e| {
            YukiError::ExecutionFailed(format!(
                "Diretório raiz não acessível para indexação: '{}' ({})",
                path.display(),
                e
            ))
        })?;

        if !canonical.is_dir() {
            return Err(YukiError::ExecutionFailed(format!(
                "Raiz de indexação não é um diretório: '{}'",
                canonical.display()
            )));
        }

        Ok(Self {
            root_dir: canonical,
        })
    }

    /// Retorna a raiz autorizada canônica.
    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    /// Executa a indexação incremental de todos os documentos autorizados na árvore de diretórios.
    pub fn index_workspace(&self, conn: &mut Connection) -> Result<IndexingReport, YukiError> {
        let mut report = IndexingReport {
            indexed_count: 0,
            skipped_count: 0,
            deleted_count: 0,
            total_chunks: 0,
        };

        let mut existing_files = Vec::new();
        self.scan_directory(&self.root_dir, &mut existing_files)?;

        // 1. Processar cada arquivo descoberto
        for file_path in &existing_files {
            let relative_path = file_path.strip_prefix(&self.root_dir).map_err(|_| {
                YukiError::SecurityViolation("Escape do diretório raiz".to_string())
            })?;
            let source_uri = format!("file://{}", file_path.to_string_lossy().replace('\\', "/"));

            let raw_bytes = fs::read(file_path).map_err(|e| {
                YukiError::ExecutionFailed(format!(
                    "Falha ao ler arquivo '{}': {}",
                    file_path.display(),
                    e
                ))
            })?;

            let raw_hash = compute_sha256(&raw_bytes);

            // Verificar delta hash no SQLite
            let existing_hash: Option<String> = {
                let mut stmt = conn
                    .prepare("SELECT raw_hash_sha256 FROM indexed_documents WHERE source_uri = ?")
                    .map_err(|e| YukiError::ExecutionFailed(e.to_string()))?;
                stmt.query_row(params![source_uri], |r| r.get(0)).ok()
            };

            if let Some(h) = existing_hash {
                if h == raw_hash {
                    report.skipped_count += 1;
                    continue;
                }
            }

            // Converter bytes para texto UTF-8
            let content_str = match String::from_utf8(raw_bytes.clone()) {
                Ok(s) => s,
                Err(_) => {
                    // Arquivo binário ou encoding inválido -> ignorar
                    continue;
                }
            };

            let title = relative_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("documento")
                .to_string();

            let chunks = Self::chunk_content(&content_str);
            if chunks.is_empty() {
                continue;
            }

            // Persistir documento e chunks dentro de uma transação SQLite
            let tx = conn
                .transaction()
                .map_err(|e| YukiError::ExecutionFailed(e.to_string()))?;

            // Limpar chunks anteriores se já existia
            let doc_id_opt: Option<String> = tx
                .query_row(
                    "SELECT doc_id FROM indexed_documents WHERE source_uri = ?",
                    params![source_uri],
                    |r| r.get(0),
                )
                .ok();

            let doc_id = doc_id_opt.unwrap_or_else(|| generate_unique_id("doc"));

            tx.execute("DELETE FROM chunks_fts WHERE doc_id = ?", params![doc_id])
                .map_err(|e| YukiError::ExecutionFailed(e.to_string()))?;

            tx.execute(
                "DELETE FROM document_chunks WHERE doc_id = ?",
                params![doc_id],
            )
            .map_err(|e| YukiError::ExecutionFailed(e.to_string()))?;

            let now_str = now_utc().to_rfc3339();
            let extracted_hash = compute_sha256(content_str.as_bytes());

            tx.execute(
                r#"
                INSERT INTO indexed_documents (
                    doc_id, source_uri, domain, source_kind, title,
                    raw_hash_sha256, extracted_hash_sha256, byte_size, char_count,
                    created_at, indexed_at
                ) VALUES (?, ?, 'local.workspace', 'LocalWorkspace', ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT(source_uri) DO UPDATE SET
                    title = excluded.title,
                    raw_hash_sha256 = excluded.raw_hash_sha256,
                    extracted_hash_sha256 = excluded.extracted_hash_sha256,
                    byte_size = excluded.byte_size,
                    char_count = excluded.char_count,
                    indexed_at = excluded.indexed_at
                "#,
                params![
                    doc_id,
                    source_uri,
                    title,
                    raw_hash,
                    extracted_hash,
                    raw_bytes.len() as i64,
                    content_str.chars().count() as i64,
                    now_str,
                    now_str,
                ],
            )
            .map_err(|e| YukiError::ExecutionFailed(e.to_string()))?;

            for chunk in &chunks {
                let chunk_id = format!("{}:{}", doc_id, chunk.chunk_index);
                tx.execute(
                    r#"
                    INSERT INTO document_chunks (
                        chunk_id, doc_id, chunk_index, heading_context,
                        content_text, content_hash_sha256, token_count
                    ) VALUES (?, ?, ?, ?, ?, ?, ?)
                    "#,
                    params![
                        chunk_id,
                        doc_id,
                        chunk.chunk_index as i64,
                        chunk.heading_context,
                        chunk.content_text,
                        chunk.content_hash,
                        chunk.token_count as i64,
                    ],
                )
                .map_err(|e| YukiError::ExecutionFailed(e.to_string()))?;

                tx.execute(
                    r#"
                    INSERT INTO chunks_fts (
                        chunk_id, doc_id, content_text, heading_context, title
                    ) VALUES (?, ?, ?, ?, ?)
                    "#,
                    params![
                        chunk_id,
                        doc_id,
                        chunk.content_text,
                        chunk.heading_context,
                        title,
                    ],
                )
                .map_err(|e| YukiError::ExecutionFailed(e.to_string()))?;
            }

            tx.commit()
                .map_err(|e| YukiError::ExecutionFailed(e.to_string()))?;

            report.indexed_count += 1;
            report.total_chunks += chunks.len();
        }

        // 2. Sincronizar exclusões de arquivos no disco
        let root_prefix = format!(
            "file://{}",
            self.root_dir.to_string_lossy().replace('\\', "/")
        );
        let rows: Vec<(String, String)> = {
            let mut check_stmt = conn
                .prepare("SELECT doc_id, source_uri FROM indexed_documents WHERE source_uri LIKE ? || '%'")
                .map_err(|e| YukiError::ExecutionFailed(e.to_string()))?;

            let res = check_stmt
                .query_map(params![root_prefix], |r| Ok((r.get(0)?, r.get(1)?)))
                .map_err(|e| YukiError::ExecutionFailed(e.to_string()))?
                .filter_map(|res| res.ok())
                .collect();
            res
        };

        for (doc_id, uri) in rows {
            let path_str = uri.strip_prefix("file://").unwrap_or(&uri);
            let p = PathBuf::from(path_str);
            if !p.exists() {
                let tx = conn
                    .transaction()
                    .map_err(|e| YukiError::ExecutionFailed(e.to_string()))?;

                tx.execute("DELETE FROM chunks_fts WHERE doc_id = ?", params![doc_id])
                    .map_err(|e| YukiError::ExecutionFailed(e.to_string()))?;
                tx.execute(
                    "DELETE FROM document_chunks WHERE doc_id = ?",
                    params![doc_id],
                )
                .map_err(|e| YukiError::ExecutionFailed(e.to_string()))?;
                tx.execute(
                    "DELETE FROM indexed_documents WHERE doc_id = ?",
                    params![doc_id],
                )
                .map_err(|e| YukiError::ExecutionFailed(e.to_string()))?;

                tx.commit()
                    .map_err(|e| YukiError::ExecutionFailed(e.to_string()))?;

                report.deleted_count += 1;
            }
        }

        Ok(report)
    }

    /// Varre recursivamente diretórios respeitando filtros de segurança e extensões permitidas.
    fn scan_directory(&self, dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), YukiError> {
        let entries = fs::read_dir(dir).map_err(|e| {
            YukiError::ExecutionFailed(format!("Falha ao ler diretório '{}': {}", dir.display(), e))
        })?;

        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = entry.file_name().to_string_lossy().to_string();

            // Filtro 1: Arquivos e pastas ocultas iniciadas com ponto (.git, .env, etc)
            if file_name.starts_with('.') {
                continue;
            }

            // Filtro 2: Pastas excluídas (build artifacts, dependências)
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                if matches!(
                    file_name.to_lowercase().as_str(),
                    "target" | "node_modules" | "dist" | "build" | ".tmp"
                ) {
                    continue;
                }

                // Recursão protegida
                let canonical_child = fs::canonicalize(&path).unwrap_or(path.clone());
                if canonical_child.starts_with(&self.root_dir) {
                    self.scan_directory(&canonical_child, files)?;
                }
                continue;
            }

            // Filtro 3: Nomes de arquivos contendo padrões de segredo
            let lower_name = file_name.to_lowercase();
            if lower_name.contains("secret")
                || lower_name.contains("token")
                || lower_name.contains("credential")
                || lower_name.ends_with(".key")
                || lower_name.ends_with(".pem")
                || lower_name == "id_rsa"
            {
                continue;
            }

            // Filtro 4: Extensões autorizadas (.md, .txt, .rs)
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if matches!(ext.to_lowercase().as_str(), "md" | "txt" | "rs") {
                    let canonical = fs::canonicalize(&path).unwrap_or(path.clone());
                    // Path traversal check
                    if canonical.starts_with(&self.root_dir) {
                        files.push(canonical);
                    }
                }
            }
        }

        Ok(())
    }

    /// Divide o conteúdo textual em blocos lógicos preservando cabeçalhos e parágrafos.
    pub fn chunk_content(text: &str) -> Vec<DocumentChunk> {
        let mut chunks = Vec::new();
        let lines: Vec<&str> = text.lines().collect();

        let mut current_heading: Option<String> = None;
        let mut current_buf = String::new();
        let mut chunk_index = 0;

        for line in lines {
            let trimmed = line.trim();

            if trimmed.starts_with('#') {
                // Atualizar cabeçalho
                let heading_title = trimmed.trim_start_matches('#').trim().to_string();
                if !heading_title.is_empty() {
                    // Se o buffer acumulado já tiver conteúdo razoável (> 200 caracteres), finalizar chunk
                    if current_buf.chars().count() > 200 {
                        let content = current_buf.trim().to_string();
                        if !content.is_empty() {
                            let hash = compute_sha256(content.as_bytes());
                            let token_count = content.split_whitespace().count();
                            chunks.push(DocumentChunk {
                                chunk_index,
                                heading_context: current_heading.clone(),
                                content_text: content,
                                content_hash: hash,
                                token_count,
                            });
                            chunk_index += 1;
                            current_buf.clear();
                        }
                    }
                    current_heading = Some(heading_title);
                }
            }

            current_buf.push_str(line);
            current_buf.push('\n');

            // Limite de chunk: ~800 caracteres em quebra de parágrafo dupla
            if current_buf.chars().count() > 800 && trimmed.is_empty() {
                let content = current_buf.trim().to_string();
                if !content.is_empty() {
                    let hash = compute_sha256(content.as_bytes());
                    let token_count = content.split_whitespace().count();
                    chunks.push(DocumentChunk {
                        chunk_index,
                        heading_context: current_heading.clone(),
                        content_text: content,
                        content_hash: hash,
                        token_count,
                    });
                    chunk_index += 1;
                    current_buf.clear();
                }
            }
        }

        // Finalizar último chunk
        let final_content = current_buf.trim().to_string();
        if !final_content.is_empty() {
            let hash = compute_sha256(final_content.as_bytes());
            let token_count = final_content.split_whitespace().count();
            chunks.push(DocumentChunk {
                chunk_index,
                heading_context: current_heading,
                content_text: final_content,
                content_hash: hash,
                token_count,
            });
        }

        chunks
    }
}
