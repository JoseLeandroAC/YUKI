use crate::capabilities::research::provider::SearchProvider;
use crate::contracts::errors::YukiError;
use crate::contracts::research::{
    now_iso8601, ResearchBudget, ResearchBudgetTracker, ResearchSearchInput, ResearchSearchResult,
    SearchResultItem, SourceKind,
};
use crate::security::credentials::{CredentialBroker, SecretRef};
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;

/// Wire types específicos da API oficial Brave Search (REST v1 /web/search).
///
/// INVARIANTE:
/// Tipos de transporte estritamente privados ao adapter, nunca vazados para o Yuki Core.
#[derive(Debug, Deserialize)]
struct BraveSearchWireResponse {
    #[serde(default)]
    web: Option<BraveWebSectionWire>,
}

#[derive(Debug, Deserialize)]
struct BraveWebSectionWire {
    #[serde(default)]
    results: Option<Vec<BraveWebResultItemWire>>,
}

#[derive(Debug, Deserialize)]
struct BraveWebResultItemWire {
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    page_age: Option<String>,
    #[serde(default)]
    age: Option<String>,
}

/// Configuração governada para o adapter do Brave Search.
#[derive(Debug, Clone)]
pub struct BraveSearchConfig {
    pub endpoint: String,
    pub live_enabled: bool,
    pub secret_ref: SecretRef,
    pub safesearch: String,
    pub search_lang: Option<String>,
    pub country: Option<String>,
    pub max_response_bytes: usize,
    pub budget: ResearchBudget,
    pub budget_tracker: Option<Arc<ResearchBudgetTracker>>,
}

impl BraveSearchConfig {
    pub fn new() -> Self {
        let secret_ref = SecretRef::new(
            "secret://env/YUKI_BRAVE_SEARCH_API_KEY",
            "cred_brave_search_primary",
        )
        .expect("SecretRef canônica para Brave Search é válida");

        Self {
            endpoint: "https://api.search.brave.com/res/v1/web/search".to_string(),
            live_enabled: false, // Desabilitado por padrão (fail-closed)
            secret_ref,
            safesearch: "moderate".to_string(),
            search_lang: Some("pt".to_string()),
            country: Some("BR".to_string()),
            max_response_bytes: 512 * 1024, // 512 KiB teto defensivo
            budget: ResearchBudget::default(),
            budget_tracker: None,
        }
    }

    /// Carrega configurações de variáveis de ambiente com defaults seguros.
    pub fn from_env() -> Self {
        let mut config = Self::new();

        if let Ok(live_var) = std::env::var("YUKI_RESEARCH_LIVE_ENABLED") {
            config.live_enabled =
                live_var.trim().eq_ignore_ascii_case("true") || live_var.trim() == "1";
        }

        if let Ok(ep) = std::env::var("YUKI_BRAVE_SEARCH_ENDPOINT") {
            let ep_trimmed = ep.trim();
            if !ep_trimmed.is_empty() {
                config.endpoint = ep_trimmed.to_string();
            }
        }

        if let Ok(lang) = std::env::var("YUKI_BRAVE_SEARCH_LANG") {
            let lang_trimmed = lang.trim();
            if !lang_trimmed.is_empty() {
                config.search_lang = Some(lang_trimmed.to_string());
            }
        }

        if let Ok(country) = std::env::var("YUKI_BRAVE_SEARCH_COUNTRY") {
            let country_trimmed = country.trim();
            if !country_trimmed.is_empty() {
                config.country = Some(country_trimmed.to_string());
            }
        }

        config
    }

    pub fn with_live_enabled(mut self, enabled: bool) -> Self {
        self.live_enabled = enabled;
        self
    }

    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = endpoint.into();
        self
    }

    pub fn with_secret_ref(mut self, secret_ref: SecretRef) -> Self {
        self.secret_ref = secret_ref;
        self
    }

    pub fn with_budget(mut self, budget: ResearchBudget) -> Self {
        self.budget = budget;
        self
    }

    pub fn with_budget_tracker(mut self, tracker: Arc<ResearchBudgetTracker>) -> Self {
        self.budget = tracker.budget().clone();
        self.budget_tracker = Some(tracker);
        self
    }

    pub fn with_max_response_bytes(mut self, max_bytes: usize) -> Self {
        self.max_response_bytes = max_bytes;
        self
    }
}

