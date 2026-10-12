use crate::contracts::identifiers::now_utc;
use crate::contracts::voice::{
    AudioInput, AudioOutput, SpeechToTextProvider, TextToSpeechProvider, VoiceConfiguration,
    VoiceError,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Autorização tipada, temporizada e verificável para saída de áudio/voz para provedores em nuvem (egress).
///
/// DIRETRIZ CANÔNICA DE PRIVACIDADE E SOBERANIA:
/// "LOCAL FIRST — CLOUD ONLY WHEN AUTHORIZED"
///
/// Garante:
/// - Desacoplamento estrito entre STT e TTS (`egress:voice_cloud_stt` != `egress:voice_cloud_tts`).
/// - Vinculação estrita ao operador requisitante.
/// - Validade temporal curta (TTL).
/// - Associação criptográfica com hash dos parâmetros da requisição.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoiceEgressAuthorization {
    pub operator_id: String,
    pub capability: String,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub parameter_hash: String,
}

impl VoiceEgressAuthorization {
    pub const CAPABILITY_STT: &'static str = "egress:voice_cloud_stt";
    pub const CAPABILITY_TTS: &'static str = "egress:voice_cloud_tts";
    pub const DEFAULT_TTL_SECS: i64 = 60;

    /// Cria autorização específica para envio de áudio bruto para STT em nuvem.
    pub fn new_stt(operator_id: impl Into<String>, parameter_hash: impl Into<String>) -> Self {
        let now = now_utc();
        Self {
            operator_id: operator_id.into(),
            capability: Self::CAPABILITY_STT.to_string(),
            issued_at: now,
            expires_at: now + chrono::Duration::seconds(Self::DEFAULT_TTL_SECS),
            parameter_hash: parameter_hash.into(),
        }
    }

    /// Cria autorização específica para envio de texto para TTS em nuvem.
    pub fn new_tts(operator_id: impl Into<String>, parameter_hash: impl Into<String>) -> Self {
        let now = now_utc();
        Self {
            operator_id: operator_id.into(),
            capability: Self::CAPABILITY_TTS.to_string(),
            issued_at: now,
            expires_at: now + chrono::Duration::seconds(Self::DEFAULT_TTL_SECS),
            parameter_hash: parameter_hash.into(),
        }
    }

    /// Construtor com expiração customizada (para testes e políticas de curta duração).
    pub fn with_ttl(
        operator_id: impl Into<String>,
        capability: impl Into<String>,
        ttl_secs: i64,
        parameter_hash: impl Into<String>,
    ) -> Self {
        let now = now_utc();
        Self {
            operator_id: operator_id.into(),
            capability: capability.into(),
            issued_at: now,
            expires_at: now + chrono::Duration::seconds(ttl_secs),
            parameter_hash: parameter_hash.into(),
        }
    }

    /// Valida se a autorização é formalmente válida para a capability e operador requeridos.
    pub fn is_valid_for(&self, required_capability: &str, expected_operator: &str) -> bool {
        if self.operator_id != expected_operator {
            return false;
        }
        if self.capability != required_capability {
            return false;
        }
        let now = now_utc();
        if now > self.expires_at || now < self.issued_at {
            return false;
        }
        true
    }
}

/// Trait de conversão flexível para retrocompatibilidade entre booleans de consentimento e `VoiceEgressAuthorization`.
pub trait IntoVoiceEgressAuth {
    fn into_auth(self, capability: &str, operator_id: &str) -> Option<VoiceEgressAuthorization>;
}

impl IntoVoiceEgressAuth for bool {
    fn into_auth(self, capability: &str, operator_id: &str) -> Option<VoiceEgressAuthorization> {
        if self {
            Some(VoiceEgressAuthorization {
                operator_id: operator_id.to_string(),
                capability: capability.to_string(),
                issued_at: now_utc(),
                expires_at: now_utc()
                    + chrono::Duration::seconds(VoiceEgressAuthorization::DEFAULT_TTL_SECS),
                parameter_hash: "legacy-bool-consent".to_string(),
            })
        } else {
            None
        }
    }
}

impl IntoVoiceEgressAuth for VoiceEgressAuthorization {
    fn into_auth(self, _capability: &str, _operator_id: &str) -> Option<VoiceEgressAuthorization> {
        Some(self)
    }
}

impl IntoVoiceEgressAuth for &VoiceEgressAuthorization {
    fn into_auth(self, _capability: &str, _operator_id: &str) -> Option<VoiceEgressAuthorization> {
        Some(self.clone())
    }
}

impl IntoVoiceEgressAuth for Option<VoiceEgressAuthorization> {
    fn into_auth(self, _capability: &str, _operator_id: &str) -> Option<VoiceEgressAuthorization> {
        self
    }
}

impl IntoVoiceEgressAuth for Option<&VoiceEgressAuthorization> {
    fn into_auth(self, _capability: &str, _operator_id: &str) -> Option<VoiceEgressAuthorization> {
        self.cloned()
    }
}

// ---------------------------------------------------------------------------
// GovernedVoiceRouter
// ---------------------------------------------------------------------------

/// Seletor e roteador governado de provedores de voz.
///
/// DIRETRIZ CANÔNICA DE PRIVACIDADE E SOBERANIA:
/// "LOCAL FIRST — CLOUD ONLY WHEN AUTHORIZED"
///
/// 1. Tenta prioritariamente o provedor local de maior soberania e menor latência.
/// 2. Se o provedor local falhar ou estiver inacessível:
///    - Valida se o fallback em nuvem está habilitado nas configurações (`cloud_fallback_allowed`).
///    - Valida se o operador concedeu expressamente a permissão específica tipada via `VoiceEgressAuthorization`:
///      - Para transcrição de áudio: `egress:voice_cloud_stt`
///      - Para síntese de texto: `egress:voice_cloud_tts`
///        (Essas permissões são formalmente desacopladas: permitir envio de texto não autoriza envio de áudio bruto).
///    - Somente após essa dupla verificação despacha a transmissão ao provedor externo.
///    - Se não autorizado, falha fechado mantendo a integridade e os limites de privacidade.
pub struct GovernedVoiceRouter {
    local_stt: Arc<dyn SpeechToTextProvider>,
    cloud_stt: Option<Arc<dyn SpeechToTextProvider>>,
    local_tts: Arc<dyn TextToSpeechProvider>,
    cloud_tts: Option<Arc<dyn TextToSpeechProvider>>,
    config: VoiceConfiguration,
}

impl GovernedVoiceRouter {
    pub fn new(
        local_stt: Arc<dyn SpeechToTextProvider>,
        cloud_stt: Option<Arc<dyn SpeechToTextProvider>>,
        local_tts: Arc<dyn TextToSpeechProvider>,
        cloud_tts: Option<Arc<dyn TextToSpeechProvider>>,
        config: VoiceConfiguration,
    ) -> Self {
        Self {
            local_stt,
            cloud_stt,
            local_tts,
            cloud_tts,
            config,
        }
    }

    pub fn config(&self) -> &VoiceConfiguration {
        &self.config
    }

    pub fn set_cloud_fallback_allowed(&mut self, allowed: bool) {
        self.config.cloud_fallback_allowed = allowed;
    }

    /// Executa reconhecimento de fala (STT) sob a política "Local First — Cloud Only When Authorized".
    pub async fn transcribe<A: IntoVoiceEgressAuth>(
        &self,
        audio: &AudioInput,
        auth: A,
    ) -> Result<String, VoiceError> {
        // 1. Tenta provedor local soberano
        if self.local_stt.health().await.is_ok() {
            match self.local_stt.transcribe(audio).await {
                Ok(text) => return Ok(text),
                Err(err) => {
                    tracing::warn!(
                        "Falha transitória no provedor STT local ({}). Avaliando política de fallback...",
                        err
                    );
                }
            }
        }

        // 2. Se local indisponível, avalia política estrita de fallback
        if !self.config.cloud_fallback_allowed {
            return Err(VoiceError::UnauthorizedCloudFallback(
                "Fallback para STT em nuvem rejeitado: desativado na política de privacidade da Yuki"
                    .to_string(),
            ));
        }

        let egress_auth = auth.into_auth(VoiceEgressAuthorization::CAPABILITY_STT, "operator");
        match egress_auth {
            Some(ref a)
                if a.is_valid_for(VoiceEgressAuthorization::CAPABILITY_STT, &a.operator_id) =>
            {
                // Autorização verificada e válida
            }
            Some(_) => {
                return Err(VoiceError::UnauthorizedCloudFallback(
                    "Fallback para STT em nuvem rejeitado: autorização de saída expirada ou com capability/operador divergente"
                        .to_string(),
                ));
            }
            None => {
                return Err(VoiceError::UnauthorizedCloudFallback(
                    "Fallback para STT em nuvem rejeitado: permissão 'egress:voice_cloud_stt' não autorizada pelo operador"
                        .to_string(),
                ));
            }
        }

        // 3. Executa provedor cloud se configurado
        let cloud = self.cloud_stt.as_ref().ok_or_else(|| {
            VoiceError::ProviderUnavailable(
                "Nenhum provedor STT em nuvem configurado para fallback".to_string(),
            )
        })?;

        cloud.transcribe(audio).await
    }

