# ADR-013 — Yuki Home/Cloud Event Synchronization & Offline State Federation Protocol

**Status:** ACCEPTED
**Version:** 1.0
**Domain:** Infrastructure / Distributed Systems / Federation
**Date:** 2026-09-05

---

## 1. Objective

Definir a arquitetura de sincronização, autoridade distribuída e operação durante partições de rede entre os diferentes domínios computacionais da Yuki.

Este ADR estabelece como a Yuki deve lidar com:

* Home;
* Cloud;
* Mobile;
* Edge;
* outros nós futuros;
* armazenamento distribuído;
* replicação;
* sincronização de eventos;
* sincronização de estado;
* autoridade distribuída;
* leases;
* epochs;
* fencing;
* causalidade;
* operações offline;
* recuperação após partições;
* conflitos;
* disaster recovery;
* promoção de autoridade.

O objetivo é permitir que a Yuki continue funcional durante falhas de conectividade sem criar múltiplas autoridades conflitantes ou permitir que um nó obsoleto continue produzindo mutações perigosas.

---

# 2. Context

A Yuki não será necessariamente executada em um único computador.

Sua arquitetura prevê:

```text
                    YUKI
                      │
          ┌───────────┼───────────┐
          ▼           ▼           ▼
        EDGE        HOME        CLOUD
          │           │           │
       Mobile       Server      Workers
       Watch        Storage     Services
       Devices      Devices     Overflow
```

Esses ambientes podem:

* perder conectividade;
* funcionar temporariamente offline;
* possuir dados parcialmente diferentes;
* executar tarefas locais;
* armazenar réplicas;
* possuir diferentes recursos;
* possuir diferentes níveis de confiança;
* possuir diferentes responsabilidades.

Portanto, a arquitetura não pode depender da existência permanente de:

```text
uma rede perfeita
+
um servidor central
+
um relógio global perfeitamente sincronizado
+
uma autoridade global única
```

---

# 3. Architectural Principles

## 3.1 No Global Sovereign Node

Nenhum nó da Yuki possui autoridade global permanente sobre todo o sistema.

Nem:

```text
Home
```

nem:

```text
Cloud
```

nem:

```text
Mobile
```

é constitucionalmente soberano sobre toda a Yuki.

A autoridade é vinculada a domínios e recursos.

---

## 3.2 Resource-Bound Authority

A autoridade deve ser associada ao domínio e/ou recurso sobre o qual determinada operação possui jurisdição.

```text
Domain
   │
   └── Resource Scope
          │
          └── Authority
```

Exemplo:

```text
Home Automation Domain
        │
        └── thermostat.living_room
```

pode possuir autoridade no Home, enquanto:

```text
Mobile Personal Data Domain
```

pode possuir autoridade local no dispositivo móvel.

---

## 3.3 Authority ≠ Storage

O local onde uma informação está armazenada não determina quem possui autoridade sobre ela.

```text
Cloud Replica
      ≠
Cloud Authority
```

Uma réplica pode armazenar dados completos sem possuir autorização para modificá-los.

---

## 3.4 State Ownership ≠ Execution Authority

O proprietário lógico de um recurso não precisa ser o processo que executa uma determinada operação.

Exemplo:

```text
Home Domain
   │
   └── State Authority

Cloud Worker
   │
   └── Temporary Execution Authority
```

A execução delegada deve ser limitada e protegida contra stale writers.

---

## 3.5 Security Authority ≠ State Authority

A autoridade sobre segurança permanece separada da autoridade sobre o estado de determinado recurso.

Integração com:

* ADR-008 — Credential Isolation;
* ADR-010 — Physical Safety;
* Security Controller;

continua obrigatória.

---

# 4. Authority Model

A Yuki reconhece múltiplas dimensões de autoridade.

```text
Authority Model
├── State Authority
├── Execution Authority
├── Security Authority
├── Safety Authority
├── Credential Authority
└── Observation Authority
```

Essas dimensões podem pertencer a componentes diferentes.

### Exemplo — Robotics

```text
State Authority
    → Robotics Domain

Execution Authority
    → Local Controller

Safety Authority
    → Safety Controller

Credential Authority
    → Credential Broker

Observation Authority
    → Sensors / Observation Layer

Planning
    → Yuki Core
```

