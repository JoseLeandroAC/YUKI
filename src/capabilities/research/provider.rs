use crate::contracts::errors::YukiError;
use crate::contracts::research::{
    compute_sha256, now_iso8601, ResearchFetchInput, ResearchFetchResult, ResearchSearchInput,
    ResearchSearchResult, SearchResultItem, SourceKind,
};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::RwLock;

// ============================================================================
// Provider Abstraction Traits (ADR-020)
// ============================================================================

/// Trait abstrato para provedores de busca textual estruturada.
pub trait SearchProvider: Send + Sync {
    fn search(&self, input: &ResearchSearchInput) -> Result<ResearchSearchResult, YukiError>;

    /// Indica se o provedor opera em modo live (com saída de rede para serviços externos).
    fn is_live(&self) -> bool {
        false
    }
}

/// Trait abstrato para provedores de recuperação de conteúdo de páginas.
pub trait ContentFetchProvider: Send + Sync {
    fn fetch(&self, input: &ResearchFetchInput) -> Result<ResearchFetchResult, YukiError>;

    /// Indica se o provedor opera em modo live (com saída de rede para serviços externos).
    fn is_live(&self) -> bool {
        false
    }
}

// ============================================================================
// In-Memory Mock Search Provider (Marco 1: Zero Network, Deterministic)
// ============================================================================

/// Provedor de busca em memória para testes governados offline.
pub struct MockSearchProvider {
    preset_results: RwLock<Option<Vec<SearchResultItem>>>,
    simulated_error: RwLock<Option<String>>,
    call_count: AtomicUsize,
}

impl MockSearchProvider {
    pub fn new() -> Self {
        Self {
            preset_results: RwLock::new(None),
            simulated_error: RwLock::new(None),
            call_count: AtomicUsize::new(0),
        }
    }

    /// Configura resultados predefinidos específicos para cenários de teste.
    pub fn with_preset_results(self, results: Vec<SearchResultItem>) -> Self {
        *self.preset_results.write().unwrap() = Some(results);
        self
    }

    /// Configura simulação de falha do provedor de busca.
    pub fn with_simulated_error(self, err: impl Into<String>) -> Self {
        *self.simulated_error.write().unwrap() = Some(err.into());
        self
    }

    /// Retorna o número de vezes que `search` foi chamado.
    pub fn call_count(&self) -> usize {
        self.call_count.load(Ordering::SeqCst)
    }
}

impl Default for MockSearchProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl SearchProvider for MockSearchProvider {
    fn search(&self, input: &ResearchSearchInput) -> Result<ResearchSearchResult, YukiError> {
        self.call_count.fetch_add(1, Ordering::SeqCst);

        // 1. Simulação de erro configurado
        if let Some(err) = self.simulated_error.read().unwrap().as_ref() {
            return Err(YukiError::ExecutionFailed(format!(
                "Falha simulada no provedor de busca: {}",
                err
            )));
        }

        // 2. Resultados predefinidos
        if let Some(preset) = self.preset_results.read().unwrap().as_ref() {
            let mut results = preset.clone();
            if results.len() > input.max_results as usize {
                results.truncate(input.max_results as usize);
            }
            return Ok(ResearchSearchResult {
                query: input.query.clone(),
                provider: "MockSearchProvider".to_string(),
                searched_at: now_iso8601(),
                results_count: results.len(),
                results,
            });
        }

        // 3. Resultados determinísticos padrão (ADR-020)
        let count = (input.max_results as usize).min(3);
        let mut results = Vec::with_capacity(count);

        results.push(SearchResultItem {
            cite_id: "src:1".to_string(),
            url: format!("https://mock.research.local/articles/{}", sanitize_slug(&input.query)),
            title: format!("Artigo Técnico Simulado: {}", input.query),
            snippet: format!(
                "Evidência simulada para o termo de pesquisa '{}'. Retorno determinístico em memória.",
                input.query
            ),
            domain: "mock.research.local".to_string(),
            published_date: Some("2026-10-10T07:00:00Z".to_string()),
            confidence_state: SourceKind::AggregatedSnippet,
        });

        if count >= 2 {
            results.push(SearchResultItem {
                cite_id: "src:2".to_string(),
                url: "https://mock.research.local/portal/noticias-2026".to_string(),
                title: "Portal de Referência Oficial 2026".to_string(),
                snippet: "Informações públicas catalogadas sobre eventos contemporâneos e contexto fático.".to_string(),
                domain: "mock.research.local".to_string(),
                published_date: Some("2026-10-09T18:30:00Z".to_string()),
                confidence_state: SourceKind::AggregatedSnippet,
            });
        }

        if count >= 3 {
            results.push(SearchResultItem {
                cite_id: "src:3".to_string(),
                url: "https://mock.research.local/base-conhecimento".to_string(),
                title: "Base de Conhecimento e Dados Abertos".to_string(),
                snippet: "Registro de dados estatísticos e governança informacional verificável."
                    .to_string(),
                domain: "mock.research.local".to_string(),
                published_date: Some("2026-10-08T12:00:00Z".to_string()),
                confidence_state: SourceKind::AggregatedSnippet,
            });
        }

        Ok(ResearchSearchResult {
            query: input.query.clone(),
            provider: "MockSearchProvider".to_string(),
            searched_at: now_iso8601(),
            results_count: results.len(),
            results,
        })
    }
}

