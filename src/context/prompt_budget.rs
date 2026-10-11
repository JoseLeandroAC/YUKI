//! Gestor de Orçamento Integral de Contexto e Tokens (Marco 7.2)
//!
//! Realiza a auditoria, particionamento e controle conservador do orçamento
//! completo de caracteres e tokens enviados aos Provedores de Modelos.
//!
//! INVARIANTES ARQUITETURAIS:
//! 1. Data != Instruction: Separação estrita de camadas e imunidade ontológica.
//! 2. Preservação de Instruções Essenciais: Diretrizes constitucionais e ontológicas (Tier 1)
//!    NUNCA são truncadas ou descartadas.
//! 3. Truncamento Ponderado: Quando o orçamento for excedido, o descarte progressivo
//!    ocorre estritamente da menor prioridade para a maior (Tier 6 -> Tier 5 -> Tier 4).
//! 4. Margem Conservadora de Segurança: Estimativa conservadora de tokens
//!    com margem para overhead de serialização e reserva estrita para geração de resposta.
//! 5. Falha Fechada: Quando nem mesmo o turno do usuário ou instruções essenciais couberem
//!    no limite disponível, o sistema rejeita a montagem sem descarte silencioso arbitrário.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Avisos canônicos de truncamento
pub const NOTICE_PERSONAL_CONTEXT_TRUNCATED: &str =
    "\n[... CONTEXTO PESSOAL TRUNCADO POR LIMITE DE ORÇAMENTO ...]";
pub const NOTICE_TOOL_RESULTS_TRUNCATED: &str =
    "\n[... DADOS DE FERRAMENTAS TRUNCADOS POR LIMITE DE ORÇAMENTO ...]";
pub const NOTICE_HISTORY_TRUNCATED: &str =
    "\n[... HISTÓRICO TRUNCADO POR JANELA DESLIZANTE DE ORÇAMENTO ...]";

/// Erros relacionados à gestão de orçamento de prompt.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum PromptBudgetError {
    #[error("Orçamento insuficiente para acomodar as instruções essenciais de sistema: necessário {required_chars} chars, limite {available_chars} chars")]
    EssentialInstructionsExceedBudget {
        required_chars: usize,
        available_chars: usize,
    },

    #[error("Orçamento de entrada insuficiente para acomodar o turno do usuário ({required_chars} chars) além das instruções essenciais ({available_chars} chars disponíveis)")]
    UserTurnExceedsBudget {
        required_chars: usize,
        available_chars: usize,
    },

    #[error("Orçamento total de contexto excedido mesmo após truncamento completo: {0}")]
    BudgetExhausted(String),
}

/// Níveis de prioridade de conteúdo na montagem integral do prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum PromptTier {
    /// Tier 1: Instruções de sistema e diretrizes constitucionais inegociáveis (NUNCA truncáveis).
    Tier1EssentialSystem = 1,
    /// Tier 2: Perfil de personalidade comportamental adaptativa (preservado).
    Tier2Personality = 2,
    /// Tier 3: Mensagem do usuário do turno atual (preservada; falha fechada se não couber).
    Tier3UserTurn = 3,
    /// Tier 4: Resultados de execução de ferramentas e observações externas (truncável com aviso).
    Tier4ToolResults = 4,
    /// Tier 5: Histórico conversacional de turnos anteriores (truncável por janela deslizante).
    Tier5ConversationHistory = 5,
    /// Tier 6: Contexto pessoal e memórias recuperadas (truncável por cota proporcional).
    Tier6PersonalContext = 6,
}

/// Perfil de estimativa de tokens segundo o modelo utilizado.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum TokenizerModelProfile {
    /// Estimativa conservadora para modelos multilíngues gerais (1 token ~= 3.5 chars).
    #[default]
    ConservativeMultilingual,
    /// Modelos com vocabulário expandido (ex: Gemini / Claude - 1 token ~= 3.8 chars).
    ExpandedContext,
    /// Perfil estrito de segurança alta para modelos compactos (1 token ~= 3.0 chars).
    StrictHighSafety,
}

impl TokenizerModelProfile {
    pub fn chars_per_token(&self) -> f64 {
        match self {
            Self::ConservativeMultilingual => 3.5,
            Self::ExpandedContext => 3.8,
            Self::StrictHighSafety => 3.0,
        }
    }
}

/// Configuração do orçamento integral de contexto do modelo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegralPromptBudgetConfig {
    /// Janela máxima total de contexto do modelo em tokens (ex: 8192, 32768, 1048576).
    pub max_model_context_tokens: usize,
    /// Reserva obrigatória de tokens para a geração da resposta pelo modelo.
    pub reserved_response_tokens: usize,
    /// Cota máxima percentual permitida para memórias do contexto pessoal (ex: 0.20 = 20%).
    pub max_personal_context_ratio: f32,
    /// Cota máxima percentual permitida para histórico de conversa anterior (ex: 0.35 = 35%).
    pub max_history_ratio: f32,
    /// Margem de segurança percentual para overhead de formatação/JSON (ex: 0.10 = 10%).
    pub safety_margin_ratio: f32,
    /// Perfil do tokenizer para estimativa conservadora de tokens.
    pub tokenizer_profile: TokenizerModelProfile,
}

