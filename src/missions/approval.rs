use crate::contracts::events::{AuditEvent, EventType};
use crate::contracts::identifiers::{CausationId, CorrelationId};
use crate::contracts::missions::{ApprovalStatus, MissionApproval, MissionError, MissionStep};
use crate::missions::store::SqliteMissionStore;
use crate::persistence::sqlite::SqliteAuditStore;
use std::sync::Arc;

/// Gerenciador governado do protocolo de aprovação humana (*human-in-the-loop*).
///
/// INVARIANTE INEGOCIÁVEL:
/// Nenhuma ação mutadora ou com efeito externo pode ser executada por uma missão
/// sem que haja uma aprovação formalmente concedida pelo operador, vinculada
/// estritamente ao hash exato dos parâmetros e ao tempo de validade.
pub struct MissionApprovalManager {
    store: Arc<SqliteMissionStore>,
    audit_store: Option<Arc<SqliteAuditStore>>,
}

impl MissionApprovalManager {
    pub fn new(store: Arc<SqliteMissionStore>, audit_store: Option<Arc<SqliteAuditStore>>) -> Self {
        Self { store, audit_store }
    }

    /// Cria uma nova solicitação de aprovação vinculada à etapa e aos parâmetros exatos.
    pub fn request_approval(
        &self,
        mission_id: &str,
        step: &MissionStep,
        operator_id: &str,
        validity_secs: u64,
    ) -> Result<MissionApproval, MissionError> {
        let cap = step.capability_id.as_deref().unwrap_or("unknown");
        let approval_id = format!("appr_{}_{}", step.id, uuid_ms());

        let token = format!("tok_{}", uuid_ms());
        let mut approval = MissionApproval::new(
            approval_id,
            mission_id,
            &step.id,
            cap,
            operator_id,
            &step.parameters,
            validity_secs,
        );
        approval.confirmation_token = Some(token);

        self.store.save_approval(&approval)?;

        // Emite evento de auditoria MissionApprovalRequested
        self.record_audit(
            EventType::MissionApprovalRequested,
            serde_json::json!({
                "approval_id": approval.id,
                "mission_id": approval.mission_id,
                "step_id": approval.step_id,
                "capability_id": approval.capability_id,
                "operator_id": approval.operator_id,
                "parameters_hash": approval.parameters_hash,
                "expires_at": approval.expires_at.to_rfc3339(),
                "confirmation_token": approval.confirmation_token,
            }),
        );

        Ok(approval)
    }

    /// Concede formalmente uma aprovação pendente após verificação do operador e prazo.
    pub fn grant_approval(
        &self,
        approval_id: &str,
        operator_id: &str,
    ) -> Result<MissionApproval, MissionError> {
        self.grant_approval_authenticated(approval_id, operator_id, None)
    }

    /// Concede formalmente uma aprovação pendente após verificação do operador autenticado, prazo e token.
    pub fn grant_approval_authenticated(
        &self,
        approval_id: &str,
        operator_id: &str,
        auth_token: Option<&str>,
    ) -> Result<MissionApproval, MissionError> {
        let op_trimmed = operator_id.trim();
        if op_trimmed.is_empty() || op_trimmed == "anonymous" || op_trimmed == "unauthorized" {
            return Err(MissionError::ApprovalMismatch(format!(
                "Operador '{}' não possui identidade autenticada válida",
                operator_id
            )));
        }

        let mut approval = self.store.get_approval(approval_id)?.ok_or_else(|| {
            MissionError::NotFound(format!("Aprovação '{}' não encontrada", approval_id))
        })?;

        if approval.status != ApprovalStatus::Pending {
            return Err(MissionError::ApprovalMismatch(format!(
                "Aprovação '{}' não está pendente (status atual: {:?})",
                approval_id, approval.status
            )));
        }

        if crate::contracts::identifiers::now_utc() > approval.expires_at {
            self.store
                .update_approval_status(approval_id, ApprovalStatus::Expired)?;
            return Err(MissionError::ApprovalExpired(format!(
                "Aprovação '{}' expirou em {}",
                approval_id, approval.expires_at
            )));
        }

        if approval.operator_id != operator_id {
            return Err(MissionError::ApprovalMismatch(format!(
                "Operador '{}' não autorizado a conceder aprovação vinculada ao operador '{}'",
                operator_id, approval.operator_id
            )));
        }

        // Validação estrita de token de confirmação se fornecido
        if let (Some(expected_token), Some(provided)) = (&approval.confirmation_token, auth_token) {
            if provided != expected_token {
                return Err(MissionError::ApprovalMismatch(format!(
                    "Token de confirmação inválido para a aprovação '{}'",
                    approval_id
                )));
            }
        }

        approval.status = ApprovalStatus::Granted;
        self.store
            .update_approval_status(approval_id, ApprovalStatus::Granted)?;

        // Emite evento de auditoria MissionApprovalGranted
        self.record_audit(
            EventType::MissionApprovalGranted,
            serde_json::json!({
                "approval_id": approval.id,
                "mission_id": approval.mission_id,
                "step_id": approval.step_id,
                "operator_id": operator_id,
            }),
        );

        Ok(approval)
    }

