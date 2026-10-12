use crate::capabilities::registry::CapabilityRegistry;
use crate::contracts::capability::{RiskClass, SideEffects};
use crate::contracts::events::{AuditEvent, EventType};
use crate::contracts::execution::{ExecutionResult, OperationState};
use crate::contracts::identifiers::{
    now_utc, AttemptId, CapabilityId, CausationId, CorrelationId, OperationId,
};
use crate::contracts::missions::{
    ApprovalStatus, Mission, MissionCheckpoint, MissionError, MissionStatus, MissionStep,
    MissionStepStatus,
};
use crate::contracts::verification::VerificationState;
use crate::missions::approval::MissionApprovalManager;
use crate::missions::store::SqliteMissionStore;
use crate::persistence::sqlite::SqliteAuditStore;
use crate::verification::strategy::VerificationContext;
use crate::verification::verifier::Verifier;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

/// Guardião RAII para lease exclusivo por missão.
struct MissionLeaseGuard {
    leases: Arc<Mutex<HashSet<String>>>,
    mission_id: String,
}

impl Drop for MissionLeaseGuard {
    fn drop(&mut self) {
        if let Ok(mut l) = self.leases.lock() {
            l.remove(&self.mission_id);
        }
    }
}

/// Executor governado de missões multi-etapas.
///
/// DIRETRIZES DE SEGURANÇA E VERIFICAÇÃO:
/// 1. Respeita estritamente o grafo de dependências (DAG): um passo só executa quando todas as dependências estão concluídas.
/// 2. Portão Humano (*Human-in-the-loop*): Se um passo requer aprovação soberana (por plano ou classe de risco da capacidade),
///    a execução pausa no estado `AwaitingApproval` até que a aprovação seja concedida via CLI/canal seguro pelo operador.
/// 3. Anti-replay criptográfico: A aprovação validada é imediatamente consumida (`Consumed`).
/// 4. Gravação atômica de checkpoints a cada passo concluído para permitir recuperação determinística após reinício.
/// 5. Distinção entre execução e efeito verificado: sucesso do passo requer validação formal via `Verifier`.
/// 6. Lease de exclusão mútua por missão para prevenir execuções concorrentes acidentais.
pub struct MissionRunner {
    store: Arc<SqliteMissionStore>,
    approval_manager: Arc<MissionApprovalManager>,
    capability_registry: Arc<CapabilityRegistry>,
    verifier: Arc<Verifier>,
    audit_store: Option<Arc<SqliteAuditStore>>,
    active_leases: Arc<Mutex<HashSet<String>>>,
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
            active_leases: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub fn verifier(&self) -> &Verifier {
        &self.verifier
    }

    /// Determina soberanamente se a etapa requer aprovação humana obrigatória.
    ///
    /// INVARIANTE INEGOCIÁVEL (Model Output != Command != Authorization):
    /// O plano gerado pelo modelo NUNCA pode dispensar aprovação para capacidades
    /// de alto risco (`RiskClass >= High`) ou operações com efeito colateral mutador
    /// se a política da missão exigir aprovação para mutações.
    fn step_requires_approval(&self, mission: &Mission, step: &MissionStep) -> bool {
        // 1. Se o próprio plano declarou explicitamente que requer aprovação
        if step.requires_approval {
            return true;
        }

        // 2. Independência de modelo: inspeciona os metadados soberanos da capacidade no registro
        if let Some(cap_name) = &step.capability_id {
            let cap_id = CapabilityId::new(cap_name);
            if let Some(manifest) = self.capability_registry.get_manifest(&cap_id) {
                // Risco Alto ou Crítico sempre requer portão humano obrigatório
                if manifest.risk_class >= RiskClass::High {
                    return true;
                }
                // Capacidades com efeito colateral mutador requerem aprovação se a política exigir
                if mission.policy.require_approval_for_mutations
                    && manifest.side_effects != SideEffects::None
                {
                    return true;
                }
            }
        }

        false
    }

