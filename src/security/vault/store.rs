use crate::security::credentials::{SecretMaterial, SecretRef};
use crate::security::vault::crypto::{
    decrypt_secret, encrypt_secret, EncryptedBlob, VaultCryptoError,
};
use crate::security::vault::metadata::{CredentialMetadata, CredentialStatus};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;
use std::time::SystemTime;
use thiserror::Error;

/// Versão canônica atual do formato de armazenamento do Vault.
pub const VAULT_FORMAT_VERSION: u32 = 1;

static TMP_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Erros originados no armazenamento e gestão do Vault.
#[derive(Debug, Error)]
pub enum VaultError {
    #[error("Erro criptográfico no vault: {0}")]
    Crypto(#[from] VaultCryptoError),

    #[error("Credencial '{0}' não encontrada no vault")]
    NotFound(String),

    #[error("Credencial '{0}' já existe no vault")]
    AlreadyExists(String),

    #[error("Credencial '{0}' está revogada e seu uso foi negado")]
    Revoked(String),

    #[error("Credencial '{0}' expirou")]
    Expired(String),

    #[error("Alias de credencial inválido: {0}")]
    InvalidAlias(String),

    #[error("Versão do formato do vault não suportada: {0}")]
    UnsupportedVersion(u32),

    #[error("Erro de persistência de arquivo do vault: {0}")]
    Io(String),

    #[error("Erro de serialização JSON do vault: {0}")]
    Serialization(String),
}

/// Registro persistido contendo metadados claros e segredo cifrado.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultEntry {
    pub metadata: CredentialMetadata,
    pub encrypted: EncryptedBlob,
}

/// Estrutura raiz persistida do arquivo do Vault.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultFile {
    pub version: u32,
    pub entries: BTreeMap<String, VaultEntry>,
}

impl Default for VaultFile {
    fn default() -> Self {
        Self {
            version: VAULT_FORMAT_VERSION,
            entries: BTreeMap::new(),
        }
    }
}

/// Cofre seguro de credenciais persistido em disco com criptografia autenticada.
///
/// INVARIANTES ARQUITETURAIS:
/// 1. Dados em repouso protegidos por AES-256-GCM + PBKDF2 (100.000 iterações).
/// 2. Persistência atômica via escrita em arquivo temporário + flush + sync + rename.
/// 3. Metadados operacionais estritamente separados do texto cifrado.
/// 4. Nenhum segredo ou chave mestra é exposto em logs ou mensagens de erro.
/// 5. Revogação explícita com fail-closed imediato.
pub struct CredentialVault {
    path: PathBuf,
    passphrase: SecretMaterial,
    entries: RwLock<BTreeMap<String, VaultEntry>>,
}

impl CredentialVault {
    /// Abre um cofre existente ou cria um novo se o arquivo não existir.
    pub fn open_or_create(
        path: impl Into<PathBuf>,
        passphrase: SecretMaterial,
    ) -> Result<Self, VaultError> {
        let path = path.into();

        if passphrase.expose_secret().trim().is_empty() {
            return Err(VaultCryptoError::EmptyPassphrase.into());
        }

        let entries = if path.exists() {
            let content = fs::read_to_string(&path)
                .map_err(|e| VaultError::Io(format!("Falha ao ler vault: {}", e)))?;
            let vault_file: VaultFile = serde_json::from_str(&content)
                .map_err(|e| VaultError::Serialization(format!("JSON inválido: {}", e)))?;

            if vault_file.version != VAULT_FORMAT_VERSION {
                return Err(VaultError::UnsupportedVersion(vault_file.version));
            }

            vault_file.entries
        } else {
            let default_vault = VaultFile::default();
            Self::persist_file_atomic(&path, &default_vault)?;
            default_vault.entries
        };

        Ok(Self {
            path,
            passphrase,
            entries: RwLock::new(entries),
        })
    }

    /// Armazena ou atualiza uma credencial cifrada no cofre.
    ///
    /// Valida o alias canônico contra as políticas de segurança da plataforma.
    pub fn store(
        &self,
        alias: &str,
        secret: &str,
        provider: &str,
        description: Option<String>,
    ) -> Result<CredentialMetadata, VaultError> {
        Self::validate_alias(alias)?;

        if secret.trim().is_empty() {
            return Err(VaultError::Crypto(VaultCryptoError::CorruptedData(
                "Segredo a armazenar não pode ser vazio".to_string(),
            )));
        }

        let encrypted = encrypt_secret(self.passphrase.expose_secret(), secret)?;
        let metadata = CredentialMetadata::new(alias, provider, description);

        let entry = VaultEntry {
            metadata: metadata.clone(),
            encrypted,
        };

        let mut entries = self
            .entries
            .write()
            .map_err(|_| VaultError::Io("Lock de escrita envenenado".to_string()))?;

        entries.insert(alias.to_string(), entry);

        let vault_file = VaultFile {
            version: VAULT_FORMAT_VERSION,
            entries: entries.clone(),
        };

        Self::persist_file_atomic(&self.path, &vault_file)?;

        Ok(metadata)
    }

