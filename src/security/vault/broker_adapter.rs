use crate::security::credentials::{
    CredentialBroker, CredentialError, SecretLease, SecretMaterial, SecretRef,
};
use crate::security::vault::metadata::CredentialMetadata;
use crate::security::vault::store::{CredentialVault, VaultError};
use std::sync::Arc;

/// Trait de abstração para repositórios seguros de segredos.
pub trait SecretStore: Send + Sync {
    /// Obtém o material do segredo associado ao alias, se existir e estiver ativo.
    fn get_secret(&self, alias: &str) -> Result<Option<SecretMaterial>, VaultError>;

    /// Retorna os metadados de todas as credenciais registradas.
    fn list_metadata(&self) -> Result<Vec<CredentialMetadata>, VaultError>;

    /// Retorna se a credencial existe e está ativa.
    fn is_active(&self, alias: &str) -> Result<bool, VaultError>;
}

/// Adaptador que conecta o `CredentialVault` à arquitetura de credenciais Yuki (`CredentialBroker`).
pub struct VaultSecretStore {
    vault: Arc<CredentialVault>,
}

impl VaultSecretStore {
    pub fn new(vault: Arc<CredentialVault>) -> Self {
        Self { vault }
    }

    pub fn vault(&self) -> &Arc<CredentialVault> {
        &self.vault
    }
}

impl SecretStore for VaultSecretStore {
    fn get_secret(&self, alias: &str) -> Result<Option<SecretMaterial>, VaultError> {
        match self.vault.get(alias) {
            Ok(material) => Ok(Some(material)),
            Err(VaultError::NotFound(_)) => Ok(None),
            Err(e) => Err(e),
        }
    }

    fn list_metadata(&self) -> Result<Vec<CredentialMetadata>, VaultError> {
        self.vault.list_metadata()
    }

    fn is_active(&self, alias: &str) -> Result<bool, VaultError> {
        self.vault.is_active(alias)
    }
}

impl CredentialBroker for VaultSecretStore {
    fn acquire(&self, reference: &SecretRef) -> Result<SecretLease, CredentialError> {
        let uri = reference.uri();
        let alias = uri
            .strip_prefix("secret://vault/")
            .ok_or_else(|| CredentialError::InvalidReference(uri.to_string()))?;

        if alias.is_empty() {
            return Err(CredentialError::InvalidReference(uri.to_string()));
        }

        match self.vault.get(alias) {
            Ok(material) => Ok(SecretLease::new(material)),
            Err(VaultError::NotFound(_)) => Err(CredentialError::NotFound(
                reference.sanitized_alias().to_string(),
            )),
            Err(VaultError::Revoked(a)) => Err(CredentialError::AccessDenied(format!(
                "Credencial '{}' está revogada",
                a
            ))),
            Err(VaultError::Expired(a)) => Err(CredentialError::AccessDenied(format!(
                "Credencial '{}' expirou",
                a
            ))),
            Err(e) => Err(CredentialError::BackendUnavailable(e.to_string())),
        }
    }
}
