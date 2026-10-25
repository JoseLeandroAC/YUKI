use crate::contracts::identifiers::now_utc;
use crate::contracts::voice::{VoiceConfiguration, VoiceError, VoiceSessionState};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Sessão formal de interação por voz com uma máquina de estados estrita.
#[derive(Debug, Clone)]
pub struct VoiceSession {
    pub session_id: String,
    pub device_id: String,
    pub state: VoiceSessionState,
    pub history: Vec<(VoiceSessionState, DateTime<Utc>)>,
    pub started_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub config: VoiceConfiguration,
    pub generation: u64,
    pub in_flight: bool,
}

impl VoiceSession {
    pub fn new(
        session_id: impl Into<String>,
        device_id: impl Into<String>,
        config: VoiceConfiguration,
    ) -> Self {
        Self::with_generation(session_id, device_id, config, 1, false)
    }

    pub fn with_generation(
        session_id: impl Into<String>,
        device_id: impl Into<String>,
        config: VoiceConfiguration,
        generation: u64,
        in_flight: bool,
    ) -> Self {
        let now = now_utc();
        let initial_state = VoiceSessionState::Idle;
        Self {
            session_id: session_id.into(),
            device_id: device_id.into(),
            state: initial_state.clone(),
            history: vec![(initial_state, now)],
            started_at: now,
            last_activity: now,
            config,
            generation,
            in_flight,
        }
    }

    /// Executa transição validada para o próximo estado.
    pub fn transition_to(&mut self, next_state: VoiceSessionState) -> Result<(), VoiceError> {
        if !self.state.can_transition_to(&next_state) {
            return Err(VoiceError::InvalidStateTransition {
                from: self.state.to_string(),
                to: next_state.to_string(),
            });
        }
        let now = now_utc();
        if next_state == VoiceSessionState::Idle {
            self.in_flight = false;
        }
        self.state = next_state.clone();
        self.last_activity = now;
        self.history.push((next_state, now));
        Ok(())
    }

    /// Interrompe imediatamente uma sessão em andamento (ex: usuário começou a falar durante fala da Yuki).
    pub fn interrupt(&mut self) -> Result<(), VoiceError> {
        self.transition_to(VoiceSessionState::Interrupted)
    }

    /// Reseta a sessão para o estado Idle.
    pub fn reset(&mut self) -> Result<(), VoiceError> {
        self.transition_to(VoiceSessionState::Idle)
    }

    /// Verifica se a sessão expirou por inatividade.
    pub fn is_timed_out(&self) -> bool {
        let now = now_utc();
        let elapsed_secs = (now - self.last_activity).num_seconds();
        elapsed_secs > (self.config.session_timeout_secs as i64)
    }

    /// Atualiza o timestamp de atividade.
    pub fn touch(&mut self) {
        self.last_activity = now_utc();
    }
}

/// Registro e coordenador de sessões ativas por dispositivo para evitar concorrência conflitante.
#[derive(Debug, Default, Clone)]
pub struct VoiceSessionManager {
    active_sessions: Arc<Mutex<HashMap<String, VoiceSession>>>,
}

