use crate::contracts::identifiers::now_utc;
use crate::contracts::voice::{AudioInput, SttDecodingConfigSummary, SttTimingMetrics, VoiceError};
use crate::voice::audio::AudioCodec;
use crate::voice::stt::{LocalTranscriptionResult, LocalWhisperConfig, LocalWhisperSttProvider};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Categoria do caso de teste no Corpus Vocal da Yuki.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CorpusCategory {
    /// Comandos curtos do sistema (ex: "abrir calculadora", "que horas são").
    Command,
    /// Frases curtas e objetivas do cotidiano.
    ShortPhrase,
    /// Frases longas, encadeadas, com múltiplos períodos e termos técnicos/acadêmicos.
    LongPhrase,
    /// Fala espontânea e conversacional sem roteiro rígido.
    Spontaneous,
    /// Testes em idiomas secundários (inglês, espanhol, japonês).
    Multilingual,
    /// Frases de desafio fonético com acrônimos, números ou consoantes sibilantes.
    PhoneticChallenge,
}

impl std::fmt::Display for CorpusCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Command => write!(f, "Comando"),
            Self::ShortPhrase => write!(f, "Frase Curta"),
            Self::LongPhrase => write!(f, "Frase Longa"),
            Self::Spontaneous => write!(f, "Espontânea"),
            Self::Multilingual => write!(f, "Multilíngue"),
            Self::PhoneticChallenge => write!(f, "Desafio Fonético"),
        }
    }
}

impl std::str::FromStr for CorpusCategory {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().trim() {
            "command" | "comando" => Ok(Self::Command),
            "short_phrase" | "short" | "curta" => Ok(Self::ShortPhrase),
            "long_phrase" | "long" | "longa" => Ok(Self::LongPhrase),
            "spontaneous" | "espontanea" | "espontânea" => Ok(Self::Spontaneous),
            "multilingual" | "multilingue" | "multilíngue" => Ok(Self::Multilingual),
            "phonetic" | "phonetic_challenge" | "fonetico" | "fonético" => {
                Ok(Self::PhoneticChallenge)
            }
            other => Err(format!(
                "Categoria desconhecida: '{}'. Válidas: command, short_phrase, long_phrase, spontaneous, multilingual, phonetic_challenge",
                other
            )),
        }
    }
}

/// Registro estruturado de um caso de teste no Corpus de Referência de Voz.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorpusTestCase {
    /// Identificador único e mnemônico do teste (ex: "T14_curto_calculadora").
    pub id: String,
    /// Caminho relativo ou absoluto do arquivo WAV local gravado.
    pub audio_path: String,
    /// Transcrição de referência humana canônica (ground truth). `None` se ainda pendente de validação.
    pub ground_truth: Option<String>,
    /// Indica se a transcrição de referência ainda está pendente de revisão pelo operador.
    pub review_pending: bool,
    /// Código do idioma esperado do áudio (ex: "pt", "en", "es", "ja").
    pub language: String,
    /// Dispositivo físico de captura utilizado (ex: "Microfone Realtek(R) Audio", "Redmi Buds 6 Play").
    pub device_name: Option<String>,
    /// Categoria temática do caso de teste.
    pub category: CorpusCategory,
    /// Lista de termos críticos que devem ser detectados com alta prioridade (ex: ["Yuki", "FUVEST"]).
    #[serde(default)]
    pub important_terms: Vec<String>,
    /// Duração do áudio em milissegundos (se conhecida).
    pub duration_ms: Option<u64>,
    /// Observações contextuais do teste (ex: ruído de fundo, distância do microfone, tom da voz).
    pub notes: Option<String>,
}

/// Manifesto do Corpus de Voz de Homologação da Yuki.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceCorpusManifest {
    pub version: String,
    pub description: String,
    pub test_cases: Vec<CorpusTestCase>,
}

