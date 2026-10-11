use crate::contracts::identifiers::now_utc;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Erros originados no subsistema de Memória da Yuki.
#[derive(Debug, Error)]
pub enum MemoryError {
    #[error("Memória com ID '{0}' não encontrada")]
    NotFound(String),

    #[error("Erro de persistência de memória: {0}")]
    Storage(String),

    #[error("Registro de memória inválido: {0}")]
    InvalidRecord(String),

    #[error("Conteúdo proibido detectado na memória (segredo ou padrão sensível): {0}")]
    ForbiddenContent(String),

    #[error("Violação de política de memória: {0}")]
    PolicyViolation(String),

    #[error("Erro de serialização de memória: {0}")]
    Serialization(String),
}

/// Identificador fortemente tipado de um registro de memória.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MemoryId(pub String);

impl MemoryId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn generate() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let pid = std::process::id();
        let rand_val: u32 = (nanos & 0xFFFF_FFFF) as u32 ^ pid;
        Self(format!("mem_{:x}_{:x}", nanos, rand_val))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for MemoryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Tipologia canônica de memórias (ADR-016).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryType {
    /// Fato estável sobre o usuário ou o mundo.
    Fact,
    /// Preferência explícita declarada pelo usuário.
    Preference,
    /// Resumo consolidado de interação ou evento relevante.
    Episode,
    /// Contexto operacional de projetos em andamento.
    ProjectContext,
    /// Diretriz comportamental ou instrução duradoura definida pelo usuário.
    Instruction,
}

impl MemoryType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MemoryType::Fact => "fact",
            MemoryType::Preference => "preference",
            MemoryType::Episode => "episode",
            MemoryType::ProjectContext => "project_context",
            MemoryType::Instruction => "instruction",
        }
    }

    pub fn from_str_canonical(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "fact" => Some(MemoryType::Fact),
            "preference" => Some(MemoryType::Preference),
            "episode" => Some(MemoryType::Episode),
            "project_context" | "project" => Some(MemoryType::ProjectContext),
            "instruction" => Some(MemoryType::Instruction),
            _ => None,
        }
    }
}

/// Origem epistemológica do registro de memória.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemorySource {
    /// Fornecido explicitamente e diretamente pelo usuário.
    UserDeclared,
    /// Inferido pelo modelo com base em interações ou reflexão.
    ModelInferred,
    /// Observado através de ferramentas, pesquisa ou integrações externas.
    ExternalObserved,
    /// Inicialização canônica ou configuração padrão da plataforma.
    SystemBootstrap,
}

impl MemorySource {
    pub fn as_str(&self) -> &'static str {
        match self {
            MemorySource::UserDeclared => "user_declared",
            MemorySource::ModelInferred => "model_inferred",
            MemorySource::ExternalObserved => "external_observed",
            MemorySource::SystemBootstrap => "system_bootstrap",
        }
    }

    pub fn from_str_canonical(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "user_declared" | "user" => Some(MemorySource::UserDeclared),
            "model_inferred" | "inferred" => Some(MemorySource::ModelInferred),
            "external_observed" | "observed" => Some(MemorySource::ExternalObserved),
            "system_bootstrap" | "system" => Some(MemorySource::SystemBootstrap),
            _ => None,
        }
    }
}

/// Grau tipado de certeza e confiança epistemológica da memória.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum MemoryConfidence {
    /// Inferência incerta ou preliminar.
    Low,
    /// Informação provável mas não confirmada pelo usuário.
    Medium,
    /// Afirmação clara do usuário ou observação de alta fidelidade.
    High,
    /// Fato comprovado por evidência ou explicitamente verificado.
    Verified,
}

impl MemoryConfidence {
    pub fn as_score(&self) -> f32 {
        match self {
            MemoryConfidence::Low => 0.3,
            MemoryConfidence::Medium => 0.6,
            MemoryConfidence::High => 0.9,
            MemoryConfidence::Verified => 1.0,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            MemoryConfidence::Low => "low",
            MemoryConfidence::Medium => "medium",
            MemoryConfidence::High => "high",
            MemoryConfidence::Verified => "verified",
        }
    }

    pub fn from_str_canonical(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "low" => Some(MemoryConfidence::Low),
            "medium" => Some(MemoryConfidence::Medium),
            "high" => Some(MemoryConfidence::High),
            "verified" => Some(MemoryConfidence::Verified),
            _ => None,
        }
    }
}

/// Ciclo de vida e estado de validade da memória.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryStatus {
    /// Memória ativa e elegível para recuperação de contexto.
    Active,
    /// Memória substituída por um registro mais recente ou específico.
    Superseded,
    /// Memória invalidada ou revogada pelo usuário ou política.
    Revoked,
    /// Memória retida historicamente, mas excluída do contexto operacional padrão.
    Archived,
}