impl Default for BraveSearchConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Adapter concreto para a API oficial do Brave Search (ADR-020).
///
/// INVARIANTES DE SEGURANÇA:
/// 1. Desabilitado por padrão (`live_enabled == false`). A presença de chave não autoriza live mode.
/// 2. TLS validado incondicionalmente em requisições de produção.
/// 3. Redirecionamentos desativados (`Policy::none()`).
/// 4. Credencial nunca injetada em URLs nem em mensagens de erro ou logs (Zeroize em SecretMaterial).
/// 5. Respostas truncadas e tratadas estritamente como dados brutos não confiáveis (`Data != Instruction`).
/// 6. Orçamento delimitado por turno (`ResearchBudgetTracker`).
pub struct BraveSearchProvider {
    config: BraveSearchConfig,
    credential_broker: Arc<dyn CredentialBroker>,
    client: reqwest::Client,
    budget_tracker: Arc<ResearchBudgetTracker>,
}

impl BraveSearchProvider {
    pub fn new(
        config: BraveSearchConfig,
        credential_broker: Arc<dyn CredentialBroker>,
    ) -> Result<Self, YukiError> {
        let timeout = Duration::from_millis(config.budget.search_timeout_ms);
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy() // Previne desvio de tráfego por proxies de ambiente
            .build()
            .map_err(|e| {
                YukiError::ExecutionFailed(format!(
                    "Falha ao instanciar cliente HTTP para BraveSearchProvider: {}",
                    e
                ))
            })?;

        let budget_tracker = config
            .budget_tracker
            .clone()
            .unwrap_or_else(|| Arc::new(ResearchBudgetTracker::new(config.budget.clone())));

        Ok(Self {
            config,
            credential_broker,
            client,
            budget_tracker,
        })
    }

    /// Retorna o número de buscas executadas por esta instância.
    pub fn searches_executed(&self) -> usize {
        self.budget_tracker.searches_count()
    }

    /// Helper para execução assíncrona desacoplada de contextos síncronos/Tokio.
    /// Executa em thread dedicada isolada propagando explicitamente o contexto e orçamento do turno (Marco 4).
    fn run_async<F, T>(
        &self,
        future: F,
        context: Option<Arc<crate::contracts::research::ResearchTurnContext>>,
        budget: Arc<crate::contracts::research::ResearchBudgetTracker>,
    ) -> Result<T, YukiError>
    where
        F: std::future::Future<Output = Result<T, YukiError>> + Send + 'static,
        T: Send + 'static,
    {
        let thread_handle = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| {
                    YukiError::ExecutionFailed(format!(
                        "Falha ao instanciar runtime temporário para busca: {}",
                        e
                    ))
                })?;

            if let Some(ctx) = context {
                let trk = ctx.budget_tracker.clone();
                crate::contracts::research::CURRENT_TURN_CONTEXT.sync_scope(ctx, || {
                    crate::contracts::research::CURRENT_TURN_BUDGET
                        .sync_scope(trk, || rt.block_on(future))
                })
            } else {
                crate::contracts::research::CURRENT_TURN_BUDGET
                    .sync_scope(budget, || rt.block_on(future))
            }
        });

        thread_handle.join().map_err(|_| {
            YukiError::ExecutionFailed(
                "Thread de execução HTTP em segundo plano encerrou com pânico".to_string(),
            )
        })?
    }
}

impl SearchProvider for BraveSearchProvider {
    fn is_live(&self) -> bool {
        self.config.live_enabled
    }

