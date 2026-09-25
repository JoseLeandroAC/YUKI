ADR-017 — External Platform Reuse, Runtime Adapters & Vendor Independence

Version: 1.0
Status: ACCEPTED
Domain: Architecture / Infrastructure / Agent Runtime / Vendor Independence
Date: 2026-09-25

---

1. Objective

Define how Yuki may reuse, adapt, extend, replace or reimplement external technologies without transferring architectural authority, security authority, data authority, user agency or safety authority to external providers.

This ADR establishes the architectural principle:

«Reuse Before Reimplement.»

However:

«Reusing an implementation does not mean transferring authority to that implementation.»

External platforms are treated as implementation substrates, providers, runtimes, models, protocols, capabilities or development tools according to their actual architectural role.

---

2. Core Principle

Yuki should not rebuild mature infrastructure merely to own its implementation.

The preferred order is:

REUSE
  ↓
ADAPT
  ↓
EXTEND
  ↓
REPLACE
  ↓
REIMPLEMENT

The selected strategy depends on:

- architectural compatibility;
- security;
- privacy;
- performance;
- reliability;
- cost;
- portability;
- maintainability;
- observability;
- lock-in;
- reversibility;
- operational maturity;
- future compatibility.

---

3. External Technology Is Not Architectural Authority

An external component may execute code, provide models, store information, perform research, communicate with systems or assist development.

None of these properties automatically grants authority.

Therefore:

External Implementation
      ≠
Yuki Authority

and:

Provider
      ≠
Policy Authority
      ≠
Security Authority
      ≠
Safety Authority
      ≠
User Agency

---

4. External Platform Classification

External technologies may be classified as:

4.1 Model Provider

Provides inference models.

Examples:

- Gemini;
- GPT;
- Claude;
- local models;
- future providers.

4.2 Agent Runtime

Provides execution/orchestration primitives for agents.

Examples:

- Google ADK;
- other agent frameworks;
- future runtimes.

4.3 Development Agent

Provides autonomous or semi-autonomous software engineering.

Examples:

- Google Antigravity;
- other coding agents;
- future development agents.

4.4 Research Capability

Provides research, search, grounding or information retrieval.

Examples:

- Deep Research;
- search APIs;
- academic APIs;
- specialized research engines.

4.5 Protocol

Provides interoperability.

Examples:

- MCP;
- A2A;
- REST;
- gRPC;
- future protocols.

4.6 Runtime / Infrastructure Provider

Provides execution or infrastructure.

Examples:

- Cloud Run;
- Kubernetes;
- virtual machines;
- serverless infrastructure;
- local execution environments.

4.7 Storage / Knowledge Provider

Provides storage, retrieval, indexing or memory primitives.

4.8 Security Primitive Provider

Provides primitives such as:

- identity;
- secret storage;
- cryptographic keys;
- attestation;
- encryption;
- workload identity.

These primitives do not replace Yuki's Security Controller semantics.

---

5. Yuki-Owned Architecture

The following concepts remain architecturally owned by Yuki regardless of implementation provider:

- identity model;
- authority model;
- capability model;
- permission semantics;
- authorization semantics;
- security policy;
- safety boundaries;
- human agency;
- goal ownership;
- delegation;
- information boundaries;
- privacy policy;
- Typed Context Object;
- memory policy;
- verification semantics;
- reconciliation semantics;
- durable mission semantics;
- resource abstraction;
- provider independence;
- architectural contracts.

External technology may implement portions of these mechanisms but must conform to Yuki-defined contracts.

---

6. Adapter Boundary

External components must be integrated through an appropriate abstraction boundary.

Examples:

Yuki Model Contract
        ↓
Model Adapter
        ↓
Gemini / GPT / Claude / Local Model

Yuki Agent Contract
        ↓
Agent Runtime Adapter
        ↓
ADK / Other Runtime

Yuki Development Contract
        ↓
Development Agent Interface
        ↓
Antigravity / Other Coding Agent

Yuki Integration Contract
        ↓
Integration Gateway
        ↓
MCP / REST / gRPC / A2A / Other Protocol

The Core must not depend directly on provider-specific implementation details.

---

7. Provider Independence

