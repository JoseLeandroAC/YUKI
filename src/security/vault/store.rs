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

/// String canária cifrada para validação antecipada da senha mestra.
pub const VAULT_CANARY_PLAINTEXT: &str = "yuki-vault-auth-canary-v1";

/// Limite máximo do tamanho do arquivo do vault em disco (20 MiB).
pub const MAX_VAULT_FILE_BYTES: u64 = 20 * 1024 * 1024;

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

    #[error("Senha mestra do cofre inválida")]
    InvalidPassphrase,

    #[error("Arquivo do cofre excede o limite máximo permitido: {0} bytes")]
    FileTooLarge(u64),

    #[error("Caminho do cofre inválido ou inseguro: {0}")]
    InsecurePath(String),

    #[error("Versão do formato do vault não suportada: {0}")]
    UnsupportedVersion(u32),

    #[error("Erro de persistência de arquivo do vault: {0}")]
    Io(String),

    #[error("Erro de serialização JSON do vault: {0}")]
    Serialization(String),

    #[error("Cofre em uso por outro processo: {0}")]
    ProcessLockBusy(String),
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification: Option<EncryptedBlob>,
    pub entries: BTreeMap<String, VaultEntry>,
}

impl Default for VaultFile {
    fn default() -> Self {
        Self {
            version: VAULT_FORMAT_VERSION,
            verification: None,
            entries: BTreeMap::new(),
        }
    }
}

/// Retorna o caminho canônico do arquivo de lock correspondente ao cofre.
fn lock_path_for(target_path: &Path) -> PathBuf {
    let file_name = target_path
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("vault");
    target_path.with_file_name(format!("{}.lock", file_name))
}

/// Lock interprocessos para sincronização segura de operações no Credential Vault.
pub struct VaultProcessLock {
    #[cfg(windows)]
    _file: File,
    #[cfg(not(windows))]
    lock_path: PathBuf,
}

impl VaultProcessLock {
    /// Adquire lock exclusivo para o arquivo do cofre especificado com timeout/tentativas.
    pub fn acquire(vault_path: &Path) -> Result<Self, VaultError> {
        if let Some(parent) = vault_path.parent() {
            if !parent.as_os_str().is_empty() {
                let _ = fs::create_dir_all(parent);
            }
        }
        let lock_path = lock_path_for(vault_path);

        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            let mut opts = fs::OpenOptions::new();
            opts.read(true).write(true).create(true).share_mode(0);

            let start = std::time::Instant::now();
            loop {
                match opts.open(&lock_path) {
                    Ok(file) => return Ok(Self { _file: file }),
                    Err(e)
                        if e.raw_os_error() == Some(32)
                            || e.kind() == std::io::ErrorKind::PermissionDenied =>
                    {
                        if start.elapsed() > std::time::Duration::from_millis(10_000) {
                            return Err(VaultError::ProcessLockBusy(format!(
                                "Cofre em uso por outro processo (lock em {}): {}",
                                lock_path.display(),
                                e
                            )));
                        }
                        std::thread::sleep(std::time::Duration::from_millis(15));
                    }
                    Err(e) => {
                        return Err(VaultError::Io(format!(
                            "Falha ao adquirir lock do cofre em {}: {}",
                            lock_path.display(),
                            e
                        )));
                    }
                }
            }
        }

        #[cfg(not(windows))]
        {
            let start = std::time::Instant::now();
            loop {
                match fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&lock_path)
                {
                    Ok(mut file) => {
                        let pid = std::process::id();
                        let _ = writeln!(file, "{}", pid);
                        return Ok(Self { lock_path });
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                        if let Ok(meta) = fs::metadata(&lock_path) {
                            if let Ok(elapsed) = meta.modified().and_then(|m| {
                                m.elapsed().map_err(|_| {
                                    std::io::Error::new(std::io::ErrorKind::Other, "time error")
                                })
                            }) {
                                if elapsed > std::time::Duration::from_secs(5) {
                                    let _ = fs::remove_file(&lock_path);
                                    continue;
                                }
                            }
                        }
                        if start.elapsed() > std::time::Duration::from_millis(10_000) {
                            return Err(VaultError::ProcessLockBusy(format!(
                                "Cofre em uso por outro processo (lock em {}): {}",
                                lock_path.display(),
                                e
                            )));
                        }
                        std::thread::sleep(std::time::Duration::from_millis(15));
                    }
                    Err(e) => {
                        return Err(VaultError::Io(format!(
                            "Falha ao adquirir lock do cofre em {}: {}",
                            lock_path.display(),
                            e
                        )));
                    }
                }
            }
        }
    }
}

