# ADR-009 — Yuki External Action Verification

**Versão:** 1.0
**Status:** ACCEPTED
**Domínio:** 15 — Integrations
**Data:** 2026-09-19

---

# 1. Objetivo

Definir a arquitetura pela qual a Yuki determina se uma ação externa produziu o efeito esperado.

O princípio central é:

> **A execução de uma ação não implica que seu efeito externo tenha ocorrido, e uma resposta de uma interface não constitui automaticamente prova suficiente do efeito.**

A Yuki deve distinguir explicitamente:

```text
Authorization
Execution
Effect
Evidence
Verification
Reconciliation
Compensation
```

Esses conceitos possuem responsabilidades diferentes e não devem ser fundidos em um único mecanismo.

---

# 2. Princípio Fundamental

A Yuki não deve confundir:

```text
REQUEST ACCEPTED
≠
REQUEST EXECUTED
≠
EFFECT PRODUCED
≠
EFFECT OBSERVED
≠
EFFECT VERIFIED
```

Uma resposta como:

```text
HTTP 200
HTTP 201
HTTP 202
success = true
```

pode ser evidência de determinada etapa da operação, mas não possui significado universal.

O significado da resposta depende do contrato da operação e do sistema externo.

---

# 3. Execution ≠ Effect ≠ Verification

A arquitetura estabelece:

```text
Execution ≠ Effect
Effect ≠ Verification
Verification ≠ Authorization
Verification ≠ Reconciliation
Reconciliation ≠ Compensation
```

## 3.1 Authorization

Determina se a ação pode ser executada.

Responsabilidade principal:

```text
Security Controller
Policy System
Permission System
```

---

## 3.2 Execution

Representa o processo de enviar ou executar uma operação.

Exemplo:

```text
Capability
↓
Integration Gateway
↓
Connector / Adapter
↓
External System
```

---

## 3.3 Effect

Representa a mudança que ocorreu, ou aparentemente ocorreu, no sistema externo ou no mundo físico.

A Yuki não assume acesso direto à "verdade absoluta" do mundo.

Ela trabalha com estados observados e evidências.

---

## 3.4 Evidence

É uma informação obtida de uma fonte externa ou interna que pode ser usada para avaliar o resultado da operação.

Exemplos:

* resposta de operação;
* consulta de estado;
* evento;
* webhook;
* observação de sensor;
* estado de recurso;
* confirmação de settlement;
* observação independente.

---

## 3.5 Verification

Processo de avaliar se as evidências disponíveis satisfazem os requisitos de confirmação da operação.

Verification não concede autorização.

Verification não executa automaticamente compensações.

Verification não substitui o Security Controller.

---

## 3.6 Reconciliation

Compara:

```text
Desired State
vs
Observed State
```

e procura divergências.

Reconciliation pode existir independentemente de uma operação recente.

---

## 3.7 Compensation

Representa ações corretivas quando uma operação composta não pode simplesmente ser revertida.

Saga e compensações pertencem ao Workflow/Mission layer, não ao Integration Gateway.

---

# 4. Modelo de Estados

A Yuki utiliza dimensões de estado separadas.

Não deve existir um único enum universal que tente representar todo o ciclo de uma operação.

O modelo conceitual possui:

```text
Operation State
Observed Effect State
Verification Result
```

---

# 5. Operation State

Representa o ciclo de vida da operação do ponto de vista da execução.

Estados possíveis incluem:

```text
CREATED
DISPATCHED
ACKNOWLEDGED
PROCESSING
COMPLETED
FAILED
REJECTED
CANCELLED
EXPIRED
TERMINATED
```

A implementação concreta pode possuir estados adicionais.

O conjunto não é constitucionalmente fechado.

---

# 6. Observed Effect State

Representa o estado observado do efeito, não uma afirmação metafísica sobre a realidade.

Estados conceituais:

