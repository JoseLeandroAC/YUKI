ADR-018 — Model Gateway & Provider Abstraction

Version: 1.0
Status: ACCEPTED
Domain: Architecture / Cognitive Subsystem / Model Gateway / Provider Abstraction
Date: 2026-10-03

---

1. Objective

Define the architectural boundary, abstractions, trust models, failure taxonomy, and operational contracts governing how Yuki integrates with external artificial intelligence and large language model (LLM) providers without transferring architectural, authorization, execution, or security authority to those models.

This ADR establishes that:

«Model Output != Command != Authorization»

and:

«Think != Authorize != Execute»

External models are treated as probabilistic cognitive advisers providing unprivileged intent proposals, not as privileged controllers or instruction engines.

---

2. Context

The Yuki Foundation v0.1 (MVP-0) validated the structural separation between model proposals and capability execution using an in-memory, deterministic simulation provider (`MockModelProvider`). 

To transition from Foundation to a real functional runtime (MVP-1) connecting to external model providers (such as Google Gemini, OpenAI, local models, or specialized models), Yuki requires a formal abstraction layer.

Without this formal abstraction:
1. Yuki Core would risk coupling directly to proprietary vendor SDKs, request formats, or error types, violating ADR-017 (Vendor Independence & Platform Reuse).
2. Model outputs could be inadvertently conflated with execution orders, allowing prompt injection or hallucinated outputs to compromise system integrity.
3. Credentials or sensitive contextual user data could leak into prompts or model contexts, violating ADR-008 (Credential Isolation) and ADR-015 (Privacy & Data Minimization).
4. Asynchronous network latencies or provider outages could block execution threads and degrade system reliability.

---

3. Decision

Yuki adopts a sovereign, provider-agnostic **Model Gateway** subsystem.

Specifically:
1. Yuki Core communicates exclusively through an abstract, asynchronous trait (`ModelProvider`) operating on canonical Yuki-owned types (`ModelRequest` and `ModelResponse`).
2. The subsystem cleanly separates:
   - **Model Router**: Selects the active provider adapter based on runtime configuration.
   - **Provider Adapter**: Translates canonical Yuki requests into vendor-specific HTTP/REST wire formats and parses responses back into typed Yuki structures.
   - **Proposal Parser**: Syntactically and semantically validates tool/function proposals, converting them into unprivileged `CapabilityProposal` objects.
3. Provider outputs are strictly constrained: a model may only produce conversational text or a `CapabilityProposal`. Under no circumstances does model output trigger direct execution.
4. Failures are categorized using a strongly typed `ModelError` taxonomy rather than generic strings.
5. In MVP-1, the default live adapter is the `GeminiProviderAdapter` (using direct HTTPS calls with TLS via rustls), while `MockModelProvider` is retained for offline validation and continuous integration.

---

4. Definitions

- **ModelRequest**: Canonical input passed from Yuki Core containing sanitized conversation turns, system instructions, and JSON Schemas of registered capabilities.
- **ModelResponse**: Canonical output produced by a provider adapter, structured as either direct conversational response or an unprivileged intent proposal.
- **ModelProvider**: The public async trait exposed to Yuki Core.
- **ProviderAdapter**: Low-level integration adapter (e.g., `GeminiProviderAdapter`) managing HTTP connections, wire serialization, and header injection.
- **ModelRouter**: Component responsible for dispatching requests to the appropriate configured adapter (Mock, Gemini, or future adapters).
- **CapabilityProposal**: Unprivileged proposal specifying `proposal_id`, `capability_id`, `parameters`, and rationale.
- **ModelError**: Strongly typed enum capturing all operational and protocol failure states.

---

5. Architecture

```text
[YukiCore]
    │
    ▼ (Canonical ModelRequest)
[Model Gateway / Model Router]
    │
    ├───────────────────────┬────────────────────────┐
    ▼                       ▼                        ▼
[MockModelProvider]    [GeminiProviderAdapter]  [Future Adapters]
                            │
                            ▼ (HTTPS / TLS)
                   [External Provider API]
                            │
                            ▼ (Provider JSON Wire Format)
                   [GeminiProviderAdapter]
                            │
                            ▼ (Typed ModelResponse)
                   [ProposalParser]
                            │
                            ▼
                  [CapabilityProposal]
                            │
                            ▼
                 [SecurityController]
```

---

6. Trust Boundaries

1. **Untrusted External Realm**: External LLMs and their hosting platforms reside outside Yuki's trust boundary (*Assume Breach*). Model outputs are inherently non-deterministic and potentially adversarial (vulnerable to prompt injection, data poisoning, or jailbreak attempts).
2. **Adapter Neutrality**: The provider adapter performs purely mechanical translation of wire formats. It possesses no authorization or policy evaluation logic.
3. **Proposal Barrier**: The Model Gateway is incapable of issuing `CapabilityToken` or invoking capabilities. Its only action-oriented output is a `CapabilityProposal`, which has zero authority until explicitly authorized by the `SecurityController`.

