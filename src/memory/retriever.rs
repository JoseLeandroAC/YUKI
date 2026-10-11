use crate::contracts::memory::{
    MemoryError, MemoryQuery, MemoryRecord, MemoryRepository, MemoryRetriever, MemoryStatus,
    MemoryType, DEFAULT_OPERATOR_OWNER_ID,
};
use std::collections::HashSet;
use std::sync::Arc;

/// Recuperador padrão de memórias governado por relevância, escopo do proprietário e orçamento contextual.
pub struct DefaultMemoryRetriever {
    repository: Arc<dyn MemoryRepository>,
    owner_id: String,
}

impl DefaultMemoryRetriever {
    pub fn new(repository: Arc<dyn MemoryRepository>) -> Self {
        Self {
            repository,
            owner_id: DEFAULT_OPERATOR_OWNER_ID.to_string(),
        }
    }

    pub fn with_owner(mut self, owner_id: impl Into<String>) -> Self {
        self.owner_id = owner_id.into().trim().to_string();
        self
    }
}

impl MemoryRetriever for DefaultMemoryRetriever {
    fn retrieve(&self, query: &MemoryQuery) -> Result<Vec<MemoryRecord>, MemoryError> {
        let mut q = query.clone();
        if q.owner_id.is_none() {
            q.owner_id = Some(self.owner_id.clone());
        }
        self.repository.query(&q)
    }

    fn retrieve_for_context(
        &self,
        input_text: &str,
        limit: usize,
    ) -> Result<Vec<MemoryRecord>, MemoryError> {
        let max_items = if limit == 0 { 10 } else { limit };
        let mut results = Vec::new();
        let mut seen_ids = HashSet::new();

        // 1. Sempre recuperar instruções ativas prioritárias (diretrizes explícitas do operador)
        let instructions_query = MemoryQuery {
            owner_id: Some(self.owner_id.clone()),
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

        // 2. Recuperar preferências ativas declaradas pelo operador
        let preferences_query = MemoryQuery {
            owner_id: Some(self.owner_id.clone()),
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

        // 3. Se houver espaço no orçamento e input_text não for vazio, buscar fatos e contexto relevantes
        // via FTS5 especificamente correspondentes ao prompt.
        if results.len() < max_items && !input_text.trim().is_empty() {
            let search_query = MemoryQuery {
                owner_id: Some(self.owner_id.clone()),
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

        // OBSERVAÇÃO ARQUITETURAL (MARCO 7.1):
        // Se a busca textual não retornar correspondências para o prompt, a Yuki deliberadamente
        // NÃO despeja memórias não relacionadas ou aleatórias como se fossem contextuais.
        // Fatos irrelevantes não devem poluir o contexto da conversa.

        Ok(results)
    }
}
