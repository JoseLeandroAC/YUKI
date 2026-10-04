use crate::contracts::identifiers::generate_unique_id;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::Instant;
use thiserror::Error;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Erros originados no subsistema de credenciais.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CredentialError {
    #[error("Formato de referência de segredo inválido: '{0}'")]
    InvalidReference(String),

    #[error("Credencial não encontrada para a referência: '{0}'")]
    NotFound(String),

    #[error("Backend de segredos indisponível: '{0}'")]
    BackendUnavailable(String),

    #[error("Acesso à credencial negado pela política: '{0}'")]
    AccessDenied(String),
}

/// Referência canônica e tipada para uma credencial externa.
///
/// INVARIANTE ARQUITETURAL:
/// `SecretRef != SecretMaterial`
/// `SecretRef != Automatically Safe Public Metadata`
///
/// A referência indica onde o segredo reside sem conter bytes sensíveis,
/// mas pode revelar topologia ou nomes de variáveis. O registro operacional
/// deve priorizar aliases sanitizados para minimização de dados.
#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SecretRef {
    uri: String,
    sanitized_alias: String,
}

impl SecretRef {
    /// Cria uma nova `SecretRef` a partir de uma URI e um alias sanitizado.
    pub fn new(
        uri: impl Into<String>,
        sanitized_alias: impl Into<String>,
    ) -> Result<Self, CredentialError> {
        let uri = uri.into();
        if !uri.starts_with("secret://") {
            return Err(CredentialError::InvalidReference(uri));
        }

        let alias = sanitized_alias.into();
        Self::validate_alias(&alias)?;

        Ok(Self {
            uri,
            sanitized_alias: alias,
        })
    }

    /// Valida que o alias sanitizado é seguro para auditoria e logs.
    ///
    /// REGRAS DE VALIDAÇÃO:
    /// 1. Não vazio e com tamanho máximo de 64 caracteres.
    /// 2. Contém apenas caracteres minúsculos, números, '-' ou '_'.
    /// 3. Não contém esquemas (`://`), barras (`/`), espaços em branco ou bytes sensíveis.
    fn validate_alias(alias: &str) -> Result<(), CredentialError> {
        if alias.is_empty() || alias.len() > 64 {
            return Err(CredentialError::InvalidReference(format!(
                "Alias de credencial deve ter entre 1 e 64 caracteres: '{}'",
                alias
            )));
        }

        if !alias
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
        {
            return Err(CredentialError::InvalidReference(format!(
                "Alias de credencial deve conter apenas caracteres minúsculos, números, '-' ou '_': '{}'",
                alias
            )));
        }

        Ok(())
    }

    /// Retorna a URI canônica da referência (ex: `secret://env/YUKI_GEMINI_API_KEY`).
    pub fn uri(&self) -> &str {
        &self.uri
    }

    /// Retorna o identificador operacional sanitizado seguro para logs (ex: `cred_gemini_primary`).
    pub fn sanitized_alias(&self) -> &str {
        &self.sanitized_alias
    }
}

impl fmt::Debug for SecretRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SecretRef(alias: {})", self.sanitized_alias)
    }
}

impl fmt::Display for SecretRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.sanitized_alias)
    }
}

/// Material de segredo bruto encapsulado e efêmero.
///
/// INVARIANTES DE SEGURANÇA:
/// 1. NÃO implementa `serde::Serialize` (prevenção em tempo de compilação contra vazamento em JSON/TOML).
/// 2. NÃO expõe segredos em `Debug` (imprime `SecretMaterial([REDACTED])`).
/// 3. NÃO expõe segredos em `Display` (imprime `[REDACTED]`).
/// 4. Zera a memória controlada diretamente pela Yuki no momento do `Drop` via `zeroize`.
///
/// LIMITAÇÃO DE ESCOPO:
/// A garantia de zeroization aplica-se exclusivamente às cópias sob controle direto da Yuki.
/// Cópias internas geradas por sockets de SO, buffers de TLS ou clientes HTTP de terceiros
/// estão sujeitas aos respectivos ciclos de vida dessas camadas.
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct SecretMaterial {
    inner: String,
}

impl SecretMaterial {
    pub fn new(secret: impl Into<String>) -> Self {
        Self {
            inner: secret.into(),
        }
    }

    /// Expõe temporariamente o segredo bruto sob escopo restrito.
    pub fn expose_secret(&self) -> &str {
        &self.inner
    }
}

impl fmt::Debug for SecretMaterial {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SecretMaterial([REDACTED])")
    }
}

impl fmt::Display for SecretMaterial {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[REDACTED]")
    }
}

/// Concessão temporária de acesso a uma credencial com ciclo de vida rastreado.
pub struct SecretLease {
    material: SecretMaterial,
    lease_id: String,
    issued_at: Instant,
}

impl SecretLease {
    pub fn new(material: SecretMaterial) -> Self {
        Self {
            material,
            lease_id: generate_unique_id("lease"),
            issued_at: Instant::now(),
        }
    }

    /// Identificador único da concessão para rastreamento efêmero.
    pub fn lease_id(&self) -> &str {
        &self.lease_id
    }

    /// Instante em que o lease foi emitido.
    pub fn issued_at(&self) -> Instant {
        self.issued_at
    }

    /// Expõe a fatia do segredo para montagem de headers HTTP.
    pub fn expose_secret(&self) -> &str {
        self.material.expose_secret()
    }
}

impl fmt::Debug for SecretLease {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SecretLease")
            .field("lease_id", &self.lease_id)
            .field("issued_at", &self.issued_at)
            .field("material", &"[REDACTED]")
            .finish()
    }
}

/// Contrato mediador para resolução de credenciais.
///
/// INVARIANTE:
/// `CredentialRef -> CredentialBroker -> SecretMaterial -> Transport`
pub trait CredentialBroker: Send + Sync {
    fn acquire(&self, reference: &SecretRef) -> Result<SecretLease, CredentialError>;
}

/// Backend de credenciais em variáveis de ambiente para desenvolvimento do MVP-1.
///
/// NOTA DE SEGURANÇA:
/// Este backend destina-se estritamente ao ambiente de DESENVOLVIMENTO (MVP-1).
/// Variáveis de ambiente NÃO constituem isolamento criptográfico ou de hardware.
/// Backends de produção futuros (Vault, Secret Manager, HSM) implementarão `CredentialBroker`.
pub struct EnvSecretStore;

impl EnvSecretStore {
    pub fn new() -> Self {
        Self
    }
}

impl Default for EnvSecretStore {
    fn default() -> Self {
        Self::new()
    }
}

impl CredentialBroker for EnvSecretStore {
    fn acquire(&self, reference: &SecretRef) -> Result<SecretLease, CredentialError> {
        let uri = reference.uri();
        let var_name = uri
            .strip_prefix("secret://env/")
            .ok_or_else(|| CredentialError::InvalidReference(uri.to_string()))?;

        if var_name.is_empty() {
            return Err(CredentialError::InvalidReference(uri.to_string()));
        }

        match std::env::var(var_name) {
            Ok(val) if !val.trim().is_empty() => Ok(SecretLease::new(SecretMaterial::new(
                val.trim().to_string(),
            ))),
            _ => Err(CredentialError::NotFound(
                reference.sanitized_alias().to_string(),
            )),
        }
    }
}
