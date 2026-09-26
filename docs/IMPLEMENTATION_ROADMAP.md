Yuki — Implementation Roadmap

Version: 0.1
Status: ACTIVE
Domain: Implementation / Engineering
Date: 2026-09-26

---

1. Objetivo

Este documento define o roadmap oficial para transformar a arquitetura da Yuki em uma implementação funcional.

A arquitetura da Yuki já possui uma base extensa de decisões documentadas.

O próximo desafio é transformar:

Princípios
    ↓
Arquitetura
    ↓
ADRs
    ↓
Contratos
    ↓
Código
    ↓
Testes
    ↓
Sistema funcional

O roadmap evita que a implementação cresça de maneira desordenada.

---

2. Estado inicial

Estado atual:

Arquitetura
████████████████████████████████████████  definida

ADRs
████████████████████████████████████████  definidos

Foundation Contracts
████████████████████░░░░░░░░░░░░░░░░░░░  definidos

Código
░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░  não iniciado

Testes
░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░  não iniciado

Produção
░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░  não iniciada

A Yuki está, portanto, entrando na transição:

«Architecture → Engineering»

---

3. Princípio central

A implementação não deve tentar construir toda a Yuki de uma vez.

A estratégia será:

«Construir uma fundação pequena, correta e testável; depois crescer por composição.»

Arquitetura:

Foundation
    ↓
Core
    ↓
Capabilities
    ↓
Execution
    ↓
Verification
    ↓
Workflow
    ↓
Memory / Knowledge
    ↓
Agents
    ↓
Events
    ↓
Reconciliation
    ↓
Federation
    ↓
Evolution

---

4. Regra de dependência

Um componente não deve depender de outro apenas porque ambos são conceitualmente relacionados.

A dependência deve ser justificada por contrato.

Exemplo:

Core
  ↓
Capability Contract
  ↓
Capability Implementation

e não:

Core
  ↓
importar diretamente
uma implementação específica

Isso preserva substituibilidade.

---

5. Princípio de vertical slice

A Yuki não deve permanecer por meses apenas criando abstrações.

Após uma quantidade mínima de contratos, deve existir um fluxo funcional pequeno.

Primeiro objetivo:

User Input
    ↓
Core
    ↓
Context
    ↓
Model
    ↓
Capability Proposal
    ↓
Policy
    ↓
Execution
    ↓
Verification
    ↓
Result

Esse fluxo será chamado de:

Foundation Vertical Slice

---

6. Fase 0 — Repository & Documentation Foundation

Objetivo

Garantir que o repositório seja uma fonte confiável da arquitetura antes do início do código.

Estado

Parcialmente concluída.

Tarefas

[ ] atualizar STATUS_ATUAL
[ ] Foundation Contracts
[ ] Implementation Roadmap
[ ] revisar README
[ ] corrigir referências para documentos inexistentes
[ ] verificar índice de ADRs
[ ] definir convenções de código
[ ] definir convenções de testes
[ ] definir estrutura inicial do projeto

Critério de saída

O repositório deve permitir que um desenvolvedor ou agente de desenvolvimento compreenda:

- o que é a Yuki;
- quais decisões existem;
- quais contratos existem;
- o que já foi implementado;
- o que ainda não foi implementado;
- onde começar.

---

7. Fase 1 — Foundation Contracts

Objetivo

Transformar os conceitos arquiteturais fundamentais em contratos implementáveis.

Contratos iniciais

Identifier
Error
State
Capability
Permission
Authorization
CredentialReference
Operation
Attempt
Evidence
VerificationResult
Context
Integration

Tarefas

[ ] definir interfaces/tipos
[ ] definir estados
[ ] definir validação
[ ] definir versionamento
[ ] definir serialização
[ ] criar testes
[ ] validar invariantes

Critério de saída

Cada contrato deverá:

- possuir representação no código;
- possuir validação;
- possuir testes;
- possuir documentação;
- respeitar os invariantes da Foundation.

---

8. Fase 2 — Project Skeleton

Objetivo

Criar a primeira estrutura real do software.

A estrutura concreta ainda deverá permanecer aberta o suficiente para permitir evolução.

Uma estrutura inicial possível:

yuki/
├── README.md
├── docs/
├── src/
│   ├── core/
│   ├── contracts/
│   ├── context/
│   ├── capabilities/
│   ├── security/
│   ├── execution/
│   ├── verification/
│   └── integrations/
├── tests/
└── ...

