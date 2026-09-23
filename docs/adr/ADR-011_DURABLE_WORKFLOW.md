# ADR-011 — Durable Workflow, Saga & Compensation

**Status:** ACCEPTED
**Version:** 1.0
**Date:** 2026-09-23
**Domain:** Workflow / Execution / Reliability / Distributed Systems

---

# 1. Objetivo

Este ADR define a arquitetura conceitual da **Execução Durável (Durable Workflow)** da Yuki.

A Yuki deverá ser capaz de executar missões que podem durar:

* segundos;
* minutos;
* horas;
* dias;
* semanas;
* potencialmente períodos ainda maiores.

Essas missões poderão depender de:

* agentes;
* capacidades;
* integrações;
* APIs externas;
* recursos computacionais;
* eventos;
* aprovações humanas;
* sistemas domésticos;
* dispositivos físicos;
* cloud;
* home server;
* edge devices.

A arquitetura deve permanecer funcional diante de:

* crash de processos;
* reinicialização de workers;
* perda de rede;
* indisponibilidade de serviços;
* falhas parciais;
* timeouts;
* resultados ambíguos;
* operações duplicadas;
* eventos duplicados;
* mudança de contexto;
* expiração de autorização;
* falha de compensação;
* indisponibilidade de recursos.

O objetivo não é construir um workflow engine específico.

O objetivo é estabelecer uma **abstração arquitetural durável**, independente de tecnologia, capaz de sustentar a execução de missões da Yuki durante muitos anos.

---

# 2. Contexto

A arquitetura da Yuki já possui:

* `ADR-006` — Infrastructure Resource Model;
* `ADR-007` — Integration Model & Manifest;
* `ADR-008` — Credential Isolation & Secret Delivery;
* `ADR-009` — Execution, Effect, Verification & Reconciliation;
* `ADR-010` — Physical World Safety & Hardware Interlocks.

Esses ADRs estabelecem limites importantes.

O Workflow Engine:

* não concede autorização;
* não possui credenciais soberanas;
* não controla diretamente hardware crítico;
* não substitui o Security Controller;
* não substitui o Safety Controller;
* não substitui o Resource Manager;
* não substitui o Verification Engine;
* não substitui o Reconciliation Engine;
* não transforma output de modelo em autoridade.

---

# 3. Princípio Fundamental

A Yuki diferencia:

```text
Mission
    ↓
Workflow
    ↓
Task
    ↓
Operation
    ↓
Attempt
    ↓
External Effect
    ↓
Evidence
    ↓
Verification
    ↓
Observed State
    ↓
Reconciliation
```

Cada camada possui responsabilidade própria.

Nenhuma dessas abstrações deve ser utilizada como substituta de outra sem justificativa arquitetural explícita.

---

# 4. Taxonomia

## 4.1 Mission

Uma **Mission** representa um objetivo de alto nível que deve ser alcançado.

Exemplo:

```text
"Preparar relatório financeiro mensal."
```

Mission pertence principalmente ao domínio de:

* objetivo;
* planejamento;
* contexto;
* prioridade;
* resultado esperado.

Mission não é equivalente a Workflow.

---

## 4.2 Workflow

Um **Workflow** representa a estrutura operacional necessária para avançar uma Mission.

Pode conter:

* dependências;
* tarefas;
* operações;
* esperas;
* timers;
* eventos;
* checkpoints;
* decisões;
* branches;
* compensações;
* estados de recuperação.

---

## 4.3 Task

Uma **Task** representa uma unidade lógica de trabalho dentro de um Workflow.

Uma Task pode:

* chamar uma Capability;
* utilizar um Agent;
* solicitar uma Integration;
* aguardar um evento;
* solicitar aprovação;
* produzir uma nova decisão de planejamento.

---

## 4.4 Operation

Uma **Operation** representa uma operação concreta que pode produzir um efeito.

Exemplo:

```text
Enviar pagamento de R$ X.
```

Operation possui identidade própria.

---

## 4.5 Attempt

Um **Attempt** representa uma tentativa específica de executar ou despachar uma Operation.

Uma mesma Operation pode possuir:

```text
operation_id
    ├── attempt_1
    ├── attempt_2
    └── attempt_3
```

Operation Identity não é equivalente a Attempt Identity.

---

# 5. Execution

**Execution** representa o processo de execução de uma Operation/Task por um worker ou runtime.

Execution pode falhar sem que isso determine automaticamente o resultado do efeito externo.

Exemplo:

```text
Worker
   ↓
envia requisição
   ↓
crash
```