    fn search(&self, input: &ResearchSearchInput) -> Result<ResearchSearchResult, YukiError> {
        // 1. Salvaguarda incondicional: Live mode deve estar explicitamente ativado
        if !self.config.live_enabled {
            return Err(YukiError::SecurityViolation(
                "Pesquisa web real desabilitada por padrão (YUKI_RESEARCH_LIVE_ENABLED=false). Ativação explícita do operador é necessária.".to_string(),
            ));
        }

        // 2. Verificação de orçamento de buscas por turno (ADR-020 com isolamento de contexto e deadline)
        let (active_tracker, active_ctx) =
            match crate::contracts::research::CURRENT_TURN_CONTEXT.try_with(|ctx| ctx.clone()) {
                Ok(ctx) => {
                    ctx.check_deadline()?;
                    (ctx.budget_tracker.clone(), Some(ctx))
                }
                Err(_) => {
                    match crate::contracts::research::CURRENT_TURN_BUDGET.try_with(|b| b.clone()) {
                        Ok(b) => (b, None),
                        Err(_) => (self.budget_tracker.clone(), None),
                    }
                }
            };
        active_tracker.check_and_increment_search()?;

        // 3. Resolução segura de credencial via CredentialBroker
        let lease = self
            .credential_broker
            .acquire(&self.config.secret_ref)
            .map_err(|e| {
                YukiError::ExecutionFailed(format!(
                    "Credencial para busca externa indisponível ('{}'): {}",
                    self.config.secret_ref.sanitized_alias(),
                    e
                ))
            })?;

        // 4. Montagem da URL e parâmetros validados
        let mut url = reqwest::Url::parse(&self.config.endpoint).map_err(|e| {
            YukiError::InvalidRequest(format!(
                "Endpoint inválido configurado para Brave Search: {}",
                e
            ))
        })?;

        {
            let mut pairs = url.query_pairs_mut();
            pairs.append_pair("q", &input.query);
            pairs.append_pair("count", &input.max_results.clamp(1, 10).to_string());
            pairs.append_pair("text_decorations", "0"); // Snippets em texto puro sem tags HTML
            pairs.append_pair("safesearch", &self.config.safesearch);

            if let Some(lang) = &self.config.search_lang {
                pairs.append_pair("search_lang", lang);
            }
            if let Some(country) = &self.config.country {
                pairs.append_pair("country", country);
            }

            // Mapeamento canônico de freshness do ADR-020 para Brave API
            match input.freshness.as_str() {
                "day" => {
                    pairs.append_pair("freshness", "pd");
                }
                "week" => {
                    pairs.append_pair("freshness", "pw");
                }
                "month" => {
                    pairs.append_pair("freshness", "pm");
                }
                "year" => {
                    pairs.append_pair("freshness", "py");
                }
                _ => {} // "any" omite o parâmetro
            }
        }

        let max_response_bytes = self.config.max_response_bytes;
        let token = lease.expose_secret().to_string();
        let client = self.client.clone();
        let url_clone = url.clone();

        // 5. Execução HTTP isolada
        let wire_response = self.run_async(
            async move {
                let resp = client
                    .get(url_clone)
                    .header("X-Subscription-Token", token)
                    .header("Accept", "application/json")
                    .header("User-Agent", "Yuki/0.2.0 (Governed-AI-Platform)")
                    .send()
                    .await
                    .map_err(|e| {
                        if e.is_timeout() {
                            YukiError::ExecutionFailed(
                                "Timeout na requisição ao provedor de busca".to_string(),
                            )
                        } else if e.is_connect() {
                            YukiError::ExecutionFailed(format!(
                                "Falha de conexão de rede com provedor de busca: {}",
                                e
                            ))
                        } else {
                            YukiError::ExecutionFailed(format!(
                                "Erro na requisição ao provedor de busca: {}",
                                e
                            ))
                        }
                    })?;

                let status = resp.status();
                if status == reqwest::StatusCode::BAD_REQUEST {
                    return Err(YukiError::InvalidRequest(
                        "Provedor de busca rejeitou os parâmetros da consulta (HTTP 400)".to_string(),
                    ));
                } else if status == reqwest::StatusCode::UNAUTHORIZED
                    || status == reqwest::StatusCode::FORBIDDEN
                {
                    return Err(YukiError::ExecutionFailed(
                        "Credencial de busca inválida, não autorizada ou cota esgotada (HTTP 401/403)"
                            .to_string(),
                    ));
                } else if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                    return Err(YukiError::ExecutionFailed(
                        "Limite de taxa (Rate Limit) do provedor de busca excedido (HTTP 429)"
                            .to_string(),
                    ));
                } else if status.is_server_error() {
                    return Err(YukiError::ExecutionFailed(format!(
                        "Instabilidade temporária no provedor de busca (HTTP {})",
                        status.as_u16()
                    )));
                } else if !status.is_success() {
                    return Err(YukiError::ExecutionFailed(format!(
                        "Status HTTP inesperado do provedor de busca: {}",
                        status.as_u16()
                    )));
                }

                if let Some(len) = resp.content_length() {
                    if len > max_response_bytes as u64 {
                        return Err(YukiError::ExecutionFailed(format!(
                            "Resposta do provedor de busca excedeu limite de tamanho ({} bytes)",
                            len
                        )));
                    }
                }

                let body_bytes = resp.bytes().await.map_err(|e| {
                    YukiError::ExecutionFailed(format!(
                        "Falha ao ler dados da resposta de busca: {}",
                        e
                    ))
                })?;

                if body_bytes.len() > max_response_bytes {
                    return Err(YukiError::ExecutionFailed(format!(
                        "Corpo da resposta de busca excedeu limite de {} bytes",
                        max_response_bytes
                    )));
                }

                let parsed: BraveSearchWireResponse =
                    serde_json::from_slice(&body_bytes).map_err(|e| {
                        YukiError::ExecutionFailed(format!(
                            "Resposta malformada ou JSON inválido do provedor de busca: {}",
                            e
                        ))
                    })?;

                Ok(parsed)
            },
            active_ctx.clone(),
            active_tracker.clone(),
        )?;

