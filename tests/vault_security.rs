use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use yuki::contracts::events::{AuditEvent, EventType};
use yuki::contracts::identifiers::{generate_unique_id, CausationId, CorrelationId};
use yuki::security::credentials::{
    CompositeCredentialBroker, CredentialBroker, CredentialError, SecretMaterial, SecretRef,
};
use yuki::security::vault::{CredentialStatus, CredentialVault, VaultError, VaultSecretStore};

fn test_vault_path(name: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!(
        "yuki_vault_test_{}_{}.json",
        name,
        generate_unique_id("v")
    ));
    p
}

#[test]
fn test_vault_crud_operations() {
    let path = test_vault_path("crud");
    let master = SecretMaterial::new("my-master-passphrase-12345");
    let vault = CredentialVault::open_or_create(&path, master).expect("criar vault");

    // 1. Store
    let meta = vault
        .store(
            "brave_api_key",
            "brave-secret-key-123",
            "brave",
            Some("Chave do provedor Brave".to_string()),
        )
        .expect("armazenar credencial");

    assert_eq!(meta.alias, "brave_api_key");
    assert_eq!(meta.provider, "brave");
    assert_eq!(meta.status, CredentialStatus::Active);

    // 2. Get
    let retrieved = vault.get("brave_api_key").expect("recuperar credencial");
    assert_eq!(retrieved.expose_secret(), "brave-secret-key-123");

    // 3. List metadata (sem segredos)
    let list = vault.list_metadata().expect("listar metadados");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].alias, "brave_api_key");
    assert_eq!(list[0].provider, "brave");

    // 4. Delete
    vault.delete("brave_api_key").expect("deletar credencial");
    let err = vault
        .get("brave_api_key")
        .expect_err("deve falhar após delete");
    match err {
        VaultError::NotFound(alias) => assert_eq!(alias, "brave_api_key"),
        other => panic!("Esperado NotFound, recebido: {:?}", other),
    }

    let _ = std::fs::remove_file(path);
}

#[test]
fn test_vault_persistence_across_restart() {
    let path = test_vault_path("restart");
    let master_pass = "long-secure-vault-passphrase-2026";

    // Criar e armazenar
    {
        let master = SecretMaterial::new(master_pass);
        let vault = CredentialVault::open_or_create(&path, master).expect("criar vault");
        vault
            .store(
                "langsearch_key",
                "langsearch-super-secret-token",
                "langsearch",
                None,
            )
            .expect("armazenar");
    }

    // Reabrir na mesma sessão
    {
        let master = SecretMaterial::new(master_pass);
        let vault = CredentialVault::open_or_create(&path, master).expect("reabrir vault");
        let retrieved = vault.get("langsearch_key").expect("recuperar segredo");
        assert_eq!(retrieved.expose_secret(), "langsearch-super-secret-token");
    }

    let _ = std::fs::remove_file(path);
}

#[test]
fn test_vault_wrong_passphrase_fails_closed() {
    let path = test_vault_path("wrong_pass");
    let correct_pass = "correct-passphrase-alpha";
    let wrong_pass = "wrong-passphrase-bravo";

    // Criar com senha correta
    {
        let master = SecretMaterial::new(correct_pass);
        let vault = CredentialVault::open_or_create(&path, master).expect("criar vault");
        vault
            .store("gemini_key", "AIzaSySecretGeminiToken", "gemini", None)
            .expect("armazenar");
    }

    // Reabrir com senha incorreta e tentar recuperar
    {
        let master_wrong = SecretMaterial::new(wrong_pass);
        let vault_wrong =
            CredentialVault::open_or_create(&path, master_wrong).expect("abrir arquivo");
        let err = vault_wrong
            .get("gemini_key")
            .expect_err("deve falhar decriptação");
        match err {
            VaultError::Crypto(_) => {} // DecryptionFailed
            other => panic!(
                "Esperado erro criptográfico fail-closed, obtido: {:?}",
                other
            ),
        }
    }

    let _ = std::fs::remove_file(path);
}