```text
NOT_OBSERVED
PENDING
OBSERVED_MUTATION
OBSERVED_NO_MUTATION
OBSERVED_PARTIAL_MUTATION
UNKNOWN
```

Importante:

```text
OBSERVED_MUTATION
```

não significa automaticamente:

```text
VERIFIED
```

A observação ainda precisa ser avaliada pelo mecanismo de verificação.

---

# 7. Verification Result

O resultado de verificação deve permanecer separado do estado de execução.

Estados conceituais:

```text
NOT_EVALUATED
IN_PROGRESS
CONFIRMED
FAILED
INCONCLUSIVE
```

`INCONCLUSIVE` é importante porque nem toda operação desconhecida pode ser classificada imediatamente como sucesso ou falha.

---

# 8. UNKNOWN e INCONCLUSIVE

A arquitetura trata incerteza como estado de primeira classe.

Um timeout, perda de resposta ou falha intermediária não deve ser automaticamente interpretado como:

```text
SUCCESS
```

nem como:

```text
FAILED
```

Quando não existe evidência suficiente:

```text
Observed Effect = UNKNOWN
Verification = INCONCLUSIVE
```

A Yuki deve então:

1. verificar se existe estratégia de verificação;
2. consultar evidências disponíveis;
3. avaliar idempotência;
4. determinar se retry é seguro;
5. aguardar confirmação quando apropriado;
6. escalar quando necessário.

---

# 9. Operation Identity

A Yuki deve distinguir identificadores com semânticas diferentes.

## 9.1 correlation_id

Identifica um contexto maior, como:

```text
Mission
Workflow
Goal
Session
```

---

## 9.2 causation_id

Identifica a causa ou etapa que originou uma operação.

---

## 9.3 operation_id

Identifica a operação lógica.

Uma operação pode possuir múltiplas tentativas.

---

## 9.4 attempt_id

Identifica uma tentativa concreta de execução/dispatch.

---

## 9.5 idempotency_key

Identifica a semântica de deduplicação exigida pelo destino.

A arquitetura não assume que toda integração utiliza o mesmo mecanismo.

Portanto:

```text
operation_id
≠
attempt_id
≠
idempotency_key
```

---

# 10. Idempotência

Operações mutáveis devem utilizar mecanismos de idempotência quando:

* o destino oferece suporte;
* o risco da operação exige;
* retries são possíveis;
* a semântica da operação permite.

A Yuki não impõe uma fórmula universal para geração de idempotency keys.

Uma integração pode utilizar:

* chave fornecida pelo workflow;
* chave determinística;
* chave gerada pelo provider;
* combinação de identificadores;
* mecanismo nativo do sistema externo.

A semântica deve estar definida no contrato da operação.

---

# 11. Retry

Retry não é uma solução universal.

A decisão depende de:

```text
Operation Semantics
+
Idempotency
+
Current State
+
Risk
+
Provider Contract
+
Verification Evidence
+
Retry Policy
```

### Reads

Normalmente podem ser repetidos quando respeitados:

* rate limits;
* timeout;
* custo;
* segurança;
* freshness.

### Idempotent writes

Podem ser repetidos de acordo com o contrato da operação.

### Non-idempotent writes

Não devem ser repetidos automaticamente quando o resultado anterior é desconhecido.

### Unknown result

A regra geral é:

```text
UNKNOWN
↓
VERIFY
↓
Can retry safely?
```

Não:

```text
UNKNOWN
↓
RETRY IMMEDIATELY
```

---

# 12. Retry Budget

A Yuki deve possuir mecanismos de:

* retry budget;
* backoff;
* jitter;
* rate limiting;
* retry limits;
* circuit breaking;
* storm prevention.

Os valores concretos não fazem parte deste ADR.

Eles devem ser definidos por:

* integração;
* workload;
* domínio;
* risco;
* recursos;
* provider;
* ambiente.

---

# 13. Verification Evidence

