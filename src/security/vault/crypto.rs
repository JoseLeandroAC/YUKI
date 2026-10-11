use crate::security::credentials::SecretMaterial;
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Número padrão de iterações PBKDF2 para derivação da chave mestra.
pub const PBKDF2_ITERATIONS: u32 = 100_000;

/// Limite mínimo de iterações aceito para prevenir ataques de KDF enfraquecido.
pub const MIN_PBKDF2_ITERATIONS: u32 = 10_000;

/// Limite máximo de iterações aceito para prevenir DoS por exaustão de CPU.
pub const MAX_PBKDF2_ITERATIONS: u32 = 500_000;

/// Limite máximo do tamanho do texto claro em bytes (1 MiB).
pub const MAX_PLAINTEXT_BYTES: usize = 1024 * 1024;

/// Limite máximo do ciphertext em bytes (10 MiB).
pub const MAX_CIPHERTEXT_BYTES: usize = 10 * 1024 * 1024;

/// Tamanho do salt em bytes (128 bits).
pub const SALT_LEN: usize = 16;

/// Tamanho do nonce em bytes para AES-256-GCM (96 bits).
pub const NONCE_LEN: usize = 12;

/// Tamanho da chave AES-256 em bytes (256 bits).
pub const KEY_LEN: usize = 32;

/// Erros originados nas operações criptográficas do Vault.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum VaultCryptoError {
    #[error("Erro de criptografia: {0}")]
    EncryptionFailed(String),

    #[error("Falha de autenticação/decriptação (chave incorreta ou payload corrompido)")]
    DecryptionFailed,

    #[error("Senha mestra vazia")]
    EmptyPassphrase,

    #[error("Dados criptográficos corrompidos ou em formato inválido: {0}")]
    CorruptedData(String),

    #[error("Erro ao gerar entropia do sistema: {0}")]
    EntropyError(String),
}

/// Chave de 256 bits com zeroização garantida em memória ao ser descartada.
#[derive(Zeroize, ZeroizeOnDrop)]
struct ZeroizedKey([u8; KEY_LEN]);

/// Estrutura serializável contendo o payload criptografado com parâmetros e tag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncryptedBlob {
    /// Salt em representação hexadecimal.
    pub salt: String,
    /// Nonce em representação hexadecimal.
    pub nonce: String,
    /// Texto cifrado + tag de autenticação GCM em representação hexadecimal.
    pub ciphertext: String,
    /// Número de iterações do algoritmo de derivação.
    pub iterations: u32,
}

/// Deriva uma chave de 256 bits a partir de uma senha mestra e um salt via PBKDF2-HMAC-SHA256.
fn derive_key(
    passphrase: &str,
    salt: &[u8; SALT_LEN],
    iterations: u32,
) -> Result<ZeroizedKey, VaultCryptoError> {
    if passphrase.is_empty() {
        return Err(VaultCryptoError::EmptyPassphrase);
    }
    let mut key = ZeroizedKey([0u8; KEY_LEN]);
    pbkdf2::pbkdf2_hmac::<sha2::Sha256>(passphrase.as_bytes(), salt, iterations, &mut key.0);
    Ok(key)
}

/// Converte fatia de bytes em string hexadecimal.
pub fn to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(s, "{:02x}", b);
    }
    s
}

/// Converte string hexadecimal em vetor de bytes.
pub fn from_hex(s: &str) -> Result<Vec<u8>, VaultCryptoError> {
    if !s.len().is_multiple_of(2) {
        return Err(VaultCryptoError::CorruptedData(
            "String hexadecimal com comprimento ímpar".to_string(),
        ));
    }
    (0..s.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| {
                VaultCryptoError::CorruptedData(format!("Hexadecimal inválido: {}", e))
            })
        })
        .collect()
}

