use std::time::Duration;

/// Configuração operacional do Model Gateway (MVP-1 Configurable Defaults).
///
/// NOTA ARQUITETURAL:
/// Estes parâmetros são padrões operacionais configuráveis para o ciclo MVP-1,
/// NÃO constituindo constantes constitucionais perenes da Yuki.
#[derive(Debug, Clone)]
pub struct ModelGatewayConfig {
    /// Timeout para abertura de conexão TCP/TLS com o provedor.
    pub connect_timeout: Duration,
    /// Timeout máximo para transmissão HTTP e geração completa da resposta.
    pub request_timeout: Duration,
    /// Número máximo de retentativas para erros transitórios (RateLimit, 5xx, Network).
    pub max_retries: u32,
    /// Duração base para backoff exponencial.
    pub initial_backoff: Duration,
    /// Teto máximo de espera entre retentativas.
    pub max_backoff: Duration,
    /// Tamanho máximo permitido para o payload da requisição enviada (bytes).
    pub max_request_bytes: usize,
    /// Tamanho máximo permitido para o corpo da resposta recebida (bytes).
    pub max_response_bytes: usize,
    /// Limite máximo de tokens de saída solicitados ao modelo.
    pub max_output_tokens: u32,
    /// Profundidade máxima permitida na árvore de parâmetros JSON da proposta.
    pub max_proposal_depth: usize,
    /// Tamanho máximo permitido para os parâmetros da proposta de capacidade (bytes).
    pub max_proposal_size_bytes: usize,
    /// Identificador do modelo padrão a ser invocado.
    pub model_id: String,
    /// Alias operacional sanitizado para auditoria da credencial utilizada.
    pub sanitized_credential_alias: String,
}

impl Default for ModelGatewayConfig {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(5),
            request_timeout: Duration::from_secs(30),
            max_retries: 3,
            initial_backoff: Duration::from_millis(500),
            max_backoff: Duration::from_secs(8),
            max_request_bytes: 1024 * 1024,      // 1 MB
            max_response_bytes: 2 * 1024 * 1024, // 2 MB
            max_output_tokens: 2048,
            max_proposal_depth: 5,
            max_proposal_size_bytes: 64 * 1024, // 64 KB
            model_id: "gemini-2.5-flash".to_string(),
            sanitized_credential_alias: "cred_gemini_primary".to_string(),
        }
    }
}