#[test]
fn test_vault_tampered_ciphertext_fails_closed() {
    let path = test_vault_path("tamper");
    let master_pass = "passphrase-for-tamper-test";

    // Criar e armazenar
    {
        let master = SecretMaterial::new(master_pass);
        let vault = CredentialVault::open_or_create(&path, master).expect("criar vault");
        vault
            .store("test_key", "plain-sensitive-value", "custom", None)
            .expect("armazenar");
    }

    // Adulterar arquivo JSON
    {
        let content = std::fs::read_to_string(&path).expect("ler arquivo do vault");
        let mut json: serde_json::Value = serde_json::from_str(&content).expect("parse json");

        // Modificar o último caractere do ciphertext
        if let Some(entry) = json
            .get_mut("entries")
            .and_then(|e| e.get_mut("test_key"))
            .and_then(|k| k.get_mut("encrypted"))
            .and_then(|enc| enc.get_mut("ciphertext"))
        {
            let ct_str = entry.as_str().unwrap();
            let mut modified = ct_str.to_string();
            if modified.ends_with('0') {
                modified.replace_range(modified.len() - 1.., "1");
            } else {
                modified.replace_range(modified.len() - 1.., "0");
            }
            *entry = serde_json::Value::String(modified);
        } else {
            panic!("Estrutura JSON do vault não encontrada para adulteração");
        }

        std::fs::write(&path, serde_json::to_string_pretty(&json).unwrap())
            .expect("escrever json adulterado");
    }

    // Tentar ler -> deve falhar fechado com erro criptográfico
    {
        let master = SecretMaterial::new(master_pass);
        let vault = CredentialVault::open_or_create(&path, master).expect("abrir vault");
        let err = vault
            .get("test_key")
            .expect_err("deve falhar com ciphertext adulterado");
        match err {
            VaultError::Crypto(_) => {} // DecryptionFailed / AEAD tag mismatch
            other => panic!("Esperado erro de autenticação AEAD, obtido: {:?}", other),
        }
    }

    let _ = std::fs::remove_file(path);
}

#[test]
fn test_vault_revocation_lifecycle() {
    let path = test_vault_path("revoke");
    let master = SecretMaterial::new("passphrase-revocation");
    let vault = CredentialVault::open_or_create(&path, master).expect("criar vault");

    vault
        .store("active_key", "secret-payload-123", "provider_x", None)
        .expect("armazenar");

    // Verificar ativo
    assert!(vault.is_active("active_key").expect("is_active"));
    assert_eq!(
        vault.get("active_key").expect("get").expose_secret(),
        "secret-payload-123"
    );

    // Revogar
    let revoked_meta = vault.revoke("active_key").expect("revogar");
    assert_eq!(revoked_meta.status, CredentialStatus::Revoked);
    assert!(!vault.is_active("active_key").expect("is_active"));

    // Tentar recuperar após revogação -> negado
    let err = vault
        .get("active_key")
        .expect_err("get após revogação deve falhar");
    match err {
        VaultError::Revoked(alias) => assert_eq!(alias, "active_key"),
        other => panic!("Esperado Revoked, obtido: {:?}", other),
    }

    // Metadados ainda existem para auditoria
    let metas = vault.list_metadata().expect("list_metadata");
    assert_eq!(metas.len(), 1);
    assert_eq!(metas[0].status, CredentialStatus::Revoked);

    let _ = std::fs::remove_file(path);
}

#[test]
fn test_vault_concurrency() {
    let path = test_vault_path("concurrency");
    let master = SecretMaterial::new("concurrent-vault-master-pass");
    let vault = Arc::new(CredentialVault::open_or_create(&path, master).expect("criar vault"));

    // Armazenar credencial inicial
    vault
        .store("shared_key", "initial-secret", "shared", None)
        .expect("armazenar inicial");

    let mut handles = Vec::new();

    // 4 threads leitoras
    for _ in 0..4 {
        let v = Arc::clone(&vault);
        handles.push(thread::spawn(move || {
            for _ in 0..2 {
                let s = v.get("shared_key").expect("leitura concorrente");
                assert!(
                    s.expose_secret().starts_with("initial-secret")
                        || s.expose_secret().starts_with("updated-")
                );
            }
        }));
    }

    // 2 threads escritoras
    for i in 0..2 {
        let v = Arc::clone(&vault);
        handles.push(thread::spawn(move || {
            let alias = format!("worker_key_{}", i);
            v.store(&alias, &format!("secret-from-worker-{}", i), "worker", None)
                .expect("escrita concorrente");
        }));
    }

    for h in handles {
        h.join().expect("thread join sem panic");
    }

    let metadata_list = vault.list_metadata().expect("listar final");
    assert_eq!(metadata_list.len(), 3);

    let _ = std::fs::remove_file(path);
}

#[test]
fn test_vault_zero_secrets_leakage_in_debug_and_audit() {
    let sensitive = "SUPER_SECRET_TOKEN_DO_NOT_LEAK";
    let mat = SecretMaterial::new(sensitive);

    // 1. SecretMaterial Debug e Display não revelam segredo
    let debug_str = format!("{:?}", mat);
    let display_str = format!("{}", mat);
    assert!(!debug_str.contains(sensitive));
    assert!(debug_str.contains("[REDACTED]"));
    assert_eq!(display_str, "[REDACTED]");

    // 2. SecretLease Debug não revela segredo
    let lease = yuki::security::credentials::SecretLease::new(mat);
    let lease_debug = format!("{:?}", lease);
    assert!(!lease_debug.contains(sensitive));
    assert!(lease_debug.contains("[REDACTED]"));

    // 3. Auditoria rejeita segredos no payload
    let event = AuditEvent::new(
        EventType::VaultCredentialStored,
        CorrelationId::new(),
        CausationId::new("test_causation"),
        serde_json::json!({
            "alias": "test_alias",
            "provider": "langsearch",
            "api_key": sensitive // Campo proibido!
        }),
        "test",
    );
    assert!(event.assert_no_secrets().is_err());

    // 4. Auditoria com metadados sanitizados é aprovada
    let safe_event = AuditEvent::new(
        EventType::VaultCredentialStored,
        CorrelationId::new(),
        CausationId::new("test_causation"),
        serde_json::json!({
            "alias": "test_alias",
            "provider": "langsearch",
            "status": "active"
        }),
        "test",
    );
    assert!(safe_event.assert_no_secrets().is_ok());
}

#[test]
fn test_composite_credential_broker_resolution() {
    let path = test_vault_path("composite");
    let master = SecretMaterial::new("composite-passphrase");
    let vault = Arc::new(CredentialVault::open_or_create(&path, master).expect("criar vault"));

    // Armazenar no vault
    vault
        .store(
            "langsearch_prod",
            "vault-langsearch-key-999",
            "langsearch",
            None,
        )
        .expect("armazenar no vault");

    let vault_store = Arc::new(VaultSecretStore::new(Arc::clone(&vault)));
    let broker = CompositeCredentialBroker::new().with_vault(vault_store);

    // 1. Resolução via secret://vault/
    let vault_ref = SecretRef::new("secret://vault/langsearch_prod", "langsearch_prod").unwrap();
    let lease = broker.acquire(&vault_ref).expect("adquirir via vault");
    assert_eq!(lease.expose_secret(), "vault-langsearch-key-999");

    // 2. Resolução via secret://env/
    std::env::set_var("TEST_YUKI_ENV_KEY", "env-secret-value-888");
    let env_ref = SecretRef::new("secret://env/TEST_YUKI_ENV_KEY", "test_env_key").unwrap();
    let env_lease = broker.acquire(&env_ref).expect("adquirir via env");
    assert_eq!(env_lease.expose_secret(), "env-secret-value-888");
    std::env::remove_var("TEST_YUKI_ENV_KEY");

    // 3. Credencial revogada no vault -> negada pelo broker
    vault.revoke("langsearch_prod").expect("revogar");
    let revoked_err = broker.acquire(&vault_ref).expect_err("deve ser negado");
    match revoked_err {
        CredentialError::AccessDenied(_) => {}
        other => panic!("Esperado AccessDenied, obtido: {:?}", other),
    }

    // 4. Esquema inválido -> rejeitado
    let invalid_ref = SecretRef::new("secret://unknown/foo", "foo").unwrap();
    let inv_err = broker
        .acquire(&invalid_ref)
        .expect_err("esquema desconhecido");
    match inv_err {
        CredentialError::InvalidReference(_) => {}
        other => panic!("Esperado InvalidReference, obtido: {:?}", other),
    }

    let _ = std::fs::remove_file(path);
}
