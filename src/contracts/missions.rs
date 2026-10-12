use crate::contracts::identifiers::now_utc;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;

/// Estados canônicos do ciclo de vida de uma Missão.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MissionStatus {
    Planned,
    InProgress,
    AwaitingApproval,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

impl fmt::Display for MissionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Planned => write!(f, "PLANNED"),
            Self::InProgress => write!(f, "IN_PROGRESS"),
            Self::AwaitingApproval => write!(f, "AWAITING_APPROVAL"),
            Self::Paused => write!(f, "PAUSED"),
            Self::Completed => write!(f, "COMPLETED"),
            Self::Failed => write!(f, "FAILED"),
            Self::Cancelled => write!(f, "CANCELLED"),
        }
    }
}

impl MissionStatus {
    pub fn can_transition_to(&self, next: &MissionStatus) -> bool {
        use MissionStatus::*;
        match (self, next) {
            // Qualquer estado não terminal pode ser Cancelado ou Falhar
            (Planned, Cancelled) | (InProgress, Cancelled) | (AwaitingApproval, Cancelled) | (Paused, Cancelled) => true,
            (Planned, Failed) | (InProgress, Failed) | (AwaitingApproval, Failed) | (Paused, Failed) => true,

            // Planned -> InProgress ou Paused
            (Planned, InProgress) => true,
            (Planned, Paused) => true,

            // InProgress -> AwaitingApproval ou Paused
            (InProgress, AwaitingApproval) => true,
            (InProgress, Paused) => true,

            // AwaitingApproval -> InProgress (quando aprovado) ou Paused
            (AwaitingApproval, InProgress) => true,
            (AwaitingApproval, Paused) => true,

            // Paused -> InProgress (retomada)
            (Paused, InProgress) => true,

            // InProgress -> Completed (sucesso de todos os passos)
            (InProgress, Completed) => true,

            // Estados terminais não transitam para outros estados
            (Completed, _) | (Failed, _) | (Cancelled, _) => false,

            _ => false,
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

/// Estados do ciclo de vida de uma Etapa (Step) de missão.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MissionStepStatus {
    Pending,
    Running,
    WaitingApproval,
    Completed,
    Failed,
    Skipped,
}

impl fmt::Display for MissionStepStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "PENDING"),
            Self::Running => write!(f, "RUNNING"),
            Self::WaitingApproval => write!(f, "WAITING_APPROVAL"),
            Self::Completed => write!(f, "COMPLETED"),
            Self::Failed => write!(f, "FAILED"),
            Self::Skipped => write!(f, "SKIPPED"),
        }
    }
}

impl MissionStepStatus {
    pub fn can_transition_to(&self, next: &MissionStepStatus) -> bool {
        use MissionStepStatus::*;
        match (self, next) {
            (Pending, Running) => true,
            (Pending, WaitingApproval) => true,
            (Pending, Skipped) => true,
            (WaitingApproval, Running) => true,
            (WaitingApproval, Skipped) => true,
            (WaitingApproval, Failed) => true,
            (Running, Completed) => true,
            (Running, Failed) => true,
            (Running, WaitingApproval) => true,
            _ => false,
        }
    }
}

/// Política de governança e limites de execução de uma missão.
///
/// INVARIANTE: O planejador (LLM) NUNCA pode expandir seus próprios limites.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionExecutionPolicy {
    pub max_steps: u32,
    pub max_duration_secs: u64,
    pub max_retries_per_step: u32,
    pub require_approval_for_mutations: bool,
    pub allowed_capabilities: Vec<String>,
}

impl Default for MissionExecutionPolicy {
    fn default() -> Self {
        Self {
            max_steps: 15,
            max_duration_secs: 3600, // 1 hora
            max_retries_per_step: 2,
            require_approval_for_mutations: true,
            allowed_capabilities: vec![
                "system.echo".to_string(),
                "system.time".to_string(),
                "system.info".to_string(),
                "research.search".to_string(),
                "research.fetch".to_string(),
                "research.synthesize".to_string(),
                "native.search".to_string(),
            ],
        }
    }
}

/// Etapa individual de uma missão.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionStep {
    pub id: String,
    pub mission_id: String,
    pub step_number: u32,
    pub description: String,
    pub capability_id: Option<String>,
    pub parameters: serde_json::Value,
    pub requires_approval: bool,
    pub status: MissionStepStatus,
    pub dependencies: Vec<u32>,
    pub output_payload: Option<serde_json::Value>,
    pub approval_id: Option<String>,
    pub failure_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl MissionStep {
    pub fn new(
        id: impl Into<String>,
        mission_id: impl Into<String>,
        step_number: u32,
        description: impl Into<String>,
        capability_id: Option<String>,
        parameters: serde_json::Value,
        requires_approval: bool,
        dependencies: Vec<u32>,
    ) -> Self {
        let now = now_utc();
        Self {
            id: id.into(),
            mission_id: mission_id.into(),
            step_number,
            description: description.into(),
            capability_id,
            parameters,
            requires_approval,
            status: MissionStepStatus::Pending,
            dependencies,
            output_payload: None,
            approval_id: None,
            failure_reason: None,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Estado de autorização de uma aprovação humana pendente ou concedida.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalStatus {
    Pending,
    Granted,
    Denied,
    Expired,
}

/// Registro estrito de aprovação humana vinculada.
///
/// INVARIANTE INEGOCIÁVEL:
/// Uma aprovação é vinculada criptograficamente ao hash dos parâmetros, à missão,
/// à etapa, à capacidade e ao operador. Qualquer divergência invalida a aprovação.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionApproval {
    pub id: String,
    pub mission_id: String,
    pub step_id: String,
    pub capability_id: String,
    pub operator_id: String,
    pub parameters_hash: String,
    pub status: ApprovalStatus,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

impl MissionApproval {
    pub fn new(
        id: impl Into<String>,
        mission_id: impl Into<String>,
        step_id: impl Into<String>,
        capability_id: impl Into<String>,
        operator_id: impl Into<String>,
        parameters: &serde_json::Value,
        validity_secs: u64,
    ) -> Self {
        let now = now_utc();
        let expires_at = now + chrono::Duration::seconds(validity_secs as i64);
        let parameters_hash = Self::compute_hash(parameters);

        Self {
            id: id.into(),
            mission_id: mission_id.into(),
            step_id: step_id.into(),
            capability_id: capability_id.into(),
            operator_id: operator_id.into(),
            parameters_hash,
            status: ApprovalStatus::Pending,
            created_at: now,
            expires_at,
        }
    }

    pub fn compute_hash(parameters: &serde_json::Value) -> String {
        let serialized = serde_json::to_string(parameters).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(serialized.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    pub fn is_valid_for(
        &self,
        operator_id: &str,
        mission_id: &str,
        step_id: &str,
        capability_id: &str,
        parameters: &serde_json::Value,
    ) -> bool {
        if self.status != ApprovalStatus::Granted {
            return false;
        }

        if now_utc() > self.expires_at {
            return false;
        }

        let expected_hash = Self::compute_hash(parameters);
        self.operator_id == operator_id
            && self.mission_id == mission_id
            && self.step_id == step_id
            && self.capability_id == capability_id
            && self.parameters_hash == expected_hash
    }
}

/// Ponto de checagem transacional para recuperação após falhas ou reinicializações.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionCheckpoint {
    pub id: String,
    pub mission_id: String,
    pub step_number: u32,
    pub snapshot: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

/// Entidade raiz de uma Missão na Yuki.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mission {
    pub id: String,
    pub title: String,
    pub objective: String,
    pub status: MissionStatus,
    pub steps: Vec<MissionStep>,
    pub policy: MissionExecutionPolicy,
    pub operator_id: String,
    pub correlation_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub failure_reason: Option<String>,
}

impl Mission {
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        objective: impl Into<String>,
        operator_id: impl Into<String>,
        correlation_id: impl Into<String>,
        policy: MissionExecutionPolicy,
    ) -> Self {
        let now = now_utc();
        Self {
            id: id.into(),
            title: title.into(),
            objective: objective.into(),
            status: MissionStatus::Planned,
            steps: Vec::new(),
            policy,
            operator_id: operator_id.into(),
            correlation_id: correlation_id.into(),
            created_at: now,
            updated_at: now,
            completed_at: None,
            failure_reason: None,
        }
    }

    pub fn transition_to(&mut self, next_status: MissionStatus) -> Result<(), MissionError> {
        if !self.status.can_transition_to(&next_status) {
            return Err(MissionError::InvalidStateTransition {
                from: self.status.to_string(),
                to: next_status.to_string(),
            });
        }
        self.status = next_status;
        self.updated_at = now_utc();
        if self.status == MissionStatus::Completed {
            self.completed_at = Some(self.updated_at);
        }
        Ok(())
    }
}

/// Plano de missão sugerido ou validado com ordenação topológica (DAG).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionPlan {
    pub steps: Vec<MissionStep>,
}

impl MissionPlan {
    pub fn new(steps: Vec<MissionStep>) -> Self {
        Self { steps }
    }

    /// Valida integridade do plano: limites de passos, capacidades permitidas e aciclicidade do grafo.
    pub fn validate(&self, policy: &MissionExecutionPolicy) -> Result<(), MissionError> {
        if self.steps.is_empty() {
            return Err(MissionError::PlanValidationFailed(
                "O plano de missão não possui etapas".to_string(),
            ));
        }

        if self.steps.len() > policy.max_steps as usize {
            return Err(MissionError::StepLimitExceeded {
                count: self.steps.len() as u32,
                limit: policy.max_steps,
            });
        }

        // Validação de capacidades
        for step in &self.steps {
            if let Some(cap) = &step.capability_id {
                if !policy.allowed_capabilities.contains(cap) {
                    return Err(MissionError::PlanValidationFailed(format!(
                        "Etapa {} requer capacidade não permitida pela política da missão: '{}'",
                        step.step_number, cap
                    )));
                }
            }
        }

        // Validação de dependências e ciclos (DAG acíclico)
        self.verify_acyclic()?;

        Ok(())
    }

    fn verify_acyclic(&self) -> Result<(), MissionError> {
        use std::collections::{HashMap, HashSet};

        let mut step_numbers = HashSet::new();
        let mut adj: HashMap<u32, Vec<u32>> = HashMap::new();

        for s in &self.steps {
            step_numbers.insert(s.step_number);
            adj.entry(s.step_number).or_default();
            for &dep in &s.dependencies {
                if dep == s.step_number {
                    return Err(MissionError::CircularDependencyDetected(format!(
                        "Etapa {} depende de si mesma",
                        s.step_number
                    )));
                }
                adj.entry(dep).or_default().push(s.step_number);
            }
        }

        // Verifica dependências inexistentes
        for s in &self.steps {
            for &dep in &s.dependencies {
                if !step_numbers.contains(&dep) {
                    return Err(MissionError::PlanValidationFailed(format!(
                        "Etapa {} referencia dependência inexistente: {}",
                        s.step_number, dep
                    )));
                }
            }
        }

        // Detecção de ciclo via DFS
        let mut visited = HashMap::new(); // 0 = unvisited, 1 = visiting, 2 = visited
        for &node in &step_numbers {
            visited.insert(node, 0);
        }

        fn dfs(
            node: u32,
            adj: &HashMap<u32, Vec<u32>>,
            visited: &mut HashMap<u32, i32>,
        ) -> bool {
            visited.insert(node, 1);
            if let Some(neighbors) = adj.get(&node) {
                for &neighbor in neighbors {
                    let state = visited.get(&neighbor).copied().unwrap_or(0);
                    if state == 1 {
                        return true; // Ciclo encontrado!
                    }
                    if state == 0 && dfs(neighbor, adj, visited) {
                        return true;
                    }
                }
            }
            visited.insert(node, 2);
            false
        }

        for &node in &step_numbers {
            if visited.get(&node) == Some(&0) && dfs(node, &adj, &mut visited) {
                return Err(MissionError::CircularDependencyDetected(
                    "Ciclo detectado nas dependências entre as etapas do plano".to_string(),
                ));
            }
        }

        Ok(())
    }
}

/// Erros tipados do subsistema de Missões.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MissionError {
    #[error("Transição de estado de missão inválida: de {from} para {to}")]
    InvalidStateTransition { from: String, to: String },

    #[error("Plano de missão inválido: {0}")]
    PlanValidationFailed(String),

    #[error("Dependência circular detectada no plano da missão: {0}")]
    CircularDependencyDetected(String),

    #[error("Quantidade de etapas ({count}) excede o limite configurado ({limit})")]
    StepLimitExceeded { count: u32, limit: u32 },

    #[error("Aprovação humana obrigatória não fornecida para etapa '{step_id}' e capacidade '{capability_id}'")]
    ApprovalRequired {
        step_id: String,
        capability_id: String,
    },

    #[error("Aprovação humana expirada ({0})")]
    ApprovalExpired(String),

    #[error("Divergência nos parâmetros da aprovação humana: {0}")]
    ApprovalMismatch(String),

    #[error("Falha na execução da etapa da missão: {0}")]
    ExecutionFailed(String),

    #[error("Falha na verificação do efeito da etapa da missão: {0}")]
    VerificationFailed(String),

    #[error("Erro de persistência de missões: {0}")]
    Storage(String),

    #[error("Missão ou etapa não encontrada: {0}")]
    NotFound(String),

    #[error("Missão foi cancelada pelo operador")]
    Cancelled,

    #[error("Missão foi interrompida")]
    Interrupted,
}