    /// Executa as etapas elegíveis de uma missão até a conclusão ou até necessitar de aprovação humana.
    pub async fn run_mission(&self, mission_id: &str) -> Result<Mission, MissionError> {
        // Exclusão mútua / lease por missão (MISS-04)
        let _lease_guard = {
            let mut leases = self.active_leases.lock().map_err(|_| {
                MissionError::Storage("Lock de lease de missão envenenado".to_string())
            })?;
            if leases.contains(mission_id) {
                return Err(MissionError::ExecutionFailed(format!(
                    "Missão '{}' já possui uma execução ativa em andamento (lease concorrente bloqueado)",
                    mission_id
                )));
            }
            leases.insert(mission_id.to_string());
            MissionLeaseGuard {
                leases: Arc::clone(&self.active_leases),
                mission_id: mission_id.to_string(),
            }
        };

        let mut mission = self.store.get_mission(mission_id)?.ok_or_else(|| {
            MissionError::NotFound(format!("Missão '{}' não encontrada", mission_id))
        })?;

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
                .filter(|s| {
                    s.status == MissionStepStatus::Completed
                        || s.status == MissionStepStatus::Skipped
                })
                .map(|s| s.step_number)
                .collect();

            // Encontra o próximo passo elegível cujas dependências já foram satisfeitas
            let next_step_opt = mission.steps.iter().find(|s| {
                (s.status == MissionStepStatus::Pending
                    || s.status == MissionStepStatus::WaitingApproval)
                    && s.dependencies
                        .iter()
                        .all(|dep| completed_step_numbers.contains(dep))
            });

            let next_step = match next_step_opt {
                Some(step) => step.clone(),
                None => {
                    // Não há passos pendentes executáveis no momento
                    // Verifica se todos foram completados
                    let all_done = mission.steps.iter().all(|s| {
                        s.status == MissionStepStatus::Completed
                            || s.status == MissionStepStatus::Skipped
                    });

                    if all_done {
                        mission.transition_to(MissionStatus::Completed)?;
                        self.store.update_mission_status(
                            &mission.id,
                            MissionStatus::Completed,
                            None,
                        )?;

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

            // Se o passo exige aprovação humana soberana (por plano ou por governança de risco)
            let step_needs_approval = self.step_requires_approval(&mission, &next_step);

            if step_needs_approval {
                let is_approved = if let Some(app_id) = &next_step.approval_id {
                    let app_opt = self.store.get_approval(app_id)?;
                    match app_opt {
                        Some(ref app) => {
                            let cap_name = next_step.capability_id.as_deref().unwrap_or("unknown");
                            if app.is_valid_for(
                                &mission.operator_id,
                                &mission.id,
                                &next_step.id,
                                cap_name,
                                &next_step.parameters,
                            ) {
                                // Anti-replay: consome a aprovação para que não possa ser reutilizada
                                self.store
                                    .update_approval_status(&app.id, ApprovalStatus::Consumed)?;
                                true
                            } else {
                                false
                            }
                        }
                        None => false,
                    }
                } else {
                    false
                };

                if !is_approved {
                    // Cria solicitação de aprovação se não existir ou se a anterior não é mais válida
                    let approval = match &next_step.approval_id {
                        None => {
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
                        }
                        Some(app_id) => {
                            let existing = self.store.get_approval(app_id)?;
                            match existing {
                                Some(app) if app.status == ApprovalStatus::Pending => app,
                                _ => {
                                    let app = self.approval_manager.request_approval(
                                        &mission.id,
                                        &next_step,
                                        &mission.operator_id,
                                        3600,
                                    )?;
                                    let mut updated_step = next_step.clone();
                                    updated_step.approval_id = Some(app.id.clone());
                                    updated_step.status = MissionStepStatus::WaitingApproval;
                                    self.store.save_step(&updated_step)?;
                                    app
                                }
                            }
                        }
                    };

                    mission.transition_to(MissionStatus::AwaitingApproval)?;
                    self.store.update_mission_status(
                        &mission.id,
                        MissionStatus::AwaitingApproval,
                        None,
                    )?;

                    tracing::info!(
                        "Missão '{}' aguardando aprovação humana para a etapa {} ('{}')",
                        mission.id,
                        next_step.step_number,
                        approval.id
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
        self.store
            .update_step_status(&step.id, MissionStepStatus::Running, None, None)?;

        // 2. Executa capacidade associada (se houver)
        let (output_val, verification_res) = if let Some(cap_name) = &step.capability_id {
            let cap_id = CapabilityId::new(cap_name);
            let handler = self
                .capability_registry
                .get_handler(&cap_id)
                .ok_or_else(|| {
                    let err_msg = format!(
                        "Capacidade '{}' necessária para o passo {} não encontrada no registro",
                        cap_name, step.step_number
                    );
                    let _ = self.store.update_step_status(
                        &step.id,
                        MissionStepStatus::Failed,
                        None,
                        Some(&err_msg),
                    );
                    MissionError::ExecutionFailed(err_msg)
                })?;

            let exec_res = match handler.execute(&step.parameters) {
                Ok(val) => val,
                Err(e) => {
                    let err_msg = format!(
                        "Falha na execução da capacidade '{}': {}",
                        cap_name, e
                    );
                    let _ = self.store.update_step_status(
                        &step.id,
                        MissionStepStatus::Failed,
                        None,
                        Some(&err_msg),
                    );
                    return Err(MissionError::ExecutionFailed(err_msg));
                }
            };

            // Verificação formal de efeito com Verifier (ADR-009 / MISS-07)
            let op_id = OperationId::new();
            let execution_contract = ExecutionResult {
                operation_id: op_id.clone(),
                attempt_id: AttemptId::new(),
                state: OperationState::Completed,
                output: Some(exec_res.clone()),
                error: None,
                executed_at: now_utc(),
            };
            let v_context = VerificationContext::new(
                &op_id,
                &cap_id,
                &execution_contract,
                &[], // Sem evidências externas não confiáveis a priori
            );
            let v_res = self.verifier.verify_operation(&v_context);

            if v_res.verification_state == VerificationState::VerifiedFailure {
                let err_msg = format!(
                    "Verificação de efeito falhou para a capacidade '{}': {}",
                    cap_name, v_res.verification_basis
                );
                let _ = self.store.update_step_status(
                    &step.id,
                    MissionStepStatus::Failed,
                    Some(&exec_res),
                    Some(&err_msg),
                );
                return Err(MissionError::VerificationFailed(err_msg));
            }

            (exec_res, Some(v_res))
        } else {
            (
                serde_json::json!({ "result": "completed_without_capability" }),
                None,
            )
        };

        // 3. Atualiza status do passo para Completed
        self.store.update_step_status(
            &step.id,
            MissionStepStatus::Completed,
            Some(&output_val),
            None,
        )?;

        // 4. Cria checkpoint durável com comprovação de verificação
        let checkpoint = MissionCheckpoint {
            id: format!("cp_{}_{}_{}", mission.id, step.step_number, uuid_ms()),
            mission_id: mission.id.clone(),
            step_number: step.step_number,
            snapshot: serde_json::json!({
                "step_id": step.id,
                "output": output_val,
                "verification": verification_res.as_ref().map(|v| serde_json::json!({
                    "state": format!("{:?}", v.verification_state),
                    "observed_effect": format!("{:?}", v.observed_effect_state),
                    "basis": v.verification_basis,
                })),
            }),
            created_at: now_utc(),
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
                "verification_state": verification_res.as_ref().map(|v| format!("{:?}", v.verification_state)),
            }),
        );

        Ok(())
    }

    /// Pausa uma missão em andamento.
    pub fn pause_mission(&self, mission_id: &str) -> Result<(), MissionError> {
        let mut mission = self.store.get_mission(mission_id)?.ok_or_else(|| {
            MissionError::NotFound(format!("Missão '{}' não encontrada", mission_id))
        })?;

        mission.transition_to(MissionStatus::Paused)?;
        self.store
            .update_mission_status(mission_id, MissionStatus::Paused, None)?;
        Ok(())
    }

    /// Retoma uma missão pausada.
    pub async fn resume_mission(&self, mission_id: &str) -> Result<Mission, MissionError> {
        let mut mission = self.store.get_mission(mission_id)?.ok_or_else(|| {
            MissionError::NotFound(format!("Missão '{}' não encontrada", mission_id))
        })?;

        if mission.status == MissionStatus::Paused {
            mission.transition_to(MissionStatus::InProgress)?;
            self.store
                .update_mission_status(mission_id, MissionStatus::InProgress, None)?;
        }

        self.run_mission(mission_id).await
    }

    /// Cancela formalmente uma missão.
    pub fn cancel_mission(&self, mission_id: &str, reason: &str) -> Result<(), MissionError> {
        let mut mission = self.store.get_mission(mission_id)?.ok_or_else(|| {
            MissionError::NotFound(format!("Missão '{}' não encontrada", mission_id))
        })?;

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
