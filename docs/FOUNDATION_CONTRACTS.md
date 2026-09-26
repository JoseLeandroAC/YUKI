Yuki — Foundation Contracts

Version: 0.1
Status: FOUNDATION DRAFT
Domain: Core / Contracts / Architecture
Date: 2026-09-26

---

1. Objetivo

Este documento define os contratos conceituais fundamentais da Yuki.

Seu objetivo é criar uma linguagem comum entre:

- Yuki Core;
- Context;
- Model Router;
- Capability System;
- Security Controller;
- Execution Layer;
- Integration Gateway;
- Verification Engine;
- Workflow;
- Agents;
- Memory;
- Knowledge;
- Reconciliation;
- Infrastructure;
- Development Agents.

Os contratos aqui definidos representam o significado arquitetural dos objetos.

A implementação concreta poderá utilizar diferentes linguagens, frameworks, bancos de dados, runtimes ou provedores.

---

2. Regra fundamental

A existência de um tipo neste documento não significa que ele já esteja implementado.

A sequência oficial é:

Architectural Decision
        ↓
Foundation Contract
        ↓
Implementation
        ↓
Tests
        ↓
Validation

Uma implementação não deve alterar silenciosamente o significado de um contrato.

Caso a implementação revele uma necessidade arquitetural não prevista:

Implementation Gap
        ↓
Proposal
        ↓
Architectural Review
        ↓
Contract / ADR update
        ↓
Implementation

---

3. Princípios

Os contratos devem preservar:

1. separação de responsabilidades;
2. segurança por arquitetura;
3. menor privilégio;
4. menor agência necessária;
5. minimização de dados;
6. independência de tecnologia;
7. independência de provedor;
8. verificabilidade;
9. reversibilidade quando possível;
10. rastreabilidade;
11. compatibilidade futura;
12. autoridade humana;
13. isolamento de falhas.

---

4. Taxonomia fundamental

A Yuki deve distinguir explicitamente:

Resource
Capability
Permission
Authorization
Credential
Operation
Attempt
Execution
Effect
Evidence
Verification
Integration
Context
Memory
Knowledge
Mission
Task
Workflow
Desired State
Observed State
Reconciliation Intent

Esses conceitos não devem ser tratados como sinônimos.

---

5. Resource

Definição

Resource representa um recurso de infraestrutura ou execução disponível para a Yuki.

Exemplos:

- CPU;
- GPU;
- NPU;
- armazenamento;
- memória;
- rede;
- runtime;
- dispositivo;
- acelerador;
- recurso futuro desconhecido.

O Resource Model é definido pelo ADR-006.

Resource não é

Resource ≠ Capability
Resource ≠ Permission
Resource ≠ Workload
Resource ≠ Authorization

Um recurso disponível não significa que a Yuki esteja autorizada a utilizá-lo.

---

6. Capability

Definição

Capability representa uma capacidade funcional que a Yuki sabe utilizar.

Exemplos:

research.web
calendar.read
calendar.create
finance.balance.read
image.generate
code.analyze
device.control

Capability representa:

«O que a Yuki consegue fazer.»

Não representa autorização.

Capability ≠ Permission

Uma Capability pode existir no Registry sem estar disponível para determinada missão, usuário, contexto ou integração.

---

7. Permission

Definição

Permission representa uma regra que determina se determinada ação, recurso, dado ou operação pode ser utilizada em determinado contexto.

Permission pode considerar:

- identidade;
- capability;
- recurso;
- alvo;
- dados;
- contexto;
- risco;
- política;
- tempo;
- escopo;
- condições.

Permission não é uma credencial.

---

8. Authorization

Definição

Authorization representa a decisão operacional de permitir uma ação específica dentro das políticas e condições aplicáveis.

Authorization deve ser:

- contextual;
- limitada;
- rastreável;
- revogável quando aplicável;
- compatível com políticas;
- compatível com escopo;
- compatível com risco.