O crash do worker não prova que o efeito externo não ocorreu.

Portanto:

```text
Execution Failure
≠
External Effect Failure
```

---

# 6. External Effect

A Yuki não deve assumir conhecimento absoluto do estado externo.

Por isso, o conceito utilizado pela arquitetura é:

> **Observed Effect State**

em vez de assumir que a Yuki possui acesso direto à realidade externa.

O estado externo é conhecido através de evidências.

```text
External World
      ↓
Evidence
      ↓
Verification
      ↓
Observed Effect State
```

---

# 7. Evidence

Evidence representa informação utilizada para avaliar o resultado de uma operação.

Pode vir de:

* resposta de API;
* consulta posterior;
* webhook;
* evento;
* leitura de banco;
* sensor;
* observação física;
* confirmação de dispositivo;
* sistema externo;
* múltiplas fontes independentes.

Evidence não é automaticamente verdade.

Evidence deve ser avaliada considerando características como:

* autenticidade;
* integridade;
* frescor;
* vínculo com a Operation;
* origem;
* cobertura;
* independência;
* confiabilidade da fonte.

---

# 8. Verification

Verification determina se as evidências disponíveis satisfazem os requisitos definidos para considerar determinado efeito observado como confirmado segundo uma política.

Portanto:

```text
Verification
≠
Truth
```

Uma verificação bem-sucedida significa:

> As evidências disponíveis satisfazem os critérios de verificação aplicáveis.

Não significa conhecimento absoluto do mundo.

---

# 9. Durable Workflow

Um Durable Workflow é um Workflow cujo estado necessário para recuperação permanece disponível além da vida de um processo ou worker individual.

Assim:

```text
Workflow
   ↓
Worker Crash
   ↓
State Recovery
   ↓
Outstanding Operation Analysis
   ↓
Resume / Reconcile / Wait / Recover
```

A Yuki não deve assumir que simplesmente continuar do último ponteiro seja suficiente.

Após uma falha, pode ser necessário determinar:

* qual operação estava ativa;
* se foi despachada;
* se o efeito ocorreu;
* se há evidência;
* se a operação pode ser repetida;
* se é necessário reconciliar;
* se a operação deve permanecer UNKNOWN.

---

# 10. Durable State

O estado necessário para recuperação de Workflow deve ser durável dentro das garantias de disponibilidade e persistência estabelecidas para aquele domínio.

Podem existir:

```text
Workflow State
Task State
Operation State
Attempt State
Timer State
Approval State
Compensation State
Recovery State
```

A implementação concreta desses estados permanece independente de tecnologia.

---

# 11. Durable State ≠ Event Sourcing Obrigatório

A Yuki não adota Event Sourcing como requisito constitucional global.

A implementação pode utilizar:

* snapshots;
* state stores;
* event logs;
* Event Sourcing;
* transactional outbox;
* journals;
* combinação dessas técnicas.

A escolha deve depender dos requisitos do domínio.

O requisito arquitetural é:

> O estado necessário para recuperação, auditoria e continuidade deve possuir durabilidade suficiente para o domínio em questão.

---

# 12. Workflow History

Workflow deve manter histórico suficiente para:

* recuperação;
* debugging;
* auditoria;
* replay quando aplicável;
* observabilidade;
* análise de falhas;
* reconstrução de decisões operacionais.

Workflow History não é automaticamente equivalente a Memory.

---

# 13. Workflow State ≠ Memory

Workflow State representa o estado operacional atual de uma missão.

Memory representa conhecimento persistente relevante da Yuki.

Exemplo:

```text
Workflow State:
"aguardando resposta da API."

Memory:
"José prefere determinada configuração."
```

Resultados de Workflow não devem automaticamente se tornar memória permanente.

A promoção para Memory deve seguir as políticas do Memory System.

---

# 14. Workflow State ≠ Authorization

Um Workflow estar em:

```text
APPROVED
READY
RUNNING
```

não concede automaticamente autorização.

Autorização permanece sob responsabilidade do Security Controller.

```text
Workflow State
      ≠
Authorization
```

---

# 15. Workflow Engine

O Durable Workflow Engine é responsável por:

* avanço do Workflow;
* gerenciamento de estado durável;
* timers;
* espera por eventos;
* retries;
* checkpoints;
* recuperação;
* replay quando aplicável;
* coordenação das Tasks;
* controle de dependências;
* controle do ciclo de vida do Workflow.

O Workflow Engine não é responsável por:

* conceder autorização;
* armazenar autoridade secreta;
* decidir segurança física;
* alocar globalmente recursos;
* verificar sozinho efeitos externos;
* decidir políticas de segurança;
* substituir o Mission Manager;
* substituir o Reconciliation Engine.

---

# 16. Separation of Authority

A arquitetura mantém:

```text
                 WORKFLOW ENGINE
                       │
        ┌──────────────┼──────────────┐
        ▼              ▼              ▼
   Security        Resource       Verification
   Controller       Manager          Engine
        │              │              │
   Authorization    Resources       Evidence
```

E para o domínio físico:

```text
Safety Controller
```

permanece independente.

O Workflow Engine coordena.

Ele não governa sozinho.

---

# 17. LLM como Activity, não como Workflow Engine

Modelos de linguagem podem participar da execução como atividades não determinísticas.

Exemplo:

```text
Workflow Engine
      ↓
LLM Activity
      ↓
Model Output
      ↓
Persist Result
      ↓
Continue Workflow
```

O LLM não controla:

* replay;
* persistência;
* autorização;
* segurança;
* estado durável;
* credenciais.

---

# 18. Determinism e Replay

Quando o runtime utilizar replay determinístico, decisões que precisam permanecer estáveis após replay devem ser reconstruíveis a partir do histórico persistido.

Chamadas não determinísticas, como:

* LLM;
* APIs;
* sensores;
* relógio externo;
* randomização;
* ferramentas;

devem ser tratadas como atividades externas ou eventos persistidos conforme o modelo de execução.

A Yuki não deve depender de chamar novamente um LLM e esperar que ele produza exatamente a mesma resposta.

---

# 19. Operation Identity

A arquitetura diferencia:

```text
correlation_id
causation_id
operation_id
attempt_id
idempotency_key
```

### correlation_id

Relaciona eventos pertencentes a uma mesma missão ou contexto.

### causation_id

Representa a causa que originou determinado evento/operação.

### operation_id

Identifica a operação lógica.

### attempt_id

Identifica uma tentativa específica.

### idempotency_key

Identifica uma operação para fins de deduplicação segundo o contrato do destino.

Esses identificadores não são intercambiáveis.

---

# 20. Idempotency

A Yuki não impõe uma fórmula universal para gerar `idempotency_key`.

A idempotência depende do contrato da:

* Capability;
* Integration;
* Operation;
* External System.

Quando suportada, a idempotência deve ser utilizada para reduzir o risco de efeitos duplicados.

Quando não suportada, a Yuki deve tratar operações ambíguas com maior cautela.

---

# 21. UNKNOWN

`UNKNOWN` é um estado de primeira classe.

Exemplo:

```text
Operation dispatched
      ↓
Network timeout
      ↓
Response unknown
```

A Yuki não deve assumir automaticamente:

```text
SUCCESS
```

nem:

```text
FAILED
```

O resultado pode permanecer:

```text
UNKNOWN
```

até que novas evidências estejam disponíveis.

---

# 22. UNKNOWN e Retry

A regra arquitetural é:

> UNKNOWN não deve resultar em repetição automática cega.

O fluxo preferencial é:

```text
UNKNOWN
   ↓
Can outcome be determined?
   ├── YES → Verification / Inquiry
   │            ↓
   │        Known State
   │
   └── NO
        ↓
Can operation safely retry?
        ├── YES → Retry according to policy
        └── NO  → Remain UNKNOWN / Escalate
```

O fato de uma operação estar UNKNOWN não significa que ela jamais possa ser repetida.

Significa que a repetição precisa respeitar a semântica da operação e sua política de risco.

---

# 23. Retry Policy

Retries devem considerar:

* idempotência;
* tipo da operação;
* risco;
* estado conhecido/desconhecido;
* erro transitório;
* erro permanente;
* retry budget;
* backoff;
* jitter;
* limite de tentativas;
* contexto atual.

Não existe uma política universal de retry para todas as operações.

---

# 24. Retry Budget

Cada domínio pode estabelecer limites para evitar:

* retry storms;
* loops infinitos;
* consumo excessivo de recursos;
* efeitos repetidos;
* custos inesperados.

Valores concretos de retry budget permanecem dependentes da operação e não são definidos constitucionalmente neste ADR.

---

# 25. Timeout ≠ Failure

Timeout significa que a Yuki não recebeu uma resposta dentro da janela esperada.

Ele não prova:

```text
Effect = NO_MUTATION
```

nem:

```text
Effect = MUTATED
```

Portanto:

```text
Timeout
   ↓
Outcome Evaluation
   ↓
UNKNOWN quando necessário
```

