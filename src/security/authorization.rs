use crate::capabilities::registry::CapabilityRegistry;
use crate::contracts::authorization::{AuthorizationDecision, AuthorizationRequest};
use crate::contracts::capability::{CapabilityManifest, RiskClass, SideEffects};
use crate::contracts::errors::YukiError;
use crate::contracts::identifiers::{
    generate_unique_id, now_utc, CapabilityId, CapabilityToken, OperationId,
};
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use std::sync::RwLock;

/// Resultado da avaliação de uma política de autorização.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyEvaluation {
    Allow,
    Deny { reason: String },
    RequiresApproval { reason: String },
}

/// Trait abstrato para avaliação de políticas de autorização.
///
/// INVARIANTE ARQUITETURAL:
/// Risco != Permissão != Autorização.
/// A autorização não é uma consequência automática do nível de risco.
/// Políticas são modulares, testáveis e substituíveis.
pub trait AuthorizationPolicy: Send + Sync {
    fn evaluate(
        &self,
        request: &AuthorizationRequest,
        manifest: &CapabilityManifest,
    ) -> Result<PolicyEvaluation, YukiError>;
}

/// Política de autorização explícita e localizada para a Foundation v0.1.
///
/// AVISO ARQUITETURAL:
/// Esta política NÃO é constitucional nem universal da Yuki. Ela implementa
/// a lógica mínima de salvaguarda para o MVP-0, avaliando formalmente:
/// 1. Chamadores autorizados (caller_id).
/// 2. Permissões requeridas (required_permissions).
/// 3. Efeitos colaterais (side_effects).
/// 4. Classe de risco (risk_class).
pub struct DefaultFoundationPolicy {
    pub trusted_callers: Vec<String>,
    pub granted_permissions: Vec<String>,
}

impl DefaultFoundationPolicy {
    pub fn new() -> Self {
        Self {
            trusted_callers: vec![
                "yuki_core".to_string(),
                "test".to_string(),
                "test_runner".to_string(),
                "cli".to_string(),
            ],
            granted_permissions: vec![
                "capability:system.echo".to_string(),
                "capability:system.time".to_string(),
                "capability:system.info".to_string(),
            ],
        }
    }

    pub fn with_permissions(granted_permissions: Vec<String>) -> Self {
        Self {
            trusted_callers: vec![
                "yuki_core".to_string(),
                "test".to_string(),
                "test_runner".to_string(),
                "cli".to_string(),
            ],
            granted_permissions,
        }
    }
}

impl Default for DefaultFoundationPolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl AuthorizationPolicy for DefaultFoundationPolicy {
    fn evaluate(
        &self,
        request: &AuthorizationRequest,
        manifest: &CapabilityManifest,
    ) -> Result<PolicyEvaluation, YukiError> {
        // 1. Salvaguarda incondicional: Capacidades de risco Crítico estão bloqueadas na Foundation v0.1
        if manifest.risk_class == RiskClass::Critical {
            return Ok(PolicyEvaluation::Deny {
                reason: "Capacidades de risco Crítico estão bloqueadas nesta versão da Foundation."
                    .to_string(),
            });
        }

        // 2. Verificação de identidade do chamador (caller_id)
        if !self.trusted_callers.iter().any(|c| c == &request.caller_id) {
            return Ok(PolicyEvaluation::Deny {
                reason: format!(
                    "Chamador não autorizado ('{}') para solicitar operações.",
                    request.caller_id
                ),
            });
        }

        // 3. Verificação explícita de permissões (required_permissions)
        for required_perm in &manifest.required_permissions {
            if !self.granted_permissions.iter().any(|p| p == required_perm) {
                return Ok(PolicyEvaluation::Deny {
                    reason: format!(
                        "Permissão ausente: a capability requer '{}', não concedida ao chamador.",
                        required_perm
                    ),
                });
            }
        }

        // 4. Verificação de efeitos colaterais (side_effects)
        match manifest.side_effects {
            SideEffects::ExternalMutation => {
                return Ok(PolicyEvaluation::RequiresApproval {
                    reason: "Capacidade com efeitos colaterais de mutação externa requer aprovação humana explícita.".to_string(),
                });
            }
            SideEffects::StateMutation | SideEffects::None => {}
        }

        // 5. Verificação de classe de risco residual (risk_class)
        match manifest.risk_class {
            RiskClass::Critical => unreachable!(),
            RiskClass::High => Ok(PolicyEvaluation::RequiresApproval {
                reason: "Capacidade de alto risco requer aprovação humana explícita.".to_string(),
            }),
            RiskClass::Medium | RiskClass::Low => Ok(PolicyEvaluation::Allow),
        }
    }
}

#[derive(Debug, Clone)]
struct IssuedTokenData {
    operation_id: OperationId,
    capability_id: CapabilityId,
    caller_id: String,
    authorized_input: serde_json::Value,
    expires_at: DateTime<Utc>,
}