impl VoiceCorpusManifest {
    /// Carrega o manifesto a partir de um arquivo JSON.
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self, VoiceError> {
        let path_ref = path.as_ref();
        let content = std::fs::read_to_string(path_ref).map_err(|e| {
            VoiceError::Io(format!(
                "Falha ao ler manifesto do corpus '{}': {}",
                path_ref.display(),
                e
            ))
        })?;
        serde_json::from_str(&content).map_err(|e| {
            VoiceError::Io(format!(
                "Falha ao desserializar manifesto do corpus '{}': {}",
                path_ref.display(),
                e
            ))
        })
    }

    /// Salva o manifesto em arquivo JSON formatado com indentação.
    pub fn save_to_file(&self, path: impl AsRef<Path>) -> Result<(), VoiceError> {
        let path_ref = path.as_ref();
        if let Some(parent) = path_ref.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                VoiceError::Io(format!(
                    "Falha ao criar diretório do manifesto '{}': {}",
                    parent.display(),
                    e
                ))
            })?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| {
            VoiceError::Io(format!("Falha ao serializar manifesto do corpus: {}", e))
        })?;
        std::fs::write(path_ref, json).map_err(|e| {
            VoiceError::Io(format!(
                "Falha ao gravar manifesto do corpus '{}': {}",
                path_ref.display(),
                e
            ))
        })
    }

    /// Filtra casos de teste por categoria.
    pub fn filter_by_category(&self, category: CorpusCategory) -> Vec<&CorpusTestCase> {
        self.test_cases
            .iter()
            .filter(|c| c.category == category)
            .collect()
    }

    /// Filtra casos de teste por idioma.
    pub fn filter_by_language(&self, lang: &str) -> Vec<&CorpusTestCase> {
        let norm = lang.to_lowercase();
        self.test_cases
            .iter()
            .filter(|c| c.language.to_lowercase() == norm)
            .collect()
    }
}

/// Normalização de texto estritamente voltada para avaliação objetiva (WER e CER).
///
/// Regras:
/// 1. Converte caracteres para minúsculas (`to_lowercase`).
/// 2. Remove pontuações e delimitadores gramaticais (. , ! ? ; : " ' ` ( ) [ ] { } < > - — _ / \ « » “ ” etc.).
/// 3. PRESERVA rigorosamente acentos e diacríticos em português (á, é, í, ó, ú, à, ã, õ, ç, â, ê, ô).
/// 4. PRESERVA caracteres alfanuméricos Unicode (incluindo Kanji, Hiragana e Katakana em japonês).
/// 5. Colapsa espaços múltiplos em espaço simples e remove espaços de borda (trim).
pub fn normalize_for_evaluation(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    for c in text.to_lowercase().chars() {
        if c.is_alphanumeric() {
            result.push(c);
        } else if c.is_whitespace() {
            result.push(' ');
        } else {
            // Pontuações viram espaço para evitar aglutinar palavras adjacentes (ex: "olá,yuki" -> "olá yuki")
            result.push(' ');
        }
    }
    result.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Detalhes do cálculo de Word Error Rate (WER).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WerResult {
    /// Número de substituições ($S$).
    pub substitutions: usize,
    /// Número de exclusões da referência ($D$).
    pub deletions: usize,
    /// Número de inserções na hipótese ($I$).
    pub insertions: usize,
    /// Número de acertos idênticos ($C$ ou Hits).
    pub hits: usize,
    /// Total de palavras na referência ($N = S + D + C$).
    pub reference_words: usize,
    /// Total de palavras na hipótese ($M = S + I + C$).
    pub hypothesis_words: usize,
    /// Taxa de erro de palavras: $(S + D + I) / N$. Se $N = 0$, $0.0$ se hipótese vazia, senão $1.0$.
    pub wer_rate: f64,
}

/// Calcula o Word Error Rate (WER) entre o texto de referência e a hipótese transcrita.
///
/// Utiliza o algoritmo clássico de alinhamento por Programação Dinâmica (Levenshtein ponderado por palavras)
/// com backtracking exato para decompor substituições ($S$), exclusões ($D$) e inserções ($I$).
pub fn calculate_wer(reference: &str, hypothesis: &str) -> WerResult {
    let norm_ref = normalize_for_evaluation(reference);
    let norm_hyp = normalize_for_evaluation(hypothesis);

    let ref_words: Vec<&str> = norm_ref.split_whitespace().collect();
    let hyp_words: Vec<&str> = norm_hyp.split_whitespace().collect();

    let n = ref_words.len();
    let m = hyp_words.len();

    if n == 0 {
        return if m == 0 {
            WerResult {
                substitutions: 0,
                deletions: 0,
                insertions: 0,
                hits: 0,
                reference_words: 0,
                hypothesis_words: 0,
                wer_rate: 0.0,
            }
        } else {
            WerResult {
                substitutions: 0,
                deletions: 0,
                insertions: m,
                hits: 0,
                reference_words: 0,
                hypothesis_words: m,
                wer_rate: 1.0,
            }
        };
    }

    if m == 0 {
        return WerResult {
            substitutions: 0,
            deletions: n,
            insertions: 0,
            hits: 0,
            reference_words: n,
            hypothesis_words: 0,
            wer_rate: 1.0,
        };
    }

    // Matriz de Programação Dinâmica: dp[i][j] guarda o custo mínimo de edição
    let mut dp = vec![vec![0usize; m + 1]; n + 1];

    for (i, row) in dp.iter_mut().enumerate().take(n + 1) {
        row[0] = i;
    }
    for (j, cell) in dp[0].iter_mut().enumerate().take(m + 1) {
        *cell = j;
    }

    for i in 1..=n {
        for j in 1..=m {
            let cost_sub = if ref_words[i - 1] == hyp_words[j - 1] {
                0
            } else {
                1
            };
            let match_or_sub = dp[i - 1][j - 1] + cost_sub;
            let deletion = dp[i - 1][j] + 1;
            let insertion = dp[i][j - 1] + 1;
            dp[i][j] = match_or_sub.min(deletion).min(insertion);
        }
    }

    // Backtracking a partir de (n, m) até (0, 0) para classificar cada operação
    let mut i = n;
    let mut j = m;
    let mut substitutions = 0;
    let mut deletions = 0;
    let mut insertions = 0;
    let mut hits = 0;

    while i > 0 || j > 0 {
        if i > 0 && j > 0 {
            let cost_sub = if ref_words[i - 1] == hyp_words[j - 1] {
                0
            } else {
                1
            };
            if dp[i][j] == dp[i - 1][j - 1] + cost_sub {
                if cost_sub == 0 {
                    hits += 1;
                } else {
                    substitutions += 1;
                }
                i -= 1;
                j -= 1;
                continue;
            }
        }

        if i > 0 && dp[i][j] == dp[i - 1][j] + 1 {
            deletions += 1;
            i -= 1;
            continue;
        }

        if j > 0 && dp[i][j] == dp[i][j - 1] + 1 {
            insertions += 1;
            j -= 1;
            continue;
        }

        // Caso excepcional de contingência de alinhamento
        if i > 0 {
            deletions += 1;
            i -= 1;
        } else if j > 0 {
            insertions += 1;
            j -= 1;
        }
    }

    let wer_rate = (substitutions + deletions + insertions) as f64 / n as f64;

    WerResult {
        substitutions,
        deletions,
        insertions,
        hits,
        reference_words: n,
        hypothesis_words: m,
        wer_rate,
    }
}

/// Detalhes do cálculo de Character Error Rate (CER).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CerResult {
    /// Número de substituições de caractere ($S$).
    pub substitutions: usize,
    /// Número de exclusões de caractere ($D$).
    pub deletions: usize,
    /// Número de inserções de caractere ($I$).
    pub insertions: usize,
    /// Número de caracteres idênticos ($C$ ou Hits).
    pub hits: usize,
    /// Total de caracteres na referência ($N = S + D + C$).
    pub reference_chars: usize,
    /// Total de caracteres na hipótese.
    pub hypothesis_chars: usize,
    /// Taxa de erro de caracteres: $(S + D + I) / N$.
    pub cer_rate: f64,
}

/// Calcula o Character Error Rate (CER) entre referência e hipótese.
///
/// Avalia caracteres significativos (espaços removidos para focar em integridade fonêmica e ortográfica,
/// sendo o padrão da literatura para línguas orientais como japonês e para palavras compostas/acrônimos).
pub fn calculate_cer(reference: &str, hypothesis: &str) -> CerResult {
    let norm_ref = normalize_for_evaluation(reference);
    let norm_hyp = normalize_for_evaluation(hypothesis);

    let ref_chars: Vec<char> = norm_ref.chars().filter(|c| !c.is_whitespace()).collect();
    let hyp_chars: Vec<char> = norm_hyp.chars().filter(|c| !c.is_whitespace()).collect();

    let n = ref_chars.len();
    let m = hyp_chars.len();

    if n == 0 {
        return if m == 0 {
            CerResult {
                substitutions: 0,
                deletions: 0,
                insertions: 0,
                hits: 0,
                reference_chars: 0,
                hypothesis_chars: 0,
                cer_rate: 0.0,
            }
        } else {
            CerResult {
                substitutions: 0,
                deletions: 0,
                insertions: m,
                hits: 0,
                reference_chars: 0,
                hypothesis_chars: m,
                cer_rate: 1.0,
            }
        };
    }

    if m == 0 {
        return CerResult {
            substitutions: 0,
            deletions: n,
            insertions: 0,
            hits: 0,
            reference_chars: n,
            hypothesis_chars: 0,
            cer_rate: 1.0,
        };
    }

    let mut dp = vec![vec![0usize; m + 1]; n + 1];

    for (i, row) in dp.iter_mut().enumerate().take(n + 1) {
        row[0] = i;
    }
    for (j, cell) in dp[0].iter_mut().enumerate().take(m + 1) {
        *cell = j;
    }

    for i in 1..=n {
        for j in 1..=m {
            let cost_sub = if ref_chars[i - 1] == hyp_chars[j - 1] {
                0
            } else {
                1
            };
            let match_or_sub = dp[i - 1][j - 1] + cost_sub;
            let deletion = dp[i - 1][j] + 1;
            let insertion = dp[i][j - 1] + 1;
            dp[i][j] = match_or_sub.min(deletion).min(insertion);
        }
    }

    let mut i = n;
    let mut j = m;
    let mut substitutions = 0;
    let mut deletions = 0;
    let mut insertions = 0;
    let mut hits = 0;

    while i > 0 || j > 0 {
        if i > 0 && j > 0 {
            let cost_sub = if ref_chars[i - 1] == hyp_chars[j - 1] {
                0
            } else {
                1
            };
            if dp[i][j] == dp[i - 1][j - 1] + cost_sub {
                if cost_sub == 0 {
                    hits += 1;
                } else {
                    substitutions += 1;
                }
                i -= 1;
                j -= 1;
                continue;
            }
        }

        if i > 0 && dp[i][j] == dp[i - 1][j] + 1 {
            deletions += 1;
            i -= 1;
            continue;
        }

        if j > 0 && dp[i][j] == dp[i][j - 1] + 1 {
            insertions += 1;
            j -= 1;
            continue;
        }

        if i > 0 {
            deletions += 1;
            i -= 1;
        } else if j > 0 {
            insertions += 1;
            j -= 1;
        }
    }

    let cer_rate = (substitutions + deletions + insertions) as f64 / n as f64;

    CerResult {
        substitutions,
        deletions,
        insertions,
        hits,
        reference_chars: n,
        hypothesis_chars: m,
        cer_rate,
    }
}

/// Classificação da detecção de um termo importante no teste de fala.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyTermStatus {
    /// Presente na referência e detectado corretamente na hipótese.
    TruePositive,
    /// Presente na referência mas omitido ou incorreto na hipótese.
    FalseNegative,
    /// Ausente na referência mas inserido/alucinado na hipótese.
    FalsePositive,
    /// Ausente em ambos (relevante apenas se testado como contraprova).
    TrueNegative,
}

impl std::fmt::Display for KeyTermStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TruePositive => write!(f, "TP (Acerto)"),
            Self::FalseNegative => write!(f, "FN (Omissão/Erro)"),
            Self::FalsePositive => write!(f, "FP (Alucinação)"),
            Self::TrueNegative => write!(f, "TN (Ausente)"),
        }
    }
}

/// Registro de detecção de um termo individual.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KeyTermDetail {
    pub term: String,
    pub expected_in_reference: bool,
    pub found_in_hypothesis: bool,
    pub status: KeyTermStatus,
}

/// Resultado agregado da avaliação de termos importantes (Keyword Spotting).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KeyTermsResult {
    pub total_terms: usize,
    pub true_positives: usize,
    pub false_negatives: usize,
    pub false_positives: usize,
    pub true_negatives: usize,
    /// Acurácia dos termos esperados: $TP / (TP + FN)$. Se nenhum esperado, $1.0$ se $FP == 0$, senão $0.0$.
    pub accuracy: f64,
    pub details: Vec<KeyTermDetail>,
}

fn contains_word_sequence(haystack_norm: &str, needle_norm: &str) -> bool {
    let hay_words: Vec<&str> = haystack_norm.split_whitespace().collect();
    let needle_words: Vec<&str> = needle_norm.split_whitespace().collect();

    if needle_words.is_empty() {
        return true;
    }
    if needle_words.len() > hay_words.len() {
        return false;
    }

    hay_words
        .windows(needle_words.len())
        .any(|window| window == needle_words.as_slice())
}

/// Avalia a taxa de acerto de uma lista de termos críticos entre referência e hipótese.
pub fn evaluate_key_terms(reference: &str, hypothesis: &str, terms: &[String]) -> KeyTermsResult {
    let norm_ref = normalize_for_evaluation(reference);
    let norm_hyp = normalize_for_evaluation(hypothesis);

    let mut details = Vec::new();
    let mut tp = 0;
    let mut fn_count = 0;
    let mut fp = 0;
    let mut tn = 0;

    for raw_term in terms {
        let norm_term = normalize_for_evaluation(raw_term);
        if norm_term.is_empty() {
            continue;
        }

        let in_ref = contains_word_sequence(&norm_ref, &norm_term);
        let in_hyp = contains_word_sequence(&norm_hyp, &norm_term);

        let status = match (in_ref, in_hyp) {
            (true, true) => {
                tp += 1;
                KeyTermStatus::TruePositive
            }
            (true, false) => {
                fn_count += 1;
                KeyTermStatus::FalseNegative
            }
            (false, true) => {
                fp += 1;
                KeyTermStatus::FalsePositive
            }
            (false, false) => {
                tn += 1;
                KeyTermStatus::TrueNegative
            }
        };

        details.push(KeyTermDetail {
            term: raw_term.clone(),
            expected_in_reference: in_ref,
            found_in_hypothesis: in_hyp,
            status,
        });
    }

    let expected_total = tp + fn_count;
    let accuracy = if expected_total > 0 {
        tp as f64 / expected_total as f64
    } else if fp > 0 {
        0.0
    } else {
        1.0
    };

    KeyTermsResult {
        total_terms: details.len(),
        true_positives: tp,
        false_negatives: fn_count,
        false_positives: fp,
        true_negatives: tn,
        accuracy,
        details,
    }
}

/// Estatísticas de latência com suporte a repetições para mitigar dispersão e viés de cache.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LatencyStats {
    pub median_ms: f64,
    pub mean_ms: f64,
    pub std_dev_ms: f64,
    pub min_ms: f64,
    pub max_ms: f64,
    pub runs_count: usize,
    pub raw_runs_ms: Vec<f64>,
}

/// Calcula métricas de latência robustas (mediana, média, desvio padrão, min, max) a partir de uma série de execuções.
pub fn compute_latency_stats(runs_ms: &[f64]) -> LatencyStats {
    if runs_ms.is_empty() {
        return LatencyStats {
            median_ms: 0.0,
            mean_ms: 0.0,
            std_dev_ms: 0.0,
            min_ms: 0.0,
            max_ms: 0.0,
            runs_count: 0,
            raw_runs_ms: Vec::new(),
        };
    }

    let mut sorted = runs_ms.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let n = sorted.len();
    let median_ms = if n % 2 == 1 {
        sorted[n / 2]
    } else {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
    };

    let sum: f64 = sorted.iter().sum();
    let mean_ms = sum / n as f64;

    let std_dev_ms = if n >= 2 {
        let variance = sorted
            .iter()
            .map(|x| {
                let diff = x - mean_ms;
                diff * diff
            })
            .sum::<f64>()
            / (n - 1) as f64;
        variance.sqrt()
    } else {
        0.0
    };

    let min_ms = sorted[0];
    let max_ms = sorted[n - 1];

    LatencyStats {
        median_ms,
        mean_ms,
        std_dev_ms,
        min_ms,
        max_ms,
        runs_count: n,
        raw_runs_ms: runs_ms.to_vec(),
    }
}

/// Relatório de avaliação individual de um arquivo de áudio.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SingleEvaluationReport {
    pub test_case_id: String,
    pub audio_path: String,
    pub ground_truth: Option<String>,
    pub hypothesis: String,
    pub review_pending: bool,
    pub wer: Option<WerResult>,
    pub cer: Option<CerResult>,
    pub key_terms: Option<KeyTermsResult>,
    pub latency: LatencyStats,
    pub timings: SttTimingMetrics,
    pub decoding_config: SttDecodingConfigSummary,
}

/// Relatório de avaliação abrangente do Laboratório de Qualidade de Voz da Yuki.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorpusEvaluationReport {
    pub generated_at: String,
    pub manifest_path: Option<String>,
    pub total_cases: usize,
    pub evaluated_cases: usize,
    pub review_pending_cases: usize,
    pub average_wer: Option<f64>,
    pub average_cer: Option<f64>,
    pub overall_key_terms_accuracy: Option<f64>,
    pub average_latency_ms: f64,
    pub reports: Vec<SingleEvaluationReport>,
}

impl CorpusEvaluationReport {
    /// Exporta o relatório completo de avaliação em formato JSON legível.
    pub fn export_to_json(&self, path: impl AsRef<Path>) -> Result<(), VoiceError> {
        let p = path.as_ref();
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                VoiceError::Io(format!(
                    "Falha ao criar diretório para exportação do relatório '{}': {}",
                    parent.display(),
                    e
                ))
            })?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| {
            VoiceError::Io(format!("Falha ao serializar relatório de avaliação: {}", e))
        })?;
        std::fs::write(p, json).map_err(|e| {
            VoiceError::Io(format!(
                "Falha ao salvar relatório de avaliação em '{}': {}",
                p.display(),
                e
            ))
        })
    }
}

/// Executa a avaliação objetiva de um caso de teste individual utilizando o provedor local Whisper.
pub async fn evaluate_single_test_case(
    test_case: &CorpusTestCase,
    config: &LocalWhisperConfig,
    runs: usize,
) -> Result<SingleEvaluationReport, VoiceError> {
    let runs_count = runs.max(1);
    let audio_path = Path::new(&test_case.audio_path);

    let audio_bytes = std::fs::read(audio_path).map_err(|e| {
        VoiceError::Io(format!(
            "Não foi possível abrir o áudio '{}' do teste '{}': {}",
            test_case.audio_path, test_case.id, e
        ))
    })?;

    let (format, samples) = AudioCodec::decode_wav(&audio_bytes)?;
    let duration_ms = (samples.len() as f64 / format.sample_rate as f64 * 1000.0) as u64;

    let audio_input = AudioInput::new(
        audio_bytes,
        format,
        duration_ms,
        test_case.audio_path.clone(),
    );

    let mut latencies_ms = Vec::with_capacity(runs_count);
    let mut last_result: Option<LocalTranscriptionResult> = None;

    let provider = LocalWhisperSttProvider::new(config.clone());

    for _ in 0..runs_count {
        let t_start = std::time::Instant::now();
        let res = provider.transcribe_with_metadata(&audio_input).await?;
        let elapsed_ms = t_start.elapsed().as_secs_f64() * 1000.0;
        latencies_ms.push(elapsed_ms);
        last_result = Some(res);
    }

    let final_res = last_result.ok_or_else(|| {
        VoiceError::TranscriptionFailed("Nenhuma execução foi concluída".to_string())
    })?;

    let latency_stats = compute_latency_stats(&latencies_ms);

    let (wer, cer, key_terms) = if let Some(ref ref_text) = test_case.ground_truth {
        if !test_case.review_pending {
            let wer = calculate_wer(ref_text, &final_res.text);
            let cer = calculate_cer(ref_text, &final_res.text);
            let terms = if !test_case.important_terms.is_empty() {
                Some(evaluate_key_terms(
                    ref_text,
                    &final_res.text,
                    &test_case.important_terms,
                ))
            } else {
                None
            };
            (Some(wer), Some(cer), terms)
        } else {
            (None, None, None)
        }
    } else {
        (None, None, None)
    };

    Ok(SingleEvaluationReport {
        test_case_id: test_case.id.clone(),
        audio_path: test_case.audio_path.clone(),
        ground_truth: test_case.ground_truth.clone(),
        hypothesis: final_res.text,
        review_pending: test_case.review_pending,
        wer,
        cer,
        key_terms,
        latency: latency_stats,
        timings: final_res.timings,
        decoding_config: final_res.decoding_config,
    })
}

/// Avalia uma coleção de casos de teste do corpus e compila o relatório consolidado.
pub async fn evaluate_corpus(
    manifest: &VoiceCorpusManifest,
    config: &LocalWhisperConfig,
    runs: usize,
    category_filter: Option<CorpusCategory>,
) -> Result<CorpusEvaluationReport, VoiceError> {
    let now = now_utc().to_rfc3339();
    let mut reports = Vec::new();

    let target_cases: Vec<&CorpusTestCase> = match category_filter {
        Some(cat) => manifest.filter_by_category(cat),
        None => manifest.test_cases.iter().collect(),
    };

    for case in &target_cases {
        let single_rep = evaluate_single_test_case(case, config, runs).await?;
        reports.push(single_rep);
    }

    let total_cases = reports.len();
    let mut evaluated_cases = 0;
    let mut review_pending_cases = 0;
    let mut wer_sum = 0.0;
    let mut cer_sum = 0.0;
    let mut wer_count = 0;
    let mut cer_count = 0;
    let mut key_terms_acc_sum = 0.0;
    let mut key_terms_count = 0;
    let mut latency_sum = 0.0;

    for rep in &reports {
        if rep.review_pending {
            review_pending_cases += 1;
        } else if rep.ground_truth.is_some() {
            evaluated_cases += 1;
            if let Some(ref w) = rep.wer {
                wer_sum += w.wer_rate;
                wer_count += 1;
            }
            if let Some(ref c) = rep.cer {
                cer_sum += c.cer_rate;
                cer_count += 1;
            }
            if let Some(ref kt) = rep.key_terms {
                key_terms_acc_sum += kt.accuracy;
                key_terms_count += 1;
            }
        }
        latency_sum += rep.latency.median_ms;
    }

    let average_wer = if wer_count > 0 {
        Some(wer_sum / wer_count as f64)
    } else {
        None
    };

    let average_cer = if cer_count > 0 {
        Some(cer_sum / cer_count as f64)
    } else {
        None
    };

    let overall_key_terms_accuracy = if key_terms_count > 0 {
        Some(key_terms_acc_sum / key_terms_count as f64)
    } else {
        None
    };

    let average_latency_ms = if total_cases > 0 {
        latency_sum / total_cases as f64
    } else {
        0.0
    };

    Ok(CorpusEvaluationReport {
        generated_at: now,
        manifest_path: None,
        total_cases,
        evaluated_cases,
        review_pending_cases,
        average_wer,
        average_cer,
        overall_key_terms_accuracy,
        average_latency_ms,
        reports,
    })
}
