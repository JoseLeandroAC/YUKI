use crate::contracts::memory::{
    MemoryError, MemoryQuery, MemoryRecord, MemoryRepository, MemoryRetriever, MemoryStatus,
    MemoryType,
};
use std::collections::HashSet;
use std::sync::Arc;

/// Recuperador padrão de memórias governado por relevância e orçamento contextual.
pub struct DefaultMemoryRetriever {
    repository: Arc<dyn MemoryRepository>,
}

impl DefaultMemoryRetriever {
    pub fn new(repository: Arc<dyn MemoryRepository>) -> Self {
        Self { repository }
    }
}

impl MemoryRetriever for DefaultMemoryRetriever {
    fn retrieve(&self, query: &MemoryQuery) -> Result<Vec<MemoryRecord>, MemoryError> {
        self.repository.query(query)
    }

    fn retrieve_for_context(
        &self,
        input_text: &str,
        limit: usize,
    ) -> Result<Vec<MemoryRecord>, MemoryError> {
        let max_items = if limit == 0 { 10 } else { limit };
        let mut results = Vec::new();
        let mut seen_ids = HashSet::new();

        // 1. Sempre recuperar instruções ativas prioritárias (diretrizes explícitas do usuário)
        let instructions_query = MemoryQuery {
            statuses: Some(vec![MemoryStatus::Active]),
            memory_types: Some(vec![MemoryType::Instruction]),
            limit: 5,
            ..Default::default()
        };
        for mem in self.repository.query(&instructions_query)? {
            if seen_ids.insert(mem.id.clone()) {
                results.push(mem);
            }
        }

        // 2. Recuperar preferências ativas declaradas pelo usuário
        let preferences_query = MemoryQuery {
            statuses: Some(vec![MemoryStatus::Active]),
            memory_types: Some(vec![MemoryType::Preference]),
            limit: 5,
            ..Default::default()
        };
        for mem in self.repository.query(&preferences_query)? {
            if seen_ids.insert(mem.id.clone()) {
                results.push(mem);
            }
        }

        // 3. Se houver espaço no orçamento, buscar fatos e contexto relevantes para a consulta atual
        if results.len() < max_items && !input_text.trim().is_empty() {
            let search_query = MemoryQuery {
                text_query: Some(input_text.to_string()),
                statuses: Some(vec![MemoryStatus::Active]),
                memory_types: Some(vec![
                    MemoryType::Fact,
                    MemoryType::ProjectContext,
                    MemoryType::Episode,
                ]),
                limit: max_items - results.len(),
                ..Default::default()
            };

            for mem in self.repository.query(&search_query)? {
                if seen_ids.insert(mem.id.clone()) {
                    results.push(mem);
                }
            }
        }

        // 4. Se ainda houver orçamento e a busca textual não retornou nada, trazer fatos gerais recentes
        if results.len() < max_items {
            let recent_facts_query = MemoryQuery {
                statuses: Some(vec![MemoryStatus::Active]),
                memory_types: Some(vec![MemoryType::Fact, MemoryType::ProjectContext]),
                limit: max_items - results.len(),
                ..Default::default()
            };

            for mem in self.repository.query(&recent_facts_query)? {
                if seen_ids.insert(mem.id.clone()) {
                    results.push(mem);
                }
            }
        }

        // Truncar para respeitar rigorosamente o orçamento de itens
        results.truncate(max_items);
        Ok(results)
    }
}
