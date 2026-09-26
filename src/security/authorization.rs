use crate::capabilities::registry::CapabilityRegistry;
use crate::contracts::authorization::{AuthorizationDecision, AuthorizationRequest};
use crate::contracts::capability::RiskClass;
use crate::contracts::errors::YukiError;
use crate::contracts::identifiers::{
    generate_unique_id, now_utc, CapabilityId, CapabilityToken, OperationId,
};
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use std::sync::RwLock;

#[derive(Debug, Clone)]
struct IssuedTokenData {
    operation_id: OperationId,
    capability_id: CapabilityId,
    expires_at: DateTime<Utc>,
}

pub struct SecurityController {
    issued_tokens: RwLock<HashMap<String, IssuedTokenData>>,
}

impl SecurityController {
    pub fn new() -> Self {
        Self {
            issued_tokens: RwLock::new(HashMap::new()),
        }
    }

    /// Evaluates an authorization request against policies and capability registry.
    /// Model output NEVER grants authorization directly.
    pub fn authorize(
        &self,
        request: &AuthorizationRequest,
        registry: &CapabilityRegistry,
    ) -> Result<AuthorizationDecision, YukiError> {
        // Step 1: Capability existence check
        let manifest = match registry.get_manifest(&request.capability_id) {
            Some(m) => m,
            None => {
                return Ok(AuthorizationDecision::Deny {
                    reason: format!(
                        "Capability '{}' não existe no registro.",
                        request.capability_id
                    ),
                });
            }
        };

        // Step 2: Risk and permission policy evaluation
        match manifest.risk_class {
            RiskClass::Critical => Ok(AuthorizationDecision::Deny {
                reason: "Capacidades de risco Crítico estão bloqueadas nesta versão.".to_string(),
            }),
            RiskClass::High => Ok(AuthorizationDecision::RequiresApproval {
                reason: "Capacidade requer aprovação humana explícita.".to_string(),
            }),
            RiskClass::Medium | RiskClass::Low => {
                // In Foundation v0.1, Low and Medium risk capabilities from known callers are allowed
                let token_str = format!(
                    "cap_token_{}_{}",
                    request.operation_id.0,
                    generate_unique_id("tok")
                );
                let token = CapabilityToken::new(&token_str);
                let now = now_utc();
                let expires_at = now + Duration::seconds(60);

                let mut lock = self.issued_tokens.write().map_err(|e| {
                    YukiError::ExecutionFailed(format!(
                        "Falha de lock no SecurityController: {}",
                        e
                    ))
                })?;

                lock.insert(
                    token.0.clone(),
                    IssuedTokenData {
                        operation_id: request.operation_id.clone(),
                        capability_id: request.capability_id.clone(),
                        expires_at,
                    },
                );

                Ok(AuthorizationDecision::Allow {
                    token,
                    authorized_at: now,
                    expires_at,
                })
            }
        }
    }

    /// Validates that a token is valid, active, and bound to the specified operation and capability.
    pub fn validate_token(
        &self,
        operation_id: &OperationId,
        capability_id: &CapabilityId,
        token: &CapabilityToken,
    ) -> bool {
        let lock = match self.issued_tokens.read() {
            Ok(l) => l,
            Err(_) => return false,
        };

        if let Some(data) = lock.get(&token.0) {
            data.operation_id == *operation_id
                && data.capability_id == *capability_id
                && now_utc() <= data.expires_at
        } else {
            false
        }
    }
}

impl Default for SecurityController {
    fn default() -> Self {
        Self::new()
    }
}