impl Default for IntegralPromptBudgetConfig {
    fn default() -> Self {
        Self {
            max_model_context_tokens: 8192,
            reserved_response_tokens: 2048,
            max_personal_context_ratio: 0.20,
            max_history_ratio: 0.35,
            safety_margin_ratio: 0.10,
            tokenizer_profile: TokenizerModelProfile::default(),
        }
    }
}

impl IntegralPromptBudgetConfig {
    /// Retorna o total de tokens disponíveis para entrada (input tokens).
    pub fn available_input_tokens(&self) -> usize {
        self.max_model_context_tokens
            .saturating_sub(self.reserved_response_tokens)
    }

    /// Retorna o limite de caracteres de entrada considerando a taxa do modelo
    /// e a margem de segurança contra overhead de serialização.
    pub fn effective_input_char_limit(&self) -> usize {
        let input_tokens = self.available_input_tokens();
        let raw_chars = (input_tokens as f64) * self.tokenizer_profile.chars_per_token();
        let safe_chars = raw_chars * (1.0 - self.safety_margin_ratio as f64);
        safe_chars.max(500.0) as usize
    }

    /// Estima a contagem de tokens de forma conservadora a partir de uma string.
    pub fn estimate_tokens(&self, text: &str) -> usize {
        let char_count = text.chars().count();
        if char_count == 0 {
            0
        } else {
            ((char_count as f64) / self.tokenizer_profile.chars_per_token()).ceil() as usize
        }
    }
}

/// Detalhamento auditável da partição do orçamento de prompt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptBudgetAudit {
    pub essential_system_chars: usize,
    pub personality_chars: usize,
    pub user_turn_chars: usize,
    pub personal_context_chars: usize,
    pub tool_results_chars: usize,
    pub history_chars: usize,
    pub total_assembled_chars: usize,
    pub estimated_input_tokens: usize,
    pub available_input_tokens: usize,
    pub effective_char_limit: usize,
    pub is_within_budget: bool,
    pub truncated_tiers: Vec<String>,
}

/// Prompt completamente montado e governado respeitando o orçamento integral.
#[derive(Debug, Clone)]
pub struct AssembledPrompt {
    pub system_instruction: String,
    pub prompt_input: String,
    pub audit: PromptBudgetAudit,
}

/// Montador governado de prompts com respeito estrito a orçamento e prioridade ontológica.
#[derive(Debug, Clone)]
pub struct PromptBudgetManager {
    config: IntegralPromptBudgetConfig,
}

impl PromptBudgetManager {
    pub fn new(config: IntegralPromptBudgetConfig) -> Self {
        Self { config }
    }

    pub fn default_budget() -> Self {
        Self::new(IntegralPromptBudgetConfig::default())
    }

    pub fn config(&self) -> &IntegralPromptBudgetConfig {
        &self.config
    }

