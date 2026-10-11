// ============================================================================
// Yuki Personal AI Platform — Research v1
// Testes Offline de Content-Encoding e Descompressão Delimitada (ADR-020)
// ============================================================================

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;
use yuki::capabilities::research::{
    compress_brotli, compress_deflate, compress_gzip, ContentFetchProvider, FetchResponse,
    HttpContentFetchConfig, HttpContentFetchProvider, MockDnsResolver, MockFetchTransport,
    ObservedSourceRegistry,
};
use yuki::contracts::identifiers::TurnId;
use yuki::contracts::research::{
    compute_sha256, ResearchBudget, ResearchBudgetTracker, ResearchFetchInput, ResearchTurnContext,
    SourceKind, CURRENT_TURN_CONTEXT,
};

fn setup_test_provider(
    tracker: Option<Arc<ResearchBudgetTracker>>,
) -> (
    HttpContentFetchProvider,
    Arc<MockFetchTransport>,
    Arc<MockDnsResolver>,
    Arc<ResearchBudgetTracker>,
) {
    let budget_tracker = tracker.unwrap_or_else(|| Arc::new(ResearchBudgetTracker::default()));
    let transport = Arc::new(MockFetchTransport::new());
    let resolver = Arc::new(MockDnsResolver::new());

    // Resolve example.com para IP público aprovado
    resolver.set_host_ips(
        "example.com",
        vec![IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34))],
    );

    let config = HttpContentFetchConfig::new()
        .with_live_enabled(true) // Habilita o provider (mas o transport é Mock, 100% offline!)
        .with_budget_tracker(budget_tracker.clone())
        .with_transport(transport.clone())
        .with_resolver(resolver.clone());

    let provider = HttpContentFetchProvider::new(config);
    (provider, transport, resolver, budget_tracker)
}

fn sample_html() -> &'static str {
    r#"<!DOCTYPE html>
<html>
<head><title>Página de Validação de Encoding</title></head>
<body>
    <h1>Arquitetura Yuki Research</h1>
    <p>A Yuki suporta descompressão segura com Content-Encoding preservando a integridade ontológica.</p>
</body>
</html>"#
}

// ============================================================================
// 1. Teste de Gzip Válido
// ============================================================================

#[test]
fn test_encoding_01_gzip_valid() {
    let (provider, transport, _, _) = setup_test_provider(None);
    let original_html = sample_html();
    let gzip_bytes = compress_gzip(original_html.as_bytes());

    let mut headers = HashMap::new();
    headers.insert(
        "content-type".to_string(),
        "text/html; charset=utf-8".to_string(),
    );
    headers.insert("content-encoding".to_string(), "gzip".to_string());

    transport.set_response(
        "https://example.com/gzip-test",
        Ok(FetchResponse {
            status: 200,
            headers,
            body_chunks: vec![gzip_bytes.clone()],
        }),
    );

    let input = ResearchFetchInput::new("https://example.com/gzip-test");
    let result = provider
        .fetch(&input)
        .expect("Fetch com gzip válido deve ter sucesso");

    assert_eq!(result.http_status, 200);
    assert_eq!(result.title, "Página de Validação de Encoding");
    assert!(result.extracted_text.contains("Arquitetura Yuki Research"));
    assert_eq!(result.content_encoding.as_deref(), Some("gzip"));
    assert_eq!(result.raw_network_bytes, gzip_bytes.len());
    assert_eq!(result.decompressed_bytes, original_html.len());
    assert_eq!(result.bytes_observed, original_html.len());
    assert!(!result.truncated);
    assert_eq!(
        result.content_hash_sha256,
        compute_sha256(result.extracted_text.as_bytes())
    );
}

// ============================================================================
// 2. Teste de Deflate Válido
// ============================================================================