#[cfg(not(windows))]
impl Drop for VaultProcessLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.lock_path);
    }
}

/// Cofre seguro de credenciais persistido em disco com criptografia autenticada.
///
/// INVARIANTES ARQUITETURAIS:
/// 1. Dados em repouso protegidos por AES-256-GCM + PBKDF2 (10.000 a 500.000 iterações).
/// 2. Persistência estritamente transacional: nenhuma mutação em memória antes de escrita atômica em disco.
/// 3. Autenticação antecipada da senha mestra no `open_or_create` via blob canário.
/// 4. Metadados operacionais estritamente separados do texto cifrado.
/// 5. Nenhum segredo ou chave mestra é exposto em logs ou mensagens de erro.
/// 6. Revogação explícita com fail-closed imediato.
/// 7. Sincronização interprocessos via `VaultProcessLock` e proteção atômica em disco.
pub struct CredentialVault {
    path: PathBuf,
    passphrase: SecretMaterial,
    verification: RwLock<Option<EncryptedBlob>>,
    entries: RwLock<BTreeMap<String, VaultEntry>>,
}

impl std::fmt::Debug for CredentialVault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CredentialVault")
            .field("path", &self.path)
            .field("passphrase", &"[REDACTED]")
            .finish()
    }
}

impl CredentialVault {
    /// Cria um novo cofre de credenciais. Falha com `VaultError::AlreadyExists` se o arquivo já existir.
    pub fn create_new(
        path: impl Into<PathBuf>,
        passphrase: SecretMaterial,
    ) -> Result<Self, VaultError> {
        let path = path.into();

        if passphrase.expose_secret().trim().is_empty() {
            return Err(VaultCryptoError::EmptyPassphrase.into());
        }

        let _lock = VaultProcessLock::acquire(&path)?;

        if path.exists() {
            return Err(VaultError::AlreadyExists(format!(
                "Arquivo do cofre já existe em '{}'",
                path.display()
            )));
        }

        let canary_blob = encrypt_secret(passphrase.expose_secret(), VAULT_CANARY_PLAINTEXT)?;
        let default_vault = VaultFile {
            version: VAULT_FORMAT_VERSION,
            verification: Some(canary_blob.clone()),
            entries: BTreeMap::new(),
        };
        Self::persist_file_atomic(&path, &default_vault)?;
        Ok(Self {
            path,
            passphrase,
            verification: RwLock::new(Some(canary_blob)),
            entries: RwLock::new(BTreeMap::new()),
        })
    }