---

# 26. Saga

Para workflows distribuídos de longa duração que atravessam múltiplos sistemas, a Yuki pode utilizar o **Saga Pattern**.

Uma Saga representa uma sequência de operações que pode exigir recuperação ou compensação após falhas parciais.

Exemplo:

```text
A ✓
↓
B ✓
↓
C ✗
```

Pode resultar em:

```text
Compensate B
↓
Compensate A
```

quando essas compensações existirem e forem apropriadas.

---

# 27. Saga Orchestration

A Yuki suporta coordenação explícita de Saga pelo Workflow Engine quando isso for apropriado.

Isso não significa que toda integração futura deverá obrigatoriamente utilizar uma Saga centralizada.

O padrão deve ser escolhido conforme:

* domínio;
* acoplamento;
* duração;
* confiabilidade;
* necessidade de coordenação;
* capacidade de compensação.

---

# 28. Compensation ≠ Rollback

### Rollback

Tenta restaurar um estado anterior dentro de um mecanismo transacional que suporte isso.

### Compensation

É uma nova operação destinada a neutralizar ou reduzir o efeito de uma operação anterior.

Exemplo:

```text
Pagamento
   ↓
Refund
```

Refund não é rollback do sistema de pagamentos.

É uma nova operação.

---

# 29. Compensation é uma Operation

Uma compensação deve possuir:

* `operation_id`;
* identidade própria;
* attempts;
* evidências;
* verification;
* política;
* autorização apropriada;
* histórico.

Assim:

```text
Forward Operation
       ↓
External Effect
       ↓
Compensation Operation
       ↓
New External Effect
       ↓
Evidence
       ↓
Verification
```

A compensação não apaga o histórico da operação original.

---

# 30. Compensation não é garantida

Uma compensação pode:

```text
SUCCEED
FAIL
PARTIALLY_SUCCEED
UNKNOWN
```

Por isso, o Workflow deve suportar estados como:

```text
COMPENSATION_REQUIRED
COMPENSATING
COMPENSATED
PARTIALLY_COMPENSATED
COMPENSATION_FAILED
COMPENSATION_UNKNOWN
MANUAL_RECOVERY_REQUIRED
```

---

# 31. Compensation Capability

Quando uma operação possuir compensação conhecida, essa possibilidade deve ser declarada pelo contrato correspondente.

Possibilidades:

```text
REVERSIBLE
COMPENSATABLE
PARTIALLY_COMPENSATABLE
NON_REVERSIBLE
UNKNOWN
```

A Yuki não deve inventar uma compensação que não exista.

---

# 32. Saga não implica 2PC

A arquitetura não depende de Two-Phase Commit para coordenar sistemas externos.

A razão é que:

* APIs externas;
* serviços cloud;
* sistemas físicos;
* provedores independentes;

normalmente não participam de uma transação distribuída comum.

A Yuki utiliza mecanismos apropriados ao domínio, como:

* Saga;
* compensação;
* reconciliação;
* verificação;
* intervenção humana.

---

# 33. Compensation Failure

Se uma compensação falhar:

```text
Original Operation
        ↓
Compensation
        ↓
FAILED
```

o Workflow não deve fingir que o sistema retornou ao estado anterior.

Pode ser necessário:

```text
Retry
Reconciliation
Another Compensation
Human Intervention
Manual Recovery
```

---

# 34. Reconciliation ≠ Workflow

Workflow responde:

> "Execute este processo."

Reconciliation responde:

> "O estado observado corresponde ao estado desejado?"

Portanto:

```text
Desired State
      │
      ▼
Reconciliation
      │
      ▼
Observed State
```

é conceitualmente diferente de:

```text
Mission
  ↓
Workflow
  ↓
Operations
```

---

# 35. Workflow pode solicitar Reconciliation

O Workflow pode detectar que reconciliation é necessária.

Porém:

```text
Workflow Engine
       ↓
Reconciliation Request
       ↓
Reconciliation Engine
```

O Workflow Engine não absorve a responsabilidade do Reconciliation Engine.

---

# 36. Workflow Completion ≠ World Convergence

Uma missão pode terminar:

```text
Workflow = COMPLETED
```

sem que isso prove:

```text
Observed State = Desired State
```

Da mesma forma:

```text
Workflow = FAILED
```

não necessariamente significa:

```text
Desired State ≠ Observed State
```

Essa distinção é fundamental para a Yuki.

---

# 37. Long-Running Workflows

Workflows podem permanecer em espera por longos períodos.

Exemplo:

```text
RUNNING
   ↓
WAITING_EXTERNAL_EVENT
   ↓
WAITING
   ↓
Event
   ↓
RESUMING
   ↓
RUNNING
```

Durante períodos de espera, o Workflow não deve precisar manter recursos computacionais ativos continuamente.

---

# 38. Durable Timers

Timers devem ser representados como estado durável quando sua duração ultrapassar a vida de um processo.

Exemplo:

```text
WAIT 72 HOURS
```

deve sobreviver a:

* crash;
* reboot;
* worker migration;
* indisponibilidade temporária.

---

# 39. Pause

Pause significa impedir o avanço normal do Workflow.

Não significa necessariamente:

* cancelar operações externas;
* revogar todas as credenciais;
* desligar dispositivos;
* desfazer efeitos.

Essas ações devem ser tratadas separadamente.

---

# 40. Cancel

Cancel é uma solicitação de interrupção.

O Workflow pode possuir:

```text
CANCEL_REQUESTED
CANCELLING
CANCELLED
CANCELLATION_FAILED
UNKNOWN
```

Uma operação já enviada a um sistema externo pode não ser cancelável.

---

# 41. Dynamic Replanning

A Yuki pode descobrir que o plano atual não é mais apropriado.

Nesse caso:

```text
Observation
    ↓
Planner
    ↓
Plan Mutation Proposal
    ↓
Validation
    ↓
Authorization / Policy
    ↓
Workflow Revision
```

Um agente não deve modificar arbitrariamente o estado durável do Workflow.

---

# 42. Workflow Versioning

Workflows em execução devem possuir uma definição/versionamento identificável.

Uma atualização da Yuki não deve alterar silenciosamente a semântica de uma missão que já está em execução.

Quando necessário:

```text
Workflow v1
    ↓
Migration Proposal
    ↓
Validation
    ↓
Workflow v2
```

A migração deve ser explícita e controlada.

---

# 43. Human-in-the-Loop

Workflows podem possuir estados:

```text
WAITING_APPROVAL
```

Aprovações devem possuir contexto suficiente para determinar:

* quem aprovou;
* o que foi aprovado;
* escopo;
* operação;
* momento;
* validade;
* política aplicável.

Uma aprovação pode expirar ou tornar-se inválida devido a mudança significativa de contexto.

---

# 44. Revalidation

Para workflows longos:

```text
Approval
    ↓
Time passes
    ↓
Context changes
    ↓
Revalidation
```

O Workflow Engine não deve assumir que uma autorização antiga permanece válida indefinidamente.

A decisão final pertence ao mecanismo de autorização apropriado.

---

# 45. Resource Management

Workflow pode solicitar recursos ao Resource Manager.

Exemplo:

```text
Workflow
   ↓
GPU Required
   ↓
Resource Manager
   ↓
WAITING_RESOURCE
```

Workflow Engine não substitui Resource Manager ou Scheduler.

---

# 46. Resource Fallback

Workflow não deve inventar semanticamente um fallback.

Exemplo:

```text
GPU → CPU
FP16 → INT8
```

só pode ocorrer se a Capability/Workload/Execution Strategy declarar que essa alternativa é aceitável.

---

# 47. Credential Expiration

Workflows longos podem atravessar a validade de credenciais.

O Workflow não deve armazenar credenciais brutas como estado permanente.

A renovação ou reemissão deve seguir o ADR-008.

```text
Workflow
   ↓
Credential Reference
   ↓
Credential Broker
   ↓
Current Credential Material
```

---

# 48. Authorization Revalidation

Durante workflows longos, a Yuki deve considerar:

* expiração;
* revogação;
* mudança de política;
* mudança de contexto;
* mudança de risco;
* mudança de identidade;
* mudança do alvo.

Workflow state não preserva autorização automaticamente.

---

# 49. Workflow State Security

Workflow state deve ser protegido contra:

* adulteração;
* corrupção;
* replay indevido;
* acesso não autorizado;
* alteração maliciosa.

Entretanto:

> Workflow state não é uma fonte soberana de autoridade.

Mesmo que alguém altere um estado para:

```text
AUTHORIZED
```

isso não deve conceder autorização real.

---

# 50. Worker Failure

O Worker pode morrer:

### Antes do dispatch

```text
No known dispatch
```

### Durante o dispatch

```text
Potentially UNKNOWN
```

### Depois do dispatch

```text
Potentially UNKNOWN
```

### Depois da execução

```text
Effect may exist
```

A Yuki deve usar evidências e contratos da operação para determinar a recuperação.

---

# 51. Leases e Ownership

Workers e recursos podem utilizar leases quando necessário.

Uma lease vencida não deve permitir que um worker antigo continue controlando uma operação como se ainda fosse proprietário.

Mecanismos de fencing podem ser utilizados quando apropriado.

A tecnologia específica permanece aberta.

---

# 52. Event-Driven Workflow

Workflow pode reagir a:

* eventos;
* timers;
* sensores;
* webhooks;
* mensagens;
* resultados de operações.

Eventos devem possuir mecanismos apropriados de:

* identificação;
* correlação;
* deduplicação;
* validade;
* integridade;
* origem.

---

# 53. Evento ≠ Autoridade

Um evento externo não deve automaticamente:

* conceder permissão;
* alterar Security Policy;
* liberar credenciais;
* declarar sucesso;
* ordenar ação crítica.

Eventos são dados que passam pelas políticas apropriadas.

---

# 54. Duplicate Events

A arquitetura deve tolerar eventos duplicados quando o sistema de transporte não oferecer exatamente-once.

Handlers devem ser idempotentes quando necessário.

---

# 55. Event Ordering

A Yuki não deve assumir ordenação global perfeita dos eventos.

Quando ordenação for necessária, o domínio deve declarar mecanismos adequados, como:

* sequence numbers;
* causal relationships;
* timestamps;
* version numbers;
* domain-specific ordering.

---

# 56. Exactly-Once

"Exactly-once" não é uma garantia global da Yuki.

A garantia deve ser definida por camada:

```text
Message
Processing
Workflow Transition
Operation
External Effect
```

Uma garantia exactly-once de processamento interno não implica exatamente um efeito no mundo externo.

---

# 57. Transactional Outbox

Transactional Outbox pode ser utilizado quando necessário para garantir consistência entre:

```text
Durable State
+
Event Publication
```

Porém, Outbox não é requisito constitucional deste ADR.

---

# 58. Home / Edge / Cloud

A Yuki pode executar workflows em:

```text
EDGE
HOME
CLOUD
```

A localização do Workflow Runtime é uma decisão de deployment/infrastructure e não uma autoridade arquitetural fixa.

O Home Server não é constitucionalmente definido como autoridade primária global.

---

# 59. Offline

A arquitetura deve permitir operação degradada quando:

* Cloud indisponível;
* Home indisponível;
* Edge desconectado.

Entretanto, a capacidade de continuar depende da autoridade, dados e recursos necessários para aquele Workflow.

Não existe promessa de que qualquer Workflow funcione offline.

---

# 60. Failure Domains

Falhas devem ser isoláveis por:

* Mission;
* Workflow;
* Task;
* Worker;
* Integration;
* Resource Domain;
* Provider;
* Edge;
* Home;
* Cloud.

Uma falha em uma missão não deve automaticamente interromper todas as outras.

---

# 61. Observabilidade

Cada Workflow deve poder fornecer uma representação de sua trajetória operacional.

Idealmente:

```text
Mission
 ↓
Workflow
 ↓
Task
 ↓
Operation
 ↓
Attempt
 ↓
Evidence
 ↓
Verification
 ↓
Compensation / Reconciliation
```

Isso permite investigação de:

* falhas;
* duplicações;
* timeouts;
* recovery;
* decisões;
* intervenções humanas.

---

# 62. Audit

Operational state e audit history são conceitos distintos.

O estado representa:

> "Onde estamos."

O histórico representa:

> "Como chegamos aqui."

Ambos podem ser necessários.

---

# 63. Privacy

Workflow history pode conter:

* dados pessoais;
* documentos;
* informações financeiras;
* mensagens;
* informações de terceiros;
* dados sensíveis.

A retenção deve respeitar:

* minimização;
* acesso controlado;
* proteção;
* retenção adequada;
* políticas de privacidade.

---

# 64. Security Anti-Patterns

São proibidos ou fortemente desencorajados:

### LLM como Workflow Engine

### Workflow State como Authorization

### Secrets no Workflow History

### Retry infinito

### Retry cego de UNKNOWN

### Timeout = Failure automático

### Timeout = Success automático

### Exactly-once global presumido

### Compensation automática sem contrato

### Rollback imaginário

### Worker sem ownership controlado

### Workflow Engine como Security Controller

### Workflow Engine como Resource Manager

### Workflow Engine como Safety Controller

---

# 65. Architectural Invariants

As seguintes invariantes são adotadas:

> **I1 — Workflow não possui autoridade soberana.**

> **I2 — Workflow State não concede autorização.**

> **I3 — UNKNOWN não deve ser silenciosamente convertido em SUCCESS.**

> **I4 — UNKNOWN não deve ser silenciosamente convertido em FAILED quando o efeito permanece indeterminado.**

> **I5 — Compensation não é Rollback.**

> **I6 — Compensation não é garantida.**

> **I7 — LLM output não é Workflow Authority.**

> **I8 — Exactly-once não é uma garantia global da Yuki.**

> **I9 — Operation Identity ≠ Attempt Identity ≠ Idempotency Key.**

> **I10 — Workflow ≠ Reconciliation.**

> **I11 — Workflow não pode ignorar Security Controller.**

> **I12 — Workflow não pode ignorar Safety Controller.**

> **I13 — Workflow não pode conceder recursos por conta própria.**

> **I14 — Workflow state deve ser recuperável conforme suas garantias de durabilidade.**

> **I15 — Operações ambíguas devem ser investigadas antes de serem repetidas quando houver risco de efeito duplicado.**

---

# 66. Decisões Fechadas

## D011-1 — Durable Workflow

Durable Workflow é uma abstração de primeira classe da Yuki.

## D011-2 — Limited Authority

Workflow Engine não possui autoridade soberana.

## D011-3 — Durable State

Estado necessário para recuperação deve ser durável conforme as garantias do domínio.

## D011-4 — No Mandatory Event Sourcing

Event Sourcing não é obrigatório como arquitetura global.

## D011-5 — UNKNOWN

UNKNOWN é estado de primeira classe.

## D011-6 — Retry Semantics

Retries dependem da operação, idempotência e política.

## D011-7 — Identity Separation

Operation, Attempt e Idempotency Key possuem semânticas diferentes.

## D011-8 — Compensation

Compensation é uma nova operação e não um rollback.

## D011-9 — Imperfect Compensation

Compensação pode falhar, ser parcial ou permanecer UNKNOWN.

## D011-10 — Reconciliation Separation

Workflow e Reconciliation são componentes conceitualmente distintos.

## D011-11 — LLM Isolation

LLMs podem atuar como atividades, mas não como autoridade do Workflow Engine.

## D011-12 — Authorization Separation

Workflow State não concede autorização.

## D011-13 — Authorization Revalidation

Workflows longos podem exigir revalidação de autorização.

## D011-14 — Versioned Workflows

Workflows em execução possuem definição/versionamento identificável.

## D011-15 — Explicit Workflow Mutation

Mudanças de plano ou workflow em execução devem ocorrer através de mecanismos explícitos e controlados.

## D011-16 — Recovery Analysis

Recovery deve considerar operações que possam ter sido despachadas antes de uma falha.

## D011-17 — No Global Exactly-Once

Exactly-once não é garantia global da Yuki.

## D011-18 — Technology Independence

Nenhuma tecnologia específica de Durable Execution, Event Store, Message Broker ou Workflow Runtime é constitucionalmente obrigatória neste ADR.

---

# 67. Decisões em Aberto

As seguintes questões permanecem deliberadamente abertas:

### O011-1 — Durable Workflow Runtime

Qual tecnologia ou implementação será utilizada.

Possibilidades futuras incluem:

* Temporal;
* Restate;
* Step Functions;
* runtime próprio;
* outras soluções.

---

### O011-2 — Persistence Architecture

Determinar posteriormente a combinação entre:

* state store;
* snapshots;
* event log;
* Event Sourcing;
* Outbox;
* journals.

---

### O011-3 — Workflow Definition Format

Determinar:

* DSL;
* JSON;
* YAML;
* Protobuf;
* código;
* representação híbrida.

---

### O011-4 — Workflow Migration

Definir mecanismo formal de migração de workflows ativos entre versões.

---

### O011-5 — Reconciliation Engine

Definir arquitetura detalhada do Reconciliation Engine.

---

### O011-6 — Federation

Definir sincronização de Workflow State entre:

* Edge;
* Home;
* Cloud.

---

### O011-7 — Durable Messaging

Determinar se a Yuki precisará de:

* Outbox;
* Inbox;
* Event Log;
* Message Broker;
* outros mecanismos.

---

### O011-8 — Compensation DSL

Determinar como capacidades de compensação serão declaradas.

---

# 68. Relação com Outros ADRs