Essa estrutura é inicial e não deve ser considerada constitucional.

Tarefas

[ ] criar projeto
[ ] configurar build
[ ] configurar testes
[ ] configurar lint/format
[ ] configurar CI
[ ] configurar versionamento
[ ] configurar logging estruturado
[ ] configurar configuração externa

Critério de saída

O projeto deve:

buildar
+
executar
+
testar
+
passar CI

---

9. Fase 3 — Minimal Yuki Core

Objetivo

Criar o primeiro Core executável.

O Core inicial deve ser pequeno.

Ele deverá:

- receber uma solicitação;
- criar/receber contexto;
- identificar a intenção;
- coordenar componentes;
- produzir uma proposta de ação;
- encaminhar a proposta às camadas apropriadas.

O Core não deve:

- armazenar secrets;
- conceder autorização;
- executar diretamente sistemas externos;
- controlar hardware diretamente;
- substituir o Security Controller;
- substituir o Verification Engine.

---

10. Fase 4 — Context Foundation

Objetivo

Implementar o primeiro Context Builder compatível com ADR-015 e ADR-016.

Fluxo:

Request
   ↓
Purpose
   ↓
Information Access
   ↓
Retrieval
   ↓
Data Projection
   ↓
Context Validation
   ↓
Typed Context

Tarefas

[ ] Context Request
[ ] Context Builder
[ ] Typed Context Object
[ ] provenance
[ ] freshness
[ ] epistemic state
[ ] data classification
[ ] context lifecycle
[ ] context attenuation
[ ] tests

---

11. Fase 5 — Model Router

Objetivo

Criar a primeira abstração independente de modelo.

Arquitetura:

Yuki Core
    ↓
Model Router
    ↓
Model Adapter
    ↓
Model Provider

O Core não deve chamar diretamente:

OpenAI
Google
Anthropic
local model

A chamada deve passar pelo Model Router e pelos adapters apropriados.

---

12. Fase 6 — Capability System

Objetivo

Implementar o Capability Registry e os contratos de Capability.

Fluxo:

Capability Registry
       ↓
Capability Discovery
       ↓
Capability Selection
       ↓
Capability Proposal

O sistema deve distinguir:

Capability
Permission
Authorization
Credential

---

13. Fase 7 — Security Boundary

Objetivo

Criar a primeira fronteira de segurança operacional.

Inicialmente:

Core
 ↓
Policy Evaluation
 ↓
Authorization
 ↓
Execution

O Security Controller não precisa inicialmente possuir todos os recursos descritos nos ADRs.

A primeira versão deve validar:

- identidade;
- escopo;
- capability;
- target;
- autorização;
- expiração;
- negação.

---

14. Fase 8 — Execution Layer

Objetivo

Criar o primeiro executor isolado.

Fluxo:

Operation
   ↓
Authorization
   ↓
Execution Request
   ↓
Execution Worker
   ↓
Result

O executor não deve assumir que uma autorização existe apenas porque recebeu uma proposta.

---

15. Fase 9 — Integration Gateway

Objetivo

Criar a primeira fronteira de comunicação externa.

Arquitetura:

Yuki
 ↓
Integration Gateway
 ↓
Connector / Adapter
 ↓
External System

O Core não deve fazer chamadas externas diretamente.

---

16. Fase 10 — First Real Integration

Objetivo

Implementar uma integração real simples.

A primeira integração deverá ser escolhida principalmente por:

- baixo risco;
- fácil reversibilidade;
- fácil teste;
- API estável;
- baixo impacto externo.

Exemplos possíveis:

Read-only API
Test API
Local service
Sandbox service

Não começar por:

Banco
Pagamentos
Robótica
Casa
Credenciais críticas

---

17. Fase 11 — Verification Engine

Objetivo

Garantir que:

Execution
    ≠
Effect

e:

Effect
    ≠
Verified Effect

Fluxo:

Execution
   ↓
External System
   ↓
Evidence
   ↓
Verification Engine
   ↓
Verification Result

Deve existir suporte para:

SUCCESS
FAILED
UNKNOWN
CONFLICTING
PARTIAL

conforme o domínio.

---

18. Fase 12 — Foundation Vertical Slice

