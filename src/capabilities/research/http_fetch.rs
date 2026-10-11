use crate::capabilities::research::encoding::{
    decompress_bounded_stream, SupportedContentEncoding,
};
use crate::capabilities::research::html_extract::extract_text_from_html;
use crate::capabilities::research::provider::ContentFetchProvider;
use crate::capabilities::research::ssrf::{
    resolve_and_pin_host, validate_and_parse_fetch_url, DnsResolver, SystemDnsResolver,
};
use crate::contracts::errors::YukiError;
use crate::contracts::research::{
    compute_sha256, now_iso8601, ResearchBudgetTracker, ResearchFetchInput, ResearchFetchResult,
    SourceKind,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use std::time::Duration;

// ============================================================================
// Transport Abstraction (Injectable for Deterministic Testing)
// ============================================================================

/// Requisição governada com socket pinning explícito.
#[derive(Debug, Clone)]
pub struct FetchRequest {
    pub url: reqwest::Url,
    pub pinned_addr: SocketAddr,
    pub headers: HashMap<String, String>,
    pub timeout: Duration,
}

/// Resposta governada da camada de transporte.
#[derive(Debug, Clone)]
pub struct FetchResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body_chunks: Vec<Vec<u8>>,
}

/// Trait abstrato para a camada de transporte HTTP (permite mock seguro e isolado).
pub trait FetchTransport: Send + Sync {
    fn execute(&self, req: &FetchRequest) -> Result<FetchResponse, YukiError>;
}

/// Transporte de rede real utilizando reqwest com validação TLS e socket pinning (ADR-020).
pub struct NetworkFetchTransport;

impl FetchTransport for NetworkFetchTransport {
    fn execute(&self, req: &FetchRequest) -> Result<FetchResponse, YukiError> {
        let host = req.url.host_str().ok_or_else(|| {
            YukiError::InvalidRequest("URL deve conter um host válido".to_string())
        })?;

        // 1. Construir cliente reqwest amarrando o domínio ao IP aprovado (socket pinning)
        let client = reqwest::Client::builder()
            .timeout(req.timeout)
            .redirect(reqwest::redirect::Policy::none()) // Redirecionamento supervisionado manualmente
            .resolve(host, req.pinned_addr) // Imunidade absoluta contra DNS Rebinding
            .no_proxy() // CRÍTICO: Previne que proxies herdados do ambiente (HTTP_PROXY, HTTPS_PROXY, ALL_PROXY) contornem validações de IP e socket pinning
            .build()
            .map_err(|e| {
                YukiError::ExecutionFailed(format!(
                    "Falha ao instanciar cliente HTTP com socket pinning: {}",
                    e
                ))
            })?;

        let url_clone = req.url.clone();
        let headers_clone = req.headers.clone();
        let active_context = crate::contracts::research::CURRENT_TURN_CONTEXT
            .try_with(|ctx| ctx.clone())
            .ok();
        let active_budget = crate::contracts::research::CURRENT_TURN_BUDGET
            .try_with(|b| b.clone())
            .ok();

        // 2. Execução isolada em thread dedicada propagando explicitamente o contexto do turno (Marco 4)
        let thread_handle = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| {
                    YukiError::ExecutionFailed(format!(
                        "Falha ao instanciar runtime temporário para fetch: {}",
                        e
                    ))
                })?;