Isso mantém a regra:

> Yuki pode solicitar uma ação sem possuir autoridade física para executá-la.

---

# 5. State Ownership

Cada recurso que exige autoridade exclusiva deve possuir uma definição explícita de domínio responsável.

Isso não significa necessariamente que apenas um computador físico possa armazenar seus dados.

Significa que existe uma autoridade definida para resolver mutações conflitantes naquele escopo.

### State Ownership ≠ Storage Location

```text
State Owner
      │
      ├── Primary Storage
      ├── Replicas
      ├── Backups
      └── Caches
```

A existência de qualquer desses elementos não altera automaticamente a autoridade.

---

# 6. Authority Issuer

A emissão de autoridade delegada não pode ser realizada arbitrariamente por qualquer nó.

Cada domínio deve possuir um mecanismo definido de emissão de autoridade.

```text
Domain Authority
       │
       ├── Epoch
       ├── Lease
       └── Fencing Token
```

O mecanismo concreto de emissão pode depender do domínio e de seus requisitos de consistência.

A arquitetura não determina um produto específico para essa função.

---

# 7. Epoch, Lease & Fencing Token

Esses conceitos são distintos.

## 7.1 Epoch

Representa uma geração monotônica de autoridade.

```text
Epoch 41
Epoch 42
Epoch 43
```

Uma geração posterior pode invalidar operações associadas a uma geração anterior, conforme a política do domínio.

---

## 7.2 Lease

Representa uma delegação temporária de autoridade.

```text
Domain Authority
       │
       ▼
    Lease
       │
       ▼
Worker
```

O lease pode possuir:

* escopo;
* holder;
* validade;
* permissões;
* recursos;
* epoch;
* condições de renovação;
* condições de revogação.

---

## 7.3 Fencing Token

O fencing token permite que o receptor de uma mutação diferencie uma autoridade válida de uma autoridade obsoleta.

Um token de fencing deve possuir semântica monotônica no escopo em que é usado.

O receptor deve ser capaz de rejeitar uma operação apresentada com uma geração inferior àquela já aceita.

Esse mecanismo é especialmente importante quando um worker pode permanecer ativo após perder sua autoridade lógica. A proteção ocorre no receptor da mutação, não apenas no cliente que possui o lease.

---

## 7.4 Distinção constitucional

```text
Epoch
  ≠
Lease
  ≠
Fencing Token
```

---

# 8. Stale Writer Protection

Um worker que perdeu sua autoridade não pode continuar produzindo mutações válidas simplesmente porque ainda possui memória, conexão ou credenciais antigas.

Fluxo:

```text
Worker A
Epoch 5
   │
   │ partition
   ▼
Domain Authority
Epoch 6
   │
   ▼
Worker B
Epoch 6
```

Se Worker A tentar escrever:

```text
Token Epoch 5
```

o receptor deve rejeitar a mutação quando o domínio exigir fencing.

```text
Presented Epoch < Accepted Epoch
        ↓
STALE AUTHORITY
        ↓
REJECT
```

Importante:

> **Stale Authority ≠ Invalid Historical Event**

Um evento antigo pode continuar sendo válido para auditoria ou reconstrução histórica, mesmo que sua autoridade não seja mais válida para uma nova mutação.

---

# 9. Ordering & Causality

A Yuki não exige ordenação global total para todo o sistema.

```text
Global Total Order
        ↓
não é requisito universal
```

Diferentes domínios podem utilizar diferentes mecanismos de ordenação.

---

## 9.1 HLC

Hybrid Logical Clocks podem combinar informação de tempo físico com componente lógico e são úteis para raciocínio temporal e causal em sistemas distribuídos.

Porém:

```text
HLC
≠
Global Truth
```

e:

```text
HLC
≠
Sequence Number
```

HLC não substitui mecanismos explícitos de identidade, causalidade ou ordenação por domínio.

---

## 9.2 Causal Metadata

Eventos podem carregar:

```text
event_id
operation_id
attempt_id
causation_id
correlation_id
domain_epoch
sequence_number
hlc_timestamp
```

Esses campos possuem funções diferentes.

### `event_id`

Identifica o evento.

### `operation_id`

Identifica a operação lógica.

### `attempt_id`

Identifica uma tentativa concreta de execução.

### `causation_id`