Neste ponto será realizado o primeiro fluxo completo.

Exemplo:

USER
 ↓
CORE
 ↓
CONTEXT
 ↓
MODEL ROUTER
 ↓
CAPABILITY
 ↓
POLICY
 ↓
AUTHORIZATION
 ↓
EXECUTION
 ↓
INTEGRATION
 ↓
EXTERNAL SYSTEM
 ↓
EVIDENCE
 ↓
VERIFICATION
 ↓
RESULT

Critério de sucesso

A Yuki deverá conseguir realizar pelo menos uma tarefa real de baixo risco de ponta a ponta.

Além disso, deverá ser possível demonstrar:

authorization denied
authorization expired
execution failure
external timeout
UNKNOWN
verification success
verification failure

---

19. Fase 13 — Durable Workflow

Objetivo

Adicionar execução durável.

Base:

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
Execution

Suporte inicial:

- persistência;
- pause;
- resume;
- cancel;
- retry controlado;
- timeout;
- UNKNOWN;
- recovery.

Não implementar Saga completa imediatamente.

Primeiro validar o Workflow Engine.

---

20. Fase 14 — Memory

Objetivo

Criar memória persistente controlada.

Primeira versão:

Working Memory
Episodic Memory
Preferences
Project Memory

Não implementar imediatamente toda a arquitetura cognitiva.

Primeiro validar:

write
retrieve
update
supersede
expire
delete

---

21. Fase 15 — Knowledge

Objetivo

Criar a camada de conhecimento.

Separar:

Memory
Knowledge
External Knowledge
Context

O Knowledge Layer deverá alimentar o Context Builder.

---

22. Fase 16 — Agent System

Objetivo

Adicionar agentes somente depois que:

- Core;
- Context;
- Capability;
- Security;
- Execution;
- Verification;
- Workflow

estiverem funcionais.

Arquitetura:

Supervisor
 ↓
Mission
 ↓
Task Graph
 ↓
Agents
 ↓
Capabilities

Agentes não devem receber autoridade adicional por serem agentes.

---

23. Fase 17 — Events & Background

Objetivo

Implementar:

- Event Router;
- background tasks;
- watchers;
- scheduling;
- notifications;
- durable events;
- cancellation;
- backpressure;
- deduplication.

Primeiro uso:

Event
 ↓
Router
 ↓
Priority
 ↓
Task

Depois:

Event
 ↓
Attention Manager
 ↓
Yuki

---

24. Fase 18 — Reconciliation

Objetivo

Implementar o Reconciliation Engine.

Fluxo:

Desired State
+
Observed State
+
Policy
        ↓
Reconciliation
        ↓
Reconciliation Intent
        ↓
Planner
        ↓
Workflow

O Reconciliation Engine não executa diretamente.

---

25. Fase 19 — Distributed Federation

Objetivo

Adicionar:

- Home;
- Cloud;
- Edge;
- Mobile;
- synchronization;
- authority domains;
- leases;
- fencing;
- offline operation;
- state synchronization.

Esta fase só deve começar depois que a execução local estiver sólida.

---

26. Fase 20 — Physical World

Objetivo

Adicionar capacidades físicas.

Inclui:

- Physical Gateway;
- local controller;
- Safety Controller;
- hardware interlocks;
- sensors;
- actuator integration;
- physical effect verification.

Primeiro ambiente:

Simulation

Depois:

Hardware Lab

Somente posteriormente:

Real Physical Environment

---

27. Fase 21 — Voice & Multimodal

A arquitetura de voz já está documentada.

A implementação deverá acontecer progressivamente:

Text
 ↓
Speech Input
 ↓
Speech-to-Text
 ↓
Core
 ↓
Text Response
 ↓
Text-to-Speech

Depois:

Voice
+
Vision
+
Device Context
+
Multimodal Interaction

---

28. Fase 22 — Multi-device

Adicionar:

Mobile
Watch
Desktop
Earbuds
Home
Future Devices

Todos deverão acessar a mesma identidade Yuki.

Dispositivo não é uma Yuki separada.

---

29. Fase 23 — Evolution Manager

Somente depois que a fundação estiver estável.

Fluxo:

Detect
 ↓
Analyze
 ↓
Propose
 ↓
Prototype
 ↓
Test
 ↓
Security Review
 ↓
Approval
 ↓