Provider-specific behavior must remain isolated behind adapters whenever practical.

Provider changes should not require architectural changes to:

- Yuki Core;
- Security Controller;
- Memory architecture;
- Context architecture;
- Capability contracts;
- Mission semantics;
- Goal model;
- Verification model;
- Reconciliation model.

---

8. Model Independence

No model is constitutionally required.

Yuki must be able to operate with:

- multiple cloud providers;
- local models;
- specialized models;
- future model architectures.

The Model Router determines which model or combination of models is appropriate.

Provider selection does not modify authorization.

Model Selection
      ≠
Permission

Model Output
      ≠
Authorization

---

9. Agent Runtime Independence

Agent runtimes are execution mechanisms.

An external runtime may provide:

- agent lifecycle;
- tool invocation;
- workflow primitives;
- streaming;
- multi-agent communication;
- evaluation;
- execution workers.

However:

Agent Runtime
      ≠
Yuki Mission Authority

Agent Runtime
      ≠
Security Controller

Agent Runtime
      ≠
Safety Controller

Agent Runtime
      ≠
Human Agency Authority

---

10. External Agents

External agents must be treated as bounded workers.

They receive:

- explicit task;
- minimal context;
- permitted tools;
- bounded resources;
- applicable delegation;
- applicable policy.

They do not automatically receive:

- global memory;
- credentials;
- unrestricted network access;
- unrestricted filesystem access;
- unrestricted capabilities;
- authority to modify Yuki architecture.

---

11. Development Agents

Development agents may operate inside the Yuki Development Lab.

Preferred workflow:

ADR
 ↓
Implementation Specification
 ↓
Development Agent
 ↓
Isolated Branch
 ↓
Implementation
 ↓
Tests
 ↓
Security Checks
 ↓
Architecture Review
 ↓
Human Approval
 ↓
Merge

Development agents may:

- inspect code;
- propose architecture;
- implement code;
- run tests;
- debug;
- refactor;
- generate documentation;
- prepare pull requests.

They may not independently:

- redefine Yuki constitutional architecture;
- remove security controls;
- expand privileges;
- merge protected changes without required approval;
- alter critical authority models.

---

12. External Research

External research engines are treated as information capabilities.

Research output is:

EXTERNAL DATA

and is not automatically:

TRUTH
AUTHORIZATION
POLICY
GOAL
INSTRUCTION

Research results must pass through applicable:

- provenance handling;
- source evaluation;
- context validation;
- information policy;
- prompt-injection defenses.

---

13. MCP

MCP may be adopted as an external tool/resource interoperability protocol.

MCP is not constitutional to Yuki.

Therefore:

Yuki Integration Contract
        ↓
Integration Gateway
        ↓
MCP Adapter
        ↓
MCP Server

MCP may coexist with:

- REST;
- gRPC;
- WebSockets;
- A2A;
- MQTT;
- future protocols.

---

14. A2A

A2A may be used for interoperability between agents.

A2A provides communication/interoperability.

It does not provide Yuki authorization.

External agents remain untrusted or conditionally trusted according to Yuki policy.

Agent discovery does not imply permission.

---

15. External Security Primitives

External security services may provide primitives such as:

- IAM;
- Secret Manager;
- KMS;
- workload identity;
- OAuth/OIDC;
- SPIFFE/SPIRE;
- TPM;
- HSM;
- attestation;
- mTLS.

These mechanisms may be reused.

However:

Security Primitive
      ≠
Yuki Security Authority

The Yuki Security Controller remains responsible for Yuki-level authorization semantics.

---

16. External Infrastructure

External infrastructure may implement Yuki resources.

Examples:

Cloud GPU
Cloud Run
VM
Kubernetes Node
Local GPU
NPU
Home Server
Edge Device

These become resources through ADR-006 abstractions.

Provider-specific infrastructure must not leak into Yuki Core unnecessarily.

---

17. Lock-in Management

Every external dependency should be evaluated for:

- API lock-in;
- data lock-in;
- model lock-in;
- runtime lock-in;
- protocol lock-in;
- identity lock-in;
- operational lock-in;
- cost lock-in.

The architecture should maintain an exit path whenever reasonably practical.

Exit strategy may include:

- adapter replacement;
- portable container;
- standard protocol;
- exported data;
- model abstraction;
- migration tooling;
- local fallback;
- alternative provider.

---

18. Conformance

External implementations must be evaluated against the Yuki contract.

Conformance tests should verify:

- input/output contracts;
- authorization boundaries;
- context boundaries;
- data minimization;
- credential isolation;
- failure behavior;
- retry behavior;
- observability;
- verification requirements;
- resource limits;
- security invariants.

A provider may be replaced if it no longer satisfies required contracts.

---

19. Technology Radar

Yuki may classify technologies as:

ADOPT

Production-ready implementation currently preferred.

TRIAL

Used experimentally or in limited production scope.

ASSESS

Monitored for future adoption.

HOLD

Technically interesting but not currently adopted.

REJECT

Fails architectural or security requirements.

Technology status is not constitutional.

A technology may move between categories without changing Yuki's architecture.

---

20. Google Ecosystem

Google technologies may be used as implementation substrates.

Potential roles include:

Gemini
→ Model Provider

ADK
→ Agent Runtime

Antigravity
→ Development Agent

Deep Research
→ Research Capability

MCP
→ Tool/Resource Adapter

A2A
→ Agent Interoperability Adapter

Cloud Run
→ Execution Infrastructure

Vertex AI
→ Model/AI Infrastructure Provider

Google Cloud Security Services
→ Security Primitives

These roles do not grant Google architectural authority over Yuki.

---

21. No Direct Core-to-Provider Rule

Yuki Core should not directly depend on provider-specific APIs when an appropriate abstraction boundary exists.

Preferred:

Yuki Core
 ↓
Yuki Contract
 ↓
Adapter / Gateway
 ↓
Provider

Avoid:

Yuki Core
 ↓
Provider SDK

when such coupling would compromise portability or architectural boundaries.

---

22. Graceful Degradation

External providers may become:

- unavailable;
- slow;
- rate-limited;
- degraded;
- deprecated;
- financially impractical;
- compromised;
- incompatible.

Yuki should degrade according to task and policy.

Possible fallback:

Primary Provider
 ↓
Alternative Provider
 ↓
Local Model
 ↓
Reduced Capability
 ↓
Deferred Execution

Fallback must not silently expand permissions.

---

23. External Provider Compromise

A compromised provider must not automatically compromise Yuki.

Security boundaries must limit:

- credentials;
- context;
- memory;
- network;
- filesystem;
- capabilities;
- authority.

This follows the principle:

«Compromising a tool must not mean compromising Yuki.»

---

24. Data Boundary

External providers receive only data required for the current operation and permitted by ADR-015/016.

Yuki Data
 ↓
Information Policy
 ↓
Data Access Gateway
 ↓
Data Projection
 ↓
External Provider

Raw global memory must never be treated as default provider context.

---

25. Development Lab

The Development Lab may use multiple external development agents.

Example:

              YUKI DEVELOPMENT LAB
                       │
        ┌──────────────┼──────────────┐
        ▼              ▼              ▼
   Antigravity     Other Agent    Local Agent
        │              │              │
        └──────────────┼──────────────┘
                       ▼
                  Git Branch
                       │
                       ▼
                    Tests
                       │
                       ▼
                 Security Review
                       │
                       ▼
              Architecture Review
                       │
                       ▼
                 Human Approval

Multiple agents may independently implement or review the same specification.

Agreement between agents is not itself a security guarantee.

---

26. Experimental Adoption

New technologies should first enter through:

Experiment
 ↓
Sandbox
 ↓
Benchmark
 ↓
Security Evaluation
 ↓
Conformance Test
 ↓
Limited Production
 ↓
Monitoring
 ↓
Adoption / Replacement / Removal

---

27. Architectural Invariants

INV-017-1

External implementation never automatically acquires Yuki authority.

INV-017-2

Provider selection never grants permission.

INV-017-3

Model output never constitutes authorization.

INV-017-4

External data never constitutes instruction.

INV-017-5

External agents receive only permitted context.

INV-017-6

External agents do not receive raw credentials.

INV-017-7

External runtimes do not replace Security Controller.

INV-017-8

External runtimes do not replace Safety Controller.

