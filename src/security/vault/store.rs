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

    #[error("Cofre legado sem canário e sem credenciais verificáveis: '{0}'. Inicialização explícita necessária")]
    UnverifiableLegacyVault(String),
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

/// Obtém a identidade do usuário atual no Windows (SID nativo ou Conta).
#[cfg(windows)]
fn get_windows_user_target() -> Result<String, VaultError> {
    // 1. Tenta obter o SID nativo do usuário via `whoami /user /fo csv /nh`
    if let Ok(output) = std::process::Command::new("whoami")
        .args(["/user", "/fo", "csv", "/nh"])
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let parts: Vec<&str> = stdout.trim().split(',').collect();
            if parts.len() >= 2 {
                let sid = parts[1].trim().trim_matches('"');
                if sid.starts_with("S-1-") {
                    return Ok(format!("*{}", sid));
                }
            }
        }
    }

    // 2. Fallback para USERDOMAIN\USERNAME
    if let (Ok(domain), Ok(user)) = (std::env::var("USERDOMAIN"), std::env::var("USERNAME")) {
        let domain = domain.trim();
        let user = user.trim();
        if !domain.is_empty() && !user.is_empty() {
            return Ok(format!("{}\\{}", domain, user));
        }
    }

    // 3. Fallback para USERNAME
    if let Ok(user) = std::env::var("USERNAME") {
        let user = user.trim();
        if !user.is_empty() {
            return Ok(user.to_string());
        }
    }

    Err(VaultError::Io(
        "Não foi possível determinar a identidade do usuário do Windows para aplicar permissões restritas"
            .to_string(),
    ))
}

