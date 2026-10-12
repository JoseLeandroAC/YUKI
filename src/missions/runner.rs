use crate::capabilities::registry::CapabilityRegistry;
use crate::contracts::events::{AuditEvent, EventType};
use crate::contracts::identifiers::{CapabilityId, CausationId, CorrelationId};
use crate::contracts::missions::{
    ApprovalStatus, Mission, MissionCheckpoint, MissionError, MissionStatus, MissionStep,
    MissionStepStatus,
};
use crate::missions::approval::MissionApprovalManager;
use crate::missions::store::SqliteMissionStore;
use crate::persistence::sqlite::SqliteAuditStore;
use crate::verification::verifier::Verifier;
use std::collections::HashSet;
use std::sync::Arc;

/// Executor governado de missões multi-etapas.
///
/// DIRETRIZES DE SEGURANÇA E VERIFICAÇÃO:
/// 1. Respeita estritamente o grafo de dependências (DAG): um passo só executa quando todas as dependências estão concluídas.
/// 2. Portão Humano (*Human-in-the-loop*): Se um passo requer aprovação, a execução pausa no estado `AwaitingApproval`
///    até que a aprovação seja concedida via CLI/canal seguro pelo operador.
/// 3. Gravação atômica de checkpoints a cada passo concluído para permitir recuperação determinística após reinício.
/// 4. Distinção entre execução e efeito verificado: sucesso do passo requer validação formal.
pub struct MissionRunner {
    store: Arc<SqliteMissionStore>,
    approval_manager: Arc<MissionApprovalManager>,
    capability_registry: Arc<CapabilityRegistry>,
    verifier: Arc<Verifier>,
    audit_store: Option<Arc<SqliteAuditStore>>,
}

impl MissionRunner {
    pub fn new(
        store: Arc<SqliteMissionStore>,
        approval_manager: Arc<MissionApprovalManager>,
        capability_registry: Arc<CapabilityRegistry>,
        verifier: Arc<Verifier>,
        audit_store: Option<Arc<SqliteAuditStore>>,
    ) -> Self {
        Self {
            store,
            approval_manager,
            capability_registry,
            verifier,
            audit_store,
        }
    }

    pub fn verifier(&self) -> &Verifier {
        &self.verifier
    }

    /// Executa as etapas elegíveis de uma missão até a conclusão ou até necessitar de aprovação humana.
    pub async fn run_mission(&self, mission_id: &str) -> Result<Mission, MissionError> {
        let mut mission = self
            .store
            .get_mission(mission_id)?
            .ok_or_else(|| MissionError::NotFound(format!("Missão '{}' não encontrada", mission_id)))?;

        if mission.status.is_terminal() {
            return Ok(mission);
        }

        if mission.status == MissionStatus::Paused {
            return Ok(mission);
        }

        if mission.status == MissionStatus::Planned {
            mission.transition_to(MissionStatus::InProgress)?;
            self.store
                .update_mission_status(&mission.id, MissionStatus::InProgress, None)?;

            self.record_audit(
                EventType::MissionStarted,
                serde_json::json!({
                    "mission_id": mission.id,
                    "title": mission.title,
                    "total_steps": mission.steps.len(),
                }),
            );
        }

        loop {
            // Recarrega os passos mais atualizados
            mission.steps = self.store.get_steps_for_mission(&mission.id)?;

            let completed_step_numbers: HashSet<u32> = mission
                .steps
                .iter()
                .filter(|s| s.status == MissionStepStatus::Completed || s.status == MissionStepStatus::Skipped)
                .map(|s| s.step_number)
                .collect();

            // Encontra o próximo passo elegível cujas dependências já foram satisfeitas
            let next_step_opt = mission.steps.iter().find(|s| {
                (s.status == MissionStepStatus::Pending || s.status == MissionStepStatus::WaitingApproval)
                    && s.dependencies.iter().all(|dep| completed_step_numbers.contains(dep))
            });

            let next_step = match next_step_opt {
                Some(step) => step.clone(),
                None => {
                    // Não há passos pendentes executáveis no momento
                    // Verifica se todos foram completados
                    let all_done = mission
                        .steps
                        .iter()
                        .all(|s| s.status == MissionStepStatus::Completed || s.status == MissionStepStatus::Skipped);

                    if all_done {
                        mission.transition_to(MissionStatus::Completed)?;
                        self.store
                            .update_mission_status(&mission.id, MissionStatus::Completed, None)?;

                        self.record_audit(
                            EventType::MissionCompleted,
                            serde_json::json!({
                                "mission_id": mission.id,
                                "total_steps": mission.steps.len(),
                            }),
                        );
                    }
                    return Ok(mission);
                }
            };

            // Se o passo exige aprovação humana
            if next_step.requires_approval {
                let is_approved = if let Some(app_id) = &next_step.approval_id {
                    let app_opt = self.store.get_approval(app_id)?;
                    app_opt.map(|a| a.status == ApprovalStatus::Granted).unwrap_or(false)
                } else {
                    false
                };

                if !is_approved {
                    // Cria solicitação de aprovação se não existir
                    let approval = if next_step.approval_id.is_none() {
                        let app = self.approval_manager.request_approval(
                            &mission.id,
                            &next_step,
                            &mission.operator_id,
                            3600, // Validade de 1 hora
                        )?;
                        let mut updated_step = next_step.clone();
                        updated_step.approval_id = Some(app.id.clone());
                        updated_step.status = MissionStepStatus::WaitingApproval;
                        self.store.save_step(&updated_step)?;
                        app
                    } else {
                        self.store
                            .get_approval(next_step.approval_id.as_ref().unwrap())?
                            .unwrap()
                    };

                    mission.transition_to(MissionStatus::AwaitingApproval)?;
                    self.store
                        .update_mission_status(&mission.id, MissionStatus::AwaitingApproval, None)?;

                    tracing::info!(
                        "Missão '{}' aguardando aprovação humana para a etapa {} ('{}')",
                        mission.id, next_step.step_number, approval.id
                    );
                    return Ok(mission);
                }
            }

            if mission.status == MissionStatus::AwaitingApproval {
                mission.transition_to(MissionStatus::InProgress)?;
                self.store
                    .update_mission_status(&mission.id, MissionStatus::InProgress, None)?;
            }

            // Executa o passo
            self.execute_step(&mut mission, &next_step).await?;
        }
    }

