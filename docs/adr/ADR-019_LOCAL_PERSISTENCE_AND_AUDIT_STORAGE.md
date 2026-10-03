ADR-019 — Local Persistence & Audit Storage

Version: 1.0
Status: ACCEPTED
Domain: Architecture / Persistence / Audit Storage / Infrastructure
Date: 2026-10-03

---

1. Objective

Define the architectural boundary, durability guarantees, crash recovery semantics, schema versioning, and privacy constraints governing how Yuki stores audit trails and security-relevant operational records locally on disk.

This ADR establishes that:

«Persistence Contract != SQLite»

and:

«Audit History != Operational State»

and:

«Audit Record != Truth != Authorization»

The audit store serves as an immutable, append-only historical record of security-relevant and operationally-relevant observations, not as a general-purpose operational database, memory cache, or authorization engine.

---

2. Context

The Yuki Foundation v0.1 (MVP-0) employed an ephemeral in-memory event store (`InMemoryEventStore`). While effective for single-process test suites, process termination causes the complete loss of all audit trails, authorization decisions, and verification records.

For Yuki to operate as an autonomous, durable, and remotely accessible runtime (MVP-1 and beyond):
1. Audit records must survive process restarts, container restarts, and planned host shutdowns.
2. Operators must be able to inspect security decisions and operational history via standard tools.
3. Cold disaster recovery from a clean environment must be achievable with minimal moving parts.

However, adopting an external distributed relational database (e.g., PostgreSQL or MySQL) at this phase would violate operational simplicity, introduce network dependencies, inflate operational costs, and contradict ADR-006 (Infrastructure Resource Model).

---

3. Decision

Yuki defines a sovereign, decoupled **Persistence Contract** for event auditing and adopts an embedded **SQLite Adapter** as the initial local storage engine for MVP-1.

Specifically:
1. Yuki Core communicates exclusively through an abstract, asynchronous trait (`EventStore`).
2. The initial adapter implementation for MVP-1 is `SqliteEventStore`, using an embedded, self-contained SQLite engine compiled into the binary.
3. The storage engine operates strictly as an **append-only audit log** for `AuditEvent` records.
4. **CapabilityTokens are NEVER persisted in the database**: Only immutable decision metadata (such as `token_id`, `operation_id`, `capability_id`, `authorized_at`, `consumed_at`, argument hash, and decision outcome) is recorded. This guarantees:
   $$\text{Auditability without Replayable Authority}$$
5. The storage schema is explicitly versioned from day one via embedded transactional migrations.

---

4. Definitions

- **AuditEvent**: An immutable structured record of a security-relevant or operationally-relevant event observed by Yuki.
- **Persistence Contract**: The abstract trait definition (`EventStore`) independent of underlying database technologies.
- **SqliteEventStore**: Concrete adapter implementing `EventStore` using an embedded SQLite database file.
- **Write-Ahead Logging (WAL)**: SQLite journaling mode separating concurrent readers from an exclusive writer.
- **Schema Migration**: A deterministic, forward-only SQL script updating the audit schema across versions.

---

5. Audit Semantics & Non-Replayable Authority

The persistent audit store preserves historical facts, not executable authority.

### Data Recorded in AuditEvent
- `event_id`: Unique event identifier (UUID v4).
- `correlation_id` & `causation_id`: Request tracing identifiers.
- `event_type`: Categorized event (e.g., `InputReceived`, `AuthorizationGranted`, `ExecutionCompleted`, `VerificationEvaluated`).
- `operation_id`: Unique operation binding.
- `capability_id`: Name of the capability involved.
- `arguments_hash`: Cryptographic SHA-256 digest of input parameters (preventing parameter tampering without persisting raw sensitive inputs).
- `authorization_decision`: `ALLOWED`, `DENIED`, or `REQUIRES_APPROVAL`.
- `verification_state`: `VERIFIED`, `FAILED`, or `UNKNOWN`.
- `timestamp`: UTC timestamp of event observation.

### Data Explicitly Excluded from Persistence
- Active or reusable `CapabilityToken` data.
- Cryptographic secret keys or raw secret materials (`Secret Material`).
- Unsanitized plaintext passwords or sensitive credentials.

