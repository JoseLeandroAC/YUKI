use crate::capabilities::registry::CapabilityHandler;
use crate::contracts::capability::{CapabilityManifest, RiskClass, SideEffects};
use crate::contracts::errors::YukiError;
use crate::contracts::identifiers::CapabilityId;

pub struct InfoCapability;

impl InfoCapability {
    pub fn new() -> Self {
        Self
    }
}

impl Default for InfoCapability {
    fn default() -> Self {
        Self::new()
    }
}

impl CapabilityHandler for InfoCapability {
    fn manifest(&self) -> CapabilityManifest {
        CapabilityManifest {
            id: CapabilityId::new("system.info"),
            version: "0.1.0".to_string(),
            description: "Obtém informações estáticas e estritamente minimizadas da plataforma e ambiente de execução da Yuki.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
            output_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "os": { "type": "string" },
                    "arch": { "type": "string" },
                    "yuki_version": { "type": "string" }
                },
                "required": ["os", "arch", "yuki_version"],
                "additionalProperties": false
            }),
            required_permissions: vec!["capability:system.info".to_string()],
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
                    "system.info espera objeto vazio de argumentos, recebido: {:?}",
                    map.keys()
                )));
            }
        } else if !input.is_null() {
            return Err(YukiError::InvalidRequest(
                "system.info espera objeto JSON vazio ({})".to_string(),
            ));
        }

        Ok(serde_json::json!({
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "yuki_version": env!("CARGO_PKG_VERSION")
        }))
    }
}