    /// Monta o prompt integral com truncamento progressivo ordenado por prioridade.
    pub fn assemble(
        &self,
        essential_system: &str,
        personality: &str,
        user_turn: &str,
        raw_personal_context: Option<&str>,
        raw_tool_results: Option<&str>,
        conversation_history: &[String],
    ) -> Result<AssembledPrompt, PromptBudgetError> {
        let effective_char_limit = self.config.effective_input_char_limit();
        let mut truncated_tiers = Vec::new();

        // 1. Validação Tier 1: Essential System (Constitucional e Ontológico - NUNCA truncável)
        let essential_chars = essential_system.chars().count();
        if essential_chars > effective_char_limit {
            return Err(PromptBudgetError::EssentialInstructionsExceedBudget {
                required_chars: essential_chars,
                available_chars: effective_char_limit,
            });
        }

        // 2. Validação Tier 3: Turno do Usuário (Preservação da intenção - falha fechada se não couber)
        let user_chars = user_turn.chars().count();
        let available_for_user = effective_char_limit.saturating_sub(essential_chars);
        if user_chars > available_for_user {
            return Err(PromptBudgetError::UserTurnExceedsBudget {
                required_chars: user_chars,
                available_chars: available_for_user,
            });
        }

        // 3. Tier 2: Personalidade
        let personality_chars = personality.chars().count();
        let required_base = essential_chars + user_chars + personality_chars;
        if required_base > effective_char_limit {
            // Se personalidade for excessiva, truncamos aviso mantendo núcleo
            truncated_tiers.push("Tier2Personality".to_string());
        }

        let mut budget_left = effective_char_limit.saturating_sub(required_base);

        // 4. Processa Contexto Pessoal (Tier 6 - Cota percentual configurável)
        let max_personal_chars =
            ((effective_char_limit as f32) * self.config.max_personal_context_ratio) as usize;
        let mut final_personal_context = String::new();
        if let Some(pc) = raw_personal_context {
            if !pc.trim().is_empty() {
                let allowed = max_personal_chars.min(budget_left);
                let pc_len = pc.chars().count();
                if pc_len > allowed {
                    let notice_len = NOTICE_PERSONAL_CONTEXT_TRUNCATED.chars().count();
                    if allowed > notice_len {
                        let take_chars = allowed - notice_len;
                        let truncated: String = pc.chars().take(take_chars).collect();
                        final_personal_context =
                            format!("{}{}", truncated, NOTICE_PERSONAL_CONTEXT_TRUNCATED);
                    }
                    truncated_tiers.push("Tier6PersonalContext".to_string());
                } else {
                    final_personal_context = pc.to_string();
                }
                budget_left = budget_left.saturating_sub(final_personal_context.chars().count());
            }
        }

        // 5. Processa Resultados de Ferramentas (Tier 4)
        let mut final_tool_results = String::new();
        if let Some(tr) = raw_tool_results {
            if !tr.trim().is_empty() {
                let tr_chars = tr.chars().count();
                if tr_chars > budget_left {
                    let notice_len = NOTICE_TOOL_RESULTS_TRUNCATED.chars().count();
                    if budget_left > notice_len {
                        let take_chars = budget_left - notice_len;
                        let truncated: String = tr.chars().take(take_chars).collect();
                        final_tool_results =
                            format!("{}{}", truncated, NOTICE_TOOL_RESULTS_TRUNCATED);
                    }
                    truncated_tiers.push("Tier4ToolResults".to_string());
                } else {
                    final_tool_results = tr.to_string();
                }
                budget_left = budget_left.saturating_sub(final_tool_results.chars().count());
            }
        }

        // 6. Processa Histórico Conversacional (Tier 5 - Janela deslizante mais recente primeiro)
        let max_history_chars =
            ((effective_char_limit as f32) * self.config.max_history_ratio) as usize;
        let allowed_history_chars = max_history_chars.min(budget_left);
        let mut history_entries = Vec::new();
        let mut current_history_chars = 0;

        for entry in conversation_history.iter().rev() {
            let entry_len = entry.chars().count();
            if current_history_chars + entry_len <= allowed_history_chars {
                history_entries.push(entry.clone());
                current_history_chars += entry_len;
            } else {
                truncated_tiers.push("Tier5ConversationHistory".to_string());
                break;
            }
        }
        history_entries.reverse();
        let final_history = history_entries.join("\n\n");

        // 7. Montagem estruturada dos blocos de saída
        let full_system_instruction = if personality.trim().is_empty() {
            essential_system.to_string()
        } else {
            format!("{}\n\n{}", personality, essential_system)
        };

        let mut user_prompt_parts = Vec::new();
        if !final_personal_context.is_empty() {
            user_prompt_parts.push(final_personal_context.clone());
        }
        if !final_history.is_empty() {
            user_prompt_parts.push(format!("### HISTÓRICO RECENTE:\n{}", final_history));
        }
        if !final_tool_results.is_empty() {
            user_prompt_parts.push(format!("### DADOS DE FERRAMENTAS:\n{}", final_tool_results));
        }
        user_prompt_parts.push(user_turn.to_string());

        let final_prompt_input = user_prompt_parts.join("\n\n");

        let total_assembled_chars =
            full_system_instruction.chars().count() + final_prompt_input.chars().count();

        // Verificação final rigorosa: falha fechada se total assembled estourar
        if total_assembled_chars > effective_char_limit {
            return Err(PromptBudgetError::BudgetExhausted(format!(
                "Prompt final montado ({} chars) excedeu limite efetivo ({} chars)",
                total_assembled_chars, effective_char_limit
            )));
        }

        let estimated_input_tokens = self.config.estimate_tokens(&format!(
            "{}\n{}",
            full_system_instruction, final_prompt_input
        ));

        let audit = PromptBudgetAudit {
            essential_system_chars: essential_chars,
            personality_chars,
            user_turn_chars: user_chars,
            personal_context_chars: final_personal_context.chars().count(),
            tool_results_chars: final_tool_results.chars().count(),
            history_chars: final_history.chars().count(),
            total_assembled_chars,
            estimated_input_tokens,
            available_input_tokens: self.config.available_input_tokens(),
            effective_char_limit,
            is_within_budget: true,
            truncated_tiers,
        };

        Ok(AssembledPrompt {
            system_instruction: full_system_instruction,
            prompt_input: final_prompt_input,
            audit,
        })
    }
}