#[test]
fn test_encoding_02_deflate_valid() {
    let (provider, transport, _, _) = setup_test_provider(None);
    let original_html = sample_html();
    let deflate_bytes = compress_deflate(original_html.as_bytes());

    let mut headers = HashMap::new();
    headers.insert(
        "content-type".to_string(),
        "text/html; charset=utf-8".to_string(),
    );
    headers.insert("content-encoding".to_string(), "deflate".to_string());

    transport.set_response(
        "https://example.com/deflate-test",
        Ok(FetchResponse {
            status: 200,
            headers,
            body_chunks: vec![deflate_bytes.clone()],
        }),
    );

    let input = ResearchFetchInput::new("https://example.com/deflate-test");
    let result = provider
        .fetch(&input)
        .expect("Fetch com deflate válido deve ter sucesso");

    assert_eq!(result.http_status, 200);
    assert_eq!(result.title, "Página de Validação de Encoding");
    assert!(result.extracted_text.contains("Arquitetura Yuki Research"));
    assert_eq!(result.content_encoding.as_deref(), Some("deflate"));
    assert_eq!(result.raw_network_bytes, deflate_bytes.len());
    assert_eq!(result.decompressed_bytes, original_html.len());
}

// ============================================================================
// 3. Teste de Brotli (br) Válido
// ============================================================================

#[test]
fn test_encoding_03_brotli_valid() {
    let (provider, transport, _, _) = setup_test_provider(None);
    let original_html = sample_html();
    let br_bytes = compress_brotli(original_html.as_bytes());

    let mut headers = HashMap::new();
    headers.insert(
        "content-type".to_string(),
        "text/html; charset=utf-8".to_string(),
    );
    headers.insert("content-encoding".to_string(), "br".to_string());

    transport.set_response(
        "https://example.com/brotli-test",
        Ok(FetchResponse {
            status: 200,
            headers,
            body_chunks: vec![br_bytes.clone()],
        }),
    );

    let input = ResearchFetchInput::new("https://example.com/brotli-test");
    let result = provider
        .fetch(&input)
        .expect("Fetch com Brotli válido deve ter sucesso");

    assert_eq!(result.http_status, 200);
    assert_eq!(result.title, "Página de Validação de Encoding");
    assert!(result.extracted_text.contains("Arquitetura Yuki Research"));
    assert_eq!(result.content_encoding.as_deref(), Some("br"));
    assert_eq!(result.raw_network_bytes, br_bytes.len());
    assert_eq!(result.decompressed_bytes, original_html.len());
}

// ============================================================================
// 4. Teste de Identity (Sem compressão)
// ============================================================================

#[test]
fn test_encoding_04_identity_valid() {
    let (provider, transport, _, _) = setup_test_provider(None);
    let original_html = sample_html();

    let mut headers = HashMap::new();
    headers.insert(
        "content-type".to_string(),
        "text/html; charset=utf-8".to_string(),
    );
    headers.insert("content-encoding".to_string(), "identity".to_string());

    transport.set_response(
        "https://example.com/identity-test",
        Ok(FetchResponse {
            status: 200,
            headers,
            body_chunks: vec![original_html.as_bytes().to_vec()],
        }),
    );

    let input = ResearchFetchInput::new("https://example.com/identity-test");
    let result = provider
        .fetch(&input)
        .expect("Fetch com identity deve ter sucesso");

    assert_eq!(result.http_status, 200);
    assert_eq!(result.content_encoding.as_deref(), Some("identity"));
    assert_eq!(result.raw_network_bytes, original_html.len());
    assert_eq!(result.decompressed_bytes, original_html.len());
}

// ============================================================================
// 5. Teste de Content-Encoding Desconhecido (Fail-Closed)
// ============================================================================

#[test]
fn test_encoding_05_unknown_encoding_rejected_fail_closed() {
    let (provider, transport, _, _) = setup_test_provider(None);

    let mut headers = HashMap::new();
    headers.insert(
        "content-type".to_string(),
        "text/html; charset=utf-8".to_string(),
    );
    headers.insert("content-encoding".to_string(), "zstd".to_string());

    transport.set_response(
        "https://example.com/zstd-test",
        Ok(FetchResponse {
            status: 200,
            headers,
            body_chunks: vec![b"dados zstd nao suportados".to_vec()],
        }),
    );

    let input = ResearchFetchInput::new("https://example.com/zstd-test");
    let res = provider.fetch(&input);

    assert!(res.is_err(), "Encoding desconhecido deve falhar fechado");
    let err_msg = res.err().unwrap().to_string();
    assert!(
        err_msg.contains("não suportado"),
        "Mensagem deve indicar ausência de suporte: {}",
        err_msg
    );
    assert!(err_msg.contains("zstd"));
}

// ============================================================================
// 6. Teste de Gzip Corrompido
// ============================================================================