/// Aplica permissões restritas e verificáveis em arquivos sensíveis do Vault.
/// - No Unix: modo 0o600 (leitura/escrita exclusiva para o proprietário) com verificação de metadados pós-aplicação.
/// - No Windows: remove herança de ACL e concede controle total exclusivamente ao usuário/SID atual via `icacls`,
///   com verificação das permissões efetivas.
pub fn restrict_file_permissions(path: &Path) -> Result<(), VaultError> {
    if !path.exists() {
        return Err(VaultError::Io(format!(
            "Arquivo inexistente ao tentar restringir permissões: '{}'",
            path.display()
        )));
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::Permissions::from_mode(0o600);
        fs::set_permissions(path, perms).map_err(|e| {
            VaultError::Io(format!(
                "Falha ao definir permissões 0600 em '{}': {}",
                path.display(),
                e
            ))
        })?;

        let meta = fs::metadata(path).map_err(|e| {
            VaultError::Io(format!(
                "Falha ao inspecionar metadados pós-aplicação em '{}': {}",
                path.display(),
                e
            ))
        })?;

        let mode = meta.permissions().mode();
        if mode & 0o777 != 0o600 {
            return Err(VaultError::Io(format!(
                "Permissões verificadas em '{}' divergentes do modo esperado 0600 (obtido: {:o})",
                path.display(),
                mode & 0o777
            )));
        }

        Ok(())
    }

    #[cfg(windows)]
    {
        let target = get_windows_user_target()?;
        let grant_arg = format!("{}:(F)", target);

        let output = std::process::Command::new("icacls")
            .arg(path)
            .arg("/inheritance:r")
            .arg("/grant:r")
            .arg(&grant_arg)
            .output()
            .map_err(|e| {
                VaultError::Io(format!(
                    "Falha ao executar icacls em '{}': {}",
                    path.display(),
                    e
                ))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Err(VaultError::Io(format!(
                "icacls falhou em '{}' com status {}: {} {}",
                path.display(),
                output.status,
                stderr.trim(),
                stdout.trim()
            )));
        }

        let stdout_apply = String::from_utf8_lossy(&output.stdout);
        if stdout_apply.contains("falha no processamento de 1")
            || stdout_apply.contains("Failed processing 1")
        {
            return Err(VaultError::Io(format!(
                "icacls reportou falha no processamento de '{}': {}",
                path.display(),
                stdout_apply.trim()
            )));
        }

        // Verificação das permissões efetivas após aplicação
        let verify_output = std::process::Command::new("icacls")
            .arg(path)
            .output()
            .map_err(|e| {
                VaultError::Io(format!(
                    "Falha ao verificar permissões efetivas com icacls em '{}': {}",
                    path.display(),
                    e
                ))
            })?;

        if !verify_output.status.success() {
            return Err(VaultError::Io(format!(
                "icacls falhou ao inspecionar permissões efetivas de '{}'",
                path.display()
            )));
        }

        let verify_stdout = String::from_utf8_lossy(&verify_output.stdout);
        if !verify_stdout.contains(":(F)") && !verify_stdout.contains(":(OI)(CI)(F)") {
            return Err(VaultError::Io(format!(
                "Permissão de controle total não foi verificada na saída do icacls para '{}': {}",
                path.display(),
                verify_stdout.trim()
            )));
        }

        Ok(())
    }

    #[cfg(not(any(windows, unix)))]
    {
        Ok(())
    }
}

/// Lock interprocessos para sincronização segura de operações no Credential Vault.
/// Baseado em locks de sistema operacional mantidos por descritores abertos:
/// - No Windows: `share_mode(0)` garante exclusão mútua em nível de kernel NTFS.
/// - No Unix: `libc::flock(LOCK_EX | LOCK_NB)` garante exclusão mútua em nível de kernel POSIX.
///
/// INVARIANTE: O lock é liberado automaticamente pelo kernel do SO caso o processo termine
/// inesperadamente (evitando deadlocks permanentes), e NUNCA é removido por critério de idade/tempo.
pub struct VaultProcessLock {
    _file: File,
}

impl VaultProcessLock {
    /// Adquire lock exclusivo para o cofre com timeout padrão de 10 segundos.
    pub fn acquire(vault_path: &Path) -> Result<Self, VaultError> {
        Self::acquire_with_timeout(vault_path, std::time::Duration::from_millis(10_000))
    }

    /// Adquire lock exclusivo para o cofre com timeout configurável.
    pub fn acquire_with_timeout(
        vault_path: &Path,
        timeout: std::time::Duration,
    ) -> Result<Self, VaultError> {
        if let Some(parent) = vault_path.parent() {
            if !parent.as_os_str().is_empty() {
                let _ = fs::create_dir_all(parent);
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let _ = fs::set_permissions(parent, fs::Permissions::from_mode(0o700));
                }
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
                    Ok(file) => {
                        restrict_file_permissions(&lock_path)?;
                        return Ok(Self { _file: file });
                    }
                    Err(e)
                        if e.raw_os_error() == Some(32)
                            || e.kind() == std::io::ErrorKind::PermissionDenied =>
                    {
                        if start.elapsed() > timeout {
                            return Err(VaultError::ProcessLockBusy(format!(
                                "Cofre em uso por outro processo (lock Windows em {}): {}",
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

        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            use std::os::unix::io::AsRawFd;

            let mut opts = fs::OpenOptions::new();
            opts.read(true).write(true).create(true).mode(0o600);

            let file = opts.open(&lock_path).map_err(|e| {
                VaultError::Io(format!(
                    "Falha ao abrir arquivo de lock em {}: {}",
                    lock_path.display(),
                    e
                ))
            })?;
            restrict_file_permissions(&lock_path)?;

            let fd = file.as_raw_fd();
            let start = std::time::Instant::now();
            loop {
                let ret = unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) };
                if ret == 0 {
                    return Ok(Self { _file: file });
                }
                let err = std::io::Error::last_os_error();
                if err.raw_os_error() == Some(libc::EWOULDBLOCK)
                    || err.raw_os_error() == Some(libc::EAGAIN)
                {
                    if start.elapsed() > timeout {
                        return Err(VaultError::ProcessLockBusy(format!(
                            "Cofre em uso por outro processo (flock Unix em {}): {}",
                            lock_path.display(),
                            err
                        )));
                    }
                    std::thread::sleep(std::time::Duration::from_millis(15));
                } else {
                    return Err(VaultError::Io(format!(
                        "Falha na chamada flock em {}: {}",
                        lock_path.display(),
                        err
                    )));
                }
            }
        }

        #[cfg(not(any(windows, unix)))]
        {
            let file = fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(&lock_path)
                .map_err(|e| VaultError::Io(e.to_string()))?;
            Ok(Self { _file: file })
        }
    }
}

#[cfg(unix)]
impl Drop for VaultProcessLock {
    fn drop(&mut self) {
        use std::os::unix::io::AsRawFd;
        unsafe {
            libc::flock(self._file.as_raw_fd(), libc::LOCK_UN);
        }
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
            // Migração de cofre legado (Marco 6) sem canário
            if vault_file.entries.is_empty() {
                // CORREÇÃO P1 (Marco 7.2): Cofre legado vazio sem canário não possui
                // qualquer evidência criptográfica que comprove a senha fornecida.
                // Falha fechado sem adotar senha arbitrária silenciosamente.
                return Err(VaultError::UnverifiableLegacyVault(
                    path.display().to_string(),
                ));
            }

            // Se possui entradas, valida contra a primeira entrada existente
            let (_, first_entry) = vault_file.entries.iter().next().unwrap();
            if decrypt_secret(passphrase.expose_secret(), &first_entry.encrypted).is_err() {
                return Err(VaultError::InvalidPassphrase);
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

    /// Inicializa explicitamente um cofre legado vazio sem canário prévio,
    /// estabelecendo uma nova senha mestra após ação administrativa explícita.
    pub fn initialize_unverifiable_legacy(
        path: impl Into<PathBuf>,
        passphrase: SecretMaterial,
    ) -> Result<Self, VaultError> {
        let path = path.into();
        let _lock = VaultProcessLock::acquire(&path)?;

        if !path.exists() {
            return Err(VaultError::NotFound(path.display().to_string()));
        }

        let content = fs::read_to_string(&path)
            .map_err(|e| VaultError::Io(format!("Falha ao ler vault: {}", e)))?;
        let mut vault_file: VaultFile = serde_json::from_str(&content)
            .map_err(|e| VaultError::Serialization(format!("JSON inválido: {}", e)))?;

        if !vault_file.entries.is_empty() {
            return Err(VaultError::Io(
                "O cofre possui credenciais existentes e não é um cofre legado vazio".to_string(),
            ));
        }

        let canary_blob = encrypt_secret(passphrase.expose_secret(), VAULT_CANARY_PLAINTEXT)?;
        vault_file.verification = Some(canary_blob.clone());
        Self::persist_file_atomic(&path, &vault_file)?;

        Ok(Self {
            path,
            passphrase,
            verification: RwLock::new(Some(canary_blob)),
            entries: RwLock::new(BTreeMap::new()),
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

        if self.path.exists() {
            if let Ok(content) = fs::read_to_string(&self.path) {
                if let Ok(disk_file) = serde_json::from_str::<VaultFile>(&content) {
                    *entries_guard = disk_file.entries;
                }
            }
        }

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

        if self.path.exists() {
            if let Ok(content) = fs::read_to_string(&self.path) {
                if let Ok(disk_file) = serde_json::from_str::<VaultFile>(&content) {
                    *entries_guard = disk_file.entries;
                }
            }
        }

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

        let entry = match entries.get(alias) {
            Some(e) => e.clone(),
            None => {
                drop(entries);
                if self.path.exists() {
                    let _proc_lock = VaultProcessLock::acquire(&self.path)?;
                    let mut write_guard = self
                        .entries
                        .write()
                        .map_err(|_| VaultError::Io("Lock de escrita envenenado".to_string()))?;
                    if let Ok(content) = fs::read_to_string(&self.path) {
                        if let Ok(disk_file) = serde_json::from_str::<VaultFile>(&content) {
                            *write_guard = disk_file.entries;
                        }
                    }
                    write_guard
                        .get(alias)
                        .cloned()
                        .ok_or_else(|| VaultError::NotFound(alias.to_string()))?
                } else {
                    return Err(VaultError::NotFound(alias.to_string()));
                }
            }
        };

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

        if self.path.exists() {
            if let Ok(content) = fs::read_to_string(&self.path) {
                if let Ok(disk_file) = serde_json::from_str::<VaultFile>(&content) {
                    *entries_guard = disk_file.entries;
                }
            }
        }

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

        if self.path.exists() {
            if let Ok(content) = fs::read_to_string(&self.path) {
                if let Ok(disk_file) = serde_json::from_str::<VaultFile>(&content) {
                    *entries_guard = disk_file.entries;
                }
            }
        }

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

            #[cfg(unix)]
            let mut file = {
                use std::os::unix::fs::OpenOptionsExt;
                let mut opts = fs::OpenOptions::new();
                opts.write(true).create_new(true).mode(0o600);
                opts.open(&tmp_path).map_err(|e| {
                    VaultError::Io(format!("Falha ao criar arquivo temporário do vault: {}", e))
                })?
            };

            #[cfg(not(any(windows, unix)))]
            let mut file = File::create(&tmp_path).map_err(|e| {
                VaultError::Io(format!("Falha ao criar arquivo temporário do vault: {}", e))
            })?;

            restrict_file_permissions(&tmp_path).inspect_err(|_| {
                let _ = fs::remove_file(&tmp_path);
            })?;

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

        restrict_file_permissions(target_path)?;

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