    /// Rejeita formalmente uma solicitação de aprovação.
    pub fn deny_approval(
        &self,
        approval_id: &str,
        operator_id: &str,
        reason: &str,
    ) -> Result<MissionApproval, MissionError> {
        let op_trimmed = operator_id.trim();
        if op_trimmed.is_empty() || op_trimmed == "anonymous" || op_trimmed == "unauthorized" {
            return Err(MissionError::ApprovalMismatch(format!(
                "Operador '{}' não possui identidade autenticada válida",
                operator_id
            )));
        }

        let mut approval = self.store.get_approval(approval_id)?.ok_or_else(|| {
            MissionError::NotFound(format!("Aprovação '{}' não encontrada", approval_id))
        })?;
        if approval.status != ApprovalStatus::Pending {
            return Err(MissionError::ApprovalMismatch(format!(
                "Aprovação '{}' não está pendente (status atual: {:?})",
                approval_id, approval.status
            )));
        }

        if approval.operator_id != operator_id {
            return Err(MissionError::ApprovalMismatch(format!(
                "Operador '{}' não autorizado a rejeitar aprovação vinculada ao operador '{}'",
                operator_id, approval.operator_id
            )));
        }

        approval.status = ApprovalStatus::Denied;
        self.store
            .update_approval_status(approval_id, ApprovalStatus::Denied)?;

        // Emite evento de auditoria MissionApprovalDenied
        self.record_audit(
            EventType::MissionApprovalDenied,
            serde_json::json!({
                "approval_id": approval.id,
                "mission_id": approval.mission_id,
                "step_id": approval.step_id,
                "operator_id": operator_id,
                "reason": reason,
            }),
        );

        Ok(approval)
    }

    /// Valida se uma etapa possui aprovação formalmente válida e vigente.
    pub fn verify_step_approval(
        &self,
        step: &MissionStep,
        operator_id: &str,
    ) -> Result<(), MissionError> {
        let app_id = step
            .approval_id
            .as_ref()
            .ok_or_else(|| MissionError::ApprovalRequired {
                step_id: step.id.clone(),
                capability_id: step.capability_id.clone().unwrap_or_default(),
            })?;

        let approval = self.store.get_approval(app_id)?.ok_or_else(|| {
            MissionError::NotFound(format!("Aprovação '{}' não encontrada", app_id))
        })?;

        let cap = step.capability_id.as_deref().unwrap_or("unknown");

        if !approval.is_valid_for(
            operator_id,
            &step.mission_id,
            &step.id,
            cap,
            &step.parameters,
        ) {
            return Err(MissionError::ApprovalMismatch(format!(
                "Aprovação '{}' é inválida, expirada ou possui divergência de parâmetros com a etapa '{}'",
                app_id, step.id
            )));
        }

        Ok(())
    }

    fn record_audit(&self, event_type: EventType, payload: serde_json::Value) {
        if let Some(audit) = &self.audit_store {
            let event = AuditEvent::new(
                event_type,
                CorrelationId::new(),
                CausationId::new("mission_approval_manager"),
                payload,
                "yuki_mission_approval_manager",
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
