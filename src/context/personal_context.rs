use crate::contracts::memory::{MemoryError, MemoryId, MemoryRecord, MemoryRetriever, MemoryType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Perfil do operador humano atendido pela Yuki.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorProfile {
    pub name: String,
    pub preferred_language: String,
    pub timezone: Option<String>,
}

impl Default for OperatorProfile {
    fn default() -> Self {
        Self {
            name: "Operador".to_string(),
            preferred_language: "pt-BR".to_string(),
            timezone: Some("America/Sao_Paulo".to_string()),
        }
    }
}

/// Configuração orçamentária para a construção de PersonalContext.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalContextConfig {
    /// Limite máximo em caracteres para o contexto pessoal injetado.
    pub max_chars: usize,
    /// Quantidade máxima de registros de memória a incorporar.
    pub max_items: usize,
    /// Se resumos episódicos recentes devem ser incluídos caso haja orçamento.
    pub include_episodes: bool,
}

impl Default for PersonalContextConfig {
    fn default() -> Self {
        Self {
            max_chars: 4000,
            max_items: 12,
            include_episodes: true,
        }
    }
}

/// Contexto pessoal unificado e delimitado por orçamento (ADR-016).
///
/// INVARIANTE:
/// O contexto pessoal não deve vazar credenciais, extrapolar o orçamento
/// de tokens do modelo nem simular fatos inexistentes.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PersonalContext {
    pub operator: OperatorProfile,
    pub instructions: Vec<MemoryRecord>,
    pub preferences: Vec<MemoryRecord>,
    pub facts: Vec<MemoryRecord>,
    pub projects: Vec<MemoryRecord>,
    pub episodes: Vec<MemoryRecord>,
    pub referenced_memory_ids: Vec<MemoryId>,
}

impl PersonalContext {
    /// Formata uma seção descritiva e limpa para inclusão no contexto do modelo.
    pub fn to_prompt_section(&self) -> String {
        let mut sections = Vec::new();

        sections.push(format!(
            "OPERADOR: {} (Idioma preferencial: {})",
            self.operator.name, self.operator.preferred_language
        ));

        if !self.instructions.is_empty() {
            let mut inst_lines = vec!["DIRETRIZES E INSTRUÇÕES FIXAS DO USUÁRIO:".to_string()];
            for item in &self.instructions {
                inst_lines.push(format!("- [{}]: {}", item.title, item.content));
            }
            sections.push(inst_lines.join("\n"));
        }

        if !self.preferences.is_empty() {
            let mut pref_lines = vec!["PREFERÊNCIAS DECLARADAS PELO USUÁRIO:".to_string()];
            for item in &self.preferences {
                pref_lines.push(format!("- [{}]: {}", item.title, item.content));
            }
            sections.push(pref_lines.join("\n"));
        }

        if !self.facts.is_empty() || !self.projects.is_empty() {
            let mut ctx_lines = vec!["CONTEXTO E FATOS RELEVANTES:".to_string()];
            for item in &self.facts {
                ctx_lines.push(format!("- (Fato) [{}]: {}", item.title, item.content));
            }
            for item in &self.projects {
                ctx_lines.push(format!("- (Projeto) [{}]: {}", item.title, item.content));
            }
            sections.push(ctx_lines.join("\n"));
        }

        if !self.episodes.is_empty() {
            let mut ep_lines = vec!["HISTÓRICO RECENTE:".to_string()];
            for item in &self.episodes {
                ep_lines.push(format!("- [{}]: {}", item.title, item.content));
            }
            sections.push(ep_lines.join("\n"));
        }

        sections.join("\n\n")
    }

    /// Validação sanitária rigorosa contra segredos no contexto pessoal.
    pub fn assert_no_secrets(&self) -> Result<(), String> {
        let serialized = serde_json::to_string(self)
            .unwrap_or_default()
            .to_lowercase();
        let forbidden = [
            "password",
            "api_key",
            "apikey",
            "secret_key",
            "private_key",
            "bearer ",
            "access_token",
            "refresh_token",
            "vault.json",
        ];

        for pattern in &forbidden {
            if serialized.contains(pattern) {
                return Err(format!(
                    "PersonalContext contém padrão proibido/segredo: '{}'",
                    pattern
                ));
            }
        }
        Ok(())
    }
}

/// Construtor governado de PersonalContext integrado com recuperação de memórias.
pub struct PersonalContextBuilder {
    retriever: Arc<dyn MemoryRetriever>,
    config: PersonalContextConfig,
    operator: OperatorProfile,
}

impl PersonalContextBuilder {
    pub fn new(retriever: Arc<dyn MemoryRetriever>) -> Self {
        Self {
            retriever,
            config: PersonalContextConfig::default(),
            operator: OperatorProfile::default(),
        }
    }

    pub fn with_config(mut self, config: PersonalContextConfig) -> Self {
        self.config = config;
        self
    }

    pub fn with_operator(mut self, operator: OperatorProfile) -> Self {
        self.operator = operator;
        self
    }

    /// Constrói o PersonalContext selecionando e ordenando memórias por prioridade e orçamento.
    pub fn build(&self, user_input: &str) -> Result<PersonalContext, MemoryError> {
        let memories = self
            .retriever
            .retrieve_for_context(user_input, self.config.max_items)?;

        let mut personal_ctx = PersonalContext {
            operator: self.operator.clone(),
            ..Default::default()
        };

        let mut current_chars = 0;
        let limit_chars = self.config.max_chars;

        // Função auxiliar para tentar adicionar respeitando o limite de caracteres
        let mut try_add = |record: MemoryRecord, target: &mut Vec<MemoryRecord>| -> bool {
            let entry_len = record.title.len() + record.content.len() + 32;
            if current_chars + entry_len <= limit_chars {
                current_chars += entry_len;
                personal_ctx.referenced_memory_ids.push(record.id.clone());
                target.push(record);
                true
            } else {
                false
            }
        };

        // 1. Prioridade 1: Instruções fixas
        for mem in memories
            .iter()
            .filter(|m| m.memory_type == MemoryType::Instruction)
        {
            try_add(mem.clone(), &mut personal_ctx.instructions);
        }

        // 2. Prioridade 2: Preferências do usuário
        for mem in memories
            .iter()
            .filter(|m| m.memory_type == MemoryType::Preference)
        {
            try_add(mem.clone(), &mut personal_ctx.preferences);
        }

        // 3. Prioridade 3: Fatos relevantes
        for mem in memories
            .iter()
            .filter(|m| m.memory_type == MemoryType::Fact)
        {
            try_add(mem.clone(), &mut personal_ctx.facts);
        }

        // 4. Prioridade 4: Contexto de projetos
        for mem in memories
            .iter()
            .filter(|m| m.memory_type == MemoryType::ProjectContext)
        {
            try_add(mem.clone(), &mut personal_ctx.projects);
        }

        // 5. Prioridade 5: Episódios recentes (se habilitado)
        if self.config.include_episodes {
            for mem in memories
                .iter()
                .filter(|m| m.memory_type == MemoryType::Episode)
            {
                try_add(mem.clone(), &mut personal_ctx.episodes);
            }
        }

        // Validação sanitária de segurança
        if let Err(err) = personal_ctx.assert_no_secrets() {
            return Err(MemoryError::ForbiddenContent(err));
        }

        Ok(personal_ctx)
    }
}
