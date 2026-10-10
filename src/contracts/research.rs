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

// ============================================================================
// Pure-Rust FIPS 180-4 SHA-256 (Zero Network, Zero External Dependencies)
// ============================================================================

/// Computa o hash SHA-256 determinístico de uma sequência de bytes no formato hex lowercase (64 chars).
#[allow(clippy::chunks_exact_to_as_chunks)]
pub fn compute_sha256(data: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    let mut h0: u32 = 0x6a09e667;
    let mut h1: u32 = 0xbb67ae85;
    let mut h2: u32 = 0x3c6ef372;
    let mut h3: u32 = 0xa54ff53a;
    let mut h4: u32 = 0x510e527f;
    let mut h5: u32 = 0x9b05688c;
    let mut h6: u32 = 0x1f83d9ab;
    let mut h7: u32 = 0x5be0cd19;

    let bit_len = (data.len() as u64) * 8;
    let mut padded = data.to_vec();
    padded.push(0x80);

    while (padded.len() % 64) != 56 {
        padded.push(0x00);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in padded.chunks_exact(64) {
        let mut w = [0u32; 64];
        for (i, w_val) in w.iter_mut().take(16).enumerate() {
            *w_val = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let mut a = h0;
        let mut b = h1;
        let mut c = h2;
        let mut d = h3;
        let mut e = h4;
        let mut f = h5;
        let mut g = h6;
        let mut h = h7;

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        h0 = h0.wrapping_add(a);
        h1 = h1.wrapping_add(b);
        h2 = h2.wrapping_add(c);
        h3 = h3.wrapping_add(d);
        h4 = h4.wrapping_add(e);
        h5 = h5.wrapping_add(f);
        h6 = h6.wrapping_add(g);
        h7 = h7.wrapping_add(h);
    }

    format!(
        "{:08x}{:08x}{:08x}{:08x}{:08x}{:08x}{:08x}{:08x}",
        h0, h1, h2, h3, h4, h5, h6, h7
    )
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
        // Test "hello world"
        assert_eq!(
            compute_sha256(b"hello world"),
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
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