    /// Abre um cofre existente autenticando a senha mestra antecipadamente.
    /// Falha com `VaultError::NotFound` se o arquivo não existir.
    pub fn open(path: impl Into<PathBuf>, passphrase: SecretMaterial) -> Result<Self, VaultError> {
        let path = path.into();

        if passphrase.expose_secret().trim().is_empty() {
            return Err(VaultCryptoError::EmptyPassphrase.into());
        }

        let _lock = VaultProcessLock::acquire(&path)?;

        if !path.exists() {
            return Err(VaultError::NotFound(format!(
                "Arquivo do cofre não encontrado em '{}'",
                path.display()
            )));
        }

        let symlink_meta = fs::symlink_metadata(&path)
            .map_err(|e| VaultError::Io(format!("Falha ao inspecionar caminho: {}", e)))?;
        if symlink_meta.file_type().is_symlink() {
            return Err(VaultError::InsecurePath(
                "Caminho do cofre não pode ser link simbólico".to_string(),
            ));
        }
        if symlink_meta.len() > MAX_VAULT_FILE_BYTES {
            return Err(VaultError::FileTooLarge(symlink_meta.len()));
        }

        let content = fs::read_to_string(&path)
            .map_err(|e| VaultError::Io(format!("Falha ao ler vault: {}", e)))?;
        let mut vault_file: VaultFile = serde_json::from_str(&content)
            .map_err(|e| VaultError::Serialization(format!("JSON inválido: {}", e)))?;

        if vault_file.version != VAULT_FORMAT_VERSION {
            return Err(VaultError::UnsupportedVersion(vault_file.version));
        }

        // Autenticação antecipada da senha mestra
        let verification = if let Some(ref canary_blob) = vault_file.verification {
            match decrypt_secret(passphrase.expose_secret(), canary_blob) {
                Ok(decrypted) if decrypted.expose_secret() == VAULT_CANARY_PLAINTEXT => {
                    Some(canary_blob.clone())
                }
                _ => return Err(VaultError::InvalidPassphrase),
            }
        } else {
            // Migração transparente de cofre legado (Marco 6) sem canário
            if let Some((_, first_entry)) = vault_file.entries.iter().next() {
                if decrypt_secret(passphrase.expose_secret(), &first_entry.encrypted).is_err() {
                    return Err(VaultError::InvalidPassphrase);
                }
            }
            let canary_blob = encrypt_secret(passphrase.expose_secret(), VAULT_CANARY_PLAINTEXT)?;
            vault_file.verification = Some(canary_blob.clone());
            Self::persist_file_atomic(&path, &vault_file)?;
            Some(canary_blob)
        };

        Ok(Self {
            path,
            passphrase,
            verification: RwLock::new(verification),
            entries: RwLock::new(vault_file.entries),
        })
    }

    /// Abre um cofre existente se existir, ou cria um novo se o arquivo não existir.
    pub fn open_or_create(
        path: impl Into<PathBuf>,
        passphrase: SecretMaterial,
    ) -> Result<Self, VaultError> {
        let path = path.into();
        if path.exists() {
            Self::open(path, passphrase)
        } else {
            Self::create_new(path, passphrase)
        }
    }

    /// Armazena ou atualiza uma credencial cifrada no cofre de forma estritamente transacional.
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

        let _proc_lock = VaultProcessLock::acquire(&self.path)?;

        let mut entries_guard = self
            .entries
            .write()
            .map_err(|_| VaultError::Io("Lock de escrita envenenado".to_string()))?;

        let mut staged = entries_guard.clone();
        staged.insert(alias.to_string(), entry);

        let verification = self
            .verification
            .read()
            .map_err(|_| VaultError::Io("Lock de verificação envenenado".to_string()))?
            .clone();

        let vault_file = VaultFile {
            version: VAULT_FORMAT_VERSION,
            verification,
            entries: staged.clone(),
        };

        // Escrita atômica em disco PRIMEIRO
        Self::persist_file_atomic(&self.path, &vault_file)?;

        // Mutação em memória SOMENTE APÓS sucesso da escrita em disco
        *entries_guard = staged;