        if let Some(ctx) = &active_ctx {
            ctx.check_deadline()?;
        }

        // 6. Conversão estrita e sanitização para o contrato canônico
        let mut results = Vec::new();
        if let Some(web_section) = wire_response.web {
            if let Some(raw_items) = web_section.results {
                for item in raw_items {
                    if results.len() >= input.max_results as usize {
                        break;
                    }

                    let url_str = match item.url {
                        Some(u) if u.starts_with("https://") || u.starts_with("http://") => {
                            u.trim().to_string()
                        }
                        _ => continue, // Ignora URLs com esquemas não permitidos
                    };

                    let domain = match reqwest::Url::parse(&url_str) {
                        Ok(parsed) => parsed.host_str().unwrap_or("unknown").to_string(),
                        Err(_) => continue,
                    };

                    let raw_title = item.title.unwrap_or_else(|| "Sem título".to_string());
                    let title = if raw_title.chars().count() > 200 {
                        raw_title.chars().take(200).collect()
                    } else {
                        raw_title
                    };

                    let raw_snippet = item.description.unwrap_or_default();
                    let snippet = if raw_snippet.chars().count() > 1000 {
                        raw_snippet.chars().take(1000).collect()
                    } else {
                        raw_snippet
                    };

                    let published_date = item.page_age.or(item.age);
                    let cite_id = format!("src:{}", results.len() + 1);

                    results.push(SearchResultItem {
                        cite_id,
                        url: url_str,
                        title,
                        snippet,
                        domain,
                        published_date,
                        confidence_state: SourceKind::AggregatedSnippet,
                    });
                }
            }
        }

        Ok(ResearchSearchResult {
            query: input.query.clone(),
            provider: "BraveSearchProvider".to_string(),
            searched_at: now_iso8601(),
            results_count: results.len(),
            results,
        })
    }
}