Uma `Verification Evidence` é uma representação estruturada de informação usada para avaliar o efeito de uma operação.

Modelo conceitual:

```yaml
evidence_id:
operation_id:

source:
  identity:
  type:

provenance:
  collected_at:
  source_timestamp:

integrity:
  mechanism:

freshness:
  expires_at:

binding:
  operation:
  resource:

characteristics:
  independence:
  source_trust:
  coverage:

observation:
  state:
  attributes:
```

O formato final permanece aberto.

---

# 14. Evidence Characteristics

A Yuki não deve assumir uma escala absoluta de confiança.

Uma evidência deve ser avaliada por características como:

```text
Authenticity
Integrity
Freshness
Operation Binding
Resource Binding
Source Trust
Independence
Coverage
Consistency
```

Uma evidência física não é automaticamente "absoluta".

Um sensor pode falhar.

Uma API pode estar comprometida.

Um webhook pode estar autenticado e ainda representar um estado intermediário.

Portanto:

> **Evidence quality is contextual.**

---

# 15. Evidence ≠ Truth

A Yuki não possui uma garantia universal de acesso à realidade externa.

Verification significa:

> As evidências disponíveis satisfazem os requisitos de confirmação definidos para aquela operação.

Portanto:

```text
Evidence
↓
Verification Policy
↓
Verification Result
```

e não:

```text
Evidence
↓
Absolute Truth
```

---

# 16. Verification Strategies

A estratégia é determinada por:

```text
Operation Contract
+
Risk
+
Effect Semantics
+
Available Evidence
+
System Capabilities
+
Policy
```

Estratégias possíveis incluem:

### 16.1 Synchronous Response

A resposta da operação pode ser suficiente para determinadas operações de baixo impacto quando o contrato explicitamente estabelece essa semântica.

---

### 16.2 Status Query

Consultar o estado da operação.

```text
Operation
↓
Status Query
```

---

### 16.3 Read-After-Write

Consultar o recurso afetado depois da escrita.

```text
WRITE
↓
READ
↓
COMPARE
```

Não é obrigatório universalmente.

---

### 16.4 Async Polling

Adequado para operações longas.

```text
START
↓
PENDING
↓
POLL
↓
POLL
↓
COMPLETED
```

Deve utilizar backoff e limites apropriados.

---

### 16.5 Event-Based Verification

Eventos podem fornecer evidência útil.

Antes de serem aceitos como evidência, devem passar por:

* autenticação;
* integridade;
* freshness;
* replay protection;
* binding;
* validação estrutural;
* avaliação da política.

Um evento autenticado não é automaticamente confirmação do efeito.

---

### 16.6 Independent Observation

Uma segunda fonte pode observar o efeito.

Exemplos:

* ledger;
* recurso final;
* sensor;
* sistema secundário;
* estado observado por outro canal.

---

### 16.7 Physical Observation

Para sistemas físicos:

```text
Command
↓
Controller
↓
Actuator
↓
Physical World
↓
Sensor
↓
Observation
```

O sensor fornece evidência sobre o efeito físico.

---

# 17. Verification Policy

A Capability ou Integration pode declarar requisitos de verificação.

Exemplo conceitual:

```yaml
verification:
  required: true
  acceptable_evidence:
    - resource_state
    - operation_status
  freshness:
    max_age: ...
```

A declaração não concede autorização.

A estratégia final pode depender também de:

* Security Controller;
* risco;
* contexto;
* estado atual;
* disponibilidade;
* recursos.

O formato final da política permanece aberto.

---

# 18. Risk-Aware Verification

Quanto maior o impacto potencial, maiores podem ser os requisitos de:

* qualidade da evidência;
* independência;
* freshness;
* cobertura;
* confirmação;
* auditoria;
* intervenção humana.

Não existe uma regra universal:

```text
Risk 5 = sempre humano
```

A decisão deve ser contextual.