INV-017-9

External platforms do not replace Verification Engine.

INV-017-10

External platforms do not replace Reconciliation Engine.

INV-017-11

Provider replacement should not require rewriting Yuki Core.

INV-017-12

Development agents cannot unilaterally redefine constitutional architecture.

INV-017-13

Technology choices remain replaceable implementations unless explicitly elevated by a future ADR.

---

28. Closed Decisions

D017-1 — Reuse Before Reimplement

Yuki prefers mature external implementations over unnecessary reinvention.

D017-2 — Authority Independence

External implementations do not acquire Yuki architectural authority.

D017-3 — Adapter Boundary

External providers should be integrated through appropriate contracts, adapters or gateways.

D017-4 — Model Independence

No single model provider is constitutionally required.

D017-5 — Agent Runtime Independence

No single agent runtime is constitutionally required.

D017-6 — Development Agent Subordination

Development agents operate under Yuki specifications, repository controls, tests and required human approval.

D017-7 — Provider Replacement

External providers must remain replaceable when reasonably practical.

D017-8 — External Data Boundary

External information is not automatically instruction, authority or truth.

D017-9 — Credential Isolation

External models and agents do not receive raw credentials.

D017-10 — Data Minimization

Only necessary and authorized data is provided to external systems.

D017-11 — Security Independence

External IAM/security primitives do not replace Yuki Security Controller semantics.

D017-12 — Safety Independence

External AI platforms do not replace physical Safety Controllers.

D017-13 — Verification Independence

External agent output does not automatically establish external effect.

D017-14 — Reconciliation Independence

External platforms do not become Yuki Reconciliation Authority.

D017-15 — Technology Radar

External technologies are classified by adoption status without becoming constitutional dependencies.

D017-16 — Experimental Adoption

New external technologies must be evaluated before becoming production dependencies.

D017-17 — Graceful Degradation

Provider failure must not automatically become Yuki failure when an applicable fallback exists.

D017-18 — Exit Strategy

Important external dependencies should have a practical replacement or degradation strategy.

D017-19 — Contract First

Yuki contracts precede provider-specific implementations.

D017-20 — Human Architectural Authority

Critical constitutional changes remain subject to the required human approval process.

---

29. Open Decisions

The following remain implementation decisions:

- exact adapter interface;
- model adapter schema;
- agent runtime adapter schema;
- development-agent interface;
- provider conformance framework;
- provider health scoring;
- vendor lock-in scoring;
- data export standards;
- model portability;
- runtime portability;
- provider migration tooling;
- Google-specific implementation;
- MCP adapter;
- A2A adapter;
- ADK adapter;
- Antigravity development workflow;
- Cloud Run deployment;
- alternative provider implementations.

---

30. Relationship With Existing ADRs

ADR-006
Infrastructure
        ↓
External providers become abstract resources

ADR-007
Integrations
        ↓
Provider connections use Integration Gateway

ADR-008
Credentials
        ↓
External platforms receive scoped credential access

ADR-009
Verification
        ↓
Provider output is evidence/input, not automatic truth

ADR-010
Physical Safety
        ↓
External agents cannot bypass Safety Controller

ADR-011
Durable Workflow
        ↓
External agents operate as workers/activities

ADR-012
Reconciliation
        ↓
External platforms cannot become reconciliation authority

ADR-013
Federation
        ↓
Provider location does not imply authority

ADR-014
Human Agency
        ↓
External agents operate within delegation boundaries

ADR-015
Privacy
        ↓
External providers receive purpose-bound minimal data

ADR-016
Context
        ↓
External models receive Typed Context Objects / projections

ADR-017
External Platform Reuse
        ↓
Defines how all external technologies enter Yuki

---

31. Consolidated Principle

The Yuki architecture follows:

«Own the architecture. Reuse the implementation. Control the authority. Preserve the exit path.»

Yuki does not need to own every line of code.

Yuki needs to own the contracts that determine what that code is allowed to do.

---

32. Status

ACCEPTED — v1.0

This ADR establishes the architectural basis for integrating Google, OpenAI, Anthropic, open-source projects, local models, cloud infrastructure, future agent platforms and future technologies without transferring constitutional authority to any external provider.