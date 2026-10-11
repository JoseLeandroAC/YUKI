use crate::contracts::identifiers::now_utc;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Estado operacional da credencial no vault.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CredentialStatus {
    /// Credencial ativa e apta para emissão de leases.
    Active,
    /// Credencial revogada explicitamente; requisições falham fechadas.
    Revoked,
    /// Credencial expirada por política temporal.
    Expired,
}

/// Metadados públicos e auditáveis de uma credencial no Vault.
///
/// INVARIANTE INEGOCIÁVEL:
/// `CredentialMetadata != SecretMaterial`
/// `CredentialMetadata != EncryptedBlob`
///
/// Esta estrutura NUNCA contém o segredo em claro, texto cifrado, salt ou nonce.
/// É segura para serialização, logs de auditoria e listagens administrativas.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CredentialMetadata {
    /// Identificador canônico sanitizado da credencial (ex: `langsearch_api_key`).
    pub alias: String,
    /// Identificador do provedor/serviço correspondente (ex: `langsearch`, `brave`, `gemini`).
    pub provider: String,
    /// Status operacional da credencial.
    pub status: CredentialStatus,
    /// Timestamp UTC de registro inicial da credencial.
    pub created_at: DateTime<Utc>,
    /// Timestamp UTC da última atualização ou alteração de status.
    pub updated_at: DateTime<Utc>,
    /// Descrição opcional não-sensível para operadores humanos.
    pub description: Option<String>,
}

impl CredentialMetadata {
    /// Cria novo registro de metadados com status `Active`.
    pub fn new(
        alias: impl Into<String>,
        provider: impl Into<String>,
        description: Option<String>,
    ) -> Self {
        let now = now_utc();
        Self {
            alias: alias.into(),
            provider: provider.into(),
            status: CredentialStatus::Active,
            created_at: now,
            updated_at: now,
            description,
        }
    }

    /// Retorna se a credencial está ativa e apta a ser utilizada.
    pub fn is_active(&self) -> bool {
        self.status == CredentialStatus::Active
    }

    /// Marca a credencial como revogada e atualiza o timestamp.
    pub fn revoke(&mut self) {
        self.status = CredentialStatus::Revoked;
        self.updated_at = now_utc();
    }
}