Human-in-the-loop é uma possível camada de controle, não uma consequência automática de qualquer classificação de risco.

---

# 19. Webhooks e Eventos

Eventos externos são tratados como:

```text
UNTRUSTED_EXTERNAL_DATA
```

até serem validados.

Fluxo:

```text
External Event
↓
Authentication
↓
Integrity Validation
↓
Anti-Replay
↓
Schema Validation
↓
Operation / Resource Binding
↓
Freshness Evaluation
↓
Evidence Evaluation
↓
Verification Policy
↓
Verification Result
```

Um webhook não deve diretamente definir:

```text
Effect = MUTATED
```

sem passar pela avaliação apropriada.

---

# 20. Prompt Injection

Dados externos nunca devem possuir autoridade sobre o sistema.

Portanto:

```text
External Data
≠
Instruction
≠
Authorization
```

A defesa deve privilegiar:

```text
Raw External Payload
↓
Parser
↓
Typed Representation
↓
Validation
↓
Evidence Model
↓
Verification Engine
```

Texto livre externo não deve ser interpretado pelo LLM como instrução de controle.

Sanitização de texto, quando usada, é defesa complementar e não uma fronteira de segurança suficiente.

---

# 21. Modelos de IA na Verificação

Modelos podem ajudar a:

* interpretar evidências complexas;
* detectar inconsistências;
* classificar situações;
* sugerir estratégias;
* analisar documentos;
* auxiliar investigação.

Entretanto:

```text
LLM Verification Opinion
≠
Verification Evidence
```

e:

```text
Model Output
≠
Verification Authority
```

O modelo não pode, sozinho, transformar uma operação em:

```text
CONFIRMED
```

para ações que exigem evidência formal.

Multi-model verification pode ser utilizada como mecanismo auxiliar em cenários complexos.

---

# 22. Verification Engine

A Yuki possui um conceito de `Verification Engine`.

Responsabilidades:

* selecionar/avaliar estratégias de verificação;
* coletar ou coordenar coleta de evidências;
* validar evidências;
* avaliar evidências;
* atualizar Verification Result;
* manter estado de operações pendentes;
* lidar com `UNKNOWN`;
* solicitar novas verificações;
* emitir resultados estruturados;
* fornecer informações para Mission/Workflow;
* produzir dados de auditoria.

O Verification Engine não:

* concede autorização;
* substitui Security Controller;
* executa workflows completos;
* possui autoridade irrestrita sobre integrações;
* concede novas permissões;
* decide compensações complexas sozinho.

---

# 23. Verification Engine ≠ Reconciliation Engine

Verification pergunta:

> O efeito desta operação foi suficientemente confirmado?

Reconciliation pergunta:

> O estado observado corresponde ao estado desejado?

Portanto:

```text
Verification
= Operation / Effect oriented

Reconciliation
= Desired State / Observed State oriented
```

Os mecanismos podem colaborar, mas permanecem conceitualmente distintos.

---

# 24. Reconciliation

A Yuki pode utilizar mecanismos de reconciliação para:

```text
Desired State
↓
Observed State
↓
Compare
↓
Drift
```

Reconciliation pode:

* detectar drift;
* atualizar estado;
* gerar uma nova tarefa;
* solicitar intervenção;
* iniciar uma estratégia de convergência.

A reconciliação não deve assumir automaticamente que a correção é autorizada.

---

# 25. Compensation

Compensação pertence ao Workflow/Mission layer.

Uma operação pode declarar:

```text
reversible
partially_reversible
irreversible
compensating_capability
```

quando aplicável.

Não existe obrigação universal de que toda operação de alto risco possua compensação, pois algumas ações são:

* irreversíveis;
* parcialmente reversíveis;
* compensáveis somente por outro processo;
* sem compensação possível.

---

# 26. Financial Actions

Operações financeiras devem distinguir:

```text
Order Created
Order Accepted
Order Executed
Settlement
Funds Received
```