            let fut = async move {
                let mut req_builder = client.get(url_clone);
                for (k, v) in headers_clone {
                    req_builder = req_builder.header(k, v);
                }

                let resp = req_builder.send().await.map_err(|e| {
                    if e.is_timeout() {
                        YukiError::ExecutionFailed(
                            "Timeout na requisição de leitura de página".to_string(),
                        )
                    } else if e.is_connect() {
                        YukiError::ExecutionFailed(format!(
                            "Falha de conexão com o destino aprovado: {}",
                            e
                        ))
                    } else {
                        YukiError::ExecutionFailed(format!(
                            "Erro na requisição HTTP de fetch: {}",
                            e
                        ))
                    }
                })?;

                let status = resp.status().as_u16();
                let mut headers = HashMap::new();
                for (k, v) in resp.headers() {
                    if let Ok(val_str) = v.to_str() {
                        headers.insert(k.as_str().to_lowercase(), val_str.to_string());
                    }
                }

                // Streaming seguro com aborto imediato em violação de teto de rede (prevenção contra OOM/Memory Exhaustion)
                let is_compressed = headers
                    .get("content-encoding")
                    .map(|e| !e.is_empty() && e != "identity")
                    .unwrap_or(false);
                let max_stream_limit = if is_compressed {
                    256 * 1024 // 256 KiB teto incondicional para resposta comprimida na rede
                } else {
                    1024 * 1024 // 1 MiB teto incondicional para corpo descomprimido/identity
                };

                let mut body_chunks = Vec::new();
                let mut total_bytes = 0usize;

                let mut resp_stream = resp;
                while let Some(chunk) = resp_stream.chunk().await.map_err(|e| {
                    YukiError::ExecutionFailed(format!(
                        "Falha ao ler dados de streaming da resposta HTTP: {}",
                        e
                    ))
                })? {
                    total_bytes += chunk.len();
                    if total_bytes > max_stream_limit {
                        return Err(YukiError::ExecutionFailed(format!(
                            "Corpo de resposta na rede excedeu o limite máximo de {} bytes",
                            max_stream_limit
                        )));
                    }
                    body_chunks.push(chunk.to_vec());
                }

                Ok(FetchResponse {
                    status,
                    headers,
                    body_chunks,
                })
            };

            if let Some(ctx) = active_context {
                let trk = ctx.budget_tracker.clone();
                crate::contracts::research::CURRENT_TURN_CONTEXT.sync_scope(ctx, || {
                    crate::contracts::research::CURRENT_TURN_BUDGET
                        .sync_scope(trk, || rt.block_on(fut))
                })
            } else if let Some(b) = active_budget {
                crate::contracts::research::CURRENT_TURN_BUDGET.sync_scope(b, || rt.block_on(fut))
            } else {
                rt.block_on(fut)
            }
        });

        thread_handle.join().map_err(|_| {
            YukiError::ExecutionFailed(
                "Thread de execução HTTP em segundo plano encerrou com pânico".to_string(),
            )
        })?
    }
}

/// Transporte de teste simulado em memória para testes offline determinísticos.
pub struct MockFetchTransport {
    responses: RwLock<HashMap<String, Result<FetchResponse, YukiError>>>,
}

impl MockFetchTransport {
    pub fn new() -> Self {
        Self {
            responses: RwLock::new(HashMap::new()),
        }
    }

    pub fn set_response(&self, url: impl Into<String>, response: Result<FetchResponse, YukiError>) {
        self.responses.write().unwrap().insert(url.into(), response);
    }
}

impl Default for MockFetchTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl FetchTransport for MockFetchTransport {
    fn execute(&self, req: &FetchRequest) -> Result<FetchResponse, YukiError> {
        let url_str = req.url.to_string();
        if let Some(resp) = self.responses.read().unwrap().get(&url_str) {
            return resp.clone();
        }

        Err(YukiError::ExecutionFailed(format!(
            "URL '{}' não configurada no transporte de teste simulado",
            url_str
        )))
    }
}

// ============================================================================
// Configuration
// ============================================================================

/// Configuração governada para o provedor de leitura HTTP de páginas (ADR-020).
#[derive(Clone)]
pub struct HttpContentFetchConfig {
    pub live_enabled: bool,
    pub user_agent: String,
    pub accept_encoding: String,
    pub max_compressed_bytes: usize,
    pub max_uncompressed_bytes: usize,
    pub max_redirects: usize,
    pub budget_tracker: Arc<ResearchBudgetTracker>,
    pub resolver: Arc<dyn DnsResolver>,
    pub transport: Arc<dyn FetchTransport>,
}

