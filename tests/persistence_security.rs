use std::path::PathBuf;
use std::sync::Arc;

use yuki::capabilities::registry::CapabilityRegistry;
use yuki::contracts::events::{AuditEvent, EventType};
use yuki::contracts::identifiers::{CausationId, CorrelationId, OperationId};
use yuki::contracts::input::UserInput;
use yuki::contracts::output::ResultStatus;
use yuki::core::yuki_core::YukiCore;
use yuki::execution::executor::Executor;
use yuki::models::mock::MockModelProvider;
use yuki::persistence::sqlite::SqliteAuditStore;
use yuki::security::authorization::SecurityController;
use yuki::verification::Verifier;

use std::sync::atomic::{AtomicU64, Ordering};
static SEC_TEST_DB_COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_db_path() -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let count = SEC_TEST_DB_COUNTER.fetch_add(1, Ordering::SeqCst);
    let pid = std::process::id();
    std::env::temp_dir().join(format!("yuki_sec_test_{}_{}_{:x}.db", pid, count, nanos))
}

struct TempDbCleanup(PathBuf);
impl Drop for TempDbCleanup {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
        let _ = std::fs::remove_file(format!("{}-wal", self.0.display()));
        let _ = std::fs::remove_file(format!("{}-shm", self.0.display()));
    }
}

// 1. Invariant: NEVER PERSIST CapabilityToken in SQLite
#[tokio::test]
async fn test_sec_persist_token_absent_from_database_bytes() {
    let path = temp_db_path();
    let _guard = TempDbCleanup(path.clone());

    let store = Arc::new(SqliteAuditStore::open(&path).expect("open"));
    let core = YukiCore::with_components(
        Arc::new(MockModelProvider::new()),
        Arc::new(CapabilityRegistry::new()),
        Arc::new(SecurityController::new()),
        Arc::new(Executor::new()),
        Arc::new(Verifier::new()),
        Arc::new(yuki::audit::event_store::InMemoryEventStore::new()),
    )
    .with_persistent_audit(store.clone());

    let input = UserInput::new("echo: token_leak_test");

    let result = core.process_input_async(input).await.expect("process");
    assert_eq!(result.status, ResultStatus::Success);

    // Flush WAL
    store.wal_checkpoint_passive().expect("checkpoint");
    drop(core);
    drop(store);

    // Read raw SQLite file bytes
    let raw_bytes = std::fs::read(&path).expect("read raw db bytes");
    let raw_str = String::from_utf8_lossy(&raw_bytes);

    // Inspect database tables directly
    let conn = rusqlite::Connection::open(&path).expect("open db inspect");
    let mut stmt = conn
        .prepare("SELECT payload_json FROM audit_events")
        .expect("prepare");
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .expect("query");

    for r in rows {
        let json_str = r.expect("read row");
        assert!(
            !json_str.contains("token_id"),
            "O token_id não deve estar presente no payload de auditoria: {}",
            json_str
        );
        assert!(
            !json_str.contains("capability_token"),
            "O token de autorização não deve estar no payload: {}",
            json_str
        );
    }

    // Invariant: The raw word 'token_id' or raw token secret must not appear in the DB
    assert!(!raw_str.contains("authorization_token"));
}

// 2. Invariant: SecretMaterial cannot be persisted or serialized
#[test]
fn test_sec_persist_secret_material_absent_from_persisted_records() {
    let store = SqliteAuditStore::open_in_memory().expect("open");

    // Attempt to persist an event with forbidden secret pattern
    let event = AuditEvent::new(
        EventType::InputReceived,
        CorrelationId::new(),
        CausationId::new("test"),
        serde_json::json!({ "api_key": "AIzaSySecretValue123" }),
        "adversary",
    );

    let res = store.record_event(&event);
    assert!(
        res.is_err(),
        "Evento contendo padrão de segredo deve ser rejeitado pelo store"
    );
}

// 3. Invariant: Append-only SQLite triggers prevent UPDATE and DELETE in runtime connection
#[test]
fn test_sec_persist_audit_records_immutable_triggers() {
    let path = temp_db_path();
    let _guard = TempDbCleanup(path.clone());

    let store = SqliteAuditStore::open(&path).expect("open");
    let event = AuditEvent::new(
        EventType::InputReceived,
        CorrelationId::new(),
        CausationId::new("causation"),
        serde_json::json!({ "msg": "immutable" }),
        "core",
    );
    store.record_event(&event).expect("record");
    drop(store);

    // Direct connection attempt to tamper with audit_events table
    let conn = rusqlite::Connection::open(&path).expect("open raw");

    // Attempt UPDATE
    let update_res = conn.execute(
        "UPDATE audit_events SET provenance = 'tampered' WHERE 1=1",
        [],
    );
    assert!(
        update_res.is_err(),
        "Trigger deve rejeitar UPDATE em audit_events"
    );

    // Attempt DELETE
    let delete_res = conn.execute("DELETE FROM audit_events WHERE 1=1", []);
    assert!(
        delete_res.is_err(),
        "Trigger deve rejeitar DELETE em audit_events"
    );
}

// 4. Invariant: Verification records triggers prevent UPDATE and DELETE
#[test]
fn test_sec_persist_verification_records_immutable_triggers() {
    let path = temp_db_path();
    let _guard = TempDbCleanup(path.clone());

    let store = SqliteAuditStore::open(&path).expect("open");
    let record = yuki::persistence::schema::VerificationRecord {
        verification_record_id: "rec_1".to_string(),
        operation_id: OperationId::new(),
        attempt_id: None,
        capability_id: yuki::contracts::identifiers::CapabilityId::new("system.echo"),
        strategy_id: "echo_exact_match".to_string(),
        verification_state: yuki::contracts::verification::VerificationState::VerifiedSuccess,
        observed_effect_state:
            yuki::contracts::verification::ObservedEffectState::ObservedNoMutation,
        evaluated_at: yuki::contracts::identifiers::now_utc(),
        verification_basis: "ok".to_string(),
        evidence_refs: vec![],
    };
    store.record_verification(&record).expect("record");
    drop(store);

    let conn = rusqlite::Connection::open(&path).expect("open raw");

    let update_res = conn.execute(
        "UPDATE verification_records SET verification_basis = 'tampered' WHERE 1=1",
        [],
    );
    assert!(
        update_res.is_err(),
        "Trigger deve rejeitar UPDATE em verification_records"
    );

    let delete_res = conn.execute("DELETE FROM verification_records WHERE 1=1", []);
    assert!(
        delete_res.is_err(),
        "Trigger deve rejeitar DELETE em verification_records"
    );
}

// 5. Invariant: Pre-execution persistence failure aborts capability dispatch
#[tokio::test]
async fn test_sec_persist_pre_execution_failure_blocks_dispatch() {
    // A mock failing event store that errors on AuthorizationGranted
    struct FailingEventStore;
    impl yuki::audit::event_store::EventStore for FailingEventStore {
        fn record(&self, event: AuditEvent) -> Result<(), yuki::contracts::errors::YukiError> {
            if event.event_type == EventType::AuthorizationGranted {
                Err(yuki::contracts::errors::YukiError::ExecutionFailed(
                    "Simulated disk failure on pre-execution audit".to_string(),
                ))
            } else {
                Ok(())
            }
        }
        fn get_events(&self, _id: &CorrelationId) -> Vec<AuditEvent> {
            vec![]
        }
        fn all_events(&self) -> Vec<AuditEvent> {
            vec![]
        }
    }

    let core = YukiCore::with_components(
        Arc::new(MockModelProvider::new()),
        Arc::new(CapabilityRegistry::new()),
        Arc::new(SecurityController::new()),
        Arc::new(Executor::new()),
        Arc::new(Verifier::new()),
        Arc::new(FailingEventStore),
    );

    let input = UserInput::new("echo: should_not_dispatch");

    let result = core.process_input_async(input).await;
    // INVARIANT: When required pre-execution audit write fails, dispatch is blocked!
    assert!(
        result.is_err(),
        "Falha de persistência pré-execução DEVE bloquear o despacho da capacidade"
    );
}

// 6. Invariant: Persistent store failure in pre-execution blocks capability execution
#[tokio::test]
async fn test_sec_persist_pre_execution_persistent_audit_failure_blocks_dispatch() {
    let path = temp_db_path();
    let _guard = TempDbCleanup(path.clone());

    // Configure store with tiny payload limit (10 bytes) that will fail when writing audit events
    let store = Arc::new(
        SqliteAuditStore::open(&path)
            .expect("open")
            .with_max_payload_bytes(10),
    );
    let core = YukiCore::new().with_persistent_audit(store);

    let input = UserInput::new("echo: payload_too_large_pre_exec");
    let result = core.process_input_async(input).await;

    // INVARIANT: Pre-execution persistent audit failure blocks dispatch!
    assert!(
        result.is_err(),
        "Falha no SqliteAuditStore durável na pré-execução DEVE abortar o despacho"
    );
}
