use std::time::Duration;
use thiserror::Error;

/// Taxonomia de erros do Model Gateway (Marco 2 / ADR-018).
///
/// Todas as falhas de rede, provedor, validação e segurança são mapeadas
/// para variantes fortemente tipadas, evitando strings livres e assegurando
/// sanitização de segredos e cabeçalhos sensíveis.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ModelError {
    #[error("Falha de autenticação no provedor: {0}")]
    Authentication(String),

    #[error("Autorização negada pelo provedor: {0}")]
    ProviderAuthorization(String),

    #[error("Requisição inválida para o provedor: {0}")]
    InvalidRequest(String),

    #[error("Limite de taxa excedido (Rate Limited). Retry-After: {retry_after_secs:?}s")]
    RateLimited { retry_after_secs: Option<u64> },

    #[error("Cota do provedor esgotada: {0}")]
    QuotaExceeded(String),

    #[error("Operação excedeu o tempo limite (Timeout): {0:?}")]
    Timeout(Duration),

    #[error("Falha de rede ou conectividade: {0}")]
    Network(String),

    #[error("Provedor indisponível (HTTP {status}): {message}")]
    ProviderUnavailable { status: u16, message: String },

    #[error("Erro interno do provedor: {0}")]
    ProviderInternal(String),

    #[error("Resposta do provedor malformada: {0}")]
    MalformedResponse(String),

    #[error("Saída do modelo violou o esquema esperado: {0}")]
    SchemaViolation(String),

    #[error("Conteúdo bloqueado pelas políticas de segurança do provedor: {0}")]
    ContentBlocked(String),

    #[error("Capacidade proposta não é suportada ou não existe: {0}")]
    UnsupportedCapability(String),

    #[error("Argumentos inválidos propostos para '{capability}': {details}")]
    InvalidArguments { capability: String, details: String },

    #[error("Operação cancelada")]
    Cancelled,

    #[error("Erro interno no gateway de modelos: {0}")]
    Internal(String),
}

impl ModelError {
    /// Determina se o erro é transitório e seguro para retentativa com backoff.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            ModelError::RateLimited { .. }
                | ModelError::Network(_)
                | ModelError::ProviderUnavailable { .. }
                | ModelError::Timeout(_)
        )
    }
}