impl HttpContentFetchConfig {
    pub fn new() -> Self {
        Self {
            live_enabled: false, // Desabilitado por padrão (fail-closed)
            user_agent: "Yuki/0.2.0 (Governed-AI-Platform; +https://github.com/JoseLeandroAC/YUKI)"
                .to_string(),
            accept_encoding: "gzip, deflate, br".to_string(),
            max_compressed_bytes: 256 * 1024,    // 256 KiB
            max_uncompressed_bytes: 1024 * 1024, // 1 MiB
            max_redirects: 3,                    // Máximo de 3 saltos
            budget_tracker: Arc::new(ResearchBudgetTracker::default()),
            resolver: Arc::new(SystemDnsResolver),
            transport: Arc::new(NetworkFetchTransport),
        }
    }

    pub fn from_env(budget_tracker: Arc<ResearchBudgetTracker>) -> Self {
        let mut config = Self::new();
        config.budget_tracker = budget_tracker;

        if let Ok(live_var) = std::env::var("YUKI_RESEARCH_LIVE_ENABLED") {
            config.live_enabled =
                live_var.trim().eq_ignore_ascii_case("true") || live_var.trim() == "1";
        }

        if let Ok(ua) = std::env::var("YUKI_RESEARCH_USER_AGENT") {
            let ua_trimmed = ua.trim();
            if !ua_trimmed.is_empty() {
                config.user_agent = ua_trimmed.to_string();
            }
        }

        if let Ok(ae) = std::env::var("YUKI_RESEARCH_ACCEPT_ENCODING") {
            let ae_trimmed = ae.trim();
            if !ae_trimmed.is_empty() {
                config.accept_encoding = ae_trimmed.to_string();
            }
        }

        config
    }

    pub fn with_live_enabled(mut self, enabled: bool) -> Self {
        self.live_enabled = enabled;
        self
    }

    pub fn with_user_agent(mut self, ua: impl Into<String>) -> Self {
        self.user_agent = ua.into();
        self
    }

    pub fn with_accept_encoding(mut self, enc: impl Into<String>) -> Self {
        self.accept_encoding = enc.into();
        self
    }

    pub fn with_budget_tracker(mut self, tracker: Arc<ResearchBudgetTracker>) -> Self {
        self.budget_tracker = tracker;
        self
    }

    pub fn with_resolver(mut self, resolver: Arc<dyn DnsResolver>) -> Self {
        self.resolver = resolver;
        self
    }

    pub fn with_transport(mut self, transport: Arc<dyn FetchTransport>) -> Self {
        self.transport = transport;
        self
    }

    pub fn with_max_compressed_bytes(mut self, max: usize) -> Self {
        self.max_compressed_bytes = max;
        self
    }

    pub fn with_max_uncompressed_bytes(mut self, max: usize) -> Self {
        self.max_uncompressed_bytes = max;
        self
    }

    pub fn with_max_redirects(mut self, max: usize) -> Self {
        self.max_redirects = max;
        self
    }
}

impl Default for HttpContentFetchConfig {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Concrete Provider Implementation
// ============================================================================

/// Provedor real de recuperação governada de páginas web (`research.fetch`).
///
/// INVARIANTES DE SEGURANÇA (ADR-020):
/// 1. Desabilitado por padrão (`live_enabled == false`). Falha fechada sem rede.
/// 2. Validação rigorosa de URLs (rejeição de esquemas não-HTTP/S, credenciais embutidas e portas anômalas).
/// 3. Política positiva de IPs globalmente roteáveis (bloqueio total de RFC 1918, RFC 6598, Link-Local,
///    Cloud Metadata 169.254.169.254, IPv6 loopback/ULA e IPv4-mapped IPv6 ::ffff:0:0/96).
/// 4. Imunidade contra DNS Rebinding através de resolução prévia com validação total de IPs e Socket Pinning.
/// 5. Redirecionamento supervisionado e per-hop validado (máximo de 3 saltos, proibição de downgrade HTTPS -> HTTP).
/// 6. Limite de streaming de resposta: teto de 256 KiB comprimido e 1 MiB descomprimido.
/// 7. Sanitização estrita de HTML: remoção de scripts, estilos, comentários e mídias executáveis.
/// 8. Compartilhamento governado do envelope de orçamento por turno (`ResearchBudgetTracker`).
pub struct HttpContentFetchProvider {
    config: HttpContentFetchConfig,
}

impl HttpContentFetchProvider {
    pub fn new(config: HttpContentFetchConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &HttpContentFetchConfig {
        &self.config
    }
}

impl ContentFetchProvider for HttpContentFetchProvider {
    fn is_live(&self) -> bool {
        self.config.live_enabled
    }

