use crate::personality::identity::get_core_identity_prompt;
use crate::personality::profile::PersonalityProfile;

/// Nível de urgência e modo contextual da interação (Camada 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextUrgency {
    /// Diálogo cotidiano padrão com o operador.
    Standard,
    /// Resolução técnica profunda, análise de código ou arquitetura.
    TechnicalDeep,
    /// Tratamento de erro de segurança, incidente ou ação crítica.
    SecurityCritical,
    /// Sessão criativa, design de soluções ou reflexão exploratória.
    CreativeReflective,
}

impl ContextUrgency {
    pub fn to_directive(&self) -> &'static str {
        match self {
            ContextUrgency::Standard => {
                "### MODO CONTEXTUAL: PADRÃO\n- Equilíbrio entre calor humano, clareza técnica e presteza."
            }
            ContextUrgency::TechnicalDeep => {
                "### MODO CONTEXTUAL: TÉCNICO PROFUNDO\n- Priorize precisão, trechos de código idiomáticos em Rust, menção a contratos e clareza de tipos."
            }
            ContextUrgency::SecurityCritical => {
                "### MODO CONTEXTUAL: SEGURANÇA CRÍTICA\n- Postura sóbria, concisa e direta. Zero floreios ou humor. Explique claramente o motivo da negação ou incidente com base em políticas Zero Trust."
            }
            ContextUrgency::CreativeReflective => {
                "### MODO CONTEXTUAL: CRIATIVO / EXPLORATÓRIO\n- Seja expansiva, encorajadora, sugerindo alternativas arquiteturais e pontos de melhoria construtivos."
            }
        }
    }

    /// Infere o modo contextual a partir da intenção e termos do input do usuário.
    pub fn infer_from_input(input: &str) -> Self {
        let lower = input.to_lowercase();

        if lower.contains("erro")
            || lower.contains("falha")
            || lower.contains("incidente")
            || lower.contains("negado")
            || lower.contains("vulnerabilidade")
            || lower.contains("ataque")
            || lower.contains("violação")
            || lower.contains("emergency")
        {
            ContextUrgency::SecurityCritical
        } else if lower.contains("código")
            || lower.contains("rust")
            || lower.contains("função")
            || lower.contains("trait")
            || lower.contains("struct")
            || lower.contains("compilar")
            || lower.contains("debug")
            || lower.contains("refactor")
            || lower.contains("benchmark")
        {
            ContextUrgency::TechnicalDeep
        } else if lower.contains("ideia")
            || lower.contains("conceito")
            || lower.contains("brainstorm")
            || lower.contains("sugestão")
            || lower.contains("futuro")
            || lower.contains("design")
        {
            ContextUrgency::CreativeReflective
        } else {
            ContextUrgency::Standard
        }
    }
}

/// Adaptador completo de personalidade que sintetiza as 3 camadas em um prompt governado.
pub struct PersonalityPromptAdapter;

impl PersonalityPromptAdapter {
    /// Renderiza as instruções de sistema consolidadas da personalidade Yuki.
    pub fn render_system_instructions(
        profile: &PersonalityProfile,
        urgency: ContextUrgency,
    ) -> String {
        let core = get_core_identity_prompt();
        let prof = profile.to_prompt_guidelines();
        let mode = urgency.to_directive();

        format!("{}\n\n{}\n\n{}", core, prof, mode)
    }

    /// Helper que infere a urgência do input do usuário e renderiza o prompt.
    pub fn render_for_user_input(profile: &PersonalityProfile, user_input: &str) -> String {
        let urgency = ContextUrgency::infer_from_input(user_input);
        Self::render_system_instructions(profile, urgency)
    }
}
