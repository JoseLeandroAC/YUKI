use std::path::PathBuf;
use std::sync::Arc;
use yuki::contracts::events::{AuditEvent, EventType};
use yuki::contracts::identifiers::{
    now_utc, AttemptId, CapabilityId, CausationId, CorrelationId, EvidenceId, OperationId,
};
use yuki::contracts::verification::{ObservedEffectState, VerificationState};
use yuki::persistence::errors::PersistenceError;
use yuki::persistence::schema::VerificationRecord;
use yuki::persistence::sqlite::{AuditFilter, AuditQueryStore, SqliteAuditStore};

fn temp_db_path() -> PathBuf {
    let unique = uuid_v4_simple();
    std::env::temp_dir().join(format!("yuki_test_{}.db", unique))
}

fn uuid_v4_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{:x}", nanos)
}

struct TempDbCleanup(PathBuf);
impl Drop for TempDbCleanup {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
        let _ = std::fs::remove_file(format!("{}-wal", self.0.display()));
        let _ = std::fs::remove_file(format!("{}-shm", self.0.display()));
    }
}

#[test]
fn test_persistence_audit_survives_close_reopen() {
    let path = temp_db_path();
    let _guard = TempDbCleanup(path.clone());

    let corr_id = CorrelationId::new();
    let caus_id = CausationId::new("test_causation");

    // Phase 1: Write event and close
    {
        let store = SqliteAuditStore::open(&path).expect("open failed");
        let event = AuditEvent::new(
            EventType::InputReceived,
            corr_id.clone(),
            caus_id.clone(),
            serde_json::json!({ "input": "salve o mundo" }),
            "unit_test",
        );
        store.record_event(&event).expect("record failed");
        store.wal_checkpoint_passive().expect("checkpoint");
    }

    // Phase 2: Reopen store from same path
    {
        let store = SqliteAuditStore::open(&path).expect("reopen failed");
        let filter = AuditFilter {
            correlation_id: Some(corr_id.clone()),
            ..Default::default()
        };
        let events = store.query_events(&filter).expect("query failed");

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, EventType::InputReceived);
        assert_eq!(events[0].correlation_id, corr_id);
        assert_eq!(events[0].payload["input"], "salve o mundo");
        assert_eq!(events[0].provenance, "unit_test");
    }
}

#[test]
fn test_persistence_schema_initialized_correctly() {
    let store = SqliteAuditStore::open_in_memory().expect("open in memory");
    let filter = AuditFilter::default();
    let events = store.query_events(&filter).expect("query on empty db");
    assert!(events.is_empty());
}

#[test]
fn test_persistence_migration_version_recorded() {
    let path = temp_db_path();
    let _guard = TempDbCleanup(path.clone());

    let store = SqliteAuditStore::open(&path).expect("open");
    drop(store);

    let conn = rusqlite::Connection::open(&path).expect("raw open");
    let version: i64 = conn
        .query_row("SELECT MAX(version) FROM _yuki_schema_version", [], |r| {
            r.get(0)
        })
        .expect("query version");
    assert_eq!(version, 1);
}

#[test]
fn test_persistence_newer_schema_rejected() {
    let path = temp_db_path();
    let _guard = TempDbCleanup(path.clone());

    // Initialize version 1
    let store = SqliteAuditStore::open(&path).expect("open");
    drop(store);

    // Simulate future migration version 999
    {
        let conn = rusqlite::Connection::open(&path).expect("raw open");
        conn.execute(
            "INSERT INTO _yuki_schema_version (version, name, applied_at) VALUES (999, 'future', 'now')",
            [],
        )
        .expect("insert future version");
    }

    // Reopen must fail closed
    let result = SqliteAuditStore::open(&path);
    assert!(
        matches!(
            result,
            Err(PersistenceError::SchemaVersionUnsupported {
                found: 999,
                supported: 1
            })
        ),
        "Expected SchemaVersionUnsupported, got: {:?}",
        result.err()
    );
}

#[test]
fn test_persistence_bounded_query_limit_enforced() {
    let store = SqliteAuditStore::open_in_memory().expect("open");
    let corr_id = CorrelationId::new();

    for i in 0..25 {
        let event = AuditEvent::new(
            EventType::ContextBuilt,
            corr_id.clone(),
            CausationId::new(format!("step_{}", i)),
            serde_json::json!({ "step": i }),
            "test",
        );
        store.record_event(&event).expect("record");
    }

    let filter = AuditFilter {
        limit: 10,
        ..Default::default()
    };
    let results = store.query_events(&filter).expect("query");
    assert_eq!(results.len(), 10);
}

#[test]
fn test_persistence_cursor_pagination_deterministic() {
    let store = SqliteAuditStore::open_in_memory().expect("open");
    let corr_id = CorrelationId::new();

    for i in 0..15 {
        let event = AuditEvent::new(
            EventType::ModelInvoked,
            corr_id.clone(),
            CausationId::new(format!("msg_{}", i)),
            serde_json::json!({ "i": i }),
            "test",
        );
        store.record_event(&event).expect("record");
    }

    // Page 1: 5 items
    let page1 = store
        .query_events(&AuditFilter {
            limit: 5,
            cursor_sequence: None,
            ..Default::default()
        })
        .expect("page 1");
    assert_eq!(page1.len(), 5);

    // Page 2: next 5 items
    let page2 = store
        .query_events(&AuditFilter {
            limit: 5,
            cursor_sequence: Some(5),
            ..Default::default()
        })
        .expect("page 2");
    assert_eq!(page2.len(), 5);

    // Assert disjoint sets
    let ids1: Vec<_> = page1.iter().map(|e| &e.event_id).collect();
    let ids2: Vec<_> = page2.iter().map(|e| &e.event_id).collect();
    for id in &ids1 {
        assert!(!ids2.contains(id));
    }
}

#[test]
fn test_persistence_root_causation_id_can_be_null() {
    let store = SqliteAuditStore::open_in_memory().expect("open");
    let event = AuditEvent::new(
        EventType::InputReceived,
        CorrelationId::new(),
        CausationId::new(""), // empty causation for root input
        serde_json::json!({ "root": true }),
        "user",
    );
    store.record_event(&event).expect("record root event");

    let events = store
        .query_events(&AuditFilter::default())
        .expect("query root");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].causation_id.0, "");
}

#[test]
fn test_persistence_multiple_verification_records_per_operation() {
    let store = SqliteAuditStore::open_in_memory().expect("open");
    let op_id = OperationId::new();
    let cap_id = CapabilityId::new("system.echo");

    let rec1 = VerificationRecord {
        verification_record_id: format!("{}-eval1", op_id.0),
        operation_id: op_id.clone(),
        attempt_id: Some(AttemptId::new()),
        capability_id: cap_id.clone(),
        strategy_id: "echo_exact_match".to_string(),
        verification_state: VerificationState::Unknown,
        observed_effect_state: ObservedEffectState::Unknown,
        evaluated_at: now_utc(),
        verification_basis: "Avaliação inicial inconclusiva".to_string(),
        evidence_refs: vec![],
    };

    let rec2 = VerificationRecord {
        verification_record_id: format!("{}-eval2", op_id.0),
        operation_id: op_id.clone(),
        attempt_id: Some(AttemptId::new()),
        capability_id: cap_id,
        strategy_id: "echo_exact_match".to_string(),
        verification_state: VerificationState::VerifiedSuccess,
        observed_effect_state: ObservedEffectState::ObservedNoMutation,
        evaluated_at: now_utc(),
        verification_basis: "Avaliação subsequente com nova evidência".to_string(),
        evidence_refs: vec![EvidenceId::new()],
    };

    store.record_verification(&rec1).expect("record 1");
    store.record_verification(&rec2).expect("record 2");

    let records = store.get_verification_records(&op_id, 10).expect("query");
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].verification_state, VerificationState::Unknown);
    assert_eq!(
        records[1].verification_state,
        VerificationState::VerifiedSuccess
    );
}

#[test]
fn test_persistence_oversized_payload_handled_deterministically() {
    let store = SqliteAuditStore::open_in_memory()
        .expect("open")
        .with_max_payload_bytes(1024); // 1 KiB limit for testing

    let huge_string = "a".repeat(2048);
    let event = AuditEvent::new(
        EventType::EffectObserved,
        CorrelationId::new(),
        CausationId::new("cause"),
        serde_json::json!({ "blob": huge_string }),
        "test",
    );

    let result = store.record_event(&event);
    assert!(
        matches!(result, Err(PersistenceError::PayloadTooLarge { .. })),
        "Deveria rejeitar payload que excedeu 1 KiB"
    );
}

#[test]
fn test_persistence_corrupted_db_fails_closed_without_silent_destruction() {
    let path = temp_db_path();
    let _guard = TempDbCleanup(path.clone());

    // Write garbage bytes
    std::fs::write(&path, b"NOT_A_VALID_SQLITE_DATABASE_CORRUPT_BYTES").expect("write corrupt");

    let result = SqliteAuditStore::open(&path);
    assert!(
        result.is_err(),
        "Inicialização com arquivo corrompido deve falhar fechada"
    );

    // INVARIANT: Do NOT silently delete/destroy the corrupted file!
    assert!(
        path.exists(),
        "O arquivo corrompido deve ser preservado para perícia forense"
    );
}

#[tokio::test]
async fn test_persistence_async_spawn_blocking_execution() {
    let store = Arc::new(SqliteAuditStore::open_in_memory().expect("open"));
    let corr_id = CorrelationId::new();

    let event = AuditEvent::new(
        EventType::ResponseProduced,
        corr_id.clone(),
        CausationId::new("async_test"),
        serde_json::json!({ "async": true }),
        "tokio_task",
    );

    store
        .clone()
        .record_event_async(event)
        .await
        .expect("record async");

    let filter = AuditFilter {
        correlation_id: Some(corr_id),
        ..Default::default()
    };
    let events = store.query_events(&filter).expect("query");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].payload["async"], true);
}

#[test]
fn test_persistence_query_events_propagates_read_errors_never_masks_as_empty() {
    let path = temp_db_path();
    let _guard = TempDbCleanup(path.clone());

    let store = SqliteAuditStore::open(&path).expect("open");

    // Record one valid event
    let event = AuditEvent::new(
        EventType::InputReceived,
        CorrelationId::new(),
        CausationId::new("c1"),
        serde_json::json!({ "test": true }),
        "test",
    );
    store.record_event(&event).expect("record event");

    // Intentionally drop the audit_events table behind the scenes to simulate a query/table failure
    {
        let direct_conn = rusqlite::Connection::open(&path).expect("direct conn");
        direct_conn
            .execute_batch("DROP TABLE audit_events;")
            .expect("drop table");
    }

    // INVARIANT: query_events MUST propagate Err(PersistenceError::Read(...))
    // and MUST NEVER silently return Ok(vec![]) / empty dataset on database failure!
    let result = store.query_events(&AuditFilter::default());
    assert!(
        result.is_err(),
        "query_events DEVE retornar Err quando a tabela de auditoria estiver inacessível"
    );
    match result {
        Err(PersistenceError::Read(err_msg)) => {
            assert!(
                err_msg.contains("no such table") || err_msg.contains("audit_events"),
                "Mensagem de erro deve refletir falha de leitura SQL real: {}",
                err_msg
            );
        }
        other => panic!("Esperado Err(PersistenceError::Read), obtido: {:?}", other),
    }
}
