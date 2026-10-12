use crate::contracts::voice::{
    AudioInput, AudioOutput, SpeechToTextProvider, TextToSpeechProvider, VoiceConfiguration,
    VoiceError,
};
use std::sync::Arc;

/// Seletor e roteador governado de provedores de voz.
///
/// DIRETRIZ CANÔNICA DE PRIVACIDADE E SOBERANIA:
/// "LOCAL FIRST — CLOUD ONLY WHEN AUTHORIZED"
///
/// 1. Tenta prioritariamente o provedor local de maior soberania e menor latência.
/// 2. Se o provedor local falhar ou estiver inacessível:
///    a) Valida se o fallback em nuvem está habilitado nas configurações (`cloud_fallback_allowed`).
///    b) Valida se o operador concedeu expressamente a permissão específica:
///       - Para transcrição de áudio: `egress:voice_cloud_stt`
///       - Para síntese de texto: `egress:voice_cloud_tts`
///       (Essas permissões são formalmente desacopladas: permitir envio de texto não autoriza envio de áudio bruto).
///    c) Somente após essa dupla verificação despacha a transmissão ao provedor externo.
///    d) Se não autorizado, falha fechado mantendo a integridade e os limites de privacidade.
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
    pub async fn transcribe(
        &self,
        audio: &AudioInput,
        operator_authorized_stt_cloud: bool,
    ) -> Result<String, VoiceError> {
        // 1. Tenta provedor local
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

        // 2. Se local indisponível, avalia política de fallback
        if !self.config.cloud_fallback_allowed {
            return Err(VoiceError::UnauthorizedCloudFallback(
                "Fallback para STT em nuvem rejeitado: desativado na política de privacidade da Yuki"
                    .to_string(),
            ));
        }

        if !operator_authorized_stt_cloud {
            return Err(VoiceError::UnauthorizedCloudFallback(
                "Fallback para STT em nuvem rejeitado: permissão 'egress:voice_cloud_stt' não autorizada pelo operador"
                    .to_string(),
            ));
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
    pub async fn synthesize(
        &self,
        text: &str,
        operator_authorized_tts_cloud: bool,
    ) -> Result<AudioOutput, VoiceError> {
        // 1. Tenta provedor local
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

        // 2. Se local indisponível, avalia política de fallback
        if !self.config.cloud_fallback_allowed {
            return Err(VoiceError::UnauthorizedCloudFallback(
                "Fallback para TTS em nuvem rejeitado: desativado na política de privacidade da Yuki"
                    .to_string(),
            ));
        }

        if !operator_authorized_tts_cloud {
            return Err(VoiceError::UnauthorizedCloudFallback(
                "Fallback para TTS em nuvem rejeitado: permissão 'egress:voice_cloud_tts' não autorizada pelo operador"
                    .to_string(),
            ));
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
