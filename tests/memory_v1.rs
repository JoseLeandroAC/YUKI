use std::sync::Arc;
use yuki::contracts::memory::{
    MemoryCandidate, MemoryConfidence, MemoryError, MemoryPolicy, MemoryQuery, MemoryRecord,
    MemoryRepository, MemoryRetriever, MemorySource, MemoryStatus, MemoryType,
};
use yuki::memory::policy::MemoryController;
use yuki::memory::retriever::DefaultMemoryRetriever;
use yuki::memory::store::SqliteMemoryStore;

#[test]
fn test_memory_crud_and_persistence() {
    let store = SqliteMemoryStore::open_in_memory().expect("abrir store em memória");

    // 1. Criar e armazenar
    let rec = MemoryRecord::new(
        MemoryType::Fact,
        MemorySource::UserDeclared,
        MemoryConfidence::High,
        "Nome do Operador",
        "O operador se chama José e desenvolve a plataforma Yuki em Rust.",
        vec!["profile".to_string(), "owner".to_string()],
    )
    .expect("criar registro de memória");

    let rec_id = rec.id.clone();
    store.store(&rec).expect("armazenar memória");

    // 2. Recuperar por ID
    let fetched = store
        .get_by_id(&rec_id)
        .expect("buscar por id")
        .expect("deve existir");
    assert_eq!(fetched.title, "Nome do Operador");
    assert_eq!(fetched.memory_type, MemoryType::Fact);
    assert_eq!(fetched.source, MemorySource::UserDeclared);
    assert_eq!(fetched.status, MemoryStatus::Active);

    // 3. Revogar
    store.revoke(&rec_id).expect("revogar memória");
    let revoked = store
        .get_by_id(&rec_id)
        .expect("buscar")
        .expect("deve existir");
    assert_eq!(revoked.status, MemoryStatus::Revoked);

    // 4. Arquivar
    store.archive(&rec_id).expect("arquivar memória");
    let archived = store
        .get_by_id(&rec_id)
        .expect("buscar")
        .expect("deve existir");
    assert_eq!(archived.status, MemoryStatus::Archived);

    // 5. Exclusão permanente (Direito ao Esquecimento)
    store
        .delete_permanently(&rec_id)
        .expect("deletar permanentemente");
    let deleted = store.get_by_id(&rec_id).expect("buscar");
    assert!(deleted.is_none());
}

#[test]
fn test_memory_duplicate_detection_and_superseding() {
    let store = SqliteMemoryStore::open_in_memory().expect("abrir store");

    // Inserir versão 1 de preferência
    let v1 = MemoryRecord::new(
        MemoryType::Preference,
        MemorySource::UserDeclared,
        MemoryConfidence::High,
        "Estilo de Código",
        "Prefere código idiomático em Rust com tipagem forte e sem unwrap desnecessário.",
        vec!["code".to_string()],
    )
    .expect("criar v1");
    let v1_id = v1.id.clone();
    store.store(&v1).expect("armazenar v1");

    // Inserir versão 2 da mesma preferência com o mesmo título
    let v2 = MemoryRecord::new(
        MemoryType::Preference,
        MemorySource::UserDeclared,
        MemoryConfidence::Verified,
        "Estilo de Código",
        "Prefere código idiomático em Rust 2021, Zero Trust, clippy rigoroso e zero warnings.",
        vec!["code".to_string(), "rust".to_string()],
    )
    .expect("criar v2");
    let v2_id = v2.id.clone();
    store.store(&v2).expect("armazenar v2");

    // v1 deve estar superseded_by v2
    let v1_after = store.get_by_id(&v1_id).unwrap().unwrap();
    assert_eq!(v1_after.status, MemoryStatus::Superseded);
    assert_eq!(v1_after.superseded_by, Some(v2_id.clone()));

    // v2 deve estar ativa
    let v2_after = store.get_by_id(&v2_id).unwrap().unwrap();
    assert_eq!(v2_after.status, MemoryStatus::Active);
}

#[test]
fn test_memory_fts5_text_search() {
    let store = SqliteMemoryStore::open_in_memory().expect("abrir store");

    let mem1 = MemoryRecord::new(
        MemoryType::Fact,
        MemorySource::UserDeclared,
        MemoryConfidence::High,
        "Framework Web",
        "Utiliza Axum e Tokio para serviços web e microsserviços de alta concorrência.",
        vec!["rust".to_string(), "backend".to_string()],
    )
    .unwrap();

    let mem2 = MemoryRecord::new(
        MemoryType::Fact,
        MemorySource::UserDeclared,
        MemoryConfidence::High,
        "Banco Relacional",
        "Utiliza SQLite local com WAL mode e FTS5 para armazenamento leve.",
        vec!["database".to_string(), "sqlite".to_string()],
    )
    .unwrap();

    store.store(&mem1).unwrap();
    store.store(&mem2).unwrap();

    // Busca FTS5 por 'tokio'
    let q_tokio = MemoryQuery::new().with_text("tokio").with_active_only();
    let res_tokio = store.query(&q_tokio).unwrap();
    assert_eq!(res_tokio.len(), 1);
    assert_eq!(res_tokio[0].title, "Framework Web");

    // Busca FTS5 por 'sqlite'
    let q_sqlite = MemoryQuery::new().with_text("sqlite").with_active_only();
    let res_sqlite = store.query(&q_sqlite).unwrap();
    assert_eq!(res_sqlite.len(), 1);
    assert_eq!(res_sqlite[0].title, "Banco Relacional");

    // Busca com caracteres especiais não deve quebrar
    let q_special = MemoryQuery::new()
        .with_text("tokio: \"axum\" *!")
        .with_active_only();
    let res_special = store.query(&q_special).unwrap();
    assert!(!res_special.is_empty());
}

#[test]
fn test_memory_secrets_sanitization() {
    // 1. Tentar criar memória com 'password'
    let err_pass = MemoryRecord::new(
        MemoryType::Fact,
        MemorySource::UserDeclared,
        MemoryConfidence::High,
        "Minha Senha",
        "Minha senha do banco é secret123",
        vec![],
    );
    assert!(matches!(err_pass, Err(MemoryError::ForbiddenContent(_))));

    // 2. Tentar criar memória com 'api_key'
    let err_key = MemoryRecord::new(
        MemoryType::Fact,
        MemorySource::UserDeclared,
        MemoryConfidence::High,
        "Brave Key",
        "A api_key do Brave é BSA123456789",
        vec![],
    );
    assert!(matches!(err_key, Err(MemoryError::ForbiddenContent(_))));

    // 3. Tentar criar memória referenciando vault.json
    let err_vault = MemoryRecord::new(
        MemoryType::Fact,
        MemorySource::UserDeclared,
        MemoryConfidence::High,
        "Arquivo de Segredos",
        "O cofre fica em data/vault.json",
        vec![],
    );
    assert!(matches!(err_vault, Err(MemoryError::ForbiddenContent(_))));
}

#[test]
fn test_memory_candidate_evaluation_policy() {
    let store: Arc<dyn MemoryRepository> =
        Arc::new(SqliteMemoryStore::open_in_memory().expect("abrir store"));
    let policy = MemoryPolicy {
        min_inferred_confidence: MemoryConfidence::Medium,
        max_content_bytes: 512,
        prohibit_secrets: true,
        prohibit_intimate_surveillance: true,
    };
    let controller = MemoryController::new(Arc::clone(&store), policy);

    // 1. Candidato inferido com baixa confiança -> rejeitado
    let low_cand = MemoryCandidate::new(
        MemoryType::Fact,
        MemorySource::ModelInferred,
        MemoryConfidence::Low,
        "Suposição de Feriado",
        "O usuário talvez more na França.",
        vec![],
        "Inferido a partir de menção a Paris",
    );
    let low_res = controller.evaluate_and_commit(&low_cand);
    assert!(matches!(low_res, Err(MemoryError::PolicyViolation(_))));

    // 2. Candidato contendo segredo -> rejeitado
    let secret_cand = MemoryCandidate::new(
        MemoryType::Fact,
        MemorySource::ModelInferred,
        MemoryConfidence::High,
        "Credencial Observada",
        "O bearer token retornado foi XYZ",
        vec![],
        "Extraído de resposta de API",
    );
    let sec_res = controller.evaluate_and_commit(&secret_cand);
    assert!(matches!(sec_res, Err(MemoryError::ForbiddenContent(_))));

    // 3. Candidato legítimo com confiança média -> aprovado e persistido
    let valid_cand = MemoryCandidate::new(
        MemoryType::Preference,
        MemorySource::ModelInferred,
        MemoryConfidence::Medium,
        "Preferência de Resposta",
        "Usuário valoriza explicações diretas e técnicas.",
        vec!["style".to_string()],
        "Usuário pediu 'seja direto e sem rodeios'",
    );
    let valid_res = controller
        .evaluate_and_commit(&valid_cand)
        .expect("deve aprovar e gravar");
    assert_eq!(valid_res.title, "Preferência de Resposta");

    // Verificar que existe no banco
    let fetched = store.get_by_id(&valid_res.id).unwrap().unwrap();
    assert_eq!(fetched.status, MemoryStatus::Active);
}

#[test]
fn test_memory_retriever_context_budget() {
    let store: Arc<dyn MemoryRepository> =
        Arc::new(SqliteMemoryStore::open_in_memory().expect("abrir store"));
    let retriever = DefaultMemoryRetriever::new(Arc::clone(&store));

    // Instrução prioritária
    let inst = MemoryRecord::new(
        MemoryType::Instruction,
        MemorySource::UserDeclared,
        MemoryConfidence::Verified,
        "Idioma Padrão",
        "Sempre responda em português brasileiro com tom profissional e acolhedor.",
        vec!["language".to_string()],
    )
    .unwrap();
    store.store(&inst).unwrap();

    // Preferência
    let pref = MemoryRecord::new(
        MemoryType::Preference,
        MemorySource::UserDeclared,
        MemoryConfidence::High,
        "Formato de Saída",
        "Prefere código com comentários explicativos de invariantes de segurança.",
        vec!["format".to_string()],
    )
    .unwrap();
    store.store(&pref).unwrap();

    // Fato relevante
    let fact = MemoryRecord::new(
        MemoryType::Fact,
        MemorySource::UserDeclared,
        MemoryConfidence::High,
        "Arquitetura Yuki",
        "Yuki segue Zero Trust e separação rígida de capabilities.",
        vec!["security".to_string()],
    )
    .unwrap();
    store.store(&fact).unwrap();

    // Recuperar para contexto com limite de 2 itens
    let context_memories = retriever
        .retrieve_for_context("Como estruturar segurança na Yuki?", 2)
        .expect("recuperar para contexto");

    assert_eq!(context_memories.len(), 2);
    // Instrução deve ter sido incluída por alta prioridade
    assert!(context_memories
        .iter()
        .any(|m| m.memory_type == MemoryType::Instruction));
}