impl MemoryStatus {
    pub fn is_active(&self) -> bool {
        matches!(self, MemoryStatus::Active)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            MemoryStatus::Active => "active",
            MemoryStatus::Superseded => "superseded",
            MemoryStatus::Revoked => "revoked",
            MemoryStatus::Archived => "archived",
        }
    }

    pub fn from_str_canonical(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "active" => Some(MemoryStatus::Active),
            "superseded" => Some(MemoryStatus::Superseded),
            "revoked" => Some(MemoryStatus::Revoked),
            "archived" => Some(MemoryStatus::Archived),
            _ => None,
        }
    }
}

/// Rastreabilidade completa da origem e evolução da memória.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryProvenance {
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub source_session_id: Option<String>,
    pub source_event_id: Option<String>,
    pub origin_context: Option<String>,
}

impl Default for MemoryProvenance {
    fn default() -> Self {
        let now = now_utc();
        Self {
            created_at: now,
            updated_at: now,
            source_session_id: None,
            source_event_id: None,
            origin_context: None,
        }
    }
}

/// Identificador padrão canônico do escopo do operador principal.
pub const DEFAULT_OPERATOR_OWNER_ID: &str = "operator:primary";

/// Registro persistido canônico da memória pessoal da Yuki.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryRecord {
    pub id: MemoryId,
    pub owner_id: String,
    pub memory_type: MemoryType,
    pub source: MemorySource,
    pub confidence: MemoryConfidence,
    pub status: MemoryStatus,
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub provenance: MemoryProvenance,
    pub superseded_by: Option<MemoryId>,
}

impl MemoryRecord {
    /// Cria um novo registro ativo de memória com escopo padrão do operador.
    pub fn new(
        memory_type: MemoryType,
        source: MemorySource,
        confidence: MemoryConfidence,
        title: impl Into<String>,
        content: impl Into<String>,
        tags: Vec<String>,
    ) -> Result<Self, MemoryError> {
        Self::new_with_owner(
            DEFAULT_OPERATOR_OWNER_ID,
            memory_type,
            source,
            confidence,
            title,
            content,
            tags,
        )
    }

    /// Cria um novo registro ativo de memória com escopo explícito de proprietário.
    pub fn new_with_owner(
        owner_id: impl Into<String>,
        memory_type: MemoryType,
        source: MemorySource,
        confidence: MemoryConfidence,
        title: impl Into<String>,
        content: impl Into<String>,
        tags: Vec<String>,
    ) -> Result<Self, MemoryError> {
        let id = MemoryId::generate();
        let owner_id = owner_id.into().trim().to_string();
        let title = title.into().trim().to_string();
        let content = content.into().trim().to_string();

        if owner_id.is_empty() {
            return Err(MemoryError::InvalidRecord(
                "Escopo de proprietário (owner_id) não pode ser vazio".to_string(),
            ));
        }

        if title.is_empty() {
            return Err(MemoryError::InvalidRecord(
                "Título da memória não pode ser vazio".to_string(),
            ));
        }

        if content.is_empty() {
            return Err(MemoryError::InvalidRecord(
                "Conteúdo da memória não pode ser vazio".to_string(),
            ));
        }

        let record = Self {
            id,
            owner_id,
            memory_type,
            source,
            confidence,
            status: MemoryStatus::Active,
            title,
            content,
            tags,
            provenance: MemoryProvenance::default(),
            superseded_by: None,
        };

        record.assert_no_secrets()?;
        Ok(record)
    }

    pub fn with_owner(mut self, owner_id: impl Into<String>) -> Self {
        self.owner_id = owner_id.into().trim().to_string();
        self
    }

    /// Verifica que a memória não contém senhas, chaves de API ou tokens.
    pub fn assert_no_secrets(&self) -> Result<(), MemoryError> {
        let text =
            format!("{} {} {}", self.title, self.content, self.tags.join(" ")).to_lowercase();
        let forbidden = [
            "password",
            "senha",
            "api_key",
            "apikey",
            "secret_key",
            "private_key",
            "bearer ",
            "access_token",
            "refresh_token",
            "vault.json",
        ];

        for pattern in &forbidden {
            if text.contains(pattern) {
                return Err(MemoryError::ForbiddenContent(format!(
                    "Padrão sensível/segredo detectado: '{}'",
                    pattern
                )));
            }
        }
        Ok(())
    }

    pub fn is_active(&self) -> bool {
        self.status.is_active()
    }

    pub fn revoke(&mut self) {
        self.status = MemoryStatus::Revoked;
        self.provenance.updated_at = now_utc();
    }

    pub fn archive(&mut self) {
        self.status = MemoryStatus::Archived;
        self.provenance.updated_at = now_utc();
    }

    pub fn supersede(&mut self, new_id: MemoryId) {
        self.status = MemoryStatus::Superseded;
        self.superseded_by = Some(new_id);
        self.provenance.updated_at = now_utc();
    }
}