Identifica a causa imediata.

### `correlation_id`

Relaciona eventos pertencentes a uma missão/workflow/contexto.

### `domain_epoch`

Identifica a geração de autoridade relevante.

### `sequence_number`

Permite ordenação dentro de um escopo quando necessário.

### `hlc_timestamp`

Fornece informação temporal/lógica.

---

# 10. No Global NTP Dependency

A Yuki não deve depender de sincronização física perfeita de relógios para garantir segurança ou identidade de autoridade.

Isso não significa que relógios físicos sejam inúteis.

Significa:

```text
Clock synchronization
≠
Security authority
```

e:

```text
Clock time
≠
Causal truth
```

---

# 11. Durable Persistence Model

A Yuki diferencia:

```text
Event Store
State Store
Snapshot
Audit Log
Workflow State
Memory
Knowledge
```

Esses componentes não são equivalentes.

---

## 11.1 Event Store

Pode armazenar eventos duráveis e ordenáveis necessários para reconstrução, sincronização ou histórico.

---

## 11.2 State Store

Armazena estado materializado para acesso eficiente.

Pode ser reconstruível em alguns domínios, mas não deve ser constitucionalmente considerado sempre descartável.

---

## 11.3 Snapshot

Representa uma fotografia persistida de determinado estado em um ponto temporal/versional.

Pode reduzir custo de reconstrução e sincronização.

---

## 11.4 Audit Log

Registra eventos relevantes para:

* segurança;
* responsabilidade;
* investigação;
* conformidade;
* operações críticas.

Pode possuir mecanismos próprios de integridade e retenção.

---

## 11.5 Event Sourcing

Event Sourcing é uma possível estratégia arquitetural para determinados domínios, mas **não é obrigatório para toda a Yuki**.

Da mesma forma:

```text
Event Store
≠
Event Sourcing obrigatório
```

Isso preserva a decisão do ADR-011 de que durabilidade não implica obrigatoriamente Event Sourcing.

---

# 12. Persistence Principle

A decisão constitucional é:

> Estados que exigem durabilidade devem possuir uma representação persistente, recuperável e adequada ao domínio.

A arquitetura pode utilizar diferentes combinações:

```text
Event Store
+
State Store
+
Snapshots
+
Audit Log
```

ou outras arquiteturas equivalentes.

A escolha concreta permanece aberta por domínio.

---

# 13. Event Sync ≠ State Sync

Sincronizar eventos e sincronizar estado são operações diferentes.

```text
EVENT SYNCHRONIZATION
    ↓
history / causality / missing events

STATE SYNCHRONIZATION
    ↓
materialized representation
```

Um domínio pode utilizar:

```text
Events
→ Replay
→ State
```

ou:

```text
Snapshot
→ Delta
→ State
```

ou uma combinação.

---

# 14. Federation Synchronization Protocol

O mecanismo arquitetural de reconexão é denominado:

> **Federation Synchronization Protocol**

O termo é deliberadamente abstrato.

Ele não determina NATS, gRPC, HTTP, QUIC ou qualquer outra tecnologia.

---

## 14.1 Synchronization Flow

```text
Connection Restored
       ↓
Identity / Trust Validation
       ↓
Authority State Evaluation
       ↓
Gap Detection
       ↓
┌───────────────┬─────────────────┐
│               │                 │
Delta           Snapshot          No Gap
│               │                 │
└───────────────┴─────────────────┘
                ↓
Integrity Validation
                ↓
State Materialization
                ↓
Conflict Detection
                ↓
Conflict Policy
                ↓
Reconciliation
```

---

# 15. Gap Recovery

Quando um nó identifica:

```text
received = 105
expected = 103
```

ele não deve simplesmente assumir que 103 e 104 não existem.

O fluxo é:

```text
Gap detected
     ↓
Request missing range
     ↓
Validate received events
     ↓
Apply
     ↓
Continue materialization
```

Se o gap não puder ser recuperado eficientemente:

```text
Snapshot
+
Delta
```

pode ser utilizado.

---

# 16. Snapshot + Delta Synchronization

Para grandes períodos offline:

```text
Snapshot(version=N)
+
Events(N+1...latest)
```

pode ser mais eficiente que replayar todo o histórico.

O snapshot deve possuir informações suficientes para validar:

* origem;
* domínio;
* versão;
* integridade;
* contexto temporal;
* autoridade relevante;
* compatibilidade.

Assinaturas ou mecanismos equivalentes podem ser utilizados quando apropriados ao domínio.

---

# 17. Replica ≠ Backup ≠ Authority

Esses conceitos são constitucionalmente distintos.

### Replica

Cópia operacional de dados para disponibilidade, leitura, processamento ou sincronização.

### Backup

Cópia destinada principalmente à recuperação após perda/corrupção/desastre.

### Authority

Direito ativo de aceitar ou emitir determinadas mutações dentro de um escopo.

Portanto:

```text
Replica
≠
Backup
≠
Authority
```

E:

```text
Storage Location
≠
Authority
```

---

# 18. Worker ≠ Replica ≠ Coordinator

Também são conceitos distintos.

```text
Worker
→ executa trabalho

Replica
→ mantém cópia

Coordinator
→ coordena processos

Authority
→ possui direito de decisão/mutação
```

Um mesmo componente pode exercer mais de uma função, mas as funções não devem ser confundidas conceitualmente.

---

# 19. Offline Operation

A Yuki deve continuar funcionando parcialmente durante partições quando isso for seguro e autorizado.

A possibilidade de execução offline depende de:

* risco;
* autoridade;
* credencial;
* dependências;
* segurança;
* política;
* frescor necessário;
* impacto;
* capacidade local.

---

## 19.1 Offline Classes

A Yuki pode utilizar uma classificação conceitual:

```text
SAFE_OFFLINE
CONDITIONALLY_SAFE
ONLINE_REQUIRED
FORBIDDEN_OFFLINE
```

Essas categorias não constituem uma regra universal para toda capacidade.

Uma mesma classe de ação pode possuir políticas diferentes em domínios diferentes.

---

# 20. Offline Authorization

Uma operação offline somente pode continuar se possuir autorização válida para aquele contexto.

```text
Offline
+
Valid Authority
+
Valid Credential
+
Valid Policy
+
Required Local Safety
+
Required Freshness
=
Potentially Authorized
```

Offline não significa automaticamente:

```text
authorized
```

---

# 21. Credential Expiration

Credenciais e tokens destinados a operação offline devem possuir políticas explícitas de validade.

A arquitetura não estabelece um TTL universal, como 15 minutos.

O TTL deve depender de:

* risco;
* domínio;
* credencial;
* operação;
* impacto;
* capacidade de revogação;
* requisitos de segurança.

---

# 22. Offline Revocation

Uma limitação fundamental de sistemas offline é que um nó isolado não consegue necessariamente conhecer uma revogação que ocorreu depois de sua desconexão.

Portanto, a arquitetura deve reduzir essa janela por meio de:

* TTL;
* epochs;
* leases;
* escopo limitado;
* credenciais temporárias;
* políticas locais;
* revalidação ao reconectar;
* mecanismos de fencing;
* mecanismos de emergência.

A revogação remota não deve ser assumida como instantânea em um nó sem conectividade.

---

# 23. Safety During Partition

A segurança física permanece independente da conectividade.

De acordo com ADR-010:

```text
Network Partition
      ↓
Safety Controller
      ↓
continua operando localmente
```

Interlocks e funções de segurança críticas não podem depender da disponibilidade do Home, Cloud ou Yuki Core.

A política exata de comportamento offline pertence ao sistema físico e à sua análise de risco.

---

# 24. Conflict Detection

Conflitos podem surgir quando dois nós produzem mudanças concorrentes sobre o mesmo domínio ou recurso.

O fluxo é:

```text
Incoming State/Event
        ↓
Conflict Detection
        ↓
Conflict Classification
        ↓
Domain Conflict Policy
        ↓
Resolution
```

---

# 25. Conflict Resolution Is Domain-Specific

Não existe uma regra universal:

```text
highest HLC wins
```

nem:

```text
highest Fencing Token wins
```

para toda mutação semântica.

Fencing protege autoridade.

HLC fornece informação temporal/lógica.

A resolução semântica pode utilizar:

* authoritative owner;
* merge;
* CRDT-like semantics;
* version precedence;
* operation ordering;
* rejection;
* manual review;
* reconciliation;
* domain-specific rules.