#[test]
fn test_encoding_06_corrupted_gzip_rejected() {
    let (provider, transport, _, _) = setup_test_provider(None);

    let mut headers = HashMap::new();
    headers.insert(
        "content-type".to_string(),
        "text/html; charset=utf-8".to_string(),
    );
    headers.insert("content-encoding".to_string(), "gzip".to_string());

    // Magic bytes de gzip com payload corrompido
    let corrupted_bytes = vec![0x1f, 0x8b, 0x08, 0x00, 0xde, 0xad, 0xbe, 0xef];

    transport.set_response(
        "https://example.com/corrupt-gzip",
        Ok(FetchResponse {
            status: 200,
            headers,
            body_chunks: vec![corrupted_bytes],
        }),
    );

    let input = ResearchFetchInput::new("https://example.com/corrupt-gzip");
    let res = provider.fetch(&input);

    assert!(
        res.is_err(),
        "Gzip corrompido deve retornar erro de execução"
    );
    let err = res.err().unwrap().to_string();
    assert!(
        err.contains("descomprimir stream gzip"),
        "Erro deve apontar falha de descompressão: {}",
        err
    );
}

// ============================================================================
// 7. Teste de Stream Interrompido
// ============================================================================

#[test]
fn test_encoding_07_interrupted_stream_handled() {
    let (provider, transport, _, _) = setup_test_provider(None);
    let original = sample_html();
    let gzip_bytes = compress_gzip(original.as_bytes());

    // Corta o stream pela metade
    let truncated_gzip = gzip_bytes[..gzip_bytes.len() / 2].to_vec();

    let mut headers = HashMap::new();
    headers.insert(
        "content-type".to_string(),
        "text/html; charset=utf-8".to_string(),
    );
    headers.insert("content-encoding".to_string(), "gzip".to_string());

    transport.set_response(
        "https://example.com/truncated-stream",
        Ok(FetchResponse {
            status: 200,
            headers,
            body_chunks: vec![truncated_gzip],
        }),
    );

    let input = ResearchFetchInput::new("https://example.com/truncated-stream");
    let res = provider.fetch(&input);

    assert!(res.is_err(), "Stream gzip truncado deve falhar fechado");
    let err = res.err().unwrap().to_string();
    assert!(
        err.contains("descomprimir stream gzip"),
        "Erro deve indicar descompressão incompleta: {}",
        err
    );
}

// ============================================================================
// 8. Teste de Zip Bomb (Expansão além de 1 MiB bloqueada)
// ============================================================================

#[test]
fn test_encoding_08_zip_bomb_exceeding_1mib_decompressed_limit_aborts() {
    let (provider, transport, _, _) = setup_test_provider(None);

    // Cria um payload repetitivo de 1.2 MiB que comprime para apenas ~2 KiB (Zip Bomb clássico)
    let huge_content = "A".repeat(1200 * 1024);
    let bomb_gzip = compress_gzip(huge_content.as_bytes());

    // Confirma que na rede ele é pequeno (< 20 KiB)
    assert!(bomb_gzip.len() < 20 * 1024);

    let mut headers = HashMap::new();
    headers.insert(
        "content-type".to_string(),
        "text/html; charset=utf-8".to_string(),
    );
    headers.insert("content-encoding".to_string(), "gzip".to_string());

    transport.set_response(
        "https://example.com/zip-bomb",
        Ok(FetchResponse {
            status: 200,
            headers,
            body_chunks: vec![bomb_gzip],
        }),
    );

    let input = ResearchFetchInput::new("https://example.com/zip-bomb");
    let res = provider.fetch(&input);

    assert!(
        res.is_err(),
        "Zip Bomb deve ser abortado durante a descompressão streaming"
    );
    let err = res.err().unwrap().to_string();
    assert!(
        err.contains("Zip Bomb bloqueado") || err.contains("excedeu o limite máximo"),
        "Erro deve relatar bloqueio de descompressão excessiva: {}",
        err
    );
}

// ============================================================================
// 9. Teste de Payload na Rede Acima de 256 KiB
// ============================================================================

