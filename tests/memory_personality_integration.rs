use std::sync::Arc;
use yuki::contracts::events::EventType;
use yuki::contracts::identifiers::{CausationId, CorrelationId};
use yuki::contracts::input::UserInput;
use yuki::contracts::memory::{
    MemoryCandidate, MemoryConfidence, MemoryPolicy, MemoryQuery, MemoryRepository, MemorySource,
    MemoryType,
};
use yuki::contracts::output::ResultStatus;
use yuki::core::yuki_core::YukiCore;
use yuki::memory::store::SqliteMemoryStore;
use yuki::personality::profile::{ConversationalStyle, PersonalityProfile};

#[test]
fn test_yukicore_with_personality_and_memory_pipeline() {
    let memory_store = Arc::new(SqliteMemoryStore::open_in_memory().expect("sqlite in-memory"));

    let profile = PersonalityProfile::new(
        "Yuki Precision",
        0.8,
        0.3,
        0.2,
        0.4,
        0.7,
        0.9,
        ConversationalStyle::Direct,
    );

    let core = YukiCore::new()
        .with_memory(memory_store.clone(), Some(MemoryPolicy::default()))
        .with_personality(profile);

    let input = UserInput::new("Olá Yuki, como vai você?");
    let result = core.process_input(input).expect("process input");

    assert_eq!(result.status, ResultStatus::Success);

    let events = core.event_store.get_events(&result.correlation_id);
    let event_types: Vec<EventType> = events.iter().map(|e| e.event_type.clone()).collect();

    assert!(
        event_types.contains(&EventType::PersonalityApplied),
        "Deve emitir PersonalityApplied no fluxo auditado"
    );

    let personality_event = events
        .iter()
        .find(|e| matches!(e.event_type, EventType::PersonalityApplied))
        .expect("personality event");

    assert_eq!(personality_event.payload["profile_name"], "Yuki Precision");
    assert_eq!(personality_event.payload["conversational_style"], "Direct");
}

#[test]
fn test_user_declared_memory_extraction_and_lifecycle() {
    let memory_store = Arc::new(SqliteMemoryStore::open_in_memory().expect("sqlite in-memory"));

    let core = YukiCore::new().with_memory(memory_store.clone(), Some(MemoryPolicy::default()));

    // 1. Enviar comando declarativo de memorização
    let input1 = UserInput::new("Lembre-se de que eu prefiro respostas concisas em Rust");
    let res1 = core.process_input(input1).expect("process first turn");
    assert_eq!(res1.status, ResultStatus::Success);

    // Verificar se o evento MemoryStored foi emitido
    let events1 = core.event_store.get_events(&res1.correlation_id);
    let stored_event = events1
        .iter()
        .find(|e| matches!(e.event_type, EventType::MemoryStored));
    assert!(
        stored_event.is_some(),
        "Deve emitir MemoryStored quando o usuário solicita memorização explícita"
    );

    // Verificar se a memória foi efetivamente persistida no SQLite
    let all_memories = memory_store
        .query(&MemoryQuery::new().with_active_only())
        .expect("list active memories");
    assert_eq!(all_memories.len(), 1);
    assert_eq!(
        all_memories[0].content,
        "eu prefiro respostas concisas em Rust"
    );

    // 2. No turno seguinte, verificar que o contexto pessoal é recuperado e incorporado
    let input2 = UserInput::new("Como devo escrever código em Rust?");
    let res2 = core.process_input(input2).expect("process second turn");
    assert_eq!(res2.status, ResultStatus::Success);

    let events2 = core.event_store.get_events(&res2.correlation_id);
    let context_event = events2
        .iter()
        .find(|e| matches!(e.event_type, EventType::ContextBuilt))
        .expect("context built event");

    assert_eq!(
        context_event.payload["has_personal_context"], true,
        "Contexto construído deve conter has_personal_context: true"
    );
}

#[test]
fn test_adversarial_memory_rejection_fail_closed() {
    let memory_store = Arc::new(SqliteMemoryStore::open_in_memory().expect("sqlite in-memory"));

    let core = YukiCore::new().with_memory(memory_store.clone(), Some(MemoryPolicy::default()));

    // Tentativa adversária de injetar credencial / senha em memória
    let adversarial_input =
        UserInput::new("Lembre-se: minha senha mestra é secret_pass_123456789_token");
    let res = core.process_input(adversarial_input).expect("process turn");
    assert_eq!(res.status, ResultStatus::Success);

    // Nenhuma memória deve ter sido salva
    let all_memories = memory_store
        .query(&MemoryQuery::new().with_active_only())
        .expect("list active memories");
    assert!(
        all_memories.is_empty(),
        "Memória com credencial/senha deve ser rejeitada fail-closed pelo MemoryController"
    );

    // Não deve haver evento MemoryStored
    let events = core.event_store.get_events(&res.correlation_id);
    let stored_event = events
        .iter()
        .find(|e| matches!(e.event_type, EventType::MemoryStored));
    assert!(
        stored_event.is_none(),
        "Não deve registrar evento de persistência para conteúdo proibido"
    );
}

#[tokio::test]
async fn test_explicit_commit_memory_candidate_governance() {
    let memory_store = Arc::new(SqliteMemoryStore::open_in_memory().expect("sqlite in-memory"));

    let core = YukiCore::new().with_memory(memory_store.clone(), Some(MemoryPolicy::default()));

    let cid = CorrelationId::new();
    let caus_id = CausationId::new("test_causation");

    // Candidato válido
    let valid_candidate = MemoryCandidate::new(
        MemoryType::Instruction,
        MemorySource::UserDeclared,
        MemoryConfidence::High,
        "Instrução de Formato",
        "Sempre forneça exemplos comentados",
        vec!["formatting".to_string()],
        "Configuração de teste",
    );

    let committed = core
        .commit_memory_candidate(&valid_candidate, &cid, &caus_id)
        .await
        .expect("commit valid candidate");

    assert_eq!(committed.title, "Instrução de Formato");
    assert_eq!(committed.content, "Sempre forneça exemplos comentados");

    // Candidato proibido com chave de API
    let forbidden_candidate = MemoryCandidate::new(
        MemoryType::Fact,
        MemorySource::UserDeclared,
        MemoryConfidence::High,
        "API Key Secreta",
        "chave api_key=AIzaSyAbc123456789",
        vec!["secret".to_string()],
        "Tentativa proibida",
    );

    let forbidden_res = core
        .commit_memory_candidate(&forbidden_candidate, &cid, &caus_id)
        .await;
    assert!(
        forbidden_res.is_err(),
        "Deve falhar fechado ao tentar commitar candidato com secrets"
    );
}
