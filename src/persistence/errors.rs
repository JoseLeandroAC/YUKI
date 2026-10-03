use crate::contracts::errors::YukiError;
use thiserror::Error;

/// Typed persistence errors for Yuki's local audit storage.
///
/// INVARIANT:
/// Database error details are sanitized and do not leak credentials or raw SQL strings
/// into user-facing contexts.
#[derive(Debug, Error)]
pub enum PersistenceError {
    #[error("Falha na inicialização do armazenamento persistente: {0}")]
    Initialization(String),

    #[error("Falha na execução de migração de esquema: {0}")]
    Migration(String),

    #[error("Versão de esquema não suportada: encontrada versão {found}, máxima suportada é {supported}")]
    SchemaVersionUnsupported { found: i64, supported: i64 },

    #[error("Falha na verificação de integridade do banco (quick_check): {0}")]
    IntegrityCheck(String),

    #[error("Falha na escrita no armazenamento de auditoria: {0}")]
    Write(String),

    #[error("Falha na leitura do armazenamento de auditoria: {0}")]
    Read(String),

    #[error("Payload de auditoria excede o limite máximo permitido: {size} bytes (limite: {limit} bytes)")]
    PayloadTooLarge { size: usize, limit: usize },

    #[error("Armazenamento de auditoria indisponível: {0}")]
    DatabaseUnavailable(String),

    #[error("Degradação do serviço de auditoria: {0}")]
    AuditDegraded(String),
}

impl From<PersistenceError> for YukiError {
    fn from(err: PersistenceError) -> Self {
        match err {
            PersistenceError::Write(msg) | PersistenceError::DatabaseUnavailable(msg) => {
                YukiError::ExecutionFailed(format!("Erro de persistência de auditoria: {}", msg))
            }
            other => YukiError::ExecutionFailed(other.to_string()),
        }
    }
}
