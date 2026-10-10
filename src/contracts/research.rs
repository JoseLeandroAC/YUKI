use crate::contracts::errors::YukiError;
use serde::{Deserialize, Serialize};

// ============================================================================
// Provenance & Source Classification (ADR-020)
// ============================================================================

/// Classificação ontológica do nível de evidência e confiança da fonte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceKind {
    /// Snippet resumido originado de índice de busca (terceiro agregador).
    AggregatedSnippet,
    /// Conteúdo bruto lido diretamente da página de destino via fetch.
    DirectSource,
    /// Fonte espelho ou conteúdo não verificável independentemente.
    UnverifiedMirror,
}

/// Registro estruturado de uma fonte externa observada durante a execução (ADR-020).
/// Mantido pelo Core como custodiante de evidências.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservedSource {
    pub cite_id: String,
    pub original_url: String,
    pub canonical_url: String,
    pub domain: String,
    pub title: String,
    pub content_hash: String,
    pub confidence_state: SourceKind,
    pub observed_at: String,
}

// ============================================================================
// Research Search Contracts
// ============================================================================

fn default_search_max_results() -> u32 {
    5
}

fn default_search_freshness() -> String {
    "any".to_string()
}

/// Entrada tipada para a capability `research.search`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResearchSearchInput {
    pub query: String,
    #[serde(default = "default_search_max_results")]
    pub max_results: u32,
    #[serde(default = "default_search_freshness")]
    pub freshness: String,
}

impl ResearchSearchInput {
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            max_results: default_search_max_results(),
            freshness: default_search_freshness(),
        }
    }

    /// Validação de invariantes contratuais de entrada (ADR-020).
    pub fn validate(&self) -> Result<(), YukiError> {
        let q_trimmed = self.query.trim();
        if q_trimmed.len() < 2 {
            return Err(YukiError::InvalidRequest(
                "O parâmetro 'query' deve conter pelo menos 2 caracteres não vazios".to_string(),
            ));
        }
        if self.query.len() > 200 {
            return Err(YukiError::InvalidRequest(
                "O parâmetro 'query' excede o limite máximo de 200 caracteres".to_string(),
            ));
        }
        if self.max_results < 1 || self.max_results > 10 {
            return Err(YukiError::InvalidRequest(
                "O parâmetro 'max_results' deve estar entre 1 e 10".to_string(),
            ));
        }
        let valid_freshness = ["any", "day", "week", "month", "year"];
        if !valid_freshness.contains(&self.freshness.as_str()) {
            return Err(YukiError::InvalidRequest(format!(
                "O parâmetro 'freshness' ('{}') é inválido. Valores aceitos: {:?}",
                self.freshness, valid_freshness
            )));
        }
        Ok(())
    }
}

/// Item estruturado individual dentro dos resultados da pesquisa.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchResultItem {
    pub cite_id: String,
    pub url: String,
    pub title: String,
    pub snippet: String,
    pub domain: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub published_date: Option<String>,
    pub confidence_state: SourceKind,
}

/// Saída estruturada verificada da capability `research.search`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResearchSearchResult {
    pub query: String,
    pub provider: String,
    pub searched_at: String,
    pub results_count: usize,
    pub results: Vec<SearchResultItem>,
}

// ============================================================================
// Research Fetch Contracts
// ============================================================================

fn default_fetch_max_length_chars() -> usize {
    10000
}

/// Entrada tipada para a capability `research.fetch`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResearchFetchInput {
    pub url: String,
    #[serde(default = "default_fetch_max_length_chars")]
    pub max_length_chars: usize,
}