Uma resposta de API não representa automaticamente todas essas etapas.

A estratégia de verificação pode utilizar:

* transaction status;
* settlement event;
* ledger;
* account state;
* provider confirmation;
* reconciliation;
* outra evidência apropriada.

A evidência necessária depende da operação.

---

# 27. Physical Actions

Para ações físicas, a arquitetura deve preservar:

```text
Yuki
↓
Robotics / Automation Controller
↓
Safety Layer
↓
Actuator
↓
Physical World
```

e:

```text
Physical World
↓
Sensors
↓
Controller / Observation Layer
↓
Yuki
```

A Yuki não deve substituir controladores de segurança física.

Middleware como ROS2 pode ser utilizado, mas não constitui requisito constitucional da arquitetura.

---

# 28. Failure Handling

A Yuki deve diferenciar pelo menos:

### Timeout antes de evidência suficiente de dispatch

Se não houver garantia de que a operação chegou ao destino:

```text
Effect = UNKNOWN
```

quando a incerteza persistir.

---

### Timeout depois de possível dispatch

```text
Operation = TERMINATED / UNKNOWN
Effect = UNKNOWN
Verification = INCONCLUSIVE
```

A Yuki deve verificar antes de repetir uma escrita insegura.

---

### HTTP 500

O resultado depende da semântica do provider.

Não deve ser interpretado automaticamente como:

```text
NO_MUTATION
```

nem:

```text
MUTATED
```

---

### HTTP 200

Também depende do contrato.

Pode significar:

```text
request accepted
```

ou:

```text
effect completed
```

ou outra coisa.

---

### Lost Response

O efeito pode ter acontecido.

Portanto:

```text
Lost Response
→ UNKNOWN
→ Verification
```

---

### Duplicate Event

Eventos devem possuir mecanismos de deduplicação quando aplicável.

---

### Stale Event

Evidências antigas não devem confirmar automaticamente o estado atual.

---

### Concurrent Mutation

A Yuki deve reconhecer possíveis conflitos entre:

```text
Expected State
Observed State
Concurrent Actor
```

e pode necessitar de reconciliação.

---

# 29. Retry Decision Model

Modelo conceitual:

```text
UNKNOWN
  │
  ▼
Can verify?
  │
  ├── YES → VERIFY
  │
  └── NO
       │
       ▼
Is operation safely retryable?
       │
       ├── YES → RETRY UNDER POLICY
       │
       └── NO → WAIT / ESCALATE / HUMAN
```

O resultado depende do contrato da operação.

---

# 30. Audit

A verificação deve produzir informações auditáveis.

Exemplos:

```text
trace_id
correlation_id
causation_id
operation_id
attempt_id
idempotency_key
capability_id
integration_id
execution_state
observed_effect_state
verification_result
verification_strategy
evidence_reference
timestamp
policy_reference
```

Dados sensíveis devem respeitar as regras do ADR-008.

O sistema de auditoria deve evitar armazenar segredos desnecessários.

---

# 31. Memory Integration

Verification Result não deve determinar sozinho a política de memória.

A Memory System deve considerar:

```text
Verification Result
+
Evidence
+
Source
+
Confidence / Epistemic Status
+
Importance
+
Retention Policy
```

Exemplos:

```text
CONFIRMED
→ pode virar fato confirmado

FAILED
→ pode virar fato de falha

INCONCLUSIVE
→ pode ser armazenado como evento/estado inconclusivo

Temporary Evidence
→ pode possuir retenção curta
```

Portanto:

```text
Unverified
≠
Never Store
```

e:

```text
Unverified
≠
Confirmed Fact
```

---

# 32. Evolution Integration

Falhas recorrentes de verificação podem alimentar o Evolution Manager.

Exemplo:

```text
Repeated Verification Failure
↓
Pattern Detection
↓
Research
↓
Proposal
↓
Prototype
↓
Test
↓
Security Review
↓
Approval
↓
Deployment
```