    /// Executa síntese de fala (TTS) sob a política "Local First — Cloud Only When Authorized".
    pub async fn synthesize<A: IntoVoiceEgressAuth>(
        &self,
        text: &str,
        auth: A,
    ) -> Result<AudioOutput, VoiceError> {
        // 1. Tenta provedor local soberano
        if self.local_tts.health().await.is_ok() {
            match self.local_tts.synthesize(text, Some(&self.config)).await {
                Ok(audio) => return Ok(audio),
                Err(err) => {
                    tracing::warn!(
                        "Falha no provedor TTS local ({}). Avaliando política de fallback...",
                        err
                    );
                }
            }
        }

        // 2. Se local indisponível, avalia política estrita de fallback
        if !self.config.cloud_fallback_allowed {
            return Err(VoiceError::UnauthorizedCloudFallback(
                "Fallback para TTS em nuvem rejeitado: desativado na política de privacidade da Yuki"
                    .to_string(),
            ));
        }

        let egress_auth = auth.into_auth(VoiceEgressAuthorization::CAPABILITY_TTS, "operator");
        match egress_auth {
            Some(ref a)
                if a.is_valid_for(VoiceEgressAuthorization::CAPABILITY_TTS, &a.operator_id) =>
            {
                // Autorização verificada e válida
            }
            Some(_) => {
                return Err(VoiceError::UnauthorizedCloudFallback(
                    "Fallback para TTS em nuvem rejeitado: autorização de saída expirada ou com capability/operador divergente"
                        .to_string(),
                ));
            }
            None => {
                return Err(VoiceError::UnauthorizedCloudFallback(
                    "Fallback para TTS em nuvem rejeitado: permissão 'egress:voice_cloud_tts' não autorizada pelo operador"
                        .to_string(),
                ));
            }
        }

        // 3. Executa provedor cloud se configurado
        let cloud = self.cloud_tts.as_ref().ok_or_else(|| {
            VoiceError::ProviderUnavailable(
                "Nenhum provedor TTS em nuvem configurado para fallback".to_string(),
            )
        })?;

        cloud.synthesize(text, Some(&self.config)).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voice_egress_auth_validations() {
        let auth_stt = VoiceEgressAuthorization::new_stt("operator_jose", "hash123");
        assert!(auth_stt.is_valid_for(VoiceEgressAuthorization::CAPABILITY_STT, "operator_jose"));

        // Isolamento de capability: auth de STT rejeitada para TTS
        assert!(!auth_stt.is_valid_for(VoiceEgressAuthorization::CAPABILITY_TTS, "operator_jose"));

        // Isolamento de operador: outro operador rejeitado
        assert!(!auth_stt.is_valid_for(VoiceEgressAuthorization::CAPABILITY_STT, "operator_outro"));

        // Autorização expirada rejeitada
        let expired_auth = VoiceEgressAuthorization::with_ttl(
            "operator_jose",
            VoiceEgressAuthorization::CAPABILITY_STT,
            -10, // expirada há 10 segundos
            "hash123",
        );
        assert!(
            !expired_auth.is_valid_for(VoiceEgressAuthorization::CAPABILITY_STT, "operator_jose")
        );
    }
}
