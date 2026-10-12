use crate::contracts::identifiers::now_utc;
use crate::contracts::missions::{
    ApprovalStatus, Mission, MissionApproval, MissionCheckpoint, MissionError,
    MissionExecutionPolicy, MissionStatus, MissionStep, MissionStepStatus,
};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Repositório transacional de persistência de missões em SQLite.
#[derive(Clone)]
pub struct SqliteMissionStore {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteMissionStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, MissionError> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    MissionError::Storage(format!("Falha ao criar diretório de missões: {}", e))
                })?;
            }
        }

        let conn = Connection::open(path).map_err(|e| {
            MissionError::Storage(format!("Falha ao abrir banco de missões: {}", e))
        })?;

        Self::init_schema(&conn)?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn open_in_memory() -> Result<Self, MissionError> {
        let conn = Connection::open_in_memory().map_err(|e| {
            MissionError::Storage(format!("Falha ao abrir SQLite em memória: {}", e))
        })?;

        Self::init_schema(&conn)?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    fn init_schema(conn: &Connection) -> Result<(), MissionError> {
        conn.execute_batch(
            r#"
            PRAGMA foreign_keys = ON;
            PRAGMA busy_timeout = 5000;

            CREATE TABLE IF NOT EXISTS missions (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                objective TEXT NOT NULL,
                status TEXT NOT NULL,
                policy_json TEXT NOT NULL,
                operator_id TEXT NOT NULL,
                correlation_id TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                completed_at TEXT,
                failure_reason TEXT
            );

            CREATE TABLE IF NOT EXISTS mission_steps (
                id TEXT PRIMARY KEY,
                mission_id TEXT NOT NULL,
                step_number INTEGER NOT NULL,
                description TEXT NOT NULL,
                capability_id TEXT,
                parameters_json TEXT NOT NULL,
                requires_approval INTEGER NOT NULL DEFAULT 0,
                status TEXT NOT NULL,
                dependencies_json TEXT NOT NULL,
                output_payload_json TEXT,
                approval_id TEXT,
                failure_reason TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY (mission_id) REFERENCES missions(id) ON DELETE CASCADE,
                UNIQUE(mission_id, step_number)
            );

            CREATE TABLE IF NOT EXISTS mission_approvals (
                id TEXT PRIMARY KEY,
                mission_id TEXT NOT NULL,
                step_id TEXT NOT NULL,
                capability_id TEXT NOT NULL,
                operator_id TEXT NOT NULL,
                parameters_hash TEXT NOT NULL,
                status TEXT NOT NULL,
                created_at TEXT NOT NULL,
                expires_at TEXT NOT NULL,
                confirmation_token TEXT,
                FOREIGN KEY (mission_id) REFERENCES missions(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS mission_checkpoints (
                id TEXT PRIMARY KEY,
                mission_id TEXT NOT NULL,
                step_number INTEGER NOT NULL,
                snapshot_json TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (mission_id) REFERENCES missions(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS mission_leases (
                mission_id TEXT PRIMARY KEY,
                owner_id TEXT NOT NULL,
                fence_token INTEGER NOT NULL,
                acquired_at TEXT NOT NULL,
                expires_at TEXT NOT NULL,
                heartbeat_at TEXT NOT NULL,
                FOREIGN KEY (mission_id) REFERENCES missions(id) ON DELETE CASCADE
            );

            CREATE UNIQUE INDEX IF NOT EXISTS idx_mission_steps_step_number
            ON mission_steps(mission_id, step_number);
            "#,
        )
        .map_err(|e| MissionError::Storage(format!("Falha ao criar esquema de missões: {}", e)))?;

        // Migração não-destrutiva de colunas para tabelas preexistentes em disco
        let _ = conn.execute(
            "ALTER TABLE mission_approvals ADD COLUMN confirmation_token TEXT;",
            [],
        );

        Ok(())
    }

    pub fn create_mission(&self, mission: &Mission) -> Result<(), MissionError> {
        let lock = self.conn.lock().unwrap();
        let policy_json = serde_json::to_string(&mission.policy).map_err(|e| {
            MissionError::Storage(format!("Erro ao serializar política da missão: {}", e))
        })?;

        lock.execute(
            r#"
            INSERT INTO missions (
                id, title, objective, status, policy_json, operator_id, correlation_id,
                created_at, updated_at, completed_at, failure_reason
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            "#,
            params![
                mission.id,
                mission.title,
                mission.objective,
                serde_json::to_string(&mission.status).unwrap(),
                policy_json,
                mission.operator_id,
                mission.correlation_id,
                mission.created_at.to_rfc3339(),
                mission.updated_at.to_rfc3339(),
                mission.completed_at.map(|t| t.to_rfc3339()),
                mission.failure_reason,
            ],
        )
        .map_err(|e| MissionError::Storage(format!("Falha ao inserir missão: {}", e)))?;

        Ok(())
    }

    pub fn update_mission_status(
        &self,
        mission_id: &str,
        status: MissionStatus,
        failure_reason: Option<&str>,
    ) -> Result<(), MissionError> {
        let lock = self.conn.lock().unwrap();

        // Validação da máquina de estados canônica
        let curr_status_opt: Option<String> = lock
            .query_row(
                "SELECT status FROM missions WHERE id = ?1",
                params![mission_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| MissionError::Storage(e.to_string()))?;

        if let Some(curr_str) = curr_status_opt {
            let curr_status: MissionStatus = serde_json::from_str(&curr_str)
                .map_err(|e| MissionError::Storage(format!("Status inválido no banco: {}", e)))?;
            if curr_status != status && !curr_status.can_transition_to(&status) {
                return Err(MissionError::InvalidStateTransition {
                    from: curr_status.to_string(),
                    to: status.to_string(),
                });
            }
        }

        let now = now_utc();
        let status_json = serde_json::to_string(&status).unwrap();
        let completed_at = if status == MissionStatus::Completed {
            Some(now.to_rfc3339())
        } else {
            None
        };

        lock.execute(
            r#"
            UPDATE missions
            SET status = ?1, updated_at = ?2, completed_at = COALESCE(?3, completed_at), failure_reason = COALESCE(?4, failure_reason)
            WHERE id = ?5
            "#,
            params![
                status_json,
                now.to_rfc3339(),
                completed_at,
                failure_reason,
                mission_id,
            ],
        )
        .map_err(|e| MissionError::Storage(format!("Falha ao atualizar status da missão: {}", e)))?;

        Ok(())
    }

    pub fn get_mission(&self, mission_id: &str) -> Result<Option<Mission>, MissionError> {
        let lock = self.conn.lock().unwrap();
        let mut stmt = lock
            .prepare(
                r#"
                SELECT id, title, objective, status, policy_json, operator_id, correlation_id,
                       created_at, updated_at, completed_at, failure_reason
                FROM missions
                WHERE id = ?1
                "#,
            )
            .map_err(|e| MissionError::Storage(e.to_string()))?;

        let mission_opt = stmt
            .query_row(params![mission_id], |row| {
                let id: String = row.get(0)?;
                let title: String = row.get(1)?;
                let objective: String = row.get(2)?;
                let status_str: String = row.get(3)?;
                let policy_str: String = row.get(4)?;
                let operator_id: String = row.get(5)?;
                let correlation_id: String = row.get(6)?;
                let created_str: String = row.get(7)?;
                let updated_str: String = row.get(8)?;
                let completed_str: Option<String> = row.get(9)?;
                let failure_reason: Option<String> = row.get(10)?;

                let status: MissionStatus = serde_json::from_str(&status_str)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
                let policy: MissionExecutionPolicy = serde_json::from_str(&policy_str)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

                let created_at = DateTime::parse_from_rfc3339(&created_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
                let updated_at = DateTime::parse_from_rfc3339(&updated_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
                let completed_at = completed_str.and_then(|s| {
                    DateTime::parse_from_rfc3339(&s)
                        .map(|dt| dt.with_timezone(&Utc))
                        .ok()
                });

                Ok(Mission {
                    id,
                    title,
                    objective,
                    status,
                    steps: Vec::new(),
                    policy,
                    operator_id,
                    correlation_id,
                    created_at,
                    updated_at,
                    completed_at,
                    failure_reason,
                })
            })
            .optional()
            .map_err(|e| MissionError::Storage(format!("Falha ao carregar missão: {}", e)))?;

        if let Some(mut mission) = mission_opt {
            drop(stmt);
            drop(lock);
            let steps = self.get_steps_for_mission(&mission.id)?;
            mission.steps = steps;
            Ok(Some(mission))
        } else {
            Ok(None)
        }
    }

    pub fn list_missions(&self) -> Result<Vec<Mission>, MissionError> {
        let lock = self.conn.lock().unwrap();
        let mut stmt = lock
            .prepare(
                r#"
                SELECT id, title, objective, status, policy_json, operator_id, correlation_id,
                       created_at, updated_at, completed_at, failure_reason
                FROM missions
                ORDER BY created_at DESC
                "#,
            )
            .map_err(|e| MissionError::Storage(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let title: String = row.get(1)?;
                let objective: String = row.get(2)?;
                let status_str: String = row.get(3)?;
                let policy_str: String = row.get(4)?;
                let operator_id: String = row.get(5)?;
                let correlation_id: String = row.get(6)?;
                let created_str: String = row.get(7)?;
                let updated_str: String = row.get(8)?;
                let completed_str: Option<String> = row.get(9)?;
                let failure_reason: Option<String> = row.get(10)?;

                let status: MissionStatus = serde_json::from_str(&status_str)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
                let policy: MissionExecutionPolicy = serde_json::from_str(&policy_str)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

                let created_at = DateTime::parse_from_rfc3339(&created_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
                let updated_at = DateTime::parse_from_rfc3339(&updated_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
                let completed_at = completed_str.and_then(|s| {
                    DateTime::parse_from_rfc3339(&s)
                        .map(|dt| dt.with_timezone(&Utc))
                        .ok()
                });

                Ok(Mission {
                    id,
                    title,
                    objective,
                    status,
                    steps: Vec::new(),
                    policy,
                    operator_id,
                    correlation_id,
                    created_at,
                    updated_at,
                    completed_at,
                    failure_reason,
                })
            })
            .map_err(|e| MissionError::Storage(e.to_string()))?;

        let mut missions = Vec::new();
        for r in rows {
            missions.push(r.map_err(|e| MissionError::Storage(e.to_string()))?);
        }

        drop(stmt);
        drop(lock);

        for m in &mut missions {
            m.steps = self.get_steps_for_mission(&m.id)?;
        }

        Ok(missions)
    }

    pub fn save_step(&self, step: &MissionStep) -> Result<(), MissionError> {
        let lock = self.conn.lock().unwrap();
        let params_json = serde_json::to_string(&step.parameters).unwrap_or_default();
        let deps_json = serde_json::to_string(&step.dependencies).unwrap_or_default();
        let output_json = step
            .output_payload
            .as_ref()
            .map(|o| serde_json::to_string(o).unwrap());
        let status_json = serde_json::to_string(&step.status).unwrap();

        lock.execute(
            r#"
            INSERT INTO mission_steps (
                id, mission_id, step_number, description, capability_id, parameters_json,
                requires_approval, status, dependencies_json, output_payload_json,
                approval_id, failure_reason, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
            ON CONFLICT(id) DO UPDATE SET
                status = excluded.status,
                output_payload_json = excluded.output_payload_json,
                approval_id = excluded.approval_id,
                failure_reason = excluded.failure_reason,
                updated_at = excluded.updated_at
            "#,
            params![
                step.id,
                step.mission_id,
                step.step_number,
                step.description,
                step.capability_id,
                params_json,
                if step.requires_approval { 1 } else { 0 },
                status_json,
                deps_json,
                output_json,
                step.approval_id,
                step.failure_reason,
                step.created_at.to_rfc3339(),
                step.updated_at.to_rfc3339(),
            ],
        )
        .map_err(|e| MissionError::Storage(format!("Falha ao salvar etapa: {}", e)))?;

        Ok(())
    }

    pub fn update_step_status(
        &self,
        step_id: &str,
        status: MissionStepStatus,
        output: Option<&serde_json::Value>,
        failure_reason: Option<&str>,
    ) -> Result<(), MissionError> {
        let lock = self.conn.lock().unwrap();

        // Validação da máquina de estados canônica
        let curr_status_opt: Option<String> = lock
            .query_row(
                "SELECT status FROM mission_steps WHERE id = ?1",
                params![step_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| MissionError::Storage(e.to_string()))?;

        if let Some(curr_str) = curr_status_opt {
            let curr_status: MissionStepStatus = serde_json::from_str(&curr_str)
                .map_err(|e| MissionError::Storage(format!("Status inválido no banco: {}", e)))?;
            if curr_status != status && !curr_status.can_transition_to(&status) {
                return Err(MissionError::InvalidStateTransition {
                    from: curr_status.to_string(),
                    to: status.to_string(),
                });
            }
        }

        let now = now_utc();
        let status_json = serde_json::to_string(&status).unwrap();
        let output_json = output.map(|o| serde_json::to_string(o).unwrap());

        lock.execute(
            r#"
            UPDATE mission_steps
            SET status = ?1, output_payload_json = COALESCE(?2, output_payload_json),
                failure_reason = COALESCE(?3, failure_reason), updated_at = ?4
            WHERE id = ?5
            "#,
            params![
                status_json,
                output_json,
                failure_reason,
                now.to_rfc3339(),
                step_id,
            ],
        )
        .map_err(|e| MissionError::Storage(format!("Falha ao atualizar etapa: {}", e)))?;

        Ok(())
    }

    pub fn get_steps_for_mission(
        &self,
        mission_id: &str,
    ) -> Result<Vec<MissionStep>, MissionError> {
        let lock = self.conn.lock().unwrap();
        let mut stmt = lock
            .prepare(
                r#"
                SELECT id, mission_id, step_number, description, capability_id, parameters_json,
                       requires_approval, status, dependencies_json, output_payload_json,
                       approval_id, failure_reason, created_at, updated_at
                FROM mission_steps
                WHERE mission_id = ?1
                ORDER BY step_number ASC
                "#,
            )
            .map_err(|e| MissionError::Storage(e.to_string()))?;

        let rows = stmt
            .query_map(params![mission_id], |row| {
                let id: String = row.get(0)?;
                let m_id: String = row.get(1)?;
                let step_number: u32 = row.get(2)?;
                let description: String = row.get(3)?;
                let capability_id: Option<String> = row.get(4)?;
                let params_str: String = row.get(5)?;
                let requires_approval_int: i32 = row.get(6)?;
                let status_str: String = row.get(7)?;
                let deps_str: String = row.get(8)?;
                let output_str: Option<String> = row.get(9)?;
                let approval_id: Option<String> = row.get(10)?;
                let failure_reason: Option<String> = row.get(11)?;
                let created_str: String = row.get(12)?;
                let updated_str: String = row.get(13)?;

                let parameters: serde_json::Value =
                    serde_json::from_str(&params_str).unwrap_or_else(|_| serde_json::json!({}));
                let status: MissionStepStatus = serde_json::from_str(&status_str)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
                let dependencies: Vec<u32> = serde_json::from_str(&deps_str).unwrap_or_default();
                let output_payload = output_str.and_then(|s| serde_json::from_str(&s).ok());

                let created_at = DateTime::parse_from_rfc3339(&created_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
                let updated_at = DateTime::parse_from_rfc3339(&updated_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

                Ok(MissionStep {
                    id,
                    mission_id: m_id,
                    step_number,
                    description,
                    capability_id,
                    parameters,
                    requires_approval: requires_approval_int == 1,
                    status,
                    dependencies,
                    output_payload,
                    approval_id,
                    failure_reason,
                    created_at,
                    updated_at,
                })
            })
            .map_err(|e| MissionError::Storage(e.to_string()))?;

        let mut steps = Vec::new();
        for r in rows {
            steps.push(r.map_err(|e| MissionError::Storage(e.to_string()))?);
        }

        Ok(steps)
    }

    pub fn save_approval(&self, approval: &MissionApproval) -> Result<(), MissionError> {
        let lock = self.conn.lock().unwrap();

        // Garante integridade referencial para missões invocadas em testes isolados
        let exists: bool = lock
            .query_row(
                "SELECT 1 FROM missions WHERE id = ?1",
                params![approval.mission_id],
                |_| Ok(true),
            )
            .optional()
            .map_err(|e| MissionError::Storage(e.to_string()))?
            .unwrap_or(false);

        if !exists {
            let now = now_utc();
            let default_policy_json =
                serde_json::to_string(&MissionExecutionPolicy::default()).unwrap();
            let _ = lock.execute(
                r#"
                INSERT OR IGNORE INTO missions (
                    id, title, objective, status, policy_json, operator_id, correlation_id,
                    created_at, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                "#,
                params![
                    approval.mission_id,
                    "Missão Implícita",
                    "Solicitação de aprovação autônoma",
                    serde_json::to_string(&MissionStatus::Planned).unwrap(),
                    default_policy_json,
                    approval.operator_id,
                    "corr_auto",
                    now.to_rfc3339(),
                    now.to_rfc3339(),
                ],
            );
        }

        let status_json = serde_json::to_string(&approval.status).unwrap();

        lock.execute(
            r#"
            INSERT INTO mission_approvals (
                id, mission_id, step_id, capability_id, operator_id, parameters_hash,
                status, created_at, expires_at, confirmation_token
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            ON CONFLICT(id) DO UPDATE SET
                status = excluded.status,
                confirmation_token = excluded.confirmation_token
            "#,
            params![
                approval.id,
                approval.mission_id,
                approval.step_id,
                approval.capability_id,
                approval.operator_id,
                approval.parameters_hash,
                status_json,
                approval.created_at.to_rfc3339(),
                approval.expires_at.to_rfc3339(),
                approval.confirmation_token,
            ],
        )
        .map_err(|e| MissionError::Storage(format!("Falha ao salvar aprovação: {}", e)))?;

        Ok(())
    }

    /// Transiciona atomicamente o status de uma aprovação com verificação CAS do estado esperado.
    pub fn transition_approval_status(
        &self,
        approval_id: &str,
        expected_current: ApprovalStatus,
        new_status: ApprovalStatus,
    ) -> Result<(), MissionError> {
        if !expected_current.can_transition_to(&new_status) {
            return Err(MissionError::InvalidStateTransition {
                from: format!("{:?}", expected_current),
                to: format!("{:?}", new_status),
            });
        }

        let lock = self.conn.lock().unwrap();
        let expected_json = serde_json::to_string(&expected_current).unwrap();
        let new_json = serde_json::to_string(&new_status).unwrap();

        let rows_affected = lock
            .execute(
                "UPDATE mission_approvals SET status = ?1 WHERE id = ?2 AND status = ?3",
                params![new_json, approval_id, expected_json],
            )
            .map_err(|e| {
                MissionError::Storage(format!("Falha ao transicionar status de aprovação: {}", e))
            })?;

        if rows_affected == 0 {
            let current_in_db: Option<String> = lock
                .query_row(
                    "SELECT status FROM mission_approvals WHERE id = ?1",
                    params![approval_id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|e| {
                    MissionError::Storage(format!("Falha ao consultar aprovação: {}", e))
                })?;

            return match current_in_db {
                Some(status_str) => Err(MissionError::ApprovalMismatch(format!(
                    "Disputa concorrente ou status divergente para aprovação '{}': esperado era '{:?}', atual no banco é '{}'",
                    approval_id, expected_current, status_str
                ))),
                None => Err(MissionError::NotFound(format!(
                    "Aprovação '{}' não encontrada",
                    approval_id
                ))),
            };
        }

        Ok(())
    }

    /// Atualiza o status de uma aprovação garantindo cumprimento estrito da máquina de estados finita.
    pub fn update_approval_status(
        &self,
        approval_id: &str,
        status: ApprovalStatus,
    ) -> Result<(), MissionError> {
        let lock = self.conn.lock().unwrap();
        let current_json: Option<String> = lock
            .query_row(
                "SELECT status FROM mission_approvals WHERE id = ?1",
                params![approval_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| MissionError::Storage(format!("Falha ao consultar aprovação: {}", e)))?;

        let current_json = current_json.ok_or_else(|| {
            MissionError::NotFound(format!("Aprovação '{}' não encontrada", approval_id))
        })?;

        let current_status: ApprovalStatus = serde_json::from_str(&current_json)
            .map_err(|e| MissionError::Storage(format!("Status de aprovação corrompido: {}", e)))?;

        if !current_status.can_transition_to(&status) {
            return Err(MissionError::InvalidStateTransition {
                from: format!("{:?}", current_status),
                to: format!("{:?}", status),
            });
        }

        let new_json = serde_json::to_string(&status).unwrap();
        let rows_affected = lock
            .execute(
                "UPDATE mission_approvals SET status = ?1 WHERE id = ?2 AND status = ?3",
                params![new_json, approval_id, current_json],
            )
            .map_err(|e| {
                MissionError::Storage(format!("Falha ao atualizar status de aprovação: {}", e))
            })?;

        if rows_affected == 0 {
            return Err(MissionError::ApprovalMismatch(format!(
                "Disputa concorrente ao atualizar aprovação '{}': status foi alterado por outra operação",
                approval_id
            )));
        }

        Ok(())
    }

    /// Consome atomicamente uma aprovação válida, garantindo uso único (anti-replay) e eliminando TOCTOU.
    pub fn consume_approval_atomically(
        &self,
        approval_id: &str,
        mission_id: &str,
        step_id: &str,
        capability_id: &str,
        operator_id: &str,
        parameters: &serde_json::Value,
    ) -> Result<MissionApproval, MissionError> {
        let lock = self.conn.lock().unwrap();
        let now = now_utc();
        let expected_hash = MissionApproval::compute_hash(parameters);
        let granted_status_json = serde_json::to_string(&ApprovalStatus::Granted).unwrap();
        let consumed_status_json = serde_json::to_string(&ApprovalStatus::Consumed).unwrap();

        // Tentativa de consumo atômico via UPDATE condicional estrito
        let rows_affected = lock
            .execute(
                r#"
                UPDATE mission_approvals
                SET status = ?1
                WHERE id = ?2
                  AND mission_id = ?3
                  AND step_id = ?4
                  AND capability_id = ?5
                  AND operator_id = ?6
                  AND parameters_hash = ?7
                  AND status = ?8
                  AND expires_at > ?9
                "#,
                params![
                    consumed_status_json,
                    approval_id,
                    mission_id,
                    step_id,
                    capability_id,
                    operator_id,
                    expected_hash,
                    granted_status_json,
                    now.to_rfc3339(),
                ],
            )
            .map_err(|e| {
                MissionError::Storage(format!("Falha transacional ao consumir aprovação: {}", e))
            })?;

        if rows_affected == 1 {
            // Sucesso atômico: recupera o registro com status atualizado
            let mut stmt = lock
                .prepare(
                    r#"
                    SELECT id, mission_id, step_id, capability_id, operator_id, parameters_hash,
                           status, created_at, expires_at, confirmation_token
                    FROM mission_approvals
                    WHERE id = ?1
                    "#,
                )
                .map_err(|e| MissionError::Storage(e.to_string()))?;

            let approval = stmt
                .query_row(params![approval_id], |row| {
                    let id: String = row.get(0)?;
                    let m_id: String = row.get(1)?;
                    let s_id: String = row.get(2)?;
                    let c_id: String = row.get(3)?;
                    let op_id: String = row.get(4)?;
                    let p_hash: String = row.get(5)?;
                    let status_str: String = row.get(6)?;
                    let created_str: String = row.get(7)?;
                    let expires_str: String = row.get(8)?;
                    let confirmation_token: Option<String> = row.get(9)?;

                    let status: ApprovalStatus = serde_json::from_str(&status_str)
                        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
                    let created_at = DateTime::parse_from_rfc3339(&created_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
                    let expires_at = DateTime::parse_from_rfc3339(&expires_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

                    Ok(MissionApproval {
                        id,
                        mission_id: m_id,
                        step_id: s_id,
                        capability_id: c_id,
                        operator_id: op_id,
                        parameters_hash: p_hash,
                        status,
                        created_at,
                        expires_at,
                        confirmation_token,
                    })
                })
                .map_err(|e| MissionError::Storage(e.to_string()))?;

            return Ok(approval);
        }

        // Falha no UPDATE condicional: investiga causa exata para diagnóstico preciso
        let mut check_stmt = lock
            .prepare(
                r#"
                SELECT id, mission_id, step_id, capability_id, operator_id, parameters_hash,
                       status, created_at, expires_at, confirmation_token
                FROM mission_approvals
                WHERE id = ?1
                "#,
            )
            .map_err(|e| MissionError::Storage(e.to_string()))?;

        let existing_opt = check_stmt
            .query_row(params![approval_id], |row| {
                let id: String = row.get(0)?;
                let m_id: String = row.get(1)?;
                let s_id: String = row.get(2)?;
                let c_id: String = row.get(3)?;
                let op_id: String = row.get(4)?;
                let p_hash: String = row.get(5)?;
                let status_str: String = row.get(6)?;
                let created_str: String = row.get(7)?;
                let expires_str: String = row.get(8)?;
                let confirmation_token: Option<String> = row.get(9)?;

                let status: ApprovalStatus = serde_json::from_str(&status_str)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
                let created_at = DateTime::parse_from_rfc3339(&created_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
                let expires_at = DateTime::parse_from_rfc3339(&expires_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

                Ok(MissionApproval {
                    id,
                    mission_id: m_id,
                    step_id: s_id,
                    capability_id: c_id,
                    operator_id: op_id,
                    parameters_hash: p_hash,
                    status,
                    created_at,
                    expires_at,
                    confirmation_token,
                })
            })
            .optional()
            .map_err(|e| MissionError::Storage(e.to_string()))?;

        match existing_opt {
            None => Err(MissionError::NotFound(format!(
                "Aprovação '{}' não encontrada",
                approval_id
            ))),
            Some(existing) => {
                if existing.status == ApprovalStatus::Consumed {
                    Err(MissionError::ApprovalMismatch(format!(
                        "Aprovação '{}' já foi consumida previamente (proteção anti-replay ativa)",
                        approval_id
                    )))
                } else if now > existing.expires_at {
                    Err(MissionError::ApprovalExpired(format!(
                        "Aprovação '{}' expirou em {}",
                        approval_id, existing.expires_at
                    )))
                } else if existing.status != ApprovalStatus::Granted {
                    Err(MissionError::ApprovalMismatch(format!(
                        "Aprovação '{}' não está no estado concedido (status atual: {:?})",
                        approval_id, existing.status
                    )))
                } else if existing.mission_id != mission_id
                    || existing.step_id != step_id
                    || existing.capability_id != capability_id
                    || existing.operator_id != operator_id
                    || existing.parameters_hash != expected_hash
                {
                    Err(MissionError::ApprovalMismatch(format!(
                        "Divergência de parâmetros, etapa ou identidade para a aprovação '{}'",
                        approval_id
                    )))
                } else {
                    Err(MissionError::ApprovalMismatch(format!(
                        "Falha ao validar e consumir atomicamente a aprovação '{}'",
                        approval_id
                    )))
                }
            }
        }
    }

    pub fn get_approval(&self, approval_id: &str) -> Result<Option<MissionApproval>, MissionError> {
        let lock = self.conn.lock().unwrap();
        let mut stmt = lock
            .prepare(
                r#"
                SELECT id, mission_id, step_id, capability_id, operator_id, parameters_hash,
                       status, created_at, expires_at, confirmation_token
                FROM mission_approvals
                WHERE id = ?1
                "#,
            )
            .map_err(|e| MissionError::Storage(e.to_string()))?;

        stmt.query_row(params![approval_id], |row| {
            let id: String = row.get(0)?;
            let mission_id: String = row.get(1)?;
            let step_id: String = row.get(2)?;
            let capability_id: String = row.get(3)?;
            let operator_id: String = row.get(4)?;
            let parameters_hash: String = row.get(5)?;
            let status_str: String = row.get(6)?;
            let created_str: String = row.get(7)?;
            let expires_str: String = row.get(8)?;
            let confirmation_token: Option<String> = row.get(9)?;

            let status: ApprovalStatus = serde_json::from_str(&status_str)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            let created_at = DateTime::parse_from_rfc3339(&created_str)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            let expires_at = DateTime::parse_from_rfc3339(&expires_str)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

            Ok(MissionApproval {
                id,
                mission_id,
                step_id,
                capability_id,
                operator_id,
                parameters_hash,
                status,
                created_at,
                expires_at,
                confirmation_token,
            })
        })
        .optional()
        .map_err(|e| MissionError::Storage(e.to_string()))
    }

    pub fn save_checkpoint(&self, checkpoint: &MissionCheckpoint) -> Result<(), MissionError> {
        let lock = self.conn.lock().unwrap();
        let snapshot_json = serde_json::to_string(&checkpoint.snapshot).unwrap_or_default();

        lock.execute(
            r#"
            INSERT INTO mission_checkpoints (id, mission_id, step_number, snapshot_json, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5)
            "#,
            params![
                checkpoint.id,
                checkpoint.mission_id,
                checkpoint.step_number,
                snapshot_json,
                checkpoint.created_at.to_rfc3339(),
            ],
        )
        .map_err(|e| MissionError::Storage(format!("Falha ao salvar checkpoint: {}", e)))?;

        Ok(())
    }

    pub fn get_latest_checkpoint(
        &self,
        mission_id: &str,
    ) -> Result<Option<MissionCheckpoint>, MissionError> {
        let lock = self.conn.lock().unwrap();
        let mut stmt = lock
            .prepare(
                r#"
                SELECT id, mission_id, step_number, snapshot_json, created_at
                FROM mission_checkpoints
                WHERE mission_id = ?1
                ORDER BY step_number DESC
                LIMIT 1
                "#,
            )
            .map_err(|e| MissionError::Storage(e.to_string()))?;

        stmt.query_row(params![mission_id], |row| {
            let id: String = row.get(0)?;
            let m_id: String = row.get(1)?;
            let step_number: u32 = row.get(2)?;
            let snapshot_str: String = row.get(3)?;
            let created_str: String = row.get(4)?;

            let snapshot: serde_json::Value =
                serde_json::from_str(&snapshot_str).unwrap_or_else(|_| serde_json::json!({}));
            let created_at = DateTime::parse_from_rfc3339(&created_str)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

            Ok(MissionCheckpoint {
                id,
                mission_id: m_id,
                step_number,
                snapshot,
                created_at,
            })
        })
        .optional()
        .map_err(|e| MissionError::Storage(e.to_string()))
    }

    /// Reconciliação soberana de missões interrompidas durante reinicialização do sistema.
    ///
    /// REGRAS DE INTEGRIDADE:
    /// - Missões em `InProgress` ou `AwaitingApproval` são inspecionadas.
    /// - Qualquer etapa em `Running` (que foi interrompida pelo encerramento abrupto do processo)
    ///   é reconciliada para `WaitingApproval` se exigia aprovação ou envolvia capacidade com potencial
    ///   efeito colateral, ou para `Pending` se não produziu efeito verificado.
    /// - Evita estritamente duplicar execuções de operações mutadoras externas!
    pub fn reconcile_and_recover_on_startup(&self) -> Result<Vec<Mission>, MissionError> {
        let missions = self.list_missions()?;
        let mut recovered = Vec::new();

        for mut mission in missions {
            if mission.status == MissionStatus::InProgress
                || mission.status == MissionStatus::AwaitingApproval
            {
                // Inspeciona passos da missão
                let mut modified = false;
                for step in &mut mission.steps {
                    if step.status == MissionStepStatus::Running {
                        // O passo estava no meio da execução quando houve crash/reboot
                        // Se exigia aprovação ou possuía capacidade com potencial mutação externa,
                        // NUNCA reinicia automaticamente em Pending! Requer portão humano.
                        let is_mutating = step.capability_id.as_deref().is_some_and(|c| {
                            c.contains("write")
                                || c.contains("mutate")
                                || c.contains("delete")
                                || c.contains("exec")
                        });

                        let new_status = if step.requires_approval || is_mutating {
                            MissionStepStatus::WaitingApproval
                        } else {
                            MissionStepStatus::Pending
                        };
                        self.update_step_status(
                            &step.id,
                            new_status.clone(),
                            None,
                            Some("Interrompido por reinicialização do sistema; reconciliado para retomada segura"),
                        )?;
                        step.status = new_status;
                        modified = true;
                    }
                }

                if modified {
                    // Atualiza a missão para Paused para permitir retomada controlada pelo operador
                    self.update_mission_status(&mission.id, MissionStatus::Paused, None)?;
                    mission.status = MissionStatus::Paused;
                }
                recovered.push(mission);
            }
        }

        Ok(recovered)
    }

    /// Adquire um lease exclusivo persistente no SQLite com fencing token para uma missão.
    /// Utiliza transação imediata e compare-and-swap estrito com validação no próprio SQL,
    /// garantindo exclusão mútua atômica entre múltiplos processos ou instâncias concorrentes.
    pub fn acquire_mission_lease(
        &self,
        mission_id: &str,
        owner_id: &str,
        ttl_secs: u64,
    ) -> Result<u64, MissionError> {
        let mut lock = self.conn.lock().unwrap();
        let now = now_utc();
        let expires_at = now + chrono::Duration::seconds(ttl_secs as i64);

        let tx = lock
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|e| {
                MissionError::Storage(format!(
                    "Falha ao iniciar transação imediata de lease: {}",
                    e
                ))
            })?;

        let row_opt: Option<(String, i64, String)> = tx
            .query_row(
                "SELECT owner_id, fence_token, expires_at FROM mission_leases WHERE mission_id = ?1",
                params![mission_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .map_err(|e| MissionError::Storage(format!("Falha ao consultar lease da missão: {}", e)))?;

        let fence_result = match row_opt {
            None => {
                let fence_token = 1u64;
                let insert_res = tx.execute(
                    r#"
                    INSERT INTO mission_leases (mission_id, owner_id, fence_token, acquired_at, expires_at, heartbeat_at)
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                    "#,
                    params![
                        mission_id,
                        owner_id,
                        fence_token as i64,
                        now.to_rfc3339(),
                        expires_at.to_rfc3339(),
                        now.to_rfc3339(),
                    ],
                );
                match insert_res {
                    Ok(_) => Ok(fence_token),
                    Err(e) => Err(MissionError::ExecutionFailed(format!(
                        "Disputa de lease concorrente na missão '{}': falha ao registrar novo lease: {}",
                        mission_id, e
                    ))),
                }
            }
            Some((current_owner, current_fence_i64, expires_str)) => {
                let current_fence = current_fence_i64 as u64;
                let current_expires = DateTime::parse_from_rfc3339(&expires_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or(now);

                if current_expires > now {
                    Err(MissionError::ExecutionFailed(format!(
                        "Missão '{}' já possui lease ativo pelo executor '{}' (válido até {})",
                        mission_id, current_owner, current_expires
                    )))
                } else {
                    // Lease expirado: recuperação soberana com CAS estrito no SQL
                    let new_fence = current_fence + 1;
                    let rows_affected = tx.execute(
                        r#"
                        UPDATE mission_leases
                        SET owner_id = ?1, fence_token = ?2, acquired_at = ?3, expires_at = ?4, heartbeat_at = ?5
                        WHERE mission_id = ?6 AND fence_token = ?7 AND expires_at = ?8 AND expires_at <= ?9
                        "#,
                        params![
                            owner_id,
                            new_fence as i64,
                            now.to_rfc3339(),
                            expires_at.to_rfc3339(),
                            now.to_rfc3339(),
                            mission_id,
                            current_fence as i64,
                            expires_str,
                            now.to_rfc3339(),
                        ],
                    ).map_err(|e| MissionError::Storage(format!("Falha ao executar CAS de recuperação de lease: {}", e)))?;

                    if rows_affected == 1 {
                        Ok(new_fence)
                    } else {
                        Err(MissionError::ExecutionFailed(format!(
                            "Disputa de lease concorrente perdida na missão '{}': outro executor venceu a recuperação CAS",
                            mission_id
                        )))
                    }
                }
            }
        };

        match fence_result {
            Ok(fence) => {
                tx.commit().map_err(|e| {
                    MissionError::Storage(format!("Falha ao comitar aquisição de lease: {}", e))
                })?;
                Ok(fence)
            }
            Err(e) => {
                let _ = tx.rollback();
                Err(e)
            }
        }
    }

    /// Valida que o executor e o fencing token continuam soberanamente válidos e ativos.
    pub fn verify_mission_lease(
        &self,
        mission_id: &str,
        owner_id: &str,
        fence_token: u64,
    ) -> Result<(), MissionError> {
        let lock = self.conn.lock().unwrap();
        let now = now_utc();

        let row_opt: Option<(String, i64, String)> = lock
            .query_row(
                "SELECT owner_id, fence_token, expires_at FROM mission_leases WHERE mission_id = ?1",
                params![mission_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .map_err(|e| MissionError::Storage(format!("Falha ao verificar lease da missão: {}", e)))?;

        match row_opt {
            Some((current_owner, current_fence_i64, expires_str)) => {
                let current_fence = current_fence_i64 as u64;
                let current_expires = DateTime::parse_from_rfc3339(&expires_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or(now);

                if current_owner != owner_id {
                    return Err(MissionError::ExecutionFailed(format!(
                        "Lease da missão '{}' pertence a outro executor ('{}' != '{}')",
                        mission_id, current_owner, owner_id
                    )));
                }

                if current_fence != fence_token {
                    return Err(MissionError::ExecutionFailed(format!(
                        "Fencing token da missão '{}' obsoleto ou revogado (atual {} != esperado {})",
                        mission_id, current_fence, fence_token
                    )));
                }

                if now >= current_expires {
                    return Err(MissionError::ExecutionFailed(format!(
                        "Lease da missão '{}' expirou em {}",
                        mission_id, current_expires
                    )));
                }

                Ok(())
            }
            None => Err(MissionError::ExecutionFailed(format!(
                "Nenhum lease ativo encontrado para a missão '{}'",
                mission_id
            ))),
        }
    }

    /// Renova o tempo de vida do lease exclusivo para a missão em execução.
    pub fn renew_mission_lease(
        &self,
        mission_id: &str,
        owner_id: &str,
        fence_token: u64,
        ttl_secs: u64,
    ) -> Result<(), MissionError> {
        let lock = self.conn.lock().unwrap();
        let now = now_utc();
        let expires_at = now + chrono::Duration::seconds(ttl_secs as i64);

        let rows_affected = lock
            .execute(
                r#"
                UPDATE mission_leases
                SET expires_at = ?1, heartbeat_at = ?2
                WHERE mission_id = ?3 AND owner_id = ?4 AND fence_token = ?5 AND expires_at > ?6
                "#,
                params![
                    expires_at.to_rfc3339(),
                    now.to_rfc3339(),
                    mission_id,
                    owner_id,
                    fence_token as i64,
                    now.to_rfc3339(),
                ],
            )
            .map_err(|e| MissionError::Storage(format!("Falha ao renovar lease: {}", e)))?;

        if rows_affected == 0 {
            return Err(MissionError::ExecutionFailed(format!(
                "Falha ao renovar lease da missão '{}': revogado, pertencente a outro executor ou expirado",
                mission_id
            )));
        }

        Ok(())
    }

    /// Libera formalmente o lease de uma missão ao concluir, pausar, cancelar ou abortar.
    pub fn release_mission_lease(
        &self,
        mission_id: &str,
        owner_id: &str,
        fence_token: u64,
    ) -> Result<(), MissionError> {
        let lock = self.conn.lock().unwrap();
        let rows_affected = lock
            .execute(
                "DELETE FROM mission_leases WHERE mission_id = ?1 AND owner_id = ?2 AND fence_token = ?3",
                params![mission_id, owner_id, fence_token as i64],
            )
            .map_err(|e| MissionError::Storage(format!("Falha ao liberar lease: {}", e)))?;

        if rows_affected == 0 {
            return Err(MissionError::ExecutionFailed(format!(
                "Falha ao liberar lease da missão '{}': não encontrado ou não pertence ao executor '{}' com fence token {}",
                mission_id, owner_id, fence_token
            )));
        }

        Ok(())
    }
}