A Evolution Manager não pode automaticamente:

* remover requisitos críticos;
* reduzir o nível de verificação;
* ampliar permissões;
* transformar `UNKNOWN` em `SUCCESS`.

Mudanças de segurança permanecem sujeitas às regras de Evolution e Security.

---

# 33. Architecture

Arquitetura conceitual:

```text
                         YUKI MISSION
                              │
                              ▼
                       AGENT / CAPABILITY
                              │
                              ▼
                     SECURITY / POLICY
                              │
                              ▼
                    INTEGRATION GATEWAY
                              │
                              ▼
                      EXTERNAL SYSTEM
                              │
             ┌────────────────┼────────────────┐
             ▼                ▼                ▼
         Response           Event          Observation
             │                │                │
             └────────────────┼────────────────┘
                              ▼
                    EVIDENCE PROCESSING
                              │
                              ▼
                   VERIFICATION ENGINE
                              │
                 ┌────────────┼────────────┐
                 ▼            ▼            ▼
             CONFIRMED      FAILED    INCONCLUSIVE
                 │            │            │
                 └────────────┼────────────┘
                              ▼
                MISSION / AUDIT / MEMORY
```

Reconciliation permanece como mecanismo relacionado:

```text
Desired State
      │
      ▼
Reconciliation Engine
      │
      ▼
Observed State
```

---

# 34. Separation of Responsibilities

## Yuki Core

Planeja e raciocina.

Não declara sozinho que um efeito externo ocorreu.

---

## Agent System

Executa raciocínio e tarefas autorizadas.

Não possui autoridade independente para confirmar efeitos críticos.

---

## Security Controller

Determina autorização e política de segurança.

Não é Verification Engine.

---

## Integration Gateway

Medeia comunicação externa.

Não possui autoridade universal para declarar efeitos confirmados.

---

## Connector / Adapter

Implementa comunicação e mapeamento de integração.

Não deve possuir autoridade autônoma para alterar políticas globais de verificação.

---

## Verification Engine

Avalia evidências e determina o resultado de verificação conforme a política aplicável.

---

## Reconciliation Engine

Compara estado desejado e observado.

---

## Mission / Workflow System

Coordena operações compostas, retries autorizados, compensation e Saga.

---

## Memory System

Armazena informações de acordo com política epistemológica e de retenção.

---

# 35. Security Invariants

## INV-001

**Verification cannot grant authorization.**

Confirmar um efeito não concede novas permissões.

---

## INV-002

**UNKNOWN shall never silently become SUCCESS.**

Incerteza não pode ser convertida silenciosamente em sucesso.

---

## INV-003

**Model output cannot self-authorize verification.**

Um modelo não pode sozinho alterar o estado formal de verificação de uma operação.

---

## INV-004

**Evidence cannot create permission.**

Uma evidência não concede autorização.

---

## INV-005

**External data is not system authority.**

Dados externos não podem alterar diretamente políticas ou instruções do sistema.

---

## INV-006

**Critical verification requirements cannot be weakened automatically.**

O sistema não pode reduzir automaticamente requisitos críticos de verificação para melhorar latência, custo ou taxa de sucesso.

---

## INV-007

**Execution success does not imply effect confirmation.**

Uma execução aceita ou concluída não implica automaticamente efeito confirmado.

---

# 36. Closed Decisions

## D009-1 — Orthogonal State Model

A Yuki adotará dimensões separadas para:

```text
Operation State
Observed Effect State
Verification Result
```

---

## D009-2 — Independent Verification Engine

A verificação será representada por um componente lógico independente do:

* Agent System;
* Integration Gateway;
* Security Controller.

---

## D009-3 — Operation Identity Separation

A arquitetura distinguirá:

```text
correlation_id
causation_id
operation_id
attempt_id
idempotency_key
```

---

## D009-4 — Idempotency Is Contractual

A Yuki utilizará idempotência quando aplicável, mas não imporá uma fórmula universal para geração de chaves.

A semântica será definida pelo contrato da operação e pelo destino.

---

## D009-5 — Unknown Is First-Class

Operações com resultado desconhecido não devem ser automaticamente classificadas como sucesso ou falha.

---

## D009-6 — Verification Before Unsafe Retry

Uma escrita de resultado desconhecido não deve ser automaticamente repetida quando não houver garantia suficiente de idempotência ou segurança.

---

## D009-7 — Evidence Is Not Absolute Truth

Verification determina se as evidências disponíveis satisfazem os requisitos de confirmação da operação.

Não representa conhecimento absoluto da realidade.

---

## D009-8 — Evidence Is Contextual

Nenhuma fonte de evidência é universalmente suficiente.

A qualidade da evidência depende de:

```text
Authenticity
Integrity
Freshness
Binding
Independence
Source Trust
Coverage
```

---

## D009-9 — Verification ≠ Reconciliation

Verification confirma efeitos de operações.

Reconciliation compara estado desejado e observado.

---

## D009-10 — External Evidence Is Untrusted Until Validated

Eventos, webhooks, respostas e observações externas devem passar pelas validações apropriadas antes de serem utilizados como evidência.

---

## D009-11 — Model Output Is Not Verification Authority

Modelos podem auxiliar a análise de evidências, mas não possuem autoridade isolada para declarar efeitos externos confirmados.

---

## D009-12 — Compensation Belongs to Workflow

Compensation e Saga pertencem ao Workflow/Mission layer e não ao Integration Gateway.

---

## D009-13 — Memory Must Preserve Epistemic Status

A memória deve distinguir fatos confirmados, falhas, estados inconclusivos e evidências temporárias.

---

## D009-14 — Verification Requirements Cannot Be Automatically Weakened

O sistema não pode reduzir requisitos críticos de verificação automaticamente.

---

# 37. Open Decisions

Permanecem abertas:

* formato final de `VerificationEvidence`;
* linguagem/formato de Verification Policy;
* catálogo formal de Evidence Types;
* mecanismo de Evidence Provenance;
* políticas de freshness;
* mecanismos concretos de polling;
* integração com sistemas de eventos;
* mecanismos específicos de reconciliação;
* armazenamento de evidências;
* retenção;
* integração com hardware de segurança;
* estratégias específicas para sistemas financeiros;
* estratégias específicas para robótica;
* thresholds de retry;
* retry budgets;
* tecnologia de Verification Engine.

Essas decisões não devem ser fechadas prematuramente.

---

# 38. Future ADRs

Possíveis ADRs futuros:

### ADR-010

**Physical World Safety & Hardware Interlocks**

Segurança física, controladores, interlocks, emergency stop e integração com sistemas robóticos.

---

### ADR-011

**Durable Workflow, Saga & Compensation**

Arquitetura detalhada para workflows duráveis, Saga e compensation.

---

### Possível ADR futuro

**Verification Evidence & Policy Model**

Caso o modelo de evidências e políticas cresça o suficiente para justificar uma decisão arquitetural independente.

---

# 39. Technology Independence

Este ADR não exige:

* REST;
* HTTP;
* GraphQL;
* gRPC;
* NATS;
* Kubernetes;
* ROS2;
* Protobuf;
* OpenTelemetry;
* PostgreSQL;
* Python;
* Rust;
* qualquer cloud provider.

Essas tecnologias podem implementar os princípios deste ADR, mas permanecem substituíveis.

---

# 40. Relationship With Existing Architecture

## ADR-006 — Infrastructure Resource Model

Verification pode requisitar recursos através do Resource Manager.

---

## ADR-007 — Integration Model & Manifest

Integrações podem declarar requisitos e capacidades de verificação.

---

## ADR-008 — Credential Isolation