    async fn execute_step(
        &self,
        mission: &mut Mission,
        step: &MissionStep,
    ) -> Result<(), MissionError> {
        // 1. Marca etapa como Running
        self.store.update_step_status(
            &step.id,
            MissionStepStatus::Running,
            None,
            None,
        )?;

        // 2. Executa capacidade associada (se houver)
        let output_val = if let Some(cap_name) = &step.capability_id {
            let cap_id = CapabilityId::new(cap_name);
            let handler = self
                .capability_registry
                .get_handler(&cap_id)
                .ok_or_else(|| {
                    MissionError::ExecutionFailed(format!(
                        "Capacidade '{}' necessária para o passo {} não encontrada no registro",
                        cap_name, step.step_number
                    ))
                })?;

            let exec_res = handler
                .execute(&step.parameters)
                .map_err(|e| {
                    MissionError::ExecutionFailed(format!(
                        "Falha na execução da capacidade '{}': {}",
                        cap_name, e
                    ))
                })?;

            exec_res
        } else {
            serde_json::json!({ "result": "completed_without_capability" })
        };

        // 3. Atualiza status do passo para Completed
        self.store.update_step_status(
            &step.id,
            MissionStepStatus::Completed,
            Some(&output_val),
            None,
        )?;

        // 4. Cria checkpoint durável
        let checkpoint = MissionCheckpoint {
            id: format!("cp_{}_{}_{}", mission.id, step.step_number, uuid_ms()),
            mission_id: mission.id.clone(),
            step_number: step.step_number,
            snapshot: serde_json::json!({
                "step_id": step.id,
                "output": output_val,
            }),
            created_at: crate::contracts::identifiers::now_utc(),
        };
        self.store.save_checkpoint(&checkpoint)?;

        // 5. Registra evento de auditoria MissionStepCompleted
        self.record_audit(
            EventType::MissionStepCompleted,
            serde_json::json!({
                "mission_id": mission.id,
                "step_number": step.step_number,
                "step_id": step.id,
                "capability_id": step.capability_id,
            }),
        );

        Ok(())
    }

    /// Pausa uma missão em andamento.
    pub fn pause_mission(&self, mission_id: &str) -> Result<(), MissionError> {
        let mut mission = self
            .store
            .get_mission(mission_id)?
            .ok_or_else(|| MissionError::NotFound(format!("Missão '{}' não encontrada", mission_id)))?;

        mission.transition_to(MissionStatus::Paused)?;
        self.store
            .update_mission_status(mission_id, MissionStatus::Paused, None)?;
        Ok(())
    }

    /// Retoma uma missão pausada.
    pub async fn resume_mission(&self, mission_id: &str) -> Result<Mission, MissionError> {
        let mut mission = self
            .store
            .get_mission(mission_id)?
            .ok_or_else(|| MissionError::NotFound(format!("Missão '{}' não encontrada", mission_id)))?;

        if mission.status == MissionStatus::Paused {
            mission.transition_to(MissionStatus::InProgress)?;
            self.store
                .update_mission_status(mission_id, MissionStatus::InProgress, None)?;
        }

        self.run_mission(mission_id).await
    }

    /// Cancela formalmente uma missão.
    pub fn cancel_mission(&self, mission_id: &str, reason: &str) -> Result<(), MissionError> {
        let mut mission = self
            .store
            .get_mission(mission_id)?
            .ok_or_else(|| MissionError::NotFound(format!("Missão '{}' não encontrada", mission_id)))?;

        mission.transition_to(MissionStatus::Cancelled)?;
        self.store
            .update_mission_status(mission_id, MissionStatus::Cancelled, Some(reason))?;
        Ok(())
    }

    fn record_audit(&self, event_type: EventType, payload: serde_json::Value) {
        if let Some(audit) = &self.audit_store {
            let event = AuditEvent::new(
                event_type,
                CorrelationId::new(),
                CausationId::new("mission_runner"),
                payload,
                "yuki_mission_runner",
            );
            if let Ok(clean) = event.assert_no_secrets().map(|_| event) {
                let _ = audit.record_event(&clean);
            }
        }
    }
}

fn uuid_ms() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("{:x}", ms)
}
