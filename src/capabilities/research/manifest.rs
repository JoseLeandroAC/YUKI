use crate::contracts::capability::{CapabilityManifest, RiskClass, SideEffects};
use crate::contracts::identifiers::CapabilityId;

/// Constrói o manifesto canônico para a capability `research.search` (ADR-020).
pub fn search_manifest() -> CapabilityManifest {
    CapabilityManifest {
        id: CapabilityId::new("research.search"),
        version: "0.1.0".to_string(),
        description: "Executa pesquisa textual estruturada sobre índice de busca web sem baixar páginas completas.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "required": ["query"],
            "properties": {
                "query": {
                    "type": "string",
                    "minLength": 2,
                    "maxLength": 200,
                    "description": "Termo de busca textual claro e objetivo"
                },
                "max_results": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 10,
                    "default": 5,
                    "description": "Número máximo de resultados desejados"
                },
                "freshness": {
                    "type": "string",
                    "enum": ["any", "day", "week", "month", "year"],
                    "default": "any",
                    "description": "Filtro temporal de recência dos resultados"
                }
            },
            "additionalProperties": false
        }),
        output_schema: serde_json::json!({
            "type": "object",
            "required": ["query", "provider", "searched_at", "results_count", "results"],
            "properties": {
                "query": { "type": "string" },
                "provider": { "type": "string" },
                "searched_at": { "type": "string" },
                "results_count": { "type": "integer" },
                "results": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "required": ["cite_id", "url", "title", "snippet", "domain", "confidence_state"],
                        "properties": {
                            "cite_id": { "type": "string" },
                            "url": { "type": "string" },
                            "title": { "type": "string" },
                            "snippet": { "type": "string" },
                            "domain": { "type": "string" },
                            "published_date": { "type": ["string", "null"] },
                            "confidence_state": { "type": "string" }
                        }
                    }
                }
            }
        }),
        required_permissions: vec!["capability:research.search".to_string()],
        risk_class: RiskClass::Low,
        side_effects: SideEffects::None,
        network_required: false, // Em Marco 1: Provedor Mock estritamente offline
        filesystem_required: false,
        secrets_required: false,
    }
}

/// Constrói o manifesto canônico para a capability `research.fetch` (ADR-020).
pub fn fetch_manifest() -> CapabilityManifest {
    CapabilityManifest {
        id: CapabilityId::new("research.fetch"),
        version: "0.1.0".to_string(),
        description: "Recupera conteúdo textual limpo e sanitizado de uma URL pública com hash SHA-256 e integridade.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "required": ["url"],
            "properties": {
                "url": {
                    "type": "string",
                    "maxLength": 500,
                    "description": "URL HTTPS/HTTP pública canônica a ser recuperada"
                },
                "max_length_chars": {
                    "type": "integer",
                    "minimum": 500,
                    "maximum": 30000,
                    "default": 10000,
                    "description": "Limite máximo de caracteres de texto útil a extrair"
                }
            },
            "additionalProperties": false
        }),
        output_schema: serde_json::json!({
            "type": "object",
            "required": [
                "url",
                "final_url",
                "fetched_at",
                "http_status",
                "content_type",
                "title",
                "extracted_text",
                "content_hash_sha256",
                "truncated",
                "confidence_state"
            ],
            "properties": {
                "url": { "type": "string" },
                "final_url": { "type": "string" },
                "fetched_at": { "type": "string" },
                "http_status": { "type": "integer" },
                "content_type": { "type": "string" },
                "title": { "type": "string" },
                "published_date": { "type": ["string", "null"] },
                "extracted_text": { "type": "string" },
                "content_hash_sha256": { "type": "string" },
                "truncated": { "type": "boolean" },
                "confidence_state": { "type": "string" }
            }
        }),
        required_permissions: vec!["capability:research.fetch".to_string()],
        risk_class: RiskClass::Low,
        side_effects: SideEffects::None,
        network_required: false, // Em Marco 1: Provedor Mock estritamente offline
        filesystem_required: false,
        secrets_required: false,
    }
}