/// Candidato a memória produzido por inferência ou reflexão do modelo.
///
/// INVARIANTE INEGOCIÁVEL:
/// Model Output != Memory != Persistence.
/// O modelo gera apenas MemoryCandidate; a escrita real é governada por política.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryCandidate {
    pub candidate_id: String,
    pub owner_id: String,
    pub memory_type: MemoryType,
    pub source: MemorySource,
    pub confidence: MemoryConfidence,
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub rationale: String,
    pub source_event_id: Option<String>,
}

impl MemoryCandidate {
    pub fn new(
        memory_type: MemoryType,
        source: MemorySource,
        confidence: MemoryConfidence,
        title: impl Into<String>,
        content: impl Into<String>,
        tags: Vec<String>,
        rationale: impl Into<String>,
    ) -> Self {
        Self::new_with_owner(
            DEFAULT_OPERATOR_OWNER_ID,
            memory_type,
            source,
            confidence,
            title,
            content,
            tags,
            rationale,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_with_owner(
        owner_id: impl Into<String>,
        memory_type: MemoryType,
        source: MemorySource,
        confidence: MemoryConfidence,
        title: impl Into<String>,
        content: impl Into<String>,
        tags: Vec<String>,
        rationale: impl Into<String>,
    ) -> Self {
        Self {
            candidate_id: MemoryId::generate().0,
            owner_id: owner_id.into().trim().to_string(),
            memory_type,
            source,
            confidence,
            title: title.into(),
            content: content.into(),
            tags,
            rationale: rationale.into(),
            source_event_id: None,
        }
    }

    pub fn with_owner(mut self, owner_id: impl Into<String>) -> Self {
        self.owner_id = owner_id.into().trim().to_string();
        self
    }

    /// Converte o candidato em um registro durável caso atenda às políticas sanitárias.
    pub fn to_record(&self) -> Result<MemoryRecord, MemoryError> {
        let mut record = MemoryRecord::new_with_owner(
            &self.owner_id,
            self.memory_type,
            self.source,
            self.confidence,
            &self.title,
            &self.content,
            self.tags.clone(),
        )?;
        record.provenance.source_event_id = self.source_event_id.clone();
        record.provenance.origin_context = Some(self.rationale.clone());
        Ok(record)
    }
}

/// Política governada de retenção, privacidade e validação de memórias.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryPolicy {
    /// Confiança mínima para aceitação automática de memórias inferidas.
    pub min_inferred_confidence: MemoryConfidence,
    /// Tamanho máximo de conteúdo em bytes por registro de memória.
    pub max_content_bytes: usize,
    /// Proibir explicitamente retenção de credenciais ou chaves.
    pub prohibit_secrets: bool,
    /// Proibir explicitamente gravação de dados íntimos de terceiros ou monitoramento clandestino.
    pub prohibit_intimate_surveillance: bool,
}

impl Default for MemoryPolicy {
    fn default() -> Self {
        Self {
            min_inferred_confidence: MemoryConfidence::Medium,
            max_content_bytes: 8192,
            prohibit_secrets: true,
            prohibit_intimate_surveillance: true,
        }
    }
}

impl MemoryPolicy {
    /// Avalia se um candidato pode ser aceito para persistência.
    pub fn evaluate_candidate(&self, candidate: &MemoryCandidate) -> Result<(), MemoryError> {
        if candidate.owner_id.trim().is_empty() {
            return Err(MemoryError::PolicyViolation(
                "Escopo de proprietário (owner_id) do candidato não pode ser vazio".to_string(),
            ));
        }

        if candidate.title.trim().is_empty() {
            return Err(MemoryError::PolicyViolation(
                "Título do candidato não pode ser vazio".to_string(),
            ));
        }

        if candidate.content.contains('\0') {
            return Err(MemoryError::ForbiddenContent(
                "Conteúdo contém bytes nulos inválidos".to_string(),
            ));
        }

        if candidate.content.len() > self.max_content_bytes {
            return Err(MemoryError::PolicyViolation(format!(
                "Conteúdo excede limite da política ({} > {} bytes)",
                candidate.content.len(),
                self.max_content_bytes
            )));
        }

        // Inferências devem satisfazer a confiança mínima da política
        if candidate.source == MemorySource::ModelInferred
            && candidate.confidence < self.min_inferred_confidence
        {
            return Err(MemoryError::PolicyViolation(format!(
                "Inferência do modelo possui confiança insuficiente ({:?} < {:?})",
                candidate.confidence, self.min_inferred_confidence
            )));
        }

        let combined = format!(
            "{} {} {}",
            candidate.title, candidate.content, candidate.rationale
        )
        .to_lowercase();

        // Sanitização de segredos e credenciais
        if self.prohibit_secrets {
            let forbidden = [
                "password",
                "senha",
                "api_key",
                "apikey",
                "secret_key",
                "private_key",
                "bearer ",
                "access_token",
                "refresh_token",
                "vault.json",
                "postgres://",
                "mysql://",
                "mongodb://",
                "redis://",
                "amqp://",
                "begin rsa private key",
                "begin openssh private key",
                "ssh-rsa ",
                "cartão de crédito",
                "cartao de credito",
                "credit_card",
                "cvv",
                "seed phrase",
                "mnemonic phrase",
                "auth_token",
                "authorization: ",
            ];
            for pattern in &forbidden {
                if combined.contains(pattern) {
                    return Err(MemoryError::ForbiddenContent(format!(
                        "Padrão sensível detectado no candidato a memória: '{}'",
                        pattern
                    )));
                }
            }
        }

        // Proteção contra gravação de atividades íntimas clandestinas e invasão de privacidade
        if self.prohibit_intimate_surveillance {
            let forbidden_privacy = [
                "vigilância íntima",
                "vigilancia intima",
                "câmera oculta",
                "camera oculta",
                "câmera oculta no quarto",
                "camera oculta no quarto",
                "monitorar momento íntimo",
                "monitorar momento intimo",
                "gravação oculta",
                "gravacao oculta",
                "gravação oculta de terceiros",
                "gravacao oculta de terceiros",
                "gravando intimidade",
                "gravação íntima",
                "gravacao intima",
                "espionar",
                "espionar terceiros",
                "intimidade dos moradores",
                "banheiro",
            ];
            for pattern in &forbidden_privacy {
                if combined.contains(pattern) {
                    return Err(MemoryError::ForbiddenContent(format!(
                        "Violação de privacidade ou intimidade detectada no candidato a memória: '{}'",
                        pattern
                    )));
                }
            }
        }

        Ok(())
    }
}

/// Parâmetros de consulta e recuperação no repositório de memórias.
#[derive(Debug, Clone, Default)]
pub struct MemoryQuery {
    pub owner_id: Option<String>,
    pub text_query: Option<String>,
    pub memory_types: Option<Vec<MemoryType>>,
    pub sources: Option<Vec<MemorySource>>,
    pub statuses: Option<Vec<MemoryStatus>>,
    pub min_confidence: Option<MemoryConfidence>,
    pub tags: Option<Vec<String>>,
    pub limit: usize,
    pub offset: usize,
}

impl MemoryQuery {
    pub fn new() -> Self {
        Self {
            limit: 20,
            ..Default::default()
        }
    }