CRDTs podem oferecer convergência em estruturas apropriadas, mas não são uma solução universal para todos os conflitos da Yuki.

---

# 26. Stale ≠ Invalid

Um evento pode ser:

```text
old
```

e ainda assim:

```text
valid historical evidence
```

Enquanto uma autoridade pode estar:

```text
stale
```

e portanto:

```text
unable to mutate
```

Essa distinção é obrigatória para:

* auditoria;
* replay;
* investigação;
* reconstrução;
* reconciliação.

---

# 27. Verification Boundary

A sincronização não substitui o Verification Engine.

Arquitetura:

```text
External System
      ↓
Evidence
      ↓
Verification Engine
      ↓
Validated Observation
      ↓
Observed State
      ↓
Reconciliation
```

A Yuki nunca deve assumir que:

```text
event received
=
world truth
```

---

# 28. Conflict ≠ Reconciliation

Conflito distribuído e drift semântico são problemas diferentes.

### Conflict

Duas representações ou operações concorrentes precisam ser avaliadas.

### Reconciliation

O estado observado precisa ser comparado ao Desired State.

```text
Conflict Resolution
        ≠
Reconciliation
```

O ADR-012 continua sendo responsável pela reconciliação.

---

# 29. Integration with ADR-009

ADR-009 define:

```text
Execution
Effect
Evidence
Verification
Observed Effect
```

ADR-013 adiciona a dimensão distribuída.

Portanto:

```text
Distributed Operation
        ↓
Evidence
        ↓
Verification
        ↓
Observed Effect
        ↓
Federation
        ↓
Reconciliation
```

Uma operação `UNKNOWN` não pode ser transformada silenciosamente em sucesso apenas porque outro nó recebeu um evento relacionado.

A evidência precisa ser validada e vinculada à operação.

---

# 30. Integration with ADR-011

ADR-011 define Durable Workflow.

ADR-013 adiciona:

* distribuição;
* autoridade;
* partições;
* sincronização;
* fencing;
* recuperação entre nós.

Um Workflow recuperado após uma partição deve reconstruir seu estado durável e verificar operações pendentes antes de continuar.

```text
Recovery
   ↓
Reconstruct Workflow State
   ↓
Inspect Outstanding Operations
   ↓
Verify / Reconcile
   ↓
Resume / Pause / Compensate / Escalate
```

---

# 31. Integration with ADR-012

ADR-012 define Reconciliation.

ADR-013 fornece a infraestrutura distribuída para que:

```text
Desired State
Observed State
```

possam ser sincronizados entre domínios e nós.

Reconciliation não recebe autoridade automaticamente por estar executando em um determinado nó.

---

# 32. Security Model

A arquitetura deve considerar:

* compromised node;
* stale worker;
* replay;
* forged events;
* duplicated events;
* unauthorized authority handoff;
* revoked credentials;
* partition;
* delayed packets;
* state poisoning;
* malicious synchronization;
* snapshot tampering.

---

# 33. Event Authenticity

Eventos relevantes podem exigir:

* autenticação;
* integridade;
* proveniência;
* anti-replay;
* vinculação a domínio;
* vinculação a operação;
* frescor.

A implementação pode utilizar:

* assinaturas digitais;
* MACs;
* workload identity;
* secure channels;
* hardware-backed identity;
* outros mecanismos apropriados.

Nenhuma tecnologia criptográfica específica é constitucionalmente obrigatória neste ADR.

---

# 34. Event ≠ Instruction

Eventos sincronizados entre nós são dados.

Eles não devem ser tratados automaticamente como instruções executáveis.

```text
Received Event
     ↓
Validate
     ↓
Interpret
     ↓
Policy
     ↓
Authorization
     ↓
Possible Action
```

Isso preserva o princípio:

> Data ≠ Instruction.

---

# 35. Prompt Injection

Dados sincronizados entre nós, sistemas externos ou integrações podem conter conteúdo malicioso.

A proteção deve permanecer em camadas:

```text
External Data
     ↓
Typed Representation
     ↓
Validation
     ↓
Evidence / Provenance
     ↓
Policy Evaluation
     ↓
Reasoning
     ↓
Authorization
```

Nenhum evento externo pode conceder autoridade simplesmente por conter texto que instrua a Yuki.

---

# 36. Disaster Recovery

Restaurar um backup não concede automaticamente autoridade.

```text
Backup
  ↓
Restore
  ↓
Validation
  ↓
Authority Handoff
  ↓
New Epoch
  ↓
New Authority
```

A promoção de um novo Domain Owner deve invalidar ou tornar inutilizável a autoridade anterior quando exigido pelo domínio.

---

# 37. Authority Handoff

Uma transferência de autoridade deve ser tratada explicitamente.

Possíveis etapas:

```text
Current Authority
      ↓
Prepare Handoff
      ↓
Establish New Authority
      ↓
Advance Epoch
      ↓
Fence Previous Authority
      ↓
Transfer State
      ↓
Verify
      ↓
Activate New Authority
```

O mecanismo concreto pode variar conforme o domínio.

---

# 38. Scenario A — Home Isolated

### Situação

```text
Mobile ──► Cloud ──X── Home
```

Home possui autoridade sobre:

```text
thermostat.living_room
```

Mobile solicita:

```text
22°C → 25°C
```

### Comportamento

```text
Mobile
 ↓
Desired State Update Request
 ↓
Cloud
 ↓
Pending / Deferred
 ↓
Home
```

A Cloud não pode executar diretamente a mutação física.

Quando Home reconectar:

```text
Home
 ↓
Validate request
 ↓
Validate authorization
 ↓
Become aware of latest intent
 ↓
Accept/update Desired State
 ↓
Reconciliation
 ↓
Physical Gateway
 ↓
Thermostat
 ↓
Observation
 ↓
Verification
```

Assim:

```text
Cloud Replica
≠
Home Domain Authority
```

---

# 39. Scenario B — Ambiguous External Payment

### Situação

```text
Home
 ↓
PayBill
 ↓
External Bank
 ↓
Network Failure
 ↓
UNKNOWN
```

A operação não pode ser automaticamente classificada como:

```text
FAILED
```

nem:

```text
SUCCESS
```

---

## 39.1 Recovery

Primeiro:

```text
UNKNOWN
 ↓
Determine operation semantics
 ↓
Determine idempotency support
 ↓
Determine verification options
```

Se o sistema externo possuir idempotência confiável:

```text
Retry with same contractual idempotency identity
```

pode ser permitido.

Se não possuir:

```text
No blind retry
 ↓
Inquiry / Verification
 ↓
Resolve outcome
```

APIs idempotentes podem utilizar tokens para reconhecer uma repetição da mesma operação, mas a semântica depende do serviço que implementa a idempotência. Exatamente-once não deve ser presumido apenas porque a Yuki gerou uma chave.

---

## 39.2 Yuki Idempotency Model

A Yuki diferencia:

```text
operation_id
attempt_id
causation_id
correlation_id
idempotency_key
```

A arquitetura não impõe uma fórmula universal para `idempotency_key`.

A chave deve respeitar o contrato da operação/integration.

---

## 39.3 Corrected Scenario

```text
Home
 ↓
operation_id = OP-99
 ↓
Attempt 1
 ↓
External Bank
 ↓
UNKNOWN
```

Cloud não deve simplesmente inventar uma nova operação.

Ela deve receber:

```text
OP-99
```

e consultar a política da operação.

Se a integração oferecer idempotência:

```text
OP-99
+
External Idempotency Key
↓
Safe Retry Candidate
```

Se não:

```text
OP-99
↓
Verification Inquiry
↓
Known Success?
Known Failure?
Still Unknown?
```

O resultado deve ser decidido pelo Verification Engine e pelas políticas da operação.

---

# 40. Idempotency ≠ Fencing

Esses mecanismos resolvem problemas diferentes.

### Fencing

Protege contra:

```text
stale authority
```

### Idempotency

Protege contra:

```text
duplicate operation
```

Portanto:

```text
Fencing
+
Idempotency
+
Verification
```

podem trabalhar juntos, mas um não substitui o outro.

---

# 41. Offline Federation State Machine

Um nó pode transitar entre:

```text
CONNECTED
    ↓
PARTITION_DETECTED
    ↓
OFFLINE_OPERATION
    ↓
WAITING_RECONNECT
    ↓
RECONNECTING
    ↓
AUTHORITY_VALIDATION
    ↓
SYNCING
    ↓
CONFLICT_CHECK
    ↓
RECONCILIATION
    ↓
CONNECTED
```