```text
ADR-006
Infrastructure Resource Model
        │
        ▼
Resource availability
        │
        ▼
ADR-011
Durable Workflow
        │
        ├───────────────┐
        ▼               ▼
ADR-007           ADR-008
Integrations      Credentials
        │               │
        └───────┬───────┘
                ▼
             Execution
                │
                ▼
             ADR-009
Verification / Reconciliation
                │
                ▼
             ADR-010
Physical Safety
```

---

# 69. Futuras Extensões

Possíveis ADRs futuros:

### ADR-012

**Reconciliation Engine Architecture**

### ADR-013

**Durable State & Workflow Persistence**

### ADR-014

**Workflow/Event Federation & Offline Synchronization**

### ADR-015

**Physical Device & Robotics Control**

### ADR-016

**Physical Device Identity & Attestation**

### ADR-017

**Physical Capability & Safety Manifest**

A numeração futura permanece sujeita à evolução da arquitetura.

---

# 70. Arquitetura Consolidada

```text
                           YUKI
                            │
                     MISSION MANAGER
                            │
                            ▼
                 DURABLE WORKFLOW LAYER
                            │
             ┌──────────────┼──────────────┐
             │              │              │
             ▼              ▼              ▼
          TASK GRAPH      TIMERS         EVENTS
             │
             ▼
        EXECUTION ENGINE
             │
       ┌─────┼──────┐
       ▼     ▼      ▼
  CAPABILITY INTEGRATION AGENT
       │     │      │
       └─────┼──────┘
             ▼
       EXTERNAL SYSTEM
             │
             ▼
          EVIDENCE
             │
             ▼
      VERIFICATION ENGINE
             │
             ▼
    OBSERVED EFFECT STATE
             │
       ┌─────┴──────┐
       ▼            ▼
 RECONCILIATION  COMPENSATION
     ENGINE        WORKFLOW
       │            │
       └─────┬──────┘
             ▼
        MISSION OUTCOME
```

Controles transversais:

```text
                  SECURITY CONTROLLER
                          │
             Authorization / Policy / Audit


                   RESOURCE MANAGER
                          │
             Compute / Storage / Network


                    SAFETY CONTROLLER
                          │
                  Physical Safety
```

---

# 71. Modelo Conceitual Completo

A arquitetura pode ser resumida em:

```text
                     DESIRED STATE
                          │
                          ▼
                       MISSION
                          │
                          ▼
                        PLAN
                          │
                          ▼
                       WORKFLOW
                          │
                          ▼
                     OPERATION
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
                          │
                          ▼
                 OBSERVED EFFECT STATE
                          │
                          ▼
                    RECONCILIATION
                          │
                          ▼
                    DESIRED STATE
```

Quando necessário:

```text
Operation
    │
    ▼
Failure / Partial Failure
    │
    ▼
Compensation
    │
    ▼
New Operation
    │
    ▼
Verification
```

E quando houver incerteza:

```text
UNKNOWN
   │
   ├── Inquiry
   ├── Verification
   ├── Reconciliation
   ├── Safe Retry
   └── Human Intervention
```

---

# 72. Princípios Oficiais do ADR-011

1. **Durable Execution over Process Lifetime**
2. **Workflow Authority is Limited**
3. **Durable State over Ephemeral Workers**
4. **UNKNOWN is First-Class**
5. **Operation Identity is Explicit**
6. **Idempotency is Contractual**
7. **Compensation is Not Rollback**
8. **Compensation is Imperfect**
9. **Verification is Evidence-Based**
10. **Workflow is Not Reconciliation**
11. **LLM is Not Workflow Authority**
12. **Authorization is Independent**
13. **Recovery Must Handle Ambiguity**
14. **Workflow Versions Must Be Explicit**
15. **Exactly-Once is Not a Global Promise**
16. **Technology Independence**

---

# 73. Status

**ADR-011 — ACCEPTED**

**Version:** 1.0

Este ADR estabelece a arquitetura conceitual de Durable Workflow da Yuki sem congelar prematuramente:

* runtime;
* banco;
* Event Store;
* broker;
* DSL;
* deployment topology;
* mecanismo de federation.

As decisões de implementação serão tratadas em ADRs posteriores quando houver evidência arquitetural suficiente.

---

# 74. Regra de Manutenção

Qualquer decisão futura que altere:

* durabilidade;
* recovery;
* workflow authority;
* operation identity;
* idempotency;
* compensation;
* workflow versioning;
* reconciliation boundary;

deve ser registrada através de novo ADR ou atualização explícita deste ADR.

Nenhuma decisão crítica deve permanecer exclusivamente em conversas.

**Git permanece como memória arquitetural oficial da Yuki.**