Deploy
 ↓
Monitor
 ↓
Rollback

Agentes externos poderão atuar no Development Lab.

Nenhum agente recebe autoridade irrestrita para modificar a Yuki.

---

30. Fase 24 — Production Hardening

Antes de produção real:

Security Audit
Integration Tests
Load Tests
Failure Tests
Recovery Tests
Permission Tests
Prompt Injection Tests
Data Leakage Tests
Credential Tests
Workflow Recovery Tests
Verification Tests

Também:

Backup
Restore
Monitoring
Alerting
Rate Limits
Resource Limits
Audit
Incident Response

---

31. Ordem de prioridade

A prioridade geral é:

P0 — Foundation
P1 — Core
P2 — Security
P3 — Execution
P4 — Verification
P5 — Workflow
P6 — Memory / Knowledge
P7 — Agents
P8 — Events
P9 — Reconciliation
P10 — Federation
P11 — Physical
P12 — Evolution

Isso não significa que os itens P9–P12 sejam menos importantes arquiteturalmente.

Significa que eles dependem de fundações anteriores.

---

32. Dependências

Mapa simplificado:

Foundation
    │
    ├── Core
    │
    ├── Context
    │
    ├── Model Router
    │
    └── Capability
            │
            ▼
        Security
            │
            ▼
        Execution
            │
            ▼
       Integration
            │
            ▼
       Verification
            │
            ▼
        Workflow
            │
      ┌─────┴─────┐
      ▼           ▼
   Memory      Knowledge
      │           │
      └─────┬─────┘
            ▼
          Agents
            │
            ▼
          Events
            │
            ▼
      Reconciliation
            │
            ▼
        Federation

Physical Safety permanece como uma autoridade transversal e independente.

---

33. Definition of Done — módulo

Nenhum módulo será considerado concluído apenas porque “funciona”.

Um módulo deverá possuir:

[ ] contrato definido
[ ] implementação
[ ] testes unitários
[ ] testes de integração quando aplicável
[ ] tratamento de erros
[ ] observabilidade
[ ] limites
[ ] documentação
[ ] segurança
[ ] versionamento
[ ] revisão arquitetural

---

34. Definition of Done — fase

Uma fase será considerada concluída quando:

1. seus contratos estiverem definidos;
2. sua implementação estiver funcional;
3. seus testes principais estiverem passando;
4. suas fronteiras estiverem respeitadas;
5. suas dependências estiverem documentadas;
6. o sistema anterior continuar funcionando;
7. não houver regressão arquitetural conhecida.

---

35. Vertical Slices

O desenvolvimento deverá utilizar vertical slices.

Exemplo:

Slice 1
User
→ Core
→ Model
→ Response

Depois:

Slice 2
User
→ Core
→ Capability
→ Policy
→ Execution
→ Result

Depois:

Slice 3
User
→ Core
→ Integration
→ External System
→ Verification

Cada slice aumenta a capacidade real da Yuki.

---

36. Development Agent Workflow

Agentes de desenvolvimento externos poderão executar tarefas seguindo:

ADR
 ↓
Implementation Specification
 ↓
Development Agent
 ↓
Isolated Branch
 ↓
Code
 ↓
Tests
 ↓
CI
 ↓
Architecture Review
 ↓
Security Review
 ↓
Human Review
 ↓
Merge

O agente não deve decidir sozinho:

- mudanças arquiteturais críticas;
- expansão de privilégios;
- alteração do Security Controller;
- alteração da autoridade física;
- alteração das regras de agência humana;
- remoção de controles.

---

37. Reutilização tecnológica

A Yuki seguirá:

REUSE
   ↓
ADAPT
   ↓
EXTEND
   ↓
REPLACE
   ↓
REIMPLEMENT

Não se deve implementar do zero algo que já exista de maneira suficientemente madura sem uma razão arquitetural ou técnica clara.

Porém:

Technology
    ≠
Architecture

e:

Provider
    ≠
Authority

---

38. Google Ecosystem

Quando adequado, a implementação poderá utilizar:

- Gemini;
- Google ADK;
- Antigravity;
- MCP;
- A2A;
- Vertex AI;
- Cloud Run;
- outros serviços.

Essas tecnologias devem entrar através dos contratos e adapters da Yuki.

Exemplo:

Yuki Model Router
       ↓
