use crate::contracts::errors::YukiError;

/// Trusted pre-authorization contract validator for capability inputs.
///
/// INVARIANTS:
/// - Fails closed: any schema mismatch, unexpected property, or invalid type is rejected.
/// - Operates BEFORE authorization: prevents minting tokens for malformed proposals.
/// - Provider-neutral: operates directly on canonical JSON values.
pub fn validate_capability_input(
    schema: &serde_json::Value,
    input: &serde_json::Value,
) -> Result<(), YukiError> {
    // 1. Root object shape validation
    let schema_type = schema.get("type").and_then(|v| v.as_str());
    if schema_type == Some("object") && !input.is_object() {
        return Err(YukiError::InvalidRequest(
            "Parâmetros da proposta devem constituir um objeto JSON ({})".to_string(),
        ));
    }

    let input_map = match input.as_object() {
        Some(m) => m,
        None => {
            if schema_type == Some("object") {
                return Err(YukiError::InvalidRequest(
                    "Objeto JSON esperado para validação de esquema".to_string(),
                ));
            }
            return Ok(());
        }
    };

    // 2. Required properties check
    if let Some(required) = schema.get("required").and_then(|v| v.as_array()) {
        for req_field in required {
            if let Some(field_name) = req_field.as_str() {
                if !input_map.contains_key(field_name) {
                    return Err(YukiError::InvalidRequest(format!(
                        "Campo obrigatório ausente nos parâmetros: '{}'",
                        field_name
                    )));
                }
                if input_map.get(field_name).is_some_and(|v| v.is_null()) {
                    return Err(YukiError::InvalidRequest(format!(
                        "Campo obrigatório não pode ser nulo: '{}'",
                        field_name
                    )));
                }
            }
        }
    }

    // 3. Properties type validation
    if let Some(properties) = schema.get("properties").and_then(|v| v.as_object()) {
        for (prop_name, prop_schema) in properties {
            if let Some(prop_val) = input_map.get(prop_name) {
                if let Some(expected_type) = prop_schema.get("type").and_then(|v| v.as_str()) {
                    let matches_type = match expected_type {
                        "string" => prop_val.is_string(),
                        "integer" => prop_val.is_i64() || prop_val.is_u64(),
                        "number" => prop_val.is_number(),
                        "boolean" => prop_val.is_boolean(),
                        "object" => prop_val.is_object(),
                        "array" => prop_val.is_array(),
                        _ => {
                            return Err(YukiError::InvalidRequest(format!(
                                "Tipo de esquema não suportado: '{}' para campo '{}'",
                                expected_type, prop_name
                            )));
                        }
                    };

                    if !matches_type {
                        return Err(YukiError::InvalidRequest(format!(
                            "Campo '{}' possui tipo incompatível: esperado '{}'",
                            prop_name, expected_type
                        )));
                    }
                }
            }
        }

        // 4. additionalProperties check
        let allow_additional = schema
            .get("additionalProperties")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if !allow_additional {
            for key in input_map.keys() {
                if !properties.contains_key(key) {
                    return Err(YukiError::InvalidRequest(format!(
                        "Propriedade inesperada não permitida pelo esquema: '{}'",
                        key
                    )));
                }
            }
        }
    } else {
        // If properties is absent or empty, check additionalProperties
        let allow_additional = schema
            .get("additionalProperties")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if !allow_additional && !input_map.is_empty() {
            return Err(YukiError::InvalidRequest(format!(
                "Esquema exige objeto vazio, recebidas propriedades: {:?}",
                input_map.keys()
            )));
        }
    }

    Ok(())
}
