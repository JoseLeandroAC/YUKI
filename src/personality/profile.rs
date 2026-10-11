use serde::{Deserialize, Serialize};

/// Estilos conversacionais estruturados da personalidade da Yuki.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConversationalStyle {
    /// Direto, enxuto, sem preâmbulos ou rodeios.
    Direct,
    /// Analítico, metódico, orientado a dados e trade-offs.
    Analytical,
    /// Colaborativo, proativo, construtivo e em parceria.
    Collaborative,
    /// Pedagógico, explicativo, decompondo conceitos progressivamente.
    Pedagogical,
}

impl ConversationalStyle {
    pub fn description(&self) -> &'static str {
        match self {
            ConversationalStyle::Direct => {
                "Estilo direto e objetivo, priorizando respostas concisas."
            }
            ConversationalStyle::Analytical => {
                "Estilo analítico e estruturado, pesando prós, contras e invariantes."
            }
            ConversationalStyle::Collaborative => {
                "Estilo colaborativo de parceria técnica, construtivo e atencioso."
            }
            ConversationalStyle::Pedagogical => {
                "Estilo pedagógico e didático, facilitando o aprendizado com exemplos claros."
            }
        }
    }
}

/// Perfil tipado de personalidade da Yuki (Camada 2).
///
/// Todos os atributos numéricos são estritamente limitados no intervalo [0.0, 1.0].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalityProfile {
    pub name: String,
    pub warmth: f32,
    pub formality: f32,
    pub humor: f32,
    pub expressiveness: f32,
    pub proactivity: f32,
    pub explanation_depth: f32,
    pub conversational_style: ConversationalStyle,
}

impl PersonalityProfile {
    /// Construtor com validação e delimitação rigorosa de limites.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: impl Into<String>,
        warmth: f32,
        formality: f32,
        humor: f32,
        expressiveness: f32,
        proactivity: f32,
        explanation_depth: f32,
        conversational_style: ConversationalStyle,
    ) -> Self {
        Self {
            name: name.into(),
            warmth: warmth.clamp(0.0, 1.0),
            formality: formality.clamp(0.0, 1.0),
            humor: humor.clamp(0.0, 1.0),
            expressiveness: expressiveness.clamp(0.0, 1.0),
            proactivity: proactivity.clamp(0.0, 1.0),
            explanation_depth: explanation_depth.clamp(0.0, 1.0),
            conversational_style,
        }
    }

    /// Perfil oficial e canônico "Yuki Default".
    ///
    /// Características: Acolhedora (0.7), tom natural e acessível (formality 0.4),
    /// toque sutil de humor inteligente (0.3), equilibrada em concisão (0.5),
    /// proativa sem intromissão (0.6), profundidade técnica sólida (0.8),
    /// e estilo colaborativo.
    pub fn yuki_default() -> Self {
        Self::new(
            "Yuki Default",
            0.7,
            0.4,
            0.3,
            0.5,
            0.6,
            0.8,
            ConversationalStyle::Collaborative,
        )
    }

    /// Gera a descrição textual das diretrizes de tom e postura para o modelo.
    pub fn to_prompt_guidelines(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("### PERFIL DE PERSONALIDADE: {}", self.name));
        lines.push(format!(
            "- Estilo conversacional: {}",
            self.conversational_style.description()
        ));

        if self.warmth >= 0.6 {
            lines.push("- Calor e empatia: Tom acolhedor e atencioso, sem bajulação ou servilismo artificial.".to_string());
        } else {
            lines.push("- Calor e empatia: Tom neutro e reservado.".to_string());
        }

        if self.formality <= 0.5 {
            lines.push("- Formalidade: Tom profissional porém natural e conversacional, evitando jargões burocráticos desnecessários.".to_string());
        } else {
            lines.push("- Formalidade: Tom estritamente formal e cerimonioso.".to_string());
        }

        if self.humor >= 0.25 {
            lines.push("- Leveza: Permita toques sutis de humor inteligente quando o contexto permitir, sem perder a seriedade técnica.".to_string());
        } else {
            lines.push("- Leveza: Mantenha sobriedade estrita.".to_string());
        }

        if self.explanation_depth >= 0.7 {
            lines.push("- Profundidade: Explique detalhes técnicos com fundamentação sólida, mencionando arquitetura e segurança.".to_string());
        } else {
            lines.push(
                "- Profundidade: Mantenha respostas sucintas e diretas ao ponto.".to_string(),
            );
        }

        lines.join("\n")
    }
}

impl Default for PersonalityProfile {
    fn default() -> Self {
        Self::yuki_default()
    }
}