        Ok(metadata)
    }

    /// Substitui o segredo de uma credencial existente.
    pub fn replace(
        &self,
        alias: &str,
        secret: &str,
        description: Option<String>,
    ) -> Result<CredentialMetadata, VaultError> {
        Self::validate_alias(alias)?;

        if secret.trim().is_empty() {
            return Err(VaultError::Crypto(VaultCryptoError::CorruptedData(
                "Segredo a armazenar não pode ser vazio".to_string(),
            )));
        }

        let encrypted = encrypt_secret(self.passphrase.expose_secret(), secret)?;

        let _proc_lock = VaultProcessLock::acquire(&self.path)?;

        let mut entries_guard = self
            .entries
            .write()
            .map_err(|_| VaultError::Io("Lock de escrita envenenado".to_string()))?;

        let existing = entries_guard
            .get(alias)
            .ok_or_else(|| VaultError::NotFound(alias.to_string()))?;

        let provider = existing.metadata.provider.clone();
        let desc = description.or_else(|| existing.metadata.description.clone());

        let metadata = CredentialMetadata::new(alias, &provider, desc);

        let entry = VaultEntry {
            metadata: metadata.clone(),
            encrypted,
        };

        let mut staged = entries_guard.clone();
        staged.insert(alias.to_string(), entry);

        let verification = self
            .verification
            .read()
            .map_err(|_| VaultError::Io("Lock de verificação envenenado".to_string()))?
            .clone();

        let vault_file = VaultFile {
            version: VAULT_FORMAT_VERSION,
            verification,
            entries: staged.clone(),
        };

        Self::persist_file_atomic(&self.path, &vault_file)?;
        *entries_guard = staged;

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

    /// Obtém os metadados de uma credencial específica sem decriptar o segredo.
    pub fn get_metadata(&self, alias: &str) -> Result<CredentialMetadata, VaultError> {
        Self::validate_alias(alias)?;

        let entries = self
            .entries
            .read()
            .map_err(|_| VaultError::Io("Lock de leitura envenenado".to_string()))?;

        entries
            .get(alias)
            .map(|e| e.metadata.clone())
            .ok_or_else(|| VaultError::NotFound(alias.to_string()))
    }

    /// Marca uma credencial como revogada de forma estritamente transacional.
    pub fn revoke(&self, alias: &str) -> Result<CredentialMetadata, VaultError> {
        Self::validate_alias(alias)?;
        let _proc_lock = VaultProcessLock::acquire(&self.path)?;

        let mut entries_guard = self
            .entries
            .write()
            .map_err(|_| VaultError::Io("Lock de escrita envenenado".to_string()))?;

        let mut staged = entries_guard.clone();
        let entry = staged
            .get_mut(alias)
            .ok_or_else(|| VaultError::NotFound(alias.to_string()))?;

        entry.metadata.revoke();
        let metadata = entry.metadata.clone();

        let verification = self
            .verification
            .read()
            .map_err(|_| VaultError::Io("Lock de verificação envenenado".to_string()))?
            .clone();

        let vault_file = VaultFile {
            version: VAULT_FORMAT_VERSION,
            verification,
            entries: staged.clone(),
        };

        Self::persist_file_atomic(&self.path, &vault_file)?;
        *entries_guard = staged;

        Ok(metadata)
    }

    /// Remove permanentemente uma credencial de forma estritamente transacional.
    pub fn delete(&self, alias: &str) -> Result<(), VaultError> {
        Self::validate_alias(alias)?;
        let _proc_lock = VaultProcessLock::acquire(&self.path)?;

        let mut entries_guard = self
            .entries
            .write()
            .map_err(|_| VaultError::Io("Lock de escrita envenenado".to_string()))?;

        let mut staged = entries_guard.clone();
        if staged.remove(alias).is_none() {
            return Err(VaultError::NotFound(alias.to_string()));
        }

        let verification = self
            .verification
            .read()
            .map_err(|_| VaultError::Io("Lock de verificação envenenado".to_string()))?
            .clone();

        let vault_file = VaultFile {
            version: VAULT_FORMAT_VERSION,
            verification,
            entries: staged.clone(),
        };

        Self::persist_file_atomic(&self.path, &vault_file)?;
        *entries_guard = staged;

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
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).map_err(|e| {
                    VaultError::Io(format!("Falha ao criar diretório do vault: {}", e))
                })?;
            }
        }

        if let Ok(metadata) = fs::symlink_metadata(target_path) {
            if metadata.file_type().is_symlink() {
                return Err(VaultError::InsecurePath(
                    "Caminho do cofre não pode ser link simbólico".to_string(),
                ));
            }
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
            #[cfg(windows)]
            let mut file = {
                use std::os::windows::fs::OpenOptionsExt;
                let mut opts = fs::OpenOptions::new();
                opts.write(true).create_new(true).share_mode(0);
                opts.open(&tmp_path).map_err(|e| {
                    VaultError::Io(format!("Falha ao criar arquivo temporário do vault: {}", e))
                })?
            };

            #[cfg(not(windows))]
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
