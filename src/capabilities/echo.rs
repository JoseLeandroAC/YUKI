use crate::capabilities::registry::CapabilityHandler;
use crate::contracts::capability::{CapabilityManifest, RiskClass, SideEffects};
use crate::contracts::errors::YukiError;
use crate::contracts::identifiers::CapabilityId;

pub struct EchoCapability;

impl EchoCapability {
    pub fn new() -> Self {
        Self
    }
}

impl Default for EchoCapability {
    fn default() -> Self {
        Self::new()
    }
}

impl CapabilityHandler for EchoCapability {
    fn manifest(&self) -> CapabilityManifest {
        CapabilityManifest {
            id: CapabilityId::new("system.echo"),
            version: "0.1.0".to_string(),
            description: "Recebe um texto e devolve-o através do pipeline oficial da Yuki com segurança e sem efeitos colaterais.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "message": { "type": "string" }
                },
                "required": ["message"],
                "additionalProperties": false
            }),
            output_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "echoed_message": { "type": "string" }
                },
                "required": ["echoed_message"]
            }),
            required_permissions: vec!["capability:system.echo".to_string()],
            risk_class: RiskClass::Low,
            side_effects: SideEffects::None,
            network_required: false,
            filesystem_required: false,
            secrets_required: false,
        }
    }

    fn execute(&self, input: &serde_json::Value) -> Result<serde_json::Value, YukiError> {
        let msg = if let Some(m) = input.get("message").and_then(|v| v.as_str()) {
            m.to_string()
        } else if let Some(s) = input.as_str() {
            s.to_string()
        } else {
            return Err(YukiError::ExecutionFailed(
                "system.echo requer campo 'message' como string".to_string(),
            ));
        };

        Ok(serde_json::json!({
            "echoed_message": msg
        }))
    }
}