Authorization não é Capability.

Authorization não é Credential.

Authorization não deve ser inferida simplesmente porque uma Capability existe.

---

9. Credential

Definição

Credential representa material ou mecanismo utilizado para provar identidade, autenticar uma chamada ou acessar um sistema externo.

Exemplos conceituais:

- access token;
- certificado;
- chave;
- sessão autenticada;
- credencial de workload.

A arquitetura de credenciais é definida pelo ADR-008.

Regra

Modelos, prompts e memória normal não devem receber material secreto de credenciais.

Preferencialmente:

Model
 ↓
Authorization
 ↓
Capability Token / Credential Reference
 ↓
Credential Broker
 ↓
Credential Material
 ↓
External System

---

10. Operation

Definição

Operation representa uma ação lógica concreta que a Yuki pretende realizar.

Uma Operation conecta:

Capability
+
Target
+
Parameters
+
Context
+
Authorization
+
Operation Identity

Exemplo:

Capability:
calendar.create

Target:
calendar:user

Parameters:
date = ...
title = ...

Uma Operation pode possuir várias tentativas.

---

11. Attempt

Definição

Attempt representa uma tentativa concreta de executar uma Operation.

Relação:

Operation
   │
   ├── Attempt 1
   ├── Attempt 2
   └── Attempt 3

Uma Operation não deve ser confundida com uma tentativa individual.

Isso é especialmente importante para:

- retries;
- idempotência;
- UNKNOWN;
- auditoria;
- verificação.

---

12. Execution

Definição

Execution representa o processamento/runtime que realiza uma Attempt.

Pode ocorrer em:

- processo;
- container;
- Wasm;
- MicroVM;
- worker;
- dispositivo;
- runtime externo;
- infraestrutura local;
- infraestrutura cloud.

Execution é uma implementação operacional.

Não possui autoridade arquitetural própria.

---

13. Effect

Definição

A Yuki não deve assumir acesso direto à “realidade”.

Portanto, o conceito operacional utilizado é:

Observed Effect State

Ele representa o estado do efeito que foi observado ou inferido a partir de evidências válidas.

Exemplos:

NOT_OBSERVED
UNKNOWN
OBSERVED_NO_MUTATION
OBSERVED_MUTATION
OBSERVED_PARTIAL
CONFLICTING

O significado exato dos estados deve ser definido pelo Verification Engine e pelos contratos específicos.

---

14. Evidence

Definição

Evidence representa informação utilizada para sustentar uma conclusão sobre uma Operation ou Effect.

Exemplos:

- resposta de API;
- recibo;
- evento;
- webhook;
- observação de sensor;
- leitura posterior;
- estado de recurso;
- confirmação de sistema externo.

Evidence não é automaticamente verdade.

Deve ser avaliada quanto a características como:

- autenticidade;
- integridade;
- frescor;
- vínculo com a Operation;
- origem;
- cobertura;
- independência;
- confiabilidade.

---

15. Verification

Definição

Verification é o processo de avaliar Evidence segundo uma política de verificação.

Verification responde perguntas como:

«As evidências disponíveis são suficientes para confirmar o resultado esperado desta operação?»

Verification não:

- concede autorização;
- executa automaticamente uma operação;
- substitui o Security Controller;
- substitui o Safety Controller;
- cria verdade absoluta.

---

16. VerificationResult

Um resultado de verificação deve representar pelo menos:

VerificationResult
├── operation_id
├── verification_state
├── evidence_refs
├── evaluated_at
├── verification_basis
└── observed_effect_state

O formato concreto permanece aberto.

Evitar utilizar um único "confidence_score" como substituto para toda a avaliação de evidências.

---

17. Integration

Definição

Integration representa uma ligação operacional configurada entre a Yuki e um sistema externo.

Uma Integration pode suportar várias Capabilities.

Exemplo:

Google Calendar Integration
├── calendar.read
├── calendar.create
├── calendar.update
└── calendar.delete

Integration é diferente de:

Capability
Tool
Plugin
Connector
Adapter
Provider
External System

A arquitetura está formalizada no ADR-007 e no ADR-017.

---

18. Context

Definição

Context representa o conjunto estruturado de informações relevantes para uma tarefa específica.

Context pode incluir:

- objetivo;
- intenção;
- estado atual;
- dados recuperados;
- evidências;
- preferências;
- restrições;
- hipóteses;
- informações externas;
- estado epistemológico.

Context não é autorização.

Context não é memória.

Context não é verdade.

Context não deve conter secrets como dados normais.

---

19. Typed Context Object

O Core deve utilizar um Typed Context Object como abstração arquitetural.

Conceitualmente:

TypedContext
├── context_id
├── purpose
├── task_ref
├── scope
├── inputs
├── observations
├── evidence_refs
├── assumptions
├── constraints
├── provenance
├── freshness
├── epistemic_state
├── data_classification
└── lifecycle

O formato definitivo ainda será definido.

A estrutura deve permitir:

- minimização;
- proveniência;
- validade temporal;
- classificação;
- rastreabilidade;
- atenuação entre agentes.

---

20. Memory

Definição

Memory representa informação persistente selecionada para retenção pela Yuki.

Memory pode conter:

- episódios;
- fatos;
- decisões;
- preferências;
- projetos;
- objetivos;
- relações;
- conhecimento consolidado.

Memory não deve receber automaticamente todo Context.

Context
   ↓
Memory Policy
   ↓
Selective Consolidation
   ↓
Memory

---

21. Knowledge

Definição

Knowledge representa informação estruturada ou consolidada utilizada para compreensão e recuperação.

Knowledge pode ser:

- externa;
- derivada;
- estruturada;
- documental;
- relacional;
- semântica.

Knowledge não possui autoridade operacional simplesmente por existir.

---

22. Mission

Definição

Mission representa um objetivo operacional de nível superior que a Yuki está tentando realizar.

Exemplo:

Mission:
"Preparar uma análise dos gastos deste mês."

Mission pode possuir:

- objetivo;
- contexto;
- prioridade;
- restrições;
- deadline;
- tarefas;
- workflow;
- resultados;
- estado.

Mission não é uma autorização.

---

23. Task

Definição

Task representa uma unidade de trabalho dentro de uma Mission ou Workflow.

Exemplo:

Mission
 ↓
Task 1 — coletar dados
Task 2 — validar dados
Task 3 — analisar
Task 4 — produzir resultado

Task não é necessariamente uma Operation.

Uma Task pode conter várias Operations.

---

24. Workflow

Definição

Workflow representa a orquestração durável de Tasks e Operations.

Workflow deve permitir:

- persistência;
- recuperação;
- pausa;
- retomada;
- cancelamento;
- timeout;
- espera;
- retries controlados;
- aprovação;
- compensação quando aplicável.

Workflow não possui autoridade soberana.

---

25. Desired State

Definição

Desired State representa um estado pretendido.

Exemplo:

Thermostat:
desired_temperature = 22°C

Desired State não é automaticamente:

- autorização;
- comando;
- operação;
- execução.

Ele pode originar uma Reconciliation Intent quando existir divergência relevante.

---

26. Observed State

Definição

Observed State representa o estado observado/reconstruído a partir de observações e evidências consideradas válidas para o domínio.

Fluxo:

Raw Observation
      ↓
Evidence
      ↓
Validation
      ↓
Observed State

Observed State pode ser:

KNOWN
UNKNOWN
STALE
CONFLICTING
INCOMPLETE

conforme o domínio.

---

27. Reconciliation Intent

Definição

Reconciliation Intent representa a intenção declarativa de corrigir ou analisar uma divergência entre:

Desired State
        versus
Observed State

Exemplo:

Desired:
22°C

Observed:
25°C