Model Adapter
       ↓
Gemini

e:

Yuki Agent Runtime Contract
       ↓
ADK Adapter
       ↓
Google ADK

O mesmo contrato deverá permitir substituir o componente posteriormente.

---

39. Technology Exit Test

Antes de tornar uma tecnologia estrutural, perguntar:

«Se esta tecnologia desaparecer amanhã, quanto da Yuki precisa ser reescrito?»

Objetivo:

Pouca coisa.

Se a resposta for:

"Precisamos reescrever o Core."

a integração está arquiteturalmente acoplada demais.

---

40. Architecture Fitness Tests

A implementação deverá periodicamente responder:

Provider Independence

Podemos trocar o provedor de modelo?

Runtime Independence

Podemos trocar o runtime de agentes?

Storage Independence

Podemos trocar a tecnologia de armazenamento?

Integration Independence

Podemos substituir uma integração sem alterar o Core?

Security Independence

Um componente externo comprometido consegue assumir autoridade sobre a Yuki?

Failure Isolation

A falha de uma Capability derruba o Core?

---

41. Security Gates

Antes de integrar uma nova capacidade:

Capability
 ↓
Risk Classification
 ↓
Permissions
 ↓
Isolation
 ↓
Testing
 ↓
Verification
 ↓
Registry

Antes de integrar uma plataforma externa:

Provider
 ↓
Trust Assessment
 ↓
Adapter
 ↓
Data Boundary
 ↓
Permission Boundary
 ↓
Failure Isolation
 ↓
Testing

---

42. First Production Principle

A primeira versão de produção da Yuki não precisa possuir todas as capacidades imaginadas.

Ela precisa possuir:

Poucas capacidades
+
Arquitetura correta
+
Segurança
+
Observabilidade
+
Testabilidade
+
Capacidade de evolução

É preferível:

10 capacidades bem isoladas

a:

100 capacidades acopladas

---

43. Incremental Capability Expansion

Após o primeiro Vertical Slice:

Capability 1
    ↓
Capability 2
    ↓
Capability 3
    ↓
Integration 1
    ↓
Integration 2
    ↓
Background
    ↓
Agents

Cada nova capacidade deve reutilizar a fundação.

Não criar um novo mini-framework para cada funcionalidade.

---

44. Roadmap resumido

                    YUKI
                      │
             ┌────────▼────────┐
             │   FOUNDATION     │
             └────────┬────────┘
                      │
             ┌────────▼────────┐
             │      CORE       │
             └────────┬────────┘
                      │
          ┌───────────┼───────────┐
          ▼           ▼           ▼
       CONTEXT      MODELS    CAPABILITIES
          │           │           │
          └───────────┼───────────┘
                      ▼
                  SECURITY
                      │
                      ▼
                  EXECUTION
                      │
                      ▼
                INTEGRATIONS
                      │
                      ▼
                 VERIFICATION
                      │
                      ▼
                  WORKFLOW
                      │
             ┌────────┴────────┐
             ▼                 ▼
          MEMORY           KNOWLEDGE
             └────────┬────────┘
                      ▼
                    AGENTS
                      │
                    EVENTS
                      │
               RECONCILIATION
                      │
                  FEDERATION
                      │
               PHYSICAL WORLD
                      │
                  EVOLUTION

---

45. Primeiro grande marco

O primeiro grande marco será:

YUKI FOUNDATION v0.1

Quando alcançado:

[✓] Foundation Contracts
[✓] Project Structure
[✓] Minimal Core
[✓] Context
[✓] Model Adapter
[✓] Capability
[✓] Basic Policy
[✓] Basic Authorization
[✓] Execution
[✓] Verification
[✓] Tests
[✓] CI

e pelo menos um:

END-TO-END TEST

funcionando.

---

46. Segundo grande marco

YUKI RUNTIME v0.1

Inclui:

Core
Context
Models
Capabilities
Security
Execution
Integration
Verification
Workflow
Memory
Knowledge

com uma pequena quantidade de funcionalidades reais.

---

47. Terceiro grande marco

YUKI AGENT v0.1

Inclui:

Agent
Supervisor
Mission
Task Graph
Tools
Workflow
Context
Memory
Verification

---

48. Quarto grande marco

YUKI CONTINUOUS v0.1

Inclui:

Events
Background Tasks
Attention
Scheduling
Notifications
Monitoring
Reconciliation

---

49. Quinto grande marco

YUKI DISTRIBUTED v0.1

Inclui:

Edge
Home
Cloud
Federation
Authority Domains
Offline
Synchronization
Recovery

---

50. Sexto grande marco

YUKI PHYSICAL v0.1

Inclui:

Physical Gateway
Safety Controller
Sensors
Actuators
Interlocks
Physical Verification
Simulation

---

51. Sétimo grande marco

YUKI EVOLUTION v0.1

Inclui:

Development Lab
Experimentation
Benchmarking
Security Review
Automated Testing
Canary
Rollback
Evolution Manager

---

52. Estado do roadmap

PHASE 0   Repository Foundation       🟡
PHASE 1   Foundation Contracts        🟡
PHASE 2   Project Skeleton            🔴
PHASE 3   Minimal Core                🔴
PHASE 4   Context                     🔴
PHASE 5   Model Router                🔴
PHASE 6   Capability System           🔴
PHASE 7   Security Boundary           🔴
PHASE 8   Execution                   🔴
PHASE 9   Integration Gateway         🔴
PHASE 10  First Integration            🔴
PHASE 11  Verification                🔴
PHASE 12  Vertical Slice              🔴
PHASE 13  Durable Workflow            🔴
PHASE 14  Memory                      🔴
PHASE 15  Knowledge                   🔴
PHASE 16  Agents                      🔴
PHASE 17  Events                      🔴
PHASE 18  Reconciliation              🔴
PHASE 19  Federation                  🔴
PHASE 20  Physical                    🔴
PHASE 21  Voice / Multimodal          🔴
PHASE 22  Multi-device                🔴
PHASE 23  Evolution                   🔴
PHASE 24  Production Hardening        🔴

Legenda:

✅ Concluído
🟡 Em andamento
🔴 Não iniciado

---

53. Regra contra complexidade prematura

A implementação deve evitar criar infraestrutura para problemas que ainda não existem.

Exemplo:

Não implementar inicialmente:

distributed consensus
multi-region federation
complex CRDT infrastructure
massive agent swarm
full robotics stack

se a primeira versão ainda não consegue:

receber uma solicitação
→ raciocinar
→ propor uma ação
→ autorizar
→ executar
→ verificar
→ responder

A arquitetura deve ser preparada para crescer.

A implementação deve crescer conforme a necessidade.

---

54. Regra contra simplificação perigosa

O inverso também é proibido.

Não simplificar removendo fronteiras fundamentais.

Não fazer:

Core → API

quando a arquitetura exige:

Core
 ↓
Capability
 ↓
Policy
 ↓
Authorization
 ↓
Execution
 ↓
Integration
 ↓
Verification

A implementação pode ser pequena.

A separação arquitetural deve permanecer.

---

55. Critério de maturidade

A Yuki será considerada madura progressivamente.

Não haverá um único momento de:

«“Agora a Yuki está pronta.”»

A maturidade será incremental:

M0 — Architecture
M1 — Foundation
M2 — Functional Runtime
M3 — Agent Runtime
M4 — Continuous Runtime
M5 — Distributed Runtime
M6 — Physical Integration
M7 — Controlled Evolution

---

56. Regra final

A Yuki deverá sempre crescer nesta direção:

mais capacidade
+
mais verificabilidade
+
mais isolamento
+
mais reutilização
+
mais observabilidade
+
mais autonomia controlada

e não simplesmente:

mais código.

---

57. Próximo passo imediato

Após este documento, a implementação deverá começar.

O próximo documento técnico será:

docs/IMPLEMENTATION_SPEC_FOUNDATION_V0.1.md

Ele deverá transformar o roadmap em uma especificação executável para o primeiro agente de desenvolvimento.

Esse documento deverá definir:

- estrutura inicial de diretórios;
- linguagem/runtime inicial;
- ferramentas de build;
- testes;
- CI;
- contratos que entram no primeiro release;
- primeiro Vertical Slice;
- critérios de aceitação;
- limites do agente de implementação.

---

58. Princípio final

«A Yuki não será construída de uma vez. Ela será construída corretamente, camada por camada, mantendo a arquitetura intacta enquanto suas capacidades crescem.»

«Primeiro uma fundação pequena. Depois uma Yuki cada vez maior.»