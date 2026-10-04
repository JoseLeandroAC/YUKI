use crate::models::errors::ModelError;
use crate::models::provider::{
    HealthStatus, ModelMetadata, ModelProvider, ModelRequest, ModelResponse,
};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Roteador do Model Gateway para despacho configurável e failover opcional (ADR-018).
///
/// INVARIANTE CONSTITUCIONAL DE PRIVACIDADE E JURISDIÇÃO (ADR-015, ADR-017):
/// `Provider Failure != Authorization To Send Data To Another Provider`
/// Falha do provedor primário NÃO autoriza o envio indiscriminado de dados a terceiros.
/// O failover é estritamente opt-in, desativado por padrão (`fallback: None`).
/// Qualquer fallback configurado DEVE ser previamente aprovado e elegível para processar
/// a mesma categoria de dados e propósito de contexto.
pub struct ModelRouter {
    primary: Arc<dyn ModelProvider>,
    fallback: Option<Arc<dyn ModelProvider>>,
}

impl ModelRouter {
    pub fn new(primary: Arc<dyn ModelProvider>) -> Self {
        Self {
            primary,
            fallback: None,
        }
    }

    pub fn with_fallback(
        primary: Arc<dyn ModelProvider>,
        fallback: Arc<dyn ModelProvider>,
    ) -> Self {
        Self {
            primary,
            fallback: Some(fallback),
        }
    }
}

impl ModelProvider for ModelRouter {
    fn generate<'a>(
        &'a self,
        request: &'a ModelRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, ModelError>> + Send + 'a>> {
        Box::pin(async move {
            match self.primary.generate(request).await {
                Ok(resp) => Ok(resp),
                Err(err) => {
                    if let Some(fallback) = &self.fallback {
                        if err.is_retryable()
                            || matches!(err, ModelError::ProviderUnavailable { .. })
                        {
                            tracing::warn!(
                                "Provedor primário falhou com erro transitório: {}. Ativando provedor de fallback.",
                                err
                            );
                            return fallback.generate(request).await;
                        }
                    }
                    Err(err)
                }
            }
        })
    }

    fn metadata(&self) -> ModelMetadata {
        self.primary.metadata()
    }

    fn health(&self) -> HealthStatus {
        self.primary.health()
    }
}