Verification deve respeitar Credential Broker e Secret Store.

O Verification Engine não recebe privilégios especiais sobre credenciais.

---

## Domain 07 — Agents & Tasks

Verification integra-se ao ciclo:

```text
Task
→ Execution
→ Verification
→ Completion
```

---

## Domain 08 — Capability System

Capabilities podem declarar requisitos de verificação.

---

## Domain 09 — Model Router

Modelos podem auxiliar interpretação de evidências, mas não recebem autoridade de confirmação.

---

## Domain 10 — Security

Security controla autorização e proteção.

Verification não substitui Security.

---

## Domain 11 — Evolution

Falhas de verificação podem alimentar evolução controlada.

---

## Domain 13 — Events & Background

Eventos podem funcionar como fontes de evidência após validação.

---

## Domain 14 — Infrastructure

Verification utiliza recursos através das abstrações de infraestrutura.

---

# 41. Anti-Patterns

A Yuki deve evitar:

```text
HTTP 200 == universal success
```

```text
Timeout == failure
```

```text
Timeout == automatic retry
```

```text
LLM says success == confirmed
```

```text
Webhook == absolute truth
```

```text
Sensor == absolute truth
```

```text
Unknown == success
```

```text
Unknown == automatic retry
```

```text
Verification == Authorization
```

```text
Verification == Reconciliation
```

```text
Gateway == Workflow Engine
```

```text
External Data == Instruction
```

```text
Unverified Event == Permanent Fact
```

---

# 42. Official Principles

O ADR-009 estabelece:

> **Execution is not Effect.**

> **Effect is not automatically Verification.**

> **Evidence is not absolute Truth.**

> **Unknown is a valid state.**

> **Verification does not grant Authorization.**

> **External Data is not Instruction.**

> **Models are not Verification Authority.**

> **Unsafe retries require verification or explicit policy.**

> **Verification and Reconciliation are distinct concerns.**

> **Critical verification requirements cannot be weakened automatically.**

---

# 43. Consolidated Architecture

```text
                         USER
                          │
                          ▼
                    YUKI MISSION
                          │
                          ▼
                  AGENT / CAPABILITY
                          │
                          ▼
                 SECURITY / POLICY
                          │
                          ▼
                INTEGRATION GATEWAY
                          │
                          ▼
                   EXTERNAL SYSTEM
                          │
          ┌───────────────┼───────────────┐
          │               │               │
          ▼               ▼               ▼
       RESPONSE          EVENT        OBSERVATION
          │               │               │
          └───────────────┼───────────────┘
                          ▼
                  EVIDENCE LAYER
                          │
                          ▼
               VERIFICATION ENGINE
                          │
             ┌────────────┼────────────┐
             ▼            ▼            ▼
         CONFIRMED      FAILED    INCONCLUSIVE
             │            │            │
             └────────────┼────────────┘
                          ▼
              ┌───────────┼───────────┐
              ▼           ▼           ▼
           MISSION       AUDIT      MEMORY
                                      │
                                      ▼
                               KNOWLEDGE /
                               CONTEXT

Parallel:

DESIRED STATE
      │
      ▼
RECONCILIATION
      │
      ▼
OBSERVED STATE
      │
      ▼
DRIFT / CONVERGENCE
```

---

# 44. Status

**ADR-009 — ACCEPTED**

O ADR estabelece a arquitetura conceitual de verificação de ações externas da Yuki.

Detalhes de implementação permanecem deliberadamente abertos para futuras decisões arquiteturais.

A arquitetura deve preservar:

* separação de responsabilidades;
* segurança;
* verificabilidade;
* reversibilidade quando possível;
* tratamento explícito de incerteza;
* independência tecnológica;
* compatibilidade com sistemas distribuídos;
* compatibilidade com ações assíncronas;
* compatibilidade com sistemas físicos;
* compatibilidade com múltiplos modelos;
* evolução futura.
