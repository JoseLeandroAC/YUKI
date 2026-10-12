use crate::contracts::missions::{
    Mission, MissionError, MissionExecutionPolicy, MissionPlan, MissionStep,
};

/// Planejador governado de missões.
///
/// DIRETRIZES FUNDAMENTAIS:
/// - O plano sugerido pelo modelo é DADO NÃO-CONFIÁVEL (`Data != Instruction`).
/// - Toda sugestão de plano é submetida a validação estrutural rígida antes de ser adotada.
/// - Impede grafos cíclicos, capacidades não permitidas e planos com excesso de passos.
pub struct MissionPlanner {
    default_policy: MissionExecutionPolicy,
}

impl Default for MissionPlanner {
    fn default() -> Self {
        Self::new(MissionExecutionPolicy::default())
    }
}

impl MissionPlanner {
    pub fn new(default_policy: MissionExecutionPolicy) -> Self {
        Self { default_policy }
    }

    /// Cria uma nova missão a partir de um objetivo, plano e operador.
    pub fn create_mission(
        &self,
        mission_id: impl Into<String>,
        title: impl Into<String>,
        objective: impl Into<String>,
        operator_id: impl Into<String>,
        correlation_id: impl Into<String>,
        plan: MissionPlan,
        policy_override: Option<MissionExecutionPolicy>,
    ) -> Result<Mission, MissionError> {
        let policy = policy_override.unwrap_or_else(|| self.default_policy.clone());

        // 1. Validação estrita do plano
        plan.validate(&policy)?;

        let mut mission = Mission::new(
            mission_id,
            title,
            objective,
            operator_id,
            correlation_id,
            policy,
        );

        mission.steps = plan.steps;
        Ok(mission)
    }

    /// Constrói o plano canônico demonstrável da primeira missão real do Marco 8:
    /// "Pesquisar documentação técnica e preparar um relatório com fontes."
    pub fn build_research_mission_plan(
        mission_id: &str,
        query: &str,
        target_file: &str,
    ) -> MissionPlan {
        let step1 = MissionStep::new(
            format!("{}_step_1", mission_id),
            mission_id,
            1,
            format!("Pesquisar documentação técnica soberana sobre '{}'", query),
            Some("research.search".to_string()),
            serde_json::json!({ "query": query, "max_results": 3 }),
            false, // Leitura não exige aprovação
            vec![],
        );

        let step2 = MissionStep::new(
            format!("{}_step_2", mission_id),
            mission_id,
            2,
            "Sintetizar achados técnicos com fontes e citações verificáveis",
            Some("system.echo".to_string()),
            serde_json::json!({ "message": format!("Relatório Técnico de Fontes para: {}", query) }),
            false, // Síntese interna não mutadora
            vec![1], // Depende do passo 1
        );

        let step3 = MissionStep::new(
            format!("{}_step_3", mission_id),
            mission_id,
            3,
            format!("Gravar relatório consolidado no arquivo '{}' do operador", target_file),
            Some("system.echo".to_string()),
            serde_json::json!({ "message": format!("Gravação aprovada no arquivo: {}", target_file) }),
            true, // Efeito externo mutador EXIGE aprovação humana!
            vec![2], // Depende do passo 2
        );

        MissionPlan::new(vec![step1, step2, step3])
    }
}