    /// Recupera e decripta o material de segredo associado ao alias.
    ///
    /// Falha fechado se a credencial estiver revogada ou expirada.
    pub fn get(&self, alias: &str) -> Result<SecretMaterial, VaultError> {
        Self::validate_alias(alias)?;

        let entries = self
            .entries
            .read()
            .map_err(|_| VaultError::Io("Lock de leitura envenenado".to_string()))?;

        let entry = entries
            .get(alias)
            .ok_or_else(|| VaultError::NotFound(alias.to_string()))?;

        match entry.metadata.status {
            CredentialStatus::Active => {}
            CredentialStatus::Revoked => return Err(VaultError::Revoked(alias.to_string())),
            CredentialStatus::Expired => return Err(VaultError::Expired(alias.to_string())),
        }

        let material = decrypt_secret(self.passphrase.expose_secret(), &entry.encrypted)?;
        Ok(material)
    }

    /// Marca uma credencial como revogada sem excluir seu histórico de metadados.
    pub fn revoke(&self, alias: &str) -> Result<CredentialMetadata, VaultError> {
        Self::validate_alias(alias)?;

        let mut entries = self
            .entries
            .write()
            .map_err(|_| VaultError::Io("Lock de escrita envenenado".to_string()))?;

        let entry = entries
            .get_mut(alias)
            .ok_or_else(|| VaultError::NotFound(alias.to_string()))?;

        entry.metadata.revoke();
        let metadata = entry.metadata.clone();

        let vault_file = VaultFile {
            version: VAULT_FORMAT_VERSION,
            entries: entries.clone(),
        };

        Self::persist_file_atomic(&self.path, &vault_file)?;

        Ok(metadata)
    }

    /// Remove permanentemente uma credencial do cofre.
    pub fn delete(&self, alias: &str) -> Result<(), VaultError> {
        Self::validate_alias(alias)?;

        let mut entries = self
            .entries
            .write()
            .map_err(|_| VaultError::Io("Lock de escrita envenenado".to_string()))?;

        if entries.remove(alias).is_none() {
            return Err(VaultError::NotFound(alias.to_string()));
        }

        let vault_file = VaultFile {
            version: VAULT_FORMAT_VERSION,
            entries: entries.clone(),
        };

        Self::persist_file_atomic(&self.path, &vault_file)?;

        Ok(())
    }

    /// Retorna a lista completa de metadados das credenciais sem decriptar nenhum segredo.
    pub fn list_metadata(&self) -> Result<Vec<CredentialMetadata>, VaultError> {
        let entries = self
            .entries
            .read()
            .map_err(|_| VaultError::Io("Lock de leitura envenenado".to_string()))?;

        Ok(entries.values().map(|e| e.metadata.clone()).collect())
    }

    /// Retorna se a credencial existe e está com status ativo.
    pub fn is_active(&self, alias: &str) -> Result<bool, VaultError> {
        Self::validate_alias(alias)?;

        let entries = self
            .entries
            .read()
            .map_err(|_| VaultError::Io("Lock de leitura envenenado".to_string()))?;

        Ok(entries
            .get(alias)
            .map(|e| e.metadata.is_active())
            .unwrap_or(false))
    }

    /// Valida o alias segundo as regras sanitizadas da plataforma Yuki.
    fn validate_alias(alias: &str) -> Result<(), VaultError> {
        SecretRef::new(format!("secret://vault/{}", alias), alias)
            .map(|_| ())
            .map_err(|e| VaultError::InvalidAlias(e.to_string()))
    }

    /// Persiste o arquivo do cofre de forma atômica e segura.
    fn persist_file_atomic(target_path: &Path, content: &VaultFile) -> Result<(), VaultError> {
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| VaultError::Io(format!("Falha ao criar diretório do vault: {}", e)))?;
        }

        let counter = TMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let tmp_file_name = format!(
            "{}.tmp.{}.{}",
            target_path
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or("vault"),
            timestamp,
            counter
        );

        let tmp_path = target_path.with_file_name(tmp_file_name);

        let serialized = serde_json::to_string_pretty(content)
            .map_err(|e| VaultError::Serialization(e.to_string()))?;

        {
            let mut file = File::create(&tmp_path).map_err(|e| {
                VaultError::Io(format!("Falha ao criar arquivo temporário do vault: {}", e))
            })?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = fs::set_permissions(&tmp_path, fs::Permissions::from_mode(0o600));
            }

            file.write_all(serialized.as_bytes()).map_err(|e| {
                let _ = fs::remove_file(&tmp_path);
                VaultError::Io(format!("Falha ao escrever dados do vault: {}", e))
            })?;

            file.sync_all().map_err(|e| {
                let _ = fs::remove_file(&tmp_path);
                VaultError::Io(format!("Falha ao sincronizar arquivo do vault: {}", e))
            })?;
        }

        fs::rename(&tmp_path, target_path).map_err(|e| {
            let _ = fs::remove_file(&tmp_path);
            VaultError::Io(format!("Falha no rename atômico do vault: {}", e))
        })?;

        Ok(())
    }
}

/// Configuração tipada do Credential Vault.
#[derive(Debug, Clone)]
pub struct VaultConfig {
    pub vault_path: PathBuf,
}

impl VaultConfig {
    pub fn new(vault_path: impl Into<PathBuf>) -> Self {
        Self {
            vault_path: vault_path.into(),
        }
    }

    /// Carrega configuração a partir de variáveis de ambiente com fallback padrão seguro.
    pub fn from_env() -> Self {
        let path = std::env::var("YUKI_VAULT_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("data/vault.json"));
        Self { vault_path: path }
    }
}

impl Default for VaultConfig {
    fn default() -> Self {
        Self::from_env()
    }
}