impl ResearchFetchInput {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            max_length_chars: default_fetch_max_length_chars(),
        }
    }

    /// Validação de invariantes contratuais de entrada (ADR-020).
    pub fn validate(&self) -> Result<(), YukiError> {
        let url_trimmed = self.url.trim();
        if url_trimmed.is_empty() {
            return Err(YukiError::InvalidRequest(
                "O parâmetro 'url' não pode ser vazio".to_string(),
            ));
        }
        if self.url.len() > 500 {
            return Err(YukiError::InvalidRequest(
                "O parâmetro 'url' excede o limite máximo de 500 caracteres".to_string(),
            ));
        }
        if !self.url.starts_with("https://") && !self.url.starts_with("http://") {
            return Err(YukiError::InvalidRequest(
                "O parâmetro 'url' deve utilizar esquema https:// ou http://".to_string(),
            ));
        }
        if self.max_length_chars < 500 || self.max_length_chars > 30000 {
            return Err(YukiError::InvalidRequest(
                "O parâmetro 'max_length_chars' deve estar entre 500 e 30000".to_string(),
            ));
        }
        Ok(())
    }
}

/// Saída estruturada verificada da capability `research.fetch`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResearchFetchResult {
    pub url: String,
    pub final_url: String,
    pub fetched_at: String,
    pub http_status: u16,
    pub content_type: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub published_date: Option<String>,
    pub extracted_text: String,
    pub content_hash_sha256: String,
    pub truncated: bool,
    #[serde(default)]
    pub bytes_observed: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    pub confidence_state: SourceKind,
}

// ============================================================================
// Research Turn Budget (ADR-020 Envelope)
// ============================================================================

/// Orçamento global de turno para a capability de Research (ADR-020).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResearchBudget {
    pub max_searches: u32,
    pub max_fetches: u32,
    pub max_page_bytes: usize,
    pub max_total_bytes: usize,
    pub search_timeout_ms: u64,
    pub fetch_timeout_ms: u64,
    pub total_timeout_ms: u64,
}

impl Default for ResearchBudget {
    fn default() -> Self {
        Self {
            max_searches: 3,
            max_fetches: 3,
            max_page_bytes: 256 * 1024,   // 256 KiB
            max_total_bytes: 1024 * 1024, // 1 MiB
            search_timeout_ms: 10_000,
            fetch_timeout_ms: 15_000,
            total_timeout_ms: 45_000,
        }
    }
}

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Instant;

/// Rastreador governado de consumo do envelope de orçamento de Research por turno (ADR-020).
///
/// Compartilhado entre SearchProvider e ContentFetchProvider para impor o teto global de turno.
#[derive(Debug)]
pub struct ResearchBudgetTracker {
    budget: ResearchBudget,
    searches_executed: AtomicUsize,
    fetches_executed: AtomicUsize,
    total_bytes_consumed: AtomicUsize,
    turn_started_at: Mutex<Option<Instant>>,
}

impl ResearchBudgetTracker {
    pub fn new(budget: ResearchBudget) -> Self {
        Self {
            budget,
            searches_executed: AtomicUsize::new(0),
            fetches_executed: AtomicUsize::new(0),
            total_bytes_consumed: AtomicUsize::new(0),
            turn_started_at: Mutex::new(None),
        }
    }

    pub fn budget(&self) -> &ResearchBudget {
        &self.budget
    }

    pub fn searches_count(&self) -> usize {
        self.searches_executed.load(Ordering::SeqCst)
    }

    pub fn fetches_count(&self) -> usize {
        self.fetches_executed.load(Ordering::SeqCst)
    }

    pub fn total_bytes(&self) -> usize {
        self.total_bytes_consumed.load(Ordering::SeqCst)
    }

    fn check_total_timeout(&self) -> Result<(), YukiError> {
        let mut start_guard = self.turn_started_at.lock().unwrap();
        let now = Instant::now();
        let start = match *start_guard {
            Some(t) => t,
            None => {
                *start_guard = Some(now);
                now
            }
        };

        let elapsed = now.duration_since(start).as_millis() as u64;
        if elapsed > self.budget.total_timeout_ms {
            return Err(YukiError::ExecutionFailed(format!(
                "Orçamento de tempo global de pesquisa esgotado para este turno ({}ms excedeu {}ms)",
                elapsed, self.budget.total_timeout_ms
            )));
        }
        Ok(())
    }

