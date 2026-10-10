use crate::contracts::capability::{CapabilityManifest, RiskClass, SideEffects};
use crate::contracts::identifiers::CapabilityId;

/// Constrói o manifesto canônico para a capability `research.search` (ADR-020).
pub fn search_manifest() -> CapabilityManifest {
    search_manifest_for_provider(false)
}

/// Constrói o manifesto canônico adaptado à natureza do provedor (Mock offline vs Live com egress).
pub fn search_manifest_for_provider(live_network: bool) -> CapabilityManifest {
    let mut required_permissions = vec!["capability:research.search".to_string()];
    if live_network {
        required_permissions.push("egress:web_search".to_string());
    }

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
        required_permissions,
        risk_class: if live_network {
            RiskClass::Medium
        } else {
            RiskClass::Low
        },
        side_effects: SideEffects::None,
        network_required: live_network,
        filesystem_required: false,
        secrets_required: live_network,
    }
}

/// Constrói o manifesto canônico para a capability `research.fetch` (ADR-020).
pub fn fetch_manifest() -> CapabilityManifest {
    fetch_manifest_for_provider(false)
}

/// Constrói o manifesto para `research.fetch` diferenciando provedores offline vs live (ADR-020).
pub fn fetch_manifest_for_provider(live_network: bool) -> CapabilityManifest {
    let mut required_permissions = vec!["capability:research.fetch".to_string()];
    if live_network {
        required_permissions.push("egress:web_fetch".to_string());
    }

    CapabilityManifest {
        id: CapabilityId::new("research.fetch"),
        version: "0.1.0".to_string(),
        description: if live_network {
            "Recupera conteúdo textual limpo e sanitizado de uma URL pública na web (modo live com validação SSRF e pinning).".to_string()
        } else {
            "Recupera conteúdo textual limpo e sanitizado de uma URL pública com hash SHA-256 e integridade (modo offline/mock).".to_string()
        },
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
                "bytes_observed": { "type": "integer" },
                "source_id": { "type": ["string", "null"] },
                "confidence_state": { "type": "string" }
            }
        }),
        required_permissions,
        risk_class: if live_network {
            RiskClass::High
        } else {
            RiskClass::Low
        },
        side_effects: SideEffects::None,
        network_required: live_network,
        filesystem_required: false,
        secrets_required: false,
    }
}