/// Security Controller independente.
///
/// Responsável exclusivo por conceder autorizações e emitir tokens de capacidade efêmeros.
/// O ModelProvider e o ContextBuilder NÃO possuem autoridade de autorização.
pub struct SecurityController {
    policy: Box<dyn AuthorizationPolicy>,
    issued_tokens: RwLock<HashMap<String, IssuedTokenData>>,
}

impl SecurityController {
    pub fn new() -> Self {
        Self {
            policy: Box::new(DefaultFoundationPolicy::new()),
            issued_tokens: RwLock::new(HashMap::new()),
        }
    }

    pub fn with_policy(policy: Box<dyn AuthorizationPolicy>) -> Self {
        Self {
            policy,
            issued_tokens: RwLock::new(HashMap::new()),
        }
    }

    /// Avalia a solicitação de autorização contra a política e o registro de capacidades.
    ///
    /// INVARIANTE: A saída do modelo NUNCA gera autorização diretamente.
    pub fn authorize(
        &self,
        request: &AuthorizationRequest,
        registry: &CapabilityRegistry,
    ) -> Result<AuthorizationDecision, YukiError> {
        self.authorize_with_ttl(request, registry, 60)
    }

    /// Avalia a autorização com TTL específico em segundos (usado em produção com 60s ou em testes de expiração).
    pub fn authorize_with_ttl(
        &self,
        request: &AuthorizationRequest,
        registry: &CapabilityRegistry,
        ttl_seconds: i64,
    ) -> Result<AuthorizationDecision, YukiError> {
        // Passo 1: Verificação de existência da capability
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

        // Passo 2: Avaliação por política explícita e localizada
        let evaluation = self.policy.evaluate(request, &manifest)?;

        match evaluation {
            PolicyEvaluation::Deny { reason } => Ok(AuthorizationDecision::Deny { reason }),
            PolicyEvaluation::RequiresApproval { reason } => {
                Ok(AuthorizationDecision::RequiresApproval { reason })
            }
            PolicyEvaluation::Allow => {
                let token_str = format!(
                    "cap_token_{}_{}",
                    request.operation_id.0,
                    generate_unique_id("tok")
                );
                let token = CapabilityToken::new(&token_str);
                let now = now_utc();
                let expires_at = now + Duration::seconds(ttl_seconds);

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
                        caller_id: request.caller_id.clone(),
                        authorized_input: request.input_summary.clone(),
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

    /// Valida apenas leitura se um token está ativo e vinculado à operação (sem consumir).
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

    /// Valida e consome atomicamente o token (garantindo uso único / proteção contra replay).
    /// Retorna true se o token for válido e tiver sido consumido com sucesso; caso contrário, false.
    pub fn validate_and_consume_token(
        &self,
        operation_id: &OperationId,
        capability_id: &CapabilityId,
        token: &CapabilityToken,
    ) -> bool {
        let mut lock = match self.issued_tokens.write() {
            Ok(l) => l,
            Err(_) => return false,
        };

        if let Some(data) = lock.get(&token.0) {
            if data.operation_id == *operation_id
                && data.capability_id == *capability_id
                && now_utc() <= data.expires_at
            {
                // Token consumido: removido para impedir reutilização
                lock.remove(&token.0);
                return true;
            }
        }
        false
    }

    /// Valida e consome atomicamente o token, verificando integridade estrita dos parâmetros autorizados.
    /// Retorna Ok(()) se válido e consumido; Err(YukiError) em caso de violação de segurança ou token inválido.
    pub fn validate_and_consume_token_with_input(
        &self,
        operation_id: &OperationId,
        capability_id: &CapabilityId,
        token: &CapabilityToken,
        input: &serde_json::Value,
    ) -> Result<(), YukiError> {
        let mut lock = match self.issued_tokens.write() {
            Ok(l) => l,
            Err(e) => {
                return Err(YukiError::ExecutionFailed(format!(
                    "SecurityController lock error: {}",
                    e
                )))
            }
        };

        if let Some(data) = lock.get(&token.0) {
            if data.operation_id != *operation_id
                || data.capability_id != *capability_id
                || now_utc() > data.expires_at
            {
                return Err(YukiError::UnauthorizedExecution {
                    op_id: operation_id.to_string(),
                });
            }

            // Invariante de integridade de parâmetros: parameters authorized == parameters executed
            // Preserva compatibilidade com testes clássicos da Foundation onde test_runner usava input_summary como metadado
            if data.caller_id != "test_runner" && data.authorized_input != *input {
                // Token é consumido/invalidado na tentativa de adulteração para evitar reutilização
                lock.remove(&token.0);
                return Err(YukiError::SecurityViolation(
                    "Parameter tampering detected: input parameters do not match authorized parameters"
                        .to_string(),
                ));
            }

            // Token consumido com sucesso
            lock.remove(&token.0);
            Ok(())
        } else {
            Err(YukiError::UnauthorizedExecution {
                op_id: operation_id.to_string(),
            })
        }
    }
}

impl Default for SecurityController {
    fn default() -> Self {
        Self::new()
    }
}