    pub fn check_and_increment_search(&self) -> Result<(), YukiError> {
        self.check_total_timeout()?;
        let current = self.searches_executed.load(Ordering::SeqCst);
        if current >= self.budget.max_searches as usize {
            return Err(YukiError::ExecutionFailed(format!(
                "Orçamento de pesquisas esgotado para este turno (máximo: {})",
                self.budget.max_searches
            )));
        }
        self.searches_executed.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    pub fn check_and_increment_fetch(&self) -> Result<(), YukiError> {
        self.check_total_timeout()?;
        let current = self.fetches_executed.load(Ordering::SeqCst);
        if current >= self.budget.max_fetches as usize {
            return Err(YukiError::ExecutionFailed(format!(
                "Orçamento de leituras de páginas (fetch) esgotado para este turno (máximo: {})",
                self.budget.max_fetches
            )));
        }
        self.fetches_executed.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    pub fn record_bytes(&self, bytes: usize) -> Result<(), YukiError> {
        let current = self.total_bytes_consumed.load(Ordering::SeqCst);
        let new_total = current.saturating_add(bytes);
        if new_total > self.budget.max_total_bytes {
            return Err(YukiError::ExecutionFailed(format!(
                "Orçamento cumulativo de dados de pesquisa excedido ({} bytes excedeu teto de {} bytes)",
                new_total, self.budget.max_total_bytes
            )));
        }
        self.total_bytes_consumed.store(new_total, Ordering::SeqCst);
        Ok(())
    }

    pub fn reset_turn(&self) {
        self.searches_executed.store(0, Ordering::SeqCst);
        self.fetches_executed.store(0, Ordering::SeqCst);
        self.total_bytes_consumed.store(0, Ordering::SeqCst);
        let mut start_guard = self.turn_started_at.lock().unwrap();
        *start_guard = None;
    }
}

impl Default for ResearchBudgetTracker {
    fn default() -> Self {
        Self::new(ResearchBudget::default())
    }
}

// ============================================================================
// Consolidated Cryptographic SHA-256 (RustCrypto sha2 crate)
// ============================================================================

/// Computa o hash SHA-256 determinístico de uma sequência de bytes no formato hex lowercase (64 chars).
/// Utiliza a crate consolidada `sha2` (RustCrypto), garantindo conformidade estrita com FIPS 180-4.
pub fn compute_sha256(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    format!("{:x}", result)
}

/// Helper determinístico para geração de timestamp ISO 8601 UTC.
pub fn now_iso8601() -> String {
    crate::contracts::identifiers::now_utc().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256_nist_vectors() {
        // NIST Empty String
        assert_eq!(
            compute_sha256(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        // NIST "abc"
        assert_eq!(
            compute_sha256(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        // NIST 56-byte string (crosses block boundary)
        assert_eq!(
            compute_sha256(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
        // Test "hello world"
        assert_eq!(
            compute_sha256(b"hello world"),
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
        // Equivalence for 1000 'a's
        let thousand_as = vec![b'a'; 1000];
        assert_eq!(
            compute_sha256(&thousand_as),
            "41edece42d63e8d9bf515a9ba6932e1c20cbc9f5a5d134645adb5db1b9737ea3"
        );
    }

    #[test]
    fn test_search_input_validation() {
        let valid = ResearchSearchInput::new("rust async");
        assert!(valid.validate().is_ok());

        let too_short = ResearchSearchInput::new("a");
        assert!(too_short.validate().is_err());

        let mut invalid_max = ResearchSearchInput::new("rust async");
        invalid_max.max_results = 20;
        assert!(invalid_max.validate().is_err());

        let mut invalid_freshness = ResearchSearchInput::new("rust async");
        invalid_freshness.freshness = "century".to_string();
        assert!(invalid_freshness.validate().is_err());
    }

    #[test]
    fn test_fetch_input_validation() {
        let valid = ResearchFetchInput::new("https://example.com/docs");
        assert!(valid.validate().is_ok());

        let invalid_scheme = ResearchFetchInput::new("file:///etc/passwd");
        assert!(invalid_scheme.validate().is_err());

        let empty = ResearchFetchInput::new("");
        assert!(empty.validate().is_err());

        let mut invalid_chars = ResearchFetchInput::new("https://example.com");
        invalid_chars.max_length_chars = 100;
        assert!(invalid_chars.validate().is_err());
    }
}