/// Criptografa o segredo em texto claro utilizando AES-256-GCM com chave derivada via PBKDF2.
///
/// INVARIANTES DE SEGURANÇA:
/// 1. Gera salt aleatório único de 16 bytes via CSPRNG do sistema (`getrandom`).
/// 2. Gera nonce aleatório único de 12 bytes via CSPRNG (`getrandom`).
/// 3. Utiliza 100.000 iterações PBKDF2-HMAC-SHA256.
/// 4. Zera a chave derivada em memória após o uso.
pub fn encrypt_secret(
    passphrase: &str,
    plaintext: &str,
) -> Result<EncryptedBlob, VaultCryptoError> {
    if passphrase.is_empty() {
        return Err(VaultCryptoError::EmptyPassphrase);
    }

    if plaintext.len() > MAX_PLAINTEXT_BYTES {
        return Err(VaultCryptoError::CorruptedData(format!(
            "Texto claro excede o tamanho máximo permitido de {} bytes",
            MAX_PLAINTEXT_BYTES
        )));
    }

    let mut salt = [0u8; SALT_LEN];
    getrandom::getrandom(&mut salt).map_err(|e| VaultCryptoError::EntropyError(e.to_string()))?;

    let mut nonce_bytes = [0u8; NONCE_LEN];
    getrandom::getrandom(&mut nonce_bytes)
        .map_err(|e| VaultCryptoError::EntropyError(e.to_string()))?;

    let derived_key = derive_key(passphrase, &salt, PBKDF2_ITERATIONS)?;

    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&derived_key.0));
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext_bytes = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| VaultCryptoError::EncryptionFailed(format!("{:?}", e)))?;

    Ok(EncryptedBlob {
        salt: to_hex(&salt),
        nonce: to_hex(&nonce_bytes),
        ciphertext: to_hex(&ciphertext_bytes),
        iterations: PBKDF2_ITERATIONS,
    })
}

/// Decripta um blob criptografado utilizando AES-256-GCM e a senha mestra fornecida.
///
/// INVARIANTES DE SEGURANÇA:
/// 1. Revalida a integridade do salt e do nonce.
/// 2. Valida os limites de iterações do KDF (entre 10.000 e 500.000) antes de processar.
/// 3. Valida os limites de tamanho do ciphertext antes de alocar memória.
/// 4. Verifica a autenticação AEAD (tag GCM); falha imediatamente se adulterado ou se a senha estiver incorreta.
/// 5. Retorna o segredo encapsulado em `SecretMaterial` (com zeroize on drop).
/// 6. Falha fechada (fail-closed) sem expor material sensível em mensagens de erro.
pub fn decrypt_secret(
    passphrase: &str,
    blob: &EncryptedBlob,
) -> Result<SecretMaterial, VaultCryptoError> {
    if passphrase.is_empty() {
        return Err(VaultCryptoError::EmptyPassphrase);
    }

    // 1. Validar iterações antes de processar KDF (defesa contra DoS de CPU e enfraquecimento)
    if blob.iterations < MIN_PBKDF2_ITERATIONS || blob.iterations > MAX_PBKDF2_ITERATIONS {
        return Err(VaultCryptoError::CorruptedData(format!(
            "Número de iterações ({}) fora dos limites seguros [{}..{}]",
            blob.iterations, MIN_PBKDF2_ITERATIONS, MAX_PBKDF2_ITERATIONS
        )));
    }

    // 2. Validar comprimentos de strings hexadecimais antes de decodificar
    if blob.salt.len() != SALT_LEN * 2 {
        return Err(VaultCryptoError::CorruptedData(format!(
            "Comprimento do salt hexadecimal inválido: {} (esperado {})",
            blob.salt.len(),
            SALT_LEN * 2
        )));
    }
    if blob.nonce.len() != NONCE_LEN * 2 {
        return Err(VaultCryptoError::CorruptedData(format!(
            "Comprimento do nonce hexadecimal inválido: {} (esperado {})",
            blob.nonce.len(),
            NONCE_LEN * 2
        )));
    }
    if blob.ciphertext.len() < 32 || blob.ciphertext.len() > MAX_CIPHERTEXT_BYTES * 2 {
        return Err(VaultCryptoError::CorruptedData(format!(
            "Comprimento do ciphertext hexadecimal inválido: {} caracteres",
            blob.ciphertext.len()
        )));
    }

    let salt_vec = from_hex(&blob.salt)?;
    let mut salt = [0u8; SALT_LEN];
    salt.copy_from_slice(&salt_vec);

    let nonce_vec = from_hex(&blob.nonce)?;
    let mut nonce_bytes = [0u8; NONCE_LEN];
    nonce_bytes.copy_from_slice(&nonce_vec);

    let ciphertext_vec = from_hex(&blob.ciphertext)?;

    let derived_key = derive_key(passphrase, &salt, blob.iterations)?;

    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&derived_key.0));
    let nonce = Nonce::from_slice(&nonce_bytes);

    let mut decrypted_bytes = cipher
        .decrypt(nonce, ciphertext_vec.as_slice())
        .map_err(|_| VaultCryptoError::DecryptionFailed)?;

    let plaintext_str = String::from_utf8(decrypted_bytes.clone()).map_err(|_| {
        decrypted_bytes.zeroize();
        VaultCryptoError::CorruptedData("Conteúdo decriptado não é UTF-8 válido".to_string())
    })?;

    decrypted_bytes.zeroize();
    Ok(SecretMaterial::new(plaintext_str))
}