    fn fetch(&self, input: &ResearchFetchInput) -> Result<ResearchFetchResult, YukiError> {
        // 1. Salvaguarda incondicional: Live mode deve estar explicitamente ativado
        if !self.config.live_enabled {
            return Err(YukiError::SecurityViolation(
                "Leitura web real desabilitada por padrão (YUKI_RESEARCH_LIVE_ENABLED=false). Ativação explícita do operador é necessária.".to_string(),
            ));
        }

        // 2. Validação prévia dos parâmetros contratuais de entrada
        input.validate()?;

        // 3. Verificação do orçamento global compartilhado de turno (ADR-020 com isolamento de contexto e deadline)
        let (active_tracker, active_ctx) =
            match crate::contracts::research::CURRENT_TURN_CONTEXT.try_with(|ctx| ctx.clone()) {
                Ok(ctx) => {
                    ctx.check_deadline()?;
                    (ctx.budget_tracker.clone(), Some(ctx))
                }
                Err(_) => {
                    match crate::contracts::research::CURRENT_TURN_BUDGET.try_with(|b| b.clone()) {
                        Ok(b) => (b, None),
                        Err(_) => (self.config.budget_tracker.clone(), None),
                    }
                }
            };
        active_tracker.check_and_increment_fetch()?;

        // 4. Loop de navegação e redirecionamento com validação per-hop (máximo 3 saltos)
        let mut current_url = validate_and_parse_fetch_url(&input.url)?;
        let mut redirect_hops = 0;

        loop {
            let host = current_url.host_str().ok_or_else(|| {
                YukiError::InvalidRequest("URL não contém nome de host válido".to_string())
            })?;
            let port = current_url.port_or_known_default().ok_or_else(|| {
                YukiError::InvalidRequest("Porta de rede desconhecida ou não padrão".to_string())
            })?;

            // Validação DNS e amarração estrita ao IP aprovado (Socket Pinning / Anti-DNS Rebinding)
            let pinned_addr = resolve_and_pin_host(host, port, self.config.resolver.as_ref())?;

            let mut headers = HashMap::new();
            headers.insert("User-Agent".to_string(), self.config.user_agent.clone());
            headers.insert(
                "Accept".to_string(),
                "text/html,text/plain;q=0.9,*/*;q=0.1".to_string(),
            );
            headers.insert(
                "Accept-Encoding".to_string(),
                self.config.accept_encoding.clone(),
            );

            let timeout =
                Duration::from_millis(self.config.budget_tracker.budget().fetch_timeout_ms);

            let fetch_req = FetchRequest {
                url: current_url.clone(),
                pinned_addr,
                headers,
                timeout,
            };

            let resp = self.config.transport.execute(&fetch_req)?;

            // 5. Tratamento de redirecionamento HTTP supervisionado (301, 302, 303, 307, 308)
            if (300..=399).contains(&resp.status) && resp.headers.contains_key("location") {
                if redirect_hops >= self.config.max_redirects {
                    return Err(YukiError::ExecutionFailed(format!(
                        "Limite de redirecionamentos excedido (máximo de {} saltos)",
                        self.config.max_redirects
                    )));
                }

                let location = &resp.headers["location"];
                let next_url = current_url.join(location).map_err(|e| {
                    YukiError::ExecutionFailed(format!(
                        "Cabeçalho de redirecionamento Location inválido ('{}'): {}",
                        location, e
                    ))
                })?;

                // Validação da nova URL contra a política estrita de egresso
                let next_validated = validate_and_parse_fetch_url(next_url.as_str())?;

                // Proibição estrita de downgrade de HTTPS para HTTP
                if current_url.scheme() == "https" && next_validated.scheme() == "http" {
                    return Err(YukiError::SecurityViolation(
                        "Downgrade inseguro de HTTPS para HTTP proibido em redirecionamento"
                            .to_string(),
                    ));
                }

                current_url = next_validated;
                redirect_hops += 1;
                continue;
            }

            // 6. Tratamento determinístico de status HTTP
            if resp.status == 404 {
                return Err(YukiError::ExecutionFailed(
                    "Página não encontrada no servidor remoto (HTTP 404)".to_string(),
                ));
            } else if resp.status == 429 {
                return Err(YukiError::ExecutionFailed(
                    "Limite de requisições excedido no servidor remoto (HTTP 429)".to_string(),
                ));
            } else if resp.status == 401 || resp.status == 403 {
                return Err(YukiError::ExecutionFailed(format!(
                    "Acesso não autorizado ou proibido na página remota (HTTP {})",
                    resp.status
                )));
            } else if resp.status >= 500 {
                return Err(YukiError::ExecutionFailed(format!(
                    "Erro interno no servidor remoto (HTTP {})",
                    resp.status
                )));
            } else if resp.status != 200 {
                return Err(YukiError::ExecutionFailed(format!(
                    "Status HTTP inesperado retornado pela página: {}",
                    resp.status
                )));
            }

            // 7. Validação de Content-Type permitido
            let content_type = resp
                .headers
                .get("content-type")
                .cloned()
                .unwrap_or_else(|| "text/html; charset=utf-8".to_string());

            validate_content_type(&content_type)?;

            // 8. Descompressão streaming delimitada de Content-Encoding (gzip, deflate, br, identity)
            let encoding_header = resp.headers.get("content-encoding").map(|s| s.as_str());
            let encoding = SupportedContentEncoding::parse(encoding_header)?;

            let decomp_res = decompress_bounded_stream(
                encoding,
                &resp.body_chunks,
                self.config.max_compressed_bytes,
                self.config.max_uncompressed_bytes,
            )?;

            // 9. Registro de bytes consumidos no orçamento global compartilhado
            active_tracker.record_bytes(decomp_res.decompressed_bytes)?;

            // 10. Extração e sanitização segura de texto HTML (Data != Instruction)
            let raw_text = String::from_utf8_lossy(&decomp_res.decompressed_data);
            let (extracted_text, title, truncated) =
                extract_text_from_html(&raw_text, input.max_length_chars);

            let content_hash_sha256 = compute_sha256(extracted_text.as_bytes());

            let published_date = resp.headers.get("last-modified").cloned();
            let source_id = Some(format!("src:fetch:{}", &content_hash_sha256[..16]));

            let final_title = title.unwrap_or_else(|| {
                format!(
                    "Conteúdo de {}",
                    current_url.host_str().unwrap_or("origem desconhecida")
                )
            });

            if let Some(ctx) = &active_ctx {
                ctx.check_deadline()?;
            }

            return Ok(ResearchFetchResult {
                url: input.url.clone(),
                final_url: current_url.to_string(),
                fetched_at: now_iso8601(),
                http_status: resp.status,
                content_type,
                title: final_title,
                published_date,
                extracted_text,
                content_hash_sha256,
                truncated,
                bytes_observed: decomp_res.decompressed_bytes,
                raw_network_bytes: decomp_res.raw_network_bytes,
                decompressed_bytes: decomp_res.decompressed_bytes,
                content_encoding: Some(decomp_res.encoding.as_str().to_string()),
                source_id,
                confidence_state: SourceKind::DirectSource,
            });
        }
    }
}

/// Valida se o Content-Type retornado é compatível com texto legível.
fn validate_content_type(content_type: &str) -> Result<(), YukiError> {
    let ct_lower = content_type.to_lowercase();

    let allowed = [
        "text/html",
        "text/plain",
        "application/xhtml+xml",
        "application/xml",
        "text/xml",
        "application/json",
        "text/markdown",
    ];

    if allowed.iter().any(|&prefix| ct_lower.contains(prefix)) {
        return Ok(());
    }

    Err(YukiError::InvalidRequest(format!(
        "Tipo de conteúdo não suportado ou potencialmente perigoso: '{}'. Apenas texto legível é aceito.",
        content_type
    )))
}