// ============================================================================
// In-Memory Mock Fetch Provider (Marco 1: Zero Network, Deterministic)
// ============================================================================

/// Provedor de leitura em memória para testes governados offline.
pub struct MockFetchProvider {
    preset_result: RwLock<Option<ResearchFetchResult>>,
    simulated_error: RwLock<Option<String>>,
    call_count: AtomicUsize,
}

impl MockFetchProvider {
    pub fn new() -> Self {
        Self {
            preset_result: RwLock::new(None),
            simulated_error: RwLock::new(None),
            call_count: AtomicUsize::new(0),
        }
    }

    /// Configura um resultado predefinido específico (ex.: payload hostil para testes de injeção).
    pub fn with_preset_result(self, result: ResearchFetchResult) -> Self {
        *self.preset_result.write().unwrap() = Some(result);
        self
    }

    /// Configura simulação de erro de leitura (ex.: HTTP 404 simulado ou timeout).
    pub fn with_simulated_error(self, err: impl Into<String>) -> Self {
        *self.simulated_error.write().unwrap() = Some(err.into());
        self
    }

    /// Retorna o número de vezes que `fetch` foi chamado.
    pub fn call_count(&self) -> usize {
        self.call_count.load(Ordering::SeqCst)
    }
}

impl Default for MockFetchProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ContentFetchProvider for MockFetchProvider {
    fn fetch(&self, input: &ResearchFetchInput) -> Result<ResearchFetchResult, YukiError> {
        self.call_count.fetch_add(1, Ordering::SeqCst);

        // 1. Simulação de erro configurado
        if let Some(err) = self.simulated_error.read().unwrap().as_ref() {
            return Err(YukiError::ExecutionFailed(format!(
                "Falha simulada na recuperação de página: {}",
                err
            )));
        }

        // 2. Resultado predefinido
        if let Some(preset) = self.preset_result.read().unwrap().as_ref() {
            return Ok(preset.clone());
        }

        // 3. Resultado determinístico padrão (ADR-020)
        let raw_text = format!(
            "# Documento Recuperado de {}\n\nEste é o conteúdo textual limpo e sanitizado recuperado em ambiente de teste determinístico offline (Marco 1). Nenhum socket de rede foi aberto.",
            input.url
        );

        let (extracted_text, truncated) = if raw_text.len() > input.max_length_chars {
            (raw_text[..input.max_length_chars].to_string(), true)
        } else {
            (raw_text, false)
        };

        let content_hash_sha256 = compute_sha256(extracted_text.as_bytes());
        let bytes_observed = extracted_text.len();

        Ok(ResearchFetchResult {
            url: input.url.clone(),
            final_url: input.url.clone(),
            fetched_at: now_iso8601(),
            http_status: 200,
            content_type: "text/markdown; charset=utf-8".to_string(),
            title: format!("Conteúdo de {}", input.url),
            published_date: Some("2026-10-10T07:15:00Z".to_string()),
            extracted_text,
            content_hash_sha256,
            truncated,
            bytes_observed,
            raw_network_bytes: bytes_observed,
            decompressed_bytes: bytes_observed,
            content_encoding: Some("identity".to_string()),
            source_id: Some("src:fetch:1".to_string()),
            confidence_state: SourceKind::DirectSource,
        })
    }
}

fn sanitize_slug(input: &str) -> String {
    input
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_lowercase()
}