---

7. Provider Independence & Exit Path

In strict compliance with ADR-017:
- Yuki achieves **reduced provider coupling** and a **preserved architectural exit path**.
- Provider substitution is achieved through modular adapters where semantic differences permit.
- Adapters reduce vendor lock-in; they do not claim that all AI providers are semantic clones.
- Yuki Core contains zero dependencies on vendor-specific client libraries or proprietary data models.
- Capabilities declare schemas using standard JSON Schema, which adapters project into provider-specific representations (such as Gemini Function Declarations).

---

8. Model Output Handling & Capability Proposal Boundary

Every response received from an external model undergoes strict parsing and validation:

```text
External Provider Response (Wire JSON)
        ↓
Provider Adapter (Extracts candidate function call / text)
        ↓
Typed ModelResponse
        ↓
Proposal Parser (Validates schema, checks capability existence, bounds check)
        ↓
CapabilityProposal (Attached to OperationId / CausationId)
        ↓
Security Controller (Evaluates policy, permissions, and arguments)
```

Direct execution of text or unverified execution of tool calls is architecturally prohibited.

---

9. Failure Model (ModelError Taxonomy)

Model failures must be classified into specific typed categories to enable appropriate system responses:

- `Timeout(Duration)`: Upstream provider exceeded configured request timeout.
- `NetworkUnavailable(String)`: Network connectivity or DNS resolution failure.
- `AuthenticationFailure`: Rejected API credentials or invalid permissions.
- `RateLimited`: Provider rate limits or quotas exceeded (HTTP 429).
- `ProviderUnavailable { status, message }`: Upstream service outage or server error (HTTP 5xx).
- `MalformedResponse(String)`: Output failed schema validation or payload decoding.
- `UnsupportedCapabilityProposal(String)`: Model proposed a capability not declared in the request.
- `InvalidArguments { capability, details }`: Proposed arguments failed structural validation.
- `Internal(String)`: Internal runtime or serialization errors.

Automatic retries are not applied indiscriminately; retries are governed by caller context and idempotency semantics.

---

10. Credential Boundary (ADR-008 Compliance)

- Provider adapters never manage or persistently store raw secret materials.
- Adapters reference credentials via an immutable `SecretRef` (e.g., `secret://env/YUKI_GEMINI_API_KEY`).
- Secret material (raw API keys in memory) is fetched through the `CredentialBroker` solely at the point of HTTP dispatch, injected into transport headers, and immediately dropped from operational memory.
- Secret material never enters `ContextObject`, `ModelRequest`, prompts, logs, or audit records.

---

11. Privacy & Data Minimization (ADR-015 Compliance)

- Context passed to the model must be minimal and strictly necessary for the user's intent.
- Model prompts and raw model responses are considered sensitive data.
- By default, operational logs do not record complete prompt text or raw model responses.
- Operational logs record only:
  - `operation_id` and `correlation_id`
  - Selected provider and model identifier
  - Request latency
  - Token counts and usage metadata (when provided by upstream)
  - Result status code and error classification

---

12. Observability vs. Audit

- Operational metrics and debug traces generated by `tracing` are disposable and subject to redaction.
- Formal events (such as `ModelInvoked`, `CapabilityProposed`) recorded in `EventStore` capture immutable metadata necessary for accountability without storing raw secret material.

---

13. Testing Strategy

1. **Contract and Offline Integration Tests**: 100% of standard CI tests execute against `MockModelProvider`. No external network access or API credentials are required for build verification.
2. **Live Integration Tests**: Live tests targeting external endpoints are isolated behind `#[ignore]` flags and require explicit opt-in with valid local environment configuration (`cargo test -- --ignored`).

---

14. Streaming (MVP-1 Deferral)

Token-by-token streaming (via Server-Sent Events or WebSockets) is explicitly deferred to post-MVP-1 phases. MVP-1 requires standard request-response batch cycles to prove authorization, verification, and durability.

---

15. Consequences

### Positive
- Strict decoupling of Yuki Core from external AI vendors.
- Preservation of constitutional invariants against prompt injection and unauthorized execution.
- Fast, reliable, zero-cost CI builds without external dependencies.
- Clear, typed error propagation throughout the system.

### Negative / Trade-offs
- Additional boilerplate to translate between vendor wire formats and Yuki canonical types.
- Streaming capabilities are postponed to future milestones.

---

16. Rejected Alternatives

- *Direct usage of official vendor SDKs*: Rejected due to heavy transitive dependencies, lack of unified abstractions, and architectural vendor lock-in.
- *Magic text command parsing (e.g. `!execute ...`)*: Rejected as insecure, unverified, and prone to injection.
- *Universal asynchronous conversion of all domain logic*: Rejected; domain evaluation remains pure and synchronous.

---

17. Closed Decisions

- The Model Gateway trait is asynchronous at the I/O boundary.
- The default external adapter for MVP-1 is Google Gemini via HTTPS REST.
- Model proposals are unprivileged until verified and authorized by the Security Controller.