#[test]
fn test_encoding_09_network_payload_exceeding_256kib_limit_aborts() {
    let (provider, transport, _, _) = setup_test_provider(None);

    // Cria 260 KiB de dados comprimidos
    let oversized_chunk = vec![0x1f; 260 * 1024];

    let mut headers = HashMap::new();
    headers.insert(
        "content-type".to_string(),
        "text/html; charset=utf-8".to_string(),
    );
    headers.insert("content-encoding".to_string(), "gzip".to_string());

    transport.set_response(
        "https://example.com/oversized-network",
        Ok(FetchResponse {
            status: 200,
            headers,
            body_chunks: vec![oversized_chunk],
        }),
    );

    let input = ResearchFetchInput::new("https://example.com/oversized-network");
    let res = provider.fetch(&input);

    assert!(res.is_err(), "Payload na rede > 256 KiB deve falhar");
    let err = res.err().unwrap().to_string();
    assert!(
        err.contains("Resposta comprimida excedeu o limite"),
        "Erro deve indicar limite de rede: {}",
        err
    );
}

// ============================================================================
// 10. Teste de HTML UTF-8 Complexo (Acentos, Cedilha e Símbolos)
// ============================================================================

#[test]
fn test_encoding_10_html_utf8_valid() {
    let (provider, transport, _, _) = setup_test_provider(None);

    let unicode_html = r#"<!DOCTYPE html>
<html>
<head><title>Título com Acentuação e Emojis: Inteligência Artificial 🚀</title></head>
<body>
    <p>Ação, comunicação, verificação e governança ontológica em língua portuguesa.</p>
</body>
</html>"#;

    let gzip_bytes = compress_gzip(unicode_html.as_bytes());

    let mut headers = HashMap::new();
    headers.insert(
        "content-type".to_string(),
        "text/html; charset=utf-8".to_string(),
    );
    headers.insert("content-encoding".to_string(), "gzip".to_string());

    transport.set_response(
        "https://example.com/unicode-test",
        Ok(FetchResponse {
            status: 200,
            headers,
            body_chunks: vec![gzip_bytes],
        }),
    );

    let input = ResearchFetchInput::new("https://example.com/unicode-test");
    let result = provider
        .fetch(&input)
        .expect("Fetch UTF-8 deve ter sucesso");

    assert_eq!(
        result.title,
        "Título com Acentuação e Emojis: Inteligência Artificial 🚀"
    );
    assert!(result
        .extracted_text
        .contains("governança ontológica em língua portuguesa"));
}

// ============================================================================
// 11. Teste de HTML com Entidades Especiais
// ============================================================================

#[test]
fn test_encoding_11_html_special_entities_and_accents() {
    let (provider, transport, _, _) = setup_test_provider(None);

    let entities_html = r#"<!DOCTYPE html>
<html>
<head><title>Entities &amp; Símbolos</title></head>
<body>
    <p>Operação &quot;Research&quot; &lt;v1&gt; &amp; Governança.</p>
</body>
</html>"#;

    let br_bytes = compress_brotli(entities_html.as_bytes());

    let mut headers = HashMap::new();
    headers.insert(
        "content-type".to_string(),
        "text/html; charset=utf-8".to_string(),
    );
    headers.insert("content-encoding".to_string(), "br".to_string());

    transport.set_response(
        "https://example.com/entities-test",
        Ok(FetchResponse {
            status: 200,
            headers,
            body_chunks: vec![br_bytes],
        }),
    );

    let input = ResearchFetchInput::new("https://example.com/entities-test");
    let result = provider
        .fetch(&input)
        .expect("Entidades devem ser sanitizadas");

    assert_eq!(result.title, "Entities & Símbolos");
    assert!(result
        .extracted_text
        .contains("Operação \"Research\" <v1> & Governança."));
}

// ============================================================================
// 12. Teste de Respostas Truncadas por Tamanho de Extração
// ============================================================================