Falhas podem resultar em:

```text
QUARANTINED
DEGRADED
MANUAL_REVIEW
```

---

# 42. Synchronization Invariants

Os seguintes invariantes são obrigatórios:

1. Nenhum nó possui autoridade global permanente.
2. Storage location não concede autoridade.
3. Replica não é autoridade automaticamente.
4. Backup não é autoridade automaticamente.
5. Stale authority não pode produzir mutações protegidas.
6. Epoch, Lease e Fencing Token são conceitos distintos.
7. Fencing deve ser verificável pelo receptor da mutação quando necessário.
8. HLC não substitui identidade causal ou sequência de domínio.
9. Event Sync ≠ State Sync.
10. Event Store ≠ Event Sourcing obrigatório.
11. Stale Event ≠ Invalid Event.
12. Conflict Resolution é dependente do domínio.
13. Reconciliation não concede autorização.
14. Offline não concede autorização.
15. Revogação remota não deve ser presumida instantânea durante partition.
16. Safety Controller permanece independente da conectividade quando requerido.
17. UNKNOWN não pode silenciosamente virar SUCCESS.
18. Idempotency Key da Yuki não garante idempotência de um sistema externo.
19. Workflow State não é Authorization.
20. Model output não é autoridade.
21. Dados sincronizados não são automaticamente instruções.
22. Recuperação de backup não concede autoridade automaticamente.

---

# 43. Closed Architectural Decisions

## D013-1 — Resource-Bound Authority

A autoridade da Yuki é vinculada a recursos/domínios e não existe um nó soberano global.

## D013-2 — Authority Epoch & Fencing

Mutações sujeitas a autoridade delegada devem possuir mecanismo adequado de geração, lease e/ou fencing para impedir stale writers.

## D013-3 — Separated Temporal/Causal Metadata

HLC, sequence numbers, causation IDs, operation IDs e authority epochs possuem semânticas distintas.

## D013-4 — Durable Persistence Independence

A Yuki deve possuir persistência durável apropriada para estados que exigem recuperação. Event Store, State Store, Snapshot e Audit Log são conceitos distintos.

Event Sourcing não é obrigatório globalmente.

## D013-5 — Contextual Offline Execution

Operações offline são avaliadas conforme risco, autoridade, credencial, dependências, política e requisitos de segurança/frescor.

## D013-6 — Federation Synchronization Protocol

A federação deve suportar gap detection, delta synchronization, snapshot transfer, integrity validation, conflict handling e reconciliation.

## D013-7 — State Ownership ≠ Storage Location

Localização de armazenamento não concede autoridade.

## D013-8 — Replica ≠ Backup ≠ Authority

Os três conceitos possuem responsabilidades distintas.

## D013-9 — Event Sync ≠ State Sync

Histórico de eventos e estado materializado são sincronizações diferentes.

## D013-10 — Stale ≠ Invalid

Dados históricos antigos podem permanecer válidos mesmo quando a autoridade correspondente estiver obsoleta.

## D013-11 — External Idempotency Is Contractual

Retry seguro depende da semântica da operação e do mecanismo de idempotência/verificação disponível no destino.

## D013-12 — Conflict Resolution Is Domain-Specific

Não existe uma regra universal de resolução baseada apenas em HLC ou Fencing Token.

## D013-13 — Authority Issuance Must Be Explicit

A emissão de autoridade deve possuir um mecanismo definido por domínio e não pode ser realizada arbitrariamente por qualquer nó.

## D013-14 — Safety Remains Independent

Partições de rede não podem desabilitar mecanismos de segurança física definidos pelo domínio.

## D013-15 — Recovery Does Not Imply Authority

Restaurar estado, snapshot ou backup não concede automaticamente autoridade.

---

# 44. Open Decisions

Permanecem abertas:

### O013-1 — Federation Transport

Possíveis tecnologias:

* Protobuf;
* gRPC;
* NATS;
* QUIC;
* HTTP;
* outros mecanismos futuros.

### O013-2 — Event Retention

Definir:

* retenção;
* arquivamento;
* compactação;
* pruning;
* armazenamento frio.

### O013-3 — Authority Issuer Implementation

Definir posteriormente como cada domínio implementará:

* epoch generation;
* leases;
* fencing;
* authority handoff.

### O013-4 — Consensus Requirements

Alguns domínios podem exigir mecanismos de consenso ou autoridade forte; outros não.

Não existe requisito global de consenso para toda a Yuki.

### O013-5 — Snapshot Format

Formato e protocolo de snapshots permanecem abertos.

### O013-6 — Conflict Policy Registry

Definir posteriormente como políticas de resolução de conflitos serão registradas por domínio.

---

# 45. Future ADRs

Possíveis próximos ADRs:

* **ADR-014 — Personal Goal Reconciliation & Human Agency Boundaries**
* **ADR-015 — Cross-Domain Privacy, Data Minimization & Encryption at Rest**
* **ADR-016 — Physical Device Identity & Attestation**
* **ADR-017 — Federation Authority Handoff Protocol**
* **ADR-018 — Yuki Event & State Schema Governance**
* **ADR-019 — Distributed Conflict Policy Framework**

---

# 46. Relationship with Previous ADRs

```text
ADR-006
Infrastructure Resource Model
        ↓
ADR-007
Integration Manifest
        ↓
ADR-008
Credential Isolation
        ↓
ADR-009
Execution / Effect / Verification
        ↓
ADR-010
Physical Safety
        ↓
ADR-011
Durable Workflow
        ↓
ADR-012
Reconciliation
        ↓
ADR-013
Federation / Synchronization / Offline Authority
```

ADR-013 não substitui nenhum desses documentos.

Ele fornece a camada distribuída necessária para que esses sistemas funcionem em múltiplos nós e durante partições.

---

# 47. Consolidated Architecture

```text
                         YUKI
                           │
                    ┌──────┴──────┐
                    │             │
               CONTROL PLANE   AUTHORITY
                    │             │
                    │       Domain Authorities
                    │             │
                    ▼             ▼
              RECONCILIATION   EPOCH / LEASE
                    │             │
                    └──────┬──────┘
                           │
                    FEDERATION LAYER
                           │
          ┌────────────────┼────────────────┐
          │                │                │
          ▼                ▼                ▼
        EDGE             HOME             CLOUD
          │                │                │
      Local State      Domain State     Replicas
      Local Events     Event/State      Workers
      Local Tasks      Store            Overflow
          │                │                │
          └────────────────┼────────────────┘
                           │
                    SYNCHRONIZATION
                           │
             ┌─────────────┼─────────────┐
             ▼             ▼             ▼
          Events        Snapshots       State
             │             │             │
             └─────────────┼─────────────┘
                           ▼
                  CONFLICT DETECTION
                           │
                           ▼
                   DOMAIN POLICY
                           │
                           ▼
                    RECONCILIATION
                           │
                           ▼
                     WORKFLOW
                           │
                           ▼
                      EXECUTION
                           │
                           ▼
                     EXTERNAL WORLD
                           │
                           ▼
                       EVIDENCE
                           │
                           ▼
                    VERIFICATION
```

Security Controller, Credential Broker, Resource Manager e Safety Controller permanecem independentes e transversais.

---

# 48. Final Architectural Principle

A Yuki deve ser capaz de continuar operando durante falhas distribuídas sem transformar disponibilidade em autoridade.

Portanto:

> **Offline não significa soberano.**

> **Replica não significa autoridade.**

> **Storage não significa ownership.**

> **HLC não significa verdade causal absoluta.**

> **Lease não significa autoridade permanente.**

> **Fencing não significa idempotência.**

> **Idempotência não significa verificação.**

> **Evento não significa instrução.**

> **Restaurar estado não significa possuir autoridade.**

> **Reconciliar não significa autorizar.**

> **Cloud não significa soberania.**

> **Home não significa soberania.**

A federação da Yuki deve preservar essas separações mesmo sob:

* partições;
* atrasos;
* duplicações;
* falhas;
* nós comprometidos;
* recuperação;
* mudanças de autoridade;
* operações offline;
* sincronização parcial.

---

# 49. Status

**ADR-013 — ACCEPTED**

**Version:** 1.0

As decisões constitucionais deste ADR estão fechadas.

Detalhes de implementação permanecem sujeitos a ADRs posteriores.

Este documento deve ser versionado no Git e tratado como parte da memória arquitetural oficial da Yuki.