---

6. Durability & Crash Recovery Semantics

SQLite transactions and WAL provide transactional crash-recovery properties according to SQLite's durability semantics and the configured synchronous mode.

Durability against OS, filesystem, storage-controller, or physical-media failure depends on the underlying platform, operating system configuration, and storage hardware.

### Synchronous Mode Engineering Decision
- The adapter configures WAL mode on connection initialization:
  ```sql
  PRAGMA journal_mode = WAL;
  ```
- **Engineering Choice (`NORMAL` vs `FULL`)**:
  - In MVP-1, `PRAGMA synchronous = NORMAL;` is selected for standard local runtime operations. In `NORMAL` mode with WAL, the database ensures consistency against process crashes and unexpected application termination. Full sync on every single commit (`FULL`) may be toggled via configuration when deploying to critical storage environments where power-loss survival takes precedence over disk write throughput.
- In the event of an abrupt process crash, uncommitted transactions are cleanly rolled back during the next database open operation, preventing database corruption.

---

7. Schema Evolution & Migration Ownership

The database file contains a dedicated metadata table tracking applied migrations:

```sql
CREATE TABLE IF NOT EXISTS _yuki_schema_version (
    version INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    applied_at TEXT NOT NULL
);
```

On startup, `SqliteEventStore` inspects the current schema version and executes pending transactional migrations before accepting any runtime operations. If a migration fails, the transaction rolls back, and startup aborts cleanly (*fail-closed*).

---

8. Concurrency Model

- **Single Writer, Multiple Readers**: The SQLite adapter serializes writes through an asynchronous internal mutex/lock or dedicated channel actor, completely avoiding database lock contention (`SQLITE_BUSY`).
- Readers (CLI inspection commands, health checks) operate concurrently without blocking or being blocked by the audit writer under WAL mode.

---

9. Privacy, Data Minimization & Retention (ADR-015)

- Stored events contain minimized payloads. Raw user inputs and model outputs undergo sanitization before event creation.
- **Retention Policy for MVP-1**:
  - Automatic retention/expiration policies are **OUT OF SCOPE** for MVP-1.
  - Events remain stored locally until explicit administrative action (e.g., manual vacuum/archive) or future formal retention policies are defined in post-MVP-1 milestones.
  - This design preserves ADR-015 without introducing premature automated pruning mechanisms.

---

10. Backup & Restore Boundary

- The entire audit state is encapsulated within a single file (`yuki.db`) and its ephemeral WAL companions (`yuki.db-wal`, `yuki.db-shm`).
- Safe hot backup can be performed using SQLite's online backup API or standard `VACUUM INTO 'backup.db'`.
- Restore procedure consists simply of placing a verified database backup file into the configured data path prior to process start.

---

11. Failure Model & Fail-Closed Behavior

The audit store is a critical security dependency. If the database file is inaccessible, read-only, corrupted, or runs out of disk space:
- The system must **fail closed** for operations requiring authorization and execution.
- Yuki will not execute capabilities if the corresponding audit trail cannot be durably recorded.

---

12. Consequences

### Positive
- Fully self-contained, zero-configuration local persistence.
- Complete durability of audit trails across restarts and container lifecycles.
- Non-replayable tokens ensure storage compromise cannot grant execution authority.
- Simple, transparent file-based backup and restore.

### Negative / Trade-offs
- Embedded C dependency (`rusqlite`) requiring compilation toolchain support.
- Not suited for multi-node active-active clustering (adequate for Yuki's single-instance deployment model).

---

13. Rejected Alternatives

- *Persisting active CapabilityTokens in the database*: Rejected to prevent token replay and privilege escalation from storage.
- *Unstructured flat file logging (log.txt)*: Rejected due to lack of ACID transactions, queryability, and schema enforcement.
- *External client-server RDBMS (PostgreSQL)*: Rejected for MVP-1 to avoid unnecessary operational overhead and cloud infrastructure costs.

---

14. Closed Decisions

- The abstract interface is `EventStore`; SQLite is an implementation adapter.
- Audit history is strictly append-only and distinct from operational state.
- Database transactions use WAL mode.
