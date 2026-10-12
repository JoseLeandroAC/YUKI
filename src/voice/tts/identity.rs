use crate::contracts::voice::VoiceConfiguration;
use crate::personality::adapter::ContextUrgency;
use crate::personality::profile::PersonalityProfile;
use serde::{Deserialize, Serialize};

/// Estilo comunicativo vocal canônico da Yuki.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VocalTone {
    /// Caloroso, competente, claro e acolhedor (padrão conversacional).
    WarmAndCompetent,
    /// Técnico, conciso, de alta precisão e sem rodeios (missões críticas ou segurança).
    PreciseAndDirect,
    /// Calmo e didático (explicações e revisões de documentação).
    CalmAndExplanatory,
}

/// Identidade vocal que governa os parâmetros expressivos da síntese de voz da Yuki.
///
/// INVARIANTE ONTOLÓGICO:
/// A voz da Yuki transmite calor e acolhimento humano na forma comunicativa, mas
/// NUNCA finge experiências biológicas, dores físicas, respirações simuladas ou senciência.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceIdentity {
    pub name: String,
    pub gender: String,
    pub language: String,
    pub tone: VocalTone,
    pub avoid_biological_feigning: bool,
}

impl Default for VoiceIdentity {
    fn default() -> Self {
        Self {
            name: "Yuki Voice v1".to_string(),
            gender: "feminine".to_string(),
            language: "pt-BR".to_string(),
            tone: VocalTone::WarmAndCompetent,
            avoid_biological_feigning: true,
        }
    }
}

impl VoiceIdentity {
    /// Adapta os parâmetros técnicos de configuração de voz com base no perfil de personalidade da Yuki
    /// e na urgência contextual do turno.
    pub fn adapt_configuration(
        &self,
        base_config: &VoiceConfiguration,
        profile: Option<&PersonalityProfile>,
        urgency: Option<&ContextUrgency>,
    ) -> VoiceConfiguration {
        let mut adapted = base_config.clone();

        // 1. Adaptação por urgência contextual
        match urgency.unwrap_or(&ContextUrgency::Standard) {
            ContextUrgency::CreativeReflective => {
                // Conversa reflexiva: ritmo ligeiramente mais cadenciado e expressivo
                adapted.speaking_rate = 0.95;
                adapted.pitch = 1.02;
            }
            ContextUrgency::Standard => {
                adapted.speaking_rate = 1.0;
                adapted.pitch = 1.0;
            }
            ContextUrgency::TechnicalDeep | ContextUrgency::SecurityCritical => {
                // Técnico / Segurança crítica: voz clara, precisa, ritmo mais ágil e firme
                adapted.speaking_rate = 1.08;
                adapted.pitch = 0.98; // Ligeiramente mais grave/firme para clareza
            }
        }

        // 2. Modulação por perfil de personalidade (se fornecido)
        if let Some(prof) = profile {
            // Warmth (0.0 a 1.0): modula sutilmente pitch e cadência
            let warmth = prof.warmth.clamp(0.0, 1.0);
            adapted.pitch *= 0.95 + (warmth * 0.1); // Faixa [0.95, 1.05]

            // Expressiveness: modula taxa de fala
            let expr = prof.expressiveness.clamp(0.0, 1.0);
            adapted.speaking_rate *= 0.97 + (expr * 0.06); // Faixa [0.97, 1.03]
        }

        adapted
    }
}
