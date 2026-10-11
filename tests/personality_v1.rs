use yuki::personality::adapter::{ContextUrgency, PersonalityPromptAdapter};
use yuki::personality::identity::YUKI_CORE_IDENTITY_DIRECTIVES;
use yuki::personality::profile::{ConversationalStyle, PersonalityProfile};

#[test]
fn test_personality_profile_default_invariants() {
    let default_prof = PersonalityProfile::yuki_default();

    assert_eq!(default_prof.name, "Yuki Default");
    assert_eq!(
        default_prof.conversational_style,
        ConversationalStyle::Collaborative
    );
    assert!((0.0..=1.0).contains(&default_prof.warmth));
    assert!((0.0..=1.0).contains(&default_prof.formality));
    assert!((0.0..=1.0).contains(&default_prof.humor));
    assert!((0.0..=1.0).contains(&default_prof.expressiveness));
    assert!((0.0..=1.0).contains(&default_prof.proactivity));
    assert!((0.0..=1.0).contains(&default_prof.explanation_depth));

    // Testar que o clamp protege contra limites inválidos
    let custom = PersonalityProfile::new(
        "Custom Out of Bounds",
        1.5,
        -0.5,
        2.0,
        -1.0,
        1.2,
        -0.1,
        ConversationalStyle::Direct,
    );
    assert_eq!(custom.warmth, 1.0);
    assert_eq!(custom.formality, 0.0);
    assert_eq!(custom.humor, 1.0);
    assert_eq!(custom.expressiveness, 0.0);
    assert_eq!(custom.proactivity, 1.0);
    assert_eq!(custom.explanation_depth, 0.0);
}

#[test]
fn test_personality_prompt_generation_3_tiers() {
    let profile = PersonalityProfile::yuki_default();
    let prompt =
        PersonalityPromptAdapter::render_system_instructions(&profile, ContextUrgency::Standard);

    // Camada 1: Identidade Estável e salvaguardas ontológicas
    assert!(prompt.contains("IDENTIDADE FUNDAMENTAL DA YUKI"));
    for directive in YUKI_CORE_IDENTITY_DIRECTIVES {
        assert!(prompt.contains(directive));
    }
    assert!(prompt.contains("Nunca finja ter sentimentos humanos"));

    // Camada 2: Perfil de Personalidade
    assert!(prompt.contains("PERFIL DE PERSONALIDADE: Yuki Default"));
    assert!(prompt.contains("Estilo colaborativo"));
    assert!(prompt.contains("Calor e empatia"));

    // Camada 3: Adaptação Contextual
    assert!(prompt.contains("MODO CONTEXTUAL: PADRÃO"));
}

#[test]
fn test_contextual_urgency_inference() {
    let sec_input = "Ocorreu um erro crítico de segurança e acesso negado!";
    assert_eq!(
        ContextUrgency::infer_from_input(sec_input),
        ContextUrgency::SecurityCritical
    );

    let tech_input = "Como implementar essa trait em Rust com async tokio?";
    assert_eq!(
        ContextUrgency::infer_from_input(tech_input),
        ContextUrgency::TechnicalDeep
    );

    let creative_input = "Vamos fazer um brainstorm de design para a nova funcionalidade";
    assert_eq!(
        ContextUrgency::infer_from_input(creative_input),
        ContextUrgency::CreativeReflective
    );

    let normal_input = "Olá Yuki, como você está hoje?";
    assert_eq!(
        ContextUrgency::infer_from_input(normal_input),
        ContextUrgency::Standard
    );
}