Reconciliation Intent:
"avaliar convergência para 22°C"

Reconciliation Intent não é uma autorização.

Não é um comando físico.

Não é necessariamente um Workflow.

O Planner/Workflow decide como uma intenção autorizada poderá ser executada.

---

28. Relações fundamentais

A arquitetura deve preservar:

Capability
    ≠
Permission
    ≠
Authorization
    ≠
Credential

Operation
    ≠
Attempt
    ≠
Execution
    ≠
Effect
    ≠
Evidence
    ≠
Verification

Context
    ≠
Memory
    ≠
Knowledge

Mission
    ≠
Task
    ≠
Workflow

Desired State
    ≠
Observed State
    ≠
Reconciliation Intent

---

29. Identificadores

A implementação deverá manter separados os identificadores com significados diferentes.

Conceitualmente:

entity_id
operation_id
attempt_id
execution_id
event_id
evidence_id
correlation_id
causation_id
idempotency_key

Eles não devem ser tratados como um único identificador universal.

operation_id

Identifica a Operation lógica.

attempt_id

Identifica uma tentativa específica.

execution_id

Identifica uma execução/runtime específico.

event_id

Identifica um evento.

evidence_id

Identifica uma evidência.

correlation_id

Relaciona eventos/operações dentro de uma missão ou fluxo maior.

causation_id

Representa a causa/origem lógica de um evento ou operação.

idempotency_key

Identifica uma operação para fins de deduplicação conforme o contrato do destino.

A Yuki não deve impor uma fórmula universal para "idempotency_key".

---

30. Epistemic State

Informações utilizadas pela Yuki devem poder expressar seu estado epistemológico.

Exemplos:

DECLARED
CONFIRMED
VERIFIED
VALIDATED
UNVERIFIED
INFERRED
HYPOTHESIS
STALE
CONFLICTING
UNKNOWN
TAINTED_EXTERNAL

Esses estados não devem ser confundidos com:

- autorização;
- confiança criptográfica;
- lifecycle;
- prioridade.

---

31. Provenance

Informações importantes devem permitir identificar sua origem.

Conceitualmente:

Provenance
├── source
├── origin
├── author
├── timestamp
├── transformations
├── evidence_refs
└── integrity_metadata

Provenance não transforma automaticamente uma informação em verdade.

---

32. Data Classification

Dados podem possuir classificação de sensibilidade.

A taxonomia arquitetural inicial inclui:

PUBLIC
PERSONAL
SENSITIVE
RESTRICTED
SECRET

A classificação não deve, sozinha, determinar o destino do dado.

O acesso depende de:

Purpose
+
Policy
+
Authorization
+
Sensitivity
+
Processing Environment
+
Risk

---

33. Data Projection

Data Projection representa uma representação minimizada dos dados necessários para uma tarefa.

Exemplo:

Fonte original:
100 campos

       ↓

Projection:
3 campos necessários para a tarefa

Projection reduz exposição.

Projection não concede autorização.

---

34. Capability Token

Um Capability Token representa uma autorização operacional limitada ou uma prova técnica de autorização conforme o sistema de segurança.

Ele deve possuir escopo limitado.

Pode incluir:

subject
capability
target
scope
conditions
expiration
audience
authority_domain

O formato concreto permanece aberto.

---

35. Contract Versioning

Contratos fundamentais devem possuir versão.

Alterações incompatíveis não devem ser feitas silenciosamente.

Exemplo:

Context v1
Context v2

Mudanças incompatíveis devem possuir:

- migration;
- compatibility layer;
- version negotiation;
- ou decisão arquitetural correspondente.

---

36. Serialization

Os contratos não devem assumir antecipadamente:

JSON
YAML
Protobuf
MessagePack
CBOR
database row

Esses são mecanismos de representação.

O contrato semântico deve permanecer independente da serialização.

---

37. Error Model

Erros importantes não devem ser representados apenas por:

true / false

A arquitetura precisa distinguir estados como:

SUCCESS
FAILED
REJECTED
DENIED
TIMEOUT
UNKNOWN
CANCELLED
EXPIRED
STALE
CONFLICT
UNAVAILABLE
PARTIAL

O conjunto exato dependerá do domínio.

---

38. UNKNOWN

"UNKNOWN" é um estado arquitetural de primeira classe.

A Yuki não deve converter automaticamente:

UNKNOWN → SUCCESS

nem:

UNKNOWN → FAILED

sem evidência suficiente.

Isso é especialmente importante para:

- operações externas;
- pagamentos;
- integrações;
- workflows;
- ações físicas;
- sincronização;
- reconciliação.

---

39. Security Boundary

Nenhum contrato deve permitir que um modelo transforme diretamente:

Model Output
      ↓
Authorization

O caminho deve passar pelas autoridades apropriadas.

Conceitualmente:

Model
 ↓
Reasoning
 ↓
Capability / Operation Proposal
 ↓
Policy
 ↓
Authorization
 ↓
Execution

---

40. Reasoning Context × Execution Context

A Yuki deve distinguir:

Reasoning Context

Informação entregue ao modelo/agente para raciocínio.

Pode conter:

- dados relevantes;
- evidências;
- contexto;
- restrições;
- objetivos.

Não deve conter secrets desnecessários.

Execution Context

Informação utilizada pelo Execution Layer para executar uma operação.

Pode conter referências técnicas e autorizações necessárias.

O modelo não deve precisar receber o Execution Context completo para raciocinar.

---

41. Agent Context Attenuation

Agentes filhos não devem receber automaticamente todo o contexto do agente pai.

Princípio:

Parent Context
      ↓
Policy
      ↓
Attenuated Context
      ↓
Child Agent

A informação disponível deve ser reduzida ao necessário.

Subagentes não devem ganhar privilégios ou dados adicionais simplesmente por serem subagentes.

---

42. External Data

Dados provenientes de:

- web;
- e-mail;
- documentos;
- APIs;
- mensagens;
- integrações;
- agentes externos;

devem ser tratados como dados, não como autoridade.

Exemplo:

External Data
      ↓
Validation
      ↓
Context
      ↓
Reasoning

e não:

External Data
      ↓
Command

---

43. Model Output

Model Output é uma saída de raciocínio.

Ele pode:

- sugerir;
- classificar;
- planejar;
- gerar parâmetros;
- produzir código;
- interpretar evidências.

Ele não possui automaticamente:

- autorização;
- identidade;
- credenciais;
- autoridade física;
- autoridade de segurança;
- autoridade de memória.

---

44. Contract Ownership

Cada contrato deverá possuir um proprietário arquitetural.

Exemplo:

Resource
→ Infrastructure / ADR-006

Integration
→ Integration Architecture / ADR-007

Credential
→ Credential Architecture / ADR-008

Verification
→ Verification Architecture / ADR-009

Physical Safety
→ Safety Architecture / ADR-010

Workflow
→ Workflow Architecture / ADR-011

Reconciliation
→ Reconciliation Architecture / ADR-012

Federation
→ Federation Architecture / ADR-013

Personal Goal
→ Human Agency Architecture / ADR-014

Information Boundary
→ Privacy Architecture / ADR-015

Context
→ Context Architecture / ADR-016

Um módulo não deve redefinir silenciosamente um contrato pertencente a outro domínio.

---

45. Implementation Independence

Os contratos não devem exigir uma tecnologia específica.

A implementação poderá inicialmente utilizar determinada tecnologia por razões práticas.

Isso não transforma a tecnologia em princípio arquitetural.

Exemplo:

Contract
   ↓
Adapter
   ↓
Google ADK

ou:

Contract
   ↓
Adapter
   ↓
Outro Runtime

Ambos devem ser possíveis sem alterar o significado do contrato.

