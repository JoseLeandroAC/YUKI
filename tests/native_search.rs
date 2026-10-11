use rusqlite::Connection;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use yuki::capabilities::native_search::{
    initialize_search_schema, LocalDocumentIndexer, LocalSearchEngine, NativeSearchProvider,
};
use yuki::capabilities::research::provider::SearchProvider;
use yuki::contracts::identifiers::generate_unique_id;
use yuki::contracts::research::{ResearchSearchInput, SourceKind};

fn test_temp_dir(name: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!(
        "yuki_test_native_search_{}_{}",
        name,
        generate_unique_id("ns")
    ));
    fs::create_dir_all(&p).expect("criar pasta de teste");
    p
}

fn setup_test_db() -> Arc<Mutex<Connection>> {
    let conn = Connection::open_in_memory().expect("abrir sqlite in memory");
    initialize_search_schema(&conn).expect("inicializar schema search");
    Arc::new(Mutex::new(conn))
}

#[test]
fn test_native_search_initial_indexing() {
    let temp_dir = test_temp_dir("init");

    // Criar arquivos de teste
    fs::write(
        temp_dir.join("README.md"),
        "# Yuki Platform\n\nPlataforma de inteligência artificial pessoal governada e segura.\n\n## Arquitetura\n\nBaseada em micro-kernels determinísticos e contratos auditáveis.",
    )
    .expect("escrever README");

    fs::write(
        temp_dir.join("main.rs"),
        "// Ponto de entrada\nfn main() {\n    println!(\"Inicializando Yuki Core...\");\n}\n",
    )
    .expect("escrever main.rs");

    fs::write(
        temp_dir.join("notes.txt"),
        "Anotações do operador sobre configuração do modelo Gemini e Brave Search.",
    )
    .expect("escrever notes.txt");

    let db = setup_test_db();
    let indexer = LocalDocumentIndexer::new(&temp_dir).expect("criar indexador");

    let report = {
        let mut conn = db.lock().unwrap();
        indexer
            .index_workspace(&mut conn)
            .expect("indexar workspace")
    };

    assert_eq!(report.indexed_count, 3);
    assert_eq!(report.skipped_count, 0);
    assert_eq!(report.deleted_count, 0);
    assert!(report.total_chunks >= 3);

    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn test_native_search_bm25_queries() {
    let temp_dir = test_temp_dir("bm25");

    fs::write(
        temp_dir.join("doc_rust.md"),
        "# Rust e Concorrência\n\nRust garante segurança de memória sem garbage collector através do ownership e borrow checker.",
    )
    .expect("escrever doc");

    fs::write(
        temp_dir.join("doc_sqlite.md"),
        "# Banco de Dados Local\n\nO SQLite FTS5 provê indexação full-text eficiente com ranking BM25 e tokenizador unicode61.",
    )
    .expect("escrever doc");

    let db = setup_test_db();
    let indexer = LocalDocumentIndexer::new(&temp_dir).expect("criar indexador");

    {
        let mut conn = db.lock().unwrap();
        indexer.index_workspace(&mut conn).expect("indexar");
    }

    let search_engine = LocalSearchEngine::new(Arc::clone(&db));

    // 1. Busca por termo específico do SQLite
    let results = search_engine.search("FTS5 BM25", 5).expect("buscar FTS5");
    assert!(!results.is_empty());
    assert_eq!(results[0].title, "doc_sqlite.md");
    assert!(results[0].content_text.contains("SQLite FTS5"));

    // 2. Busca por termo do Rust
    let rust_results = search_engine
        .search("ownership borrow checker", 5)
        .expect("buscar Rust");
    assert!(!rust_results.is_empty());
    assert_eq!(rust_results[0].title, "doc_rust.md");

    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn test_native_search_delta_updates() {
    let temp_dir = test_temp_dir("delta");

    let file_path = temp_dir.join("spec.md");
    fs::write(
        &file_path,
        "# Especificação v1\n\nTexto original da especificação.",
    )
    .expect("escrever spec");

    let db = setup_test_db();
    let indexer = LocalDocumentIndexer::new(&temp_dir).expect("criar indexador");

    // Primeira indexação: 1 indexado
    let r1 = {
        let mut conn = db.lock().unwrap();
        indexer.index_workspace(&mut conn).expect("r1")
    };
    assert_eq!(r1.indexed_count, 1);
    assert_eq!(r1.skipped_count, 0);

    // Segunda indexação sem alterações: 1 pulado (delta skip)
    let r2 = {
        let mut conn = db.lock().unwrap();
        indexer.index_workspace(&mut conn).expect("r2")
    };
    assert_eq!(r2.indexed_count, 0);
    assert_eq!(r2.skipped_count, 1);

    // Modificar arquivo
    fs::write(
        &file_path,
        "# Especificação v2\n\nTexto atualizado com novos parâmetros de segurança.",
    )
    .expect("atualizar spec");

    // Terceira indexação: detecta hash divergente e atualiza
    let r3 = {
        let mut conn = db.lock().unwrap();
        indexer.index_workspace(&mut conn).expect("r3")
    };
    assert_eq!(r3.indexed_count, 1);
    assert_eq!(r3.skipped_count, 0);

    // Verificar que o novo conteúdo é pesquisável
    let search_engine = LocalSearchEngine::new(Arc::clone(&db));
    let hits = search_engine
        .search("segurança", 5)
        .expect("buscar atualizado");
    assert!(!hits.is_empty());
    assert!(hits[0].content_text.contains("Especificação v2"));

    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn test_native_search_file_deletion() {
    let temp_dir = test_temp_dir("delete");

    let file1 = temp_dir.join("keep.md");
    let file2 = temp_dir.join("ephemeral.md");

    fs::write(&file1, "# Documento Permanente\n\nConteúdo durável.").expect("file1");
    fs::write(
        &file2,
        "# Documento Temporário\n\nTermoSuperSecretoUnico para deletar.",
    )
    .expect("file2");

    let db = setup_test_db();
    let indexer = LocalDocumentIndexer::new(&temp_dir).expect("criar indexador");

    {
        let mut conn = db.lock().unwrap();
        indexer.index_workspace(&mut conn).expect("indexar inicial");
    }

    let search_engine = LocalSearchEngine::new(Arc::clone(&db));
    let before_hits = search_engine.search("TermoSuperSecretoUnico", 5).unwrap();
    assert_eq!(before_hits.len(), 1);

    // Deletar arquivo 2 do disco
    fs::remove_file(&file2).expect("remover file2");

    // Reindexar -> deve sincronizar a deleção
    let report = {
        let mut conn = db.lock().unwrap();
        indexer.index_workspace(&mut conn).expect("reindexar")
    };
    assert_eq!(report.deleted_count, 1);

    // Buscar termo do arquivo deletado -> 0 resultados
    let after_hits = search_engine.search("TermoSuperSecretoUnico", 5).unwrap();
    assert_eq!(after_hits.len(), 0);

    // Arquivo permanente ainda deve existir
    let keep_hits = search_engine.search("Permanente", 5).unwrap();
    assert_eq!(keep_hits.len(), 1);

    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn test_native_search_secret_file_filtering() {
    let temp_dir = test_temp_dir("secrets");

    // Arquivo legítimo
    fs::write(
        temp_dir.join("guide.md"),
        "# Guia do Usuário\n\nInstruções de instalação.",
    )
    .unwrap();

    // Arquivos e pastas que DEVEM ser filtrados
    fs::write(temp_dir.join(".env"), "SECRET_KEY=123456").unwrap();
    fs::write(temp_dir.join(".env.production"), "TOKEN=abcdef").unwrap();
    fs::write(temp_dir.join("api_secret_token.txt"), "api_token_value").unwrap();
    fs::write(temp_dir.join("cert.key"), "PRIVATE KEY DATA").unwrap();
    fs::write(temp_dir.join("id_rsa"), "OPENSSH PRIVATE KEY").unwrap();

    let target_dir = temp_dir.join("target");
    fs::create_dir_all(&target_dir).unwrap();
    fs::write(target_dir.join("compiled.rs"), "fn compiled() {}").unwrap();

    let db = setup_test_db();
    let indexer = LocalDocumentIndexer::new(&temp_dir).expect("criar indexador");

    let report = {
        let mut conn = db.lock().unwrap();
        indexer.index_workspace(&mut conn).expect("indexar")
    };

    // Apenas `guide.md` deve ser indexado!
    assert_eq!(report.indexed_count, 1);

    let search_engine = LocalSearchEngine::new(Arc::clone(&db));
    assert_eq!(search_engine.search("SECRET_KEY", 5).unwrap().len(), 0);
    assert_eq!(search_engine.search("api_token_value", 5).unwrap().len(), 0);
    assert_eq!(search_engine.search("compiled", 5).unwrap().len(), 0);

    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn test_native_search_unicode_portuguese_support() {
    let temp_dir = test_temp_dir("pt_br");

    fs::write(
        temp_dir.join("modulo.md"),
        "# Módulo de Síntese\n\nEste módulo realiza a síntese de evidências e validação de afirmações com pontuação acentuada.",
    )
    .expect("escrever pt-br");

    let db = setup_test_db();
    let indexer = LocalDocumentIndexer::new(&temp_dir).expect("criar indexador");

    {
        let mut conn = db.lock().unwrap();
        indexer.index_workspace(&mut conn).expect("indexar");
    }

    let search_engine = LocalSearchEngine::new(Arc::clone(&db));

    // Busca exata com acento
    let hits1 = search_engine
        .search("síntese", 5)
        .expect("busca com acento");
    assert!(!hits1.is_empty());
    assert_eq!(hits1[0].title, "modulo.md");

    // Tokenizador unicode61 remove_diacritics 2 permite busca sem acento correspondendo termo acentuado
    let hits2 = search_engine
        .search("sintese", 5)
        .expect("busca sem acento");
    assert!(!hits2.is_empty());
    assert_eq!(hits2[0].title, "modulo.md");

    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn test_native_search_provider_zero_network() {
    let temp_dir = test_temp_dir("provider");

    fs::write(
        temp_dir.join("adr021.md"),
        "# ADR-021 Yuki Native Search\n\nDefinição da arquitetura soberana local para recuperação híbrida.",
    )
    .expect("escrever adr");

    let db = setup_test_db();
    let indexer = LocalDocumentIndexer::new(&temp_dir).expect("criar indexador");

    {
        let mut conn = db.lock().unwrap();
        indexer.index_workspace(&mut conn).expect("indexar");
    }

    let engine = Arc::new(LocalSearchEngine::new(Arc::clone(&db)));
    let provider = NativeSearchProvider::new(engine);

    // Invariante: Zero network
    assert!(!provider.is_live());

    let input = ResearchSearchInput::new("arquitetura híbrida");

    let result = provider.search(&input).expect("busca governada nativa");
    assert_eq!(result.provider, "NativeSearchProvider");
    assert_eq!(result.results_count, 1);
    assert_eq!(
        result.results[0].confidence_state,
        SourceKind::LocalWorkspace
    );
    assert_eq!(result.results[0].domain, "local.workspace");
    assert!(result.results[0]
        .title
        .contains("ADR-021 Yuki Native Search"));
    assert!(result.results[0].url.starts_with("file://"));

    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn test_native_search_path_traversal_defense() {
    let temp_dir = test_temp_dir("traversal");
    let outside_dir = test_temp_dir("outside");
    fs::write(
        outside_dir.join("outside_secret.txt"),
        "Outside Secret Data",
    )
    .unwrap();

    // Tentar criar indexador apontando para caminho não-diretório ou inexistente
    let non_existent = temp_dir.join("does_not_exist_subfolder");
    let err = LocalDocumentIndexer::new(&non_existent);
    assert!(err.is_err());

    let indexer = LocalDocumentIndexer::new(&temp_dir).expect("criar indexador");
    let db = setup_test_db();

    {
        let mut conn = db.lock().unwrap();
        let report = indexer.index_workspace(&mut conn).expect("indexar vazio");
        assert_eq!(report.indexed_count, 0);
    }

    let search_engine = LocalSearchEngine::new(Arc::clone(&db));
    let hits = search_engine.search("Outside Secret Data", 5).unwrap();
    assert_eq!(hits.len(), 0);

    let _ = fs::remove_dir_all(temp_dir);
    let _ = fs::remove_dir_all(outside_dir);
}
