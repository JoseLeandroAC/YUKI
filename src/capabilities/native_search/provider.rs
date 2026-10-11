use crate::capabilities::native_search::searcher::LocalSearchEngine;
use crate::capabilities::research::provider::SearchProvider;
use crate::contracts::errors::YukiError;
use crate::contracts::research::{
    now_iso8601, ResearchSearchInput, ResearchSearchResult, SearchResultItem, SourceKind,
};
use std::sync::Arc;

/// Provedor governado de busca nativa sobre documentos e código do workspace local.
///
/// INVARIANTE INEGOCIÁVEL:
/// - `is_live() -> false`: Zero conexões de rede, sem requisições HTTP, operação 100% local.
/// - Todas as fontes classificadas como `SourceKind::LocalWorkspace`.
pub struct NativeSearchProvider {
    search_engine: Arc<LocalSearchEngine>,
}

impl NativeSearchProvider {
    pub fn new(search_engine: Arc<LocalSearchEngine>) -> Self {
        Self { search_engine }
    }
}

impl SearchProvider for NativeSearchProvider {
    fn search(&self, input: &ResearchSearchInput) -> Result<ResearchSearchResult, YukiError> {
        let items = self
            .search_engine
            .search(&input.query, input.max_results as usize)?;

        let results = items
            .into_iter()
            .enumerate()
            .map(|(idx, item)| {
                let title = match &item.heading_context {
                    Some(heading) => format!("{} — {}", item.title, heading),
                    None => item.title,
                };

                SearchResultItem {
                    cite_id: format!("src:{}", idx + 1),
                    url: item.source_uri,
                    title,
                    snippet: item.snippet,
                    domain: "local.workspace".to_string(),
                    published_date: None,
                    confidence_state: SourceKind::LocalWorkspace,
                }
            })
            .collect::<Vec<SearchResultItem>>();

        Ok(ResearchSearchResult {
            query: input.query.clone(),
            provider: "NativeSearchProvider".to_string(),
            searched_at: now_iso8601(),
            results_count: results.len(),
            results,
        })
    }

    fn is_live(&self) -> bool {
        false
    }
}
