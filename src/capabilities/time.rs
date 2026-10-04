use crate::capabilities::registry::CapabilityHandler;
use crate::contracts::capability::{CapabilityManifest, RiskClass, SideEffects};
use crate::contracts::errors::YukiError;
use crate::contracts::identifiers::{now_utc, CapabilityId};

pub struct TimeCapability;

impl TimeCapability {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TimeCapability {
    fn default() -> Self {
        Self::new()
    }
}

impl CapabilityHandler for TimeCapability {
    fn manifest(&self) -> CapabilityManifest {
        CapabilityManifest {
            id: CapabilityId::new("system.time"),
            version: "0.1.0".to_string(),
            description: "Obtém a data e hora atual do sistema em formato UTC e milissegundos epoch sem mutações ou efeitos externos.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
            output_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "utc_timestamp": { "type": "string" },
                    "epoch_ms": { "type": "integer" }
                },
                "required": ["utc_timestamp", "epoch_ms"],
                "additionalProperties": false
            }),
            required_permissions: vec!["capability:system.time".to_string()],
            risk_class: RiskClass::Low,
            side_effects: SideEffects::None,
            network_required: false,
            filesystem_required: false,
            secrets_required: false,
        }
    }

    fn execute(&self, input: &serde_json::Value) -> Result<serde_json::Value, YukiError> {
        // Defensive validation: input must be an empty object or null
        if let Some(map) = input.as_object() {
            if !map.is_empty() {
                return Err(YukiError::InvalidRequest(format!(
                    "system.time espera objeto vazio de argumentos, recebido: {:?}",
                    map.keys()
                )));
            }
        } else if !input.is_null() {
            return Err(YukiError::InvalidRequest(
                "system.time espera objeto JSON vazio ({})".to_string(),
            ));
        }

        let now = now_utc();
        Ok(serde_json::json!({
            "utc_timestamp": now.to_rfc3339(),
            "epoch_ms": now.timestamp_millis()
        }))
    }
}
