use std::sync::Arc;
use yuki::context::builder::ContextBuilder;
use yuki::context::personal_context::{
    OperatorProfile, PersonalContextBuilder, PersonalContextConfig,
};
use yuki::contracts::context::ContextRequest;
use yuki::contracts::memory::{
    MemoryConfidence, MemoryRecord, MemoryRepository, MemorySource, MemoryType,
};
use yuki::memory::retriever::DefaultMemoryRetriever;
use yuki::memory::store::SqliteMemoryStore;

#[test]
fn test_personal_context_prioritization_and_budget() {
    let store: Arc<dyn MemoryRepository> =
        Arc::new(SqliteMemoryStore::open_in_memory().expect("abrir store"));

    // Adicionar memórias de vários tipos
    let inst = MemoryRecord::new(
        MemoryType::Instruction,
        MemorySource::UserDeclared,
        MemoryConfidence::Verified,
        "Instrução Primária",
        "Nunca gere comandos destrutivos sem confirmação explícita do operador.",
        vec!["safety".to_string()],
    )
    .unwrap();
    store.store(&inst).unwrap();

    let pref = MemoryRecord::new(
        MemoryType::Preference,
        MemorySource::UserDeclared,
        MemoryConfidence::High,
        "Preferência de Linguagem",
        "Responda preferencialmente em português brasileiro.",
        vec!["language".to_string()],
    )
    .unwrap();
    store.store(&pref).unwrap();

    let fact = MemoryRecord::new(
        MemoryType::Fact,
        MemorySource::UserDeclared,
        MemoryConfidence::High,
        "Projeto Atual",
        "O operador está focado no desenvolvimento do Marco 7 da Yuki.",
        vec!["yuki".to_string(), "milestone".to_string()],
    )
    .unwrap();
    store.store(&fact).unwrap();

    let ep = MemoryRecord::new(
        MemoryType::Episode,
        MemorySource::UserDeclared,
        MemoryConfidence::Medium,
        "Sessão Anterior",
        "Concluída a auditoria de segurança dos Marcos 1 a 6.",
        vec!["audit".to_string()],
    )
    .unwrap();
    store.store(&ep).unwrap();

    let retriever = Arc::new(DefaultMemoryRetriever::new(Arc::clone(&store)));
    let operator = OperatorProfile {
        name: "José".to_string(),
        preferred_language: "pt-BR".to_string(),
        timezone: Some("America/Sao_Paulo".to_string()),
    };

    let builder = PersonalContextBuilder::new(retriever)
        .with_operator(operator)
        .with_config(PersonalContextConfig {
            max_chars: 1000,
            max_items: 4,
            include_episodes: true,
        });

    let pctx = builder
        .build("Como está o Marco 7?")
        .expect("construir contexto");

    assert_eq!(pctx.operator.name, "José");
    assert_eq!(pctx.instructions.len(), 1);
    assert_eq!(pctx.preferences.len(), 1);
    assert_eq!(pctx.facts.len(), 1);
    assert_eq!(pctx.episodes.len(), 1);
    assert_eq!(pctx.referenced_memory_ids.len(), 4);

    let prompt = pctx.to_prompt_section();
    assert!(prompt.contains("OPERADOR: José"));
    assert!(prompt.contains("DIRETRIZES E INSTRUÇÕES FIXAS DO USUÁRIO"));
    assert!(prompt.contains("PREFERÊNCIAS DECLARADAS PELO USUÁRIO"));
    assert!(prompt.contains("CONTEXTO E FATOS RELEVANTES"));
    assert!(prompt.contains("HISTÓRICO RECENTE"));
}

#[test]
fn test_context_builder_integration_with_personal_context() {
    let store: Arc<dyn MemoryRepository> =
        Arc::new(SqliteMemoryStore::open_in_memory().expect("abrir store"));

    let inst = MemoryRecord::new(
        MemoryType::Instruction,
        MemorySource::UserDeclared,
        MemoryConfidence::Verified,
        "Clareza",
        "Seja direto e objetivo.",
        vec![],
    )
    .unwrap();
    store.store(&inst).unwrap();

    let retriever = Arc::new(DefaultMemoryRetriever::new(Arc::clone(&store)));
    let builder = PersonalContextBuilder::new(retriever);
    let pctx = builder.build("Olá Yuki").expect("build pctx");

    let req = ContextRequest {
        purpose: "user_turn".to_string(),
        user_input: "Olá Yuki".to_string(),
        source: "user".to_string(),
    };

    let ctx = ContextBuilder::build_with_personal_context(&req, Some(pctx))
        .expect("build context com personal");

    assert!(ctx.relevant_data.get("personal_context").is_some());
    assert!(ctx.relevant_data.get("personal_prompt_section").is_some());
    assert!(ctx
        .provenance
        .transformations
        .contains(&"personal_context_enrichment".to_string()));
}