---

46. Testing Requirements

Cada contrato implementado deverá possuir testes.

Os testes devem verificar:

Structural

O objeto possui os campos e relações esperados.

Semantic

O objeto respeita seu significado arquitetural.

Security

O objeto não permite escalada de privilégio.

Boundary

O objeto não assume responsabilidade pertencente a outro componente.

Compatibility

Mudanças de versão não quebram silenciosamente consumidores.

---

47. Contract Invariants

Os seguintes invariantes são fundamentais:

INV-FND-001
Capability ≠ Permission

INV-FND-002
Permission ≠ Authorization

INV-FND-003
Credential ≠ Authorization

INV-FND-004
Model Output ≠ Authorization

INV-FND-005
Context ≠ Authorization

INV-FND-006
Context ≠ Memory

INV-FND-007
Memory ≠ Knowledge

INV-FND-008
Operation ≠ Attempt

INV-FND-009
Attempt ≠ Execution

INV-FND-010
Execution ≠ Effect

INV-FND-011
Effect ≠ Evidence

INV-FND-012
Evidence ≠ Verification

INV-FND-013
Desired State ≠ Observed State

INV-FND-014
Reconciliation Intent ≠ Authorization

INV-FND-015
UNKNOWN ≠ SUCCESS

INV-FND-016
UNKNOWN ≠ FAILURE

INV-FND-017
External Data ≠ Instruction

INV-FND-018
External Platform ≠ Yuki Authority

INV-FND-019
Resource ≠ Permission

INV-FND-020
Storage Location ≠ Authority

---

48. First implementation target

A primeira implementação dos contratos deverá ser pequena.

Não é necessário implementar todos os objetos simultaneamente.

Ordem inicial recomendada:

1. Identifiers
        ↓
2. Errors / States
        ↓
3. Capability
        ↓
4. Permission
        ↓
5. Authorization
        ↓
6. Operation
        ↓
7. Attempt
        ↓
8. Evidence
        ↓
9. VerificationResult
        ↓
10. Context
        ↓
11. Integration

Depois:

Mission
Task
Workflow
Resource
DesiredState
ObservedState
ReconciliationIntent
Memory
Knowledge

---

49. Definition of Done

Um Foundation Contract será considerado implementado quando:

[ ] definição arquitetural existe
[ ] tipo/interface implementado
[ ] serialização definida quando necessária
[ ] validação implementada
[ ] estados inválidos tratados
[ ] testes unitários existentes
[ ] testes de fronteira existentes
[ ] invariantes verificadas
[ ] documentação atualizada
[ ] compatibilidade avaliada

---

50. Relação com ADRs

Este documento não substitui os ADRs.

A relação é:

ADR
 ↓
Decisão arquitetural
 ↓
Foundation Contract
 ↓
Implementação

ADRs respondem principalmente:

«Por que a arquitetura funciona desta maneira?»

Foundation Contracts respondem:

«Quais conceitos o software precisa compartilhar para implementar essa arquitetura?»

---

51. Evolução futura

Os contratos poderão evoluir.

A evolução deve preservar:

- compatibilidade;
- segurança;
- autoridade;
- rastreabilidade;
- migração;
- versionamento.

Quando uma alteração modificar uma decisão arquitetural, o ADR correspondente deverá ser atualizado ou um novo ADR deverá ser criado.

---

52. Status

Version: 0.1

Status: FOUNDATION DRAFT

Implementation Status: Not implemented

Next Step:

FOUNDATION CONTRACTS
        ↓
PROJECT STRUCTURE
        ↓
MINIMAL CORE
        ↓
FIRST END-TO-END FLOW

---

53. Princípio final

«Os contratos da Yuki devem ser estáveis o suficiente para permitir evolução, mas flexíveis o suficiente para não aprisionar a arquitetura a uma tecnologia específica.»

«A implementação pode mudar. O significado arquitetural deve permanecer explícito.»