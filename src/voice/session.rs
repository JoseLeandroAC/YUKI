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
}

impl VoiceSession {
    pub fn new(
        session_id: impl Into<String>,
        device_id: impl Into<String>,
        config: VoiceConfiguration,
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
    /// Retorna erro `SessionConflict` se já existir uma sessão não ociosa e não expirada ativa para o mesmo dispositivo.
    pub fn start_session(
        &self,
        session_id: impl Into<String>,
        device_id: impl Into<String>,
        config: VoiceConfiguration,
    ) -> Result<VoiceSession, VoiceError> {
        let sid = session_id.into();
        let dev = device_id.into();

        let mut lock = self.active_sessions.lock().unwrap();

        // Limpa sessões expiradas ou verifica conflitos
        if let Some(existing) = lock.get(&dev) {
            if !existing.is_timed_out()
                && existing.state != VoiceSessionState::Idle
                && !matches!(existing.state, VoiceSessionState::Failed(_))
            {
                return Err(VoiceError::SessionConflict(format!(
                    "Dispositivo '{}' já possui sessão ativa '{}' no estado '{}'",
                    dev, existing.session_id, existing.state
                )));
            }
        }

        let session = VoiceSession::new(sid, dev.clone(), config);
        lock.insert(dev, session.clone());
        Ok(session)
    }

    /// Recupera uma sessão ativa pelo ID do dispositivo.
    pub fn get_session(&self, device_id: &str) -> Option<VoiceSession> {
        let lock = self.active_sessions.lock().unwrap();
        lock.get(device_id).cloned()
    }

    /// Atualiza o estado da sessão ativa.
    pub fn update_session(&self, session: &VoiceSession) {
        let mut lock = self.active_sessions.lock().unwrap();
        lock.insert(session.device_id.clone(), session.clone());
    }

    /// Encerra e remove a sessão para o dispositivo.
    pub fn close_session(&self, device_id: &str) {
        let mut lock = self.active_sessions.lock().unwrap();
        lock.remove(device_id);
    }
}