#[test]
fn test_encoding_12_truncated_response_flag_preserved() {
    let (provider, transport, _, _) = setup_test_provider(None);

    // HTML com 2000 caracteres
    let large_body = format!(
        "<html><head><title>Longo</title></head><body>{}</body></html>",
        "Palavra ".repeat(300)
    );
    let gzip_bytes = compress_gzip(large_body.as_bytes());

    let mut headers = HashMap::new();
    headers.insert(
        "content-type".to_string(),
        "text/html; charset=utf-8".to_string(),
    );
    headers.insert("content-encoding".to_string(), "gzip".to_string());

    transport.set_response(
        "https://example.com/truncated-text",
        Ok(FetchResponse {
            status: 200,
            headers,
            body_chunks: vec![gzip_bytes],
        }),
    );

    // Solicita com teto de extração de 500 caracteres
    let input =
        ResearchFetchInput::new("https://example.com/truncated-text").with_max_length_chars(500);
    let result = provider.fetch(&input).expect("Fetch deve ter sucesso");

    assert!(
        result.truncated,
        "Flag truncated deve ser true quando texto excede limite"
    );
    assert!(result.extracted_text.len() <= 503); // 500 + "..."
    assert!(result.raw_network_bytes > 0);
    assert_eq!(result.decompressed_bytes, large_body.len());
}

// ============================================================================
// 13. Teste de Cancelamento Durante Descompressão
// ============================================================================

#[test]
fn test_encoding_13_cancellation_during_decompression() {
    let (provider, transport, _, tracker) = setup_test_provider(None);
    let gzip_bytes = compress_gzip(sample_html().as_bytes());

    let mut headers = HashMap::new();
    headers.insert(
        "content-type".to_string(),
        "text/html; charset=utf-8".to_string(),
    );
    headers.insert("content-encoding".to_string(), "gzip".to_string());

    transport.set_response(
        "https://example.com/cancel-test",
        Ok(FetchResponse {
            status: 200,
            headers,
            body_chunks: vec![gzip_bytes],
        }),
    );

    let turn_id = TurnId::new();
    // Contexto com deadline já expirada (timeout de 0 ms)
    let ctx = Arc::new(ResearchTurnContext::new(
        turn_id,
        "cancel_session",
        tracker,
        0, // 0 ms = expira imediatamente
    ));

    std::thread::sleep(std::time::Duration::from_millis(5));

    let input = ResearchFetchInput::new("https://example.com/cancel-test");
    let res = CURRENT_TURN_CONTEXT.sync_scope(ctx, || provider.fetch(&input));

    assert!(
        res.is_err(),
        "Execução com deadline expirada deve abortar com segurança"
    );
    let err = res.err().unwrap().to_string();
    assert!(err.contains("Prazo de execução do turno de pesquisa excedido"));
}

// ============================================================================
// 14. Teste de Múltiplas Requisições Concorrentes com Encodings Diversos
// ============================================================================

#[test]
fn test_encoding_14_concurrent_requests_different_encodings() {
    let tracker = Arc::new(ResearchBudgetTracker::new(ResearchBudget {
        max_searches: 3,
        max_fetches: 10,
        max_page_bytes: 256 * 1024,
        max_total_bytes: 2 * 1024 * 1024,
        search_timeout_ms: 10000,
        fetch_timeout_ms: 15000,
        total_timeout_ms: 45000,
    }));

    let (provider, transport, _, _) = setup_test_provider(Some(tracker));
    let provider = Arc::new(provider);

    // Registra 4 páginas com encodings diferentes
    let html_gzip =
        compress_gzip(b"<html><head><title>Gzip</title></head><body>Gzip Body</body></html>");
    let mut h1 = HashMap::new();
    h1.insert("content-type".to_string(), "text/html".to_string());
    h1.insert("content-encoding".to_string(), "gzip".to_string());
    transport.set_response(
        "https://example.com/p1",
        Ok(FetchResponse {
            status: 200,
            headers: h1,
            body_chunks: vec![html_gzip],
        }),
    );

    let html_deflate = compress_deflate(
        b"<html><head><title>Deflate</title></head><body>Deflate Body</body></html>",
    );
    let mut h2 = HashMap::new();
    h2.insert("content-type".to_string(), "text/html".to_string());
    h2.insert("content-encoding".to_string(), "deflate".to_string());
    transport.set_response(
        "https://example.com/p2",
        Ok(FetchResponse {
            status: 200,
            headers: h2,
            body_chunks: vec![html_deflate],
        }),
    );

    let html_br =
        compress_brotli(b"<html><head><title>Brotli</title></head><body>Brotli Body</body></html>");
    let mut h3 = HashMap::new();
    h3.insert("content-type".to_string(), "text/html".to_string());
    h3.insert("content-encoding".to_string(), "br".to_string());
    transport.set_response(
        "https://example.com/p3",
        Ok(FetchResponse {
            status: 200,
            headers: h3,
            body_chunks: vec![html_br],
        }),
    );

    let html_id =
        b"<html><head><title>Identity</title></head><body>Identity Body</body></html>".to_vec();
    let mut h4 = HashMap::new();
    h4.insert("content-type".to_string(), "text/html".to_string());
    h4.insert("content-encoding".to_string(), "identity".to_string());
    transport.set_response(
        "https://example.com/p4",
        Ok(FetchResponse {
            status: 200,
            headers: h4,
            body_chunks: vec![html_id],
        }),
    );

    let mut handles = Vec::new();
    for url in [
        "https://example.com/p1",
        "https://example.com/p2",
        "https://example.com/p3",
        "https://example.com/p4",
    ] {
        let prov = provider.clone();
        let handle = std::thread::spawn(move || {
            let input = ResearchFetchInput::new(url);
            prov.fetch(&input)
        });
        handles.push(handle);
    }

    for h in handles {
        let res = h
            .join()
            .unwrap()
            .expect("Fetch concorrente deve ter sucesso");
        assert_eq!(res.http_status, 200);
        assert!(!res.extracted_text.is_empty());
    }
}