    pub fn with_owner(mut self, owner_id: impl Into<String>) -> Self {
        self.owner_id = Some(owner_id.into().trim().to_string());
        self
    }

    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = offset;
        self
    }

    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.text_query = Some(text.into());
        self
    }

    pub fn with_type(mut self, mem_type: MemoryType) -> Self {
        self.memory_types = Some(vec![mem_type]);
        self
    }

    pub fn with_active_only(mut self) -> Self {
        self.statuses = Some(vec![MemoryStatus::Active]);
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }
}

/// Trait canônica para armazenamento durável de memórias.
pub trait MemoryRepository: Send + Sync {
    /// Insere ou atualiza um registro de memória.
    fn store(&self, record: &MemoryRecord) -> Result<(), MemoryError>;

    /// Obtém um registro por seu MemoryId.
    fn get_by_id(&self, id: &MemoryId) -> Result<Option<MemoryRecord>, MemoryError>;

    /// Marca uma memória como revogada pelo usuário.
    fn revoke(&self, id: &MemoryId) -> Result<(), MemoryError>;

    /// Marca uma memória como arquivada.
    fn archive(&self, id: &MemoryId) -> Result<(), MemoryError>;

    /// Exclusão física permanente (direito ao esquecimento).
    fn delete_permanently(&self, id: &MemoryId) -> Result<(), MemoryError>;

    /// Executa uma consulta filtrada contra o repositório.
    fn query(&self, query: &MemoryQuery) -> Result<Vec<MemoryRecord>, MemoryError>;

    /// Contabiliza registros correspondentes aos critérios de consulta.
    fn count(&self, query: &MemoryQuery) -> Result<usize, MemoryError>;
}

/// Trait canônica para seleção e recuperação semântica/textual de memórias para o contexto.
pub trait MemoryRetriever: Send + Sync {
    /// Recupera memórias relevantes para uma consulta textual governada por orçamento.
    fn retrieve(&self, query: &MemoryQuery) -> Result<Vec<MemoryRecord>, MemoryError>;

    /// Seleciona memórias ativas relevantes para um prompt de entrada do usuário.
    fn retrieve_for_context(
        &self,
        input_text: &str,
        limit: usize,
    ) -> Result<Vec<MemoryRecord>, MemoryError>;
}