impl VoiceSessionManager {
    pub fn new() -> Self {
        Self {
            active_sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Inicia uma nova sessão para o dispositivo fornecido.
    ///
    /// Regras de liberação e concorrência:
    /// 1. Se existir sessão anterior não expirada:
    ///    - Se `in_flight == true` (operação assíncrona ainda ativa), rejeita com `SessionConflict`
    ///      mesmo que o estado seja `Interrupted`.
    ///    - Se `in_flight == false` e o estado for `Idle`, `Failed(_)` ou `Interrupted`, permite novo turno.
    /// 2. Toda nova sessão recebe uma geração monotonicamente crescente para isolar atualizações tardias.
    pub fn start_session(
        &self,
        session_id: impl Into<String>,
        device_id: impl Into<String>,
        config: VoiceConfiguration,
    ) -> Result<VoiceSession, VoiceError> {
        let sid = session_id.into();
        let dev = device_id.into();

        let mut lock = self.active_sessions.lock().unwrap();

        let next_generation = if let Some(existing) = lock.get(&dev) {
            if !existing.is_timed_out() {
                // Bloqueia novo turno se houver operação assíncrona ainda executando no hardware/core
                if existing.in_flight {
                    return Err(VoiceError::SessionConflict(format!(
                        "Dispositivo '{}' possui operação assíncrona em andamento na sessão '{}' (estado '{}')",
                        dev, existing.session_id, existing.state
                    )));
                }

                // Permite novo turno se a operação antiga já encerrou (Idle, Failed ou Interrupted)
                let can_start_new_turn = existing.state == VoiceSessionState::Idle
                    || matches!(existing.state, VoiceSessionState::Failed(_))
                    || existing.state == VoiceSessionState::Interrupted;

                if !can_start_new_turn {
                    return Err(VoiceError::SessionConflict(format!(
                        "Dispositivo '{}' já possui sessão ativa '{}' no estado '{}'",
                        dev, existing.session_id, existing.state
                    )));
                }
            }
            existing.generation.saturating_add(1)
        } else {
            1
        };

        let session =
            VoiceSession::with_generation(sid, dev.clone(), config, next_generation, true);
        lock.insert(dev, session.clone());
        Ok(session)
    }

    /// Recupera uma sessão ativa pelo ID do dispositivo.
    pub fn get_session(&self, device_id: &str) -> Option<VoiceSession> {
        let lock = self.active_sessions.lock().unwrap();
        lock.get(device_id).cloned()
    }

    /// Atualiza o estado da sessão ativa.
    ///
    /// Proteção contra sobrescrita tardia:
    /// Rejeita atualizações caso o `session_id` ou a `generation` da sessão fornecida
    /// divirjam do registro ativo atual no dispositivo.
    pub fn update_session(&self, session: &VoiceSession) -> bool {
        let mut lock = self.active_sessions.lock().unwrap();
        if let Some(existing) = lock.get_mut(&session.device_id) {
            if existing.session_id == session.session_id
                && existing.generation == session.generation
            {
                *existing = session.clone();
                return true;
            } else {
                tracing::warn!(
                    device_id = %session.device_id,
                    stale_session_id = %session.session_id,
                    current_session_id = %existing.session_id,
                    stale_generation = session.generation,
                    current_generation = existing.generation,
                    "Atualização tardia ignorada: sessão obsoleta não pode sobrescrever a sessão ativa"
                );
                return false;
            }
        }
        false
    }

    /// Marca o encerramento da operação assíncrona em curso, limpando `in_flight`
    /// e aplicando opcionalmente um estado terminal se a geração e o `session_id` coincidirem.
    pub fn finish_operation(
        &self,
        device_id: &str,
        session_id: &str,
        generation: u64,
        final_state: Option<VoiceSessionState>,
    ) {
        let mut lock = self.active_sessions.lock().unwrap();
        if let Some(existing) = lock.get_mut(device_id) {
            if existing.session_id == session_id && existing.generation == generation {
                existing.in_flight = false;
                if let Some(state) = final_state {
                    let _ = existing.transition_to(state);
                }
            }
        }
    }

    /// Verifica se a sessão ativa para o dispositivo foi sinalizada como `Interrupted`
    /// ou se foi substituída por geração ou sessão posterior.
    pub fn is_interrupted(&self, device_id: &str, session_id: &str, generation: u64) -> bool {
        let lock = self.active_sessions.lock().unwrap();
        if let Some(existing) = lock.get(device_id) {
            if existing.session_id != session_id || existing.generation != generation {
                return true; // Sessão substituída por geração posterior é tratada como interrompida/obsoleta
            }
            existing.state == VoiceSessionState::Interrupted
        } else {
            true // Sessão inexistente ou fechada é tratada como interrompida/inválida
        }
    }

    /// Encerra e remove a sessão para o dispositivo.
    pub fn close_session(&self, device_id: &str) {
        let mut lock = self.active_sessions.lock().unwrap();
        lock.remove(device_id);
    }
}