// ============================================================================
// 15. Teste de Proveniência, Hashes e SourceRegistry
// ============================================================================

#[test]
fn test_encoding_15_provenance_hashes_and_registry_metadata() {
    let (provider, transport, _, _) = setup_test_provider(None);
    let original = sample_html();
    let gzip_bytes = compress_gzip(original.as_bytes());

    let mut headers = HashMap::new();
    headers.insert(
        "content-type".to_string(),
        "text/html; charset=utf-8".to_string(),
    );
    headers.insert("content-encoding".to_string(), "gzip".to_string());

    transport.set_response(
        "https://example.com/provenance-test",
        Ok(FetchResponse {
            status: 200,
            headers,
            body_chunks: vec![gzip_bytes],
        }),
    );

    let input = ResearchFetchInput::new("https://example.com/provenance-test");
    let result = provider.fetch(&input).unwrap();

    let turn_id = TurnId::new();
    let mut source_registry = ObservedSourceRegistry::new(turn_id);
    let evidence = source_registry
        .register_fetch_result(&result, "research.fetch")
        .expect("Registro no SourceRegistry deve ter sucesso");

    let obs = source_registry
        .get_observation_by_cite_id(&evidence.cite_id)
        .expect("Observação deve existir");

    // Valida proveniência de codificação e metadados
    assert_eq!(obs.source_kind, SourceKind::DirectSource);
    assert_eq!(obs.content_hash_sha256, result.content_hash_sha256);
    assert_eq!(result.content_encoding.as_deref(), Some("gzip"));
    assert_eq!(obs.evidence_scope, format!("fetch:url={}", result.url));
    assert_eq!(evidence.evidence_scope, format!("fetch:url={}", result.url));
    assert!(evidence.is_full_page);
}

// ============================================================================
// 16. Teste de Orçamento de Turno Registrando Bytes Descomprimidos
// ============================================================================

#[test]
fn test_encoding_16_budget_tracker_records_decompressed_bytes() {
    let tracker = Arc::new(ResearchBudgetTracker::default());
    let (provider, transport, _, tracker_ref) = setup_test_provider(Some(tracker));

    let original = sample_html();
    let gzip_bytes = compress_gzip(original.as_bytes());

    let mut headers = HashMap::new();
    headers.insert(
        "content-type".to_string(),
        "text/html; charset=utf-8".to_string(),
    );
    headers.insert("content-encoding".to_string(), "gzip".to_string());

    transport.set_response(
        "https://example.com/budget-test",
        Ok(FetchResponse {
            status: 200,
            headers,
            body_chunks: vec![gzip_bytes],
        }),
    );

    let input = ResearchFetchInput::new("https://example.com/budget-test");
    let result = provider.fetch(&input).unwrap();

    assert_eq!(tracker_ref.fetches_count(), 1);
    // Bytes consumidos no orçamento deve ser exatamente o tamanho descomprimido!
    assert_eq!(tracker_ref.total_bytes(), result.decompressed_bytes);
    assert_eq!(result.decompressed_bytes, original.len());
}
