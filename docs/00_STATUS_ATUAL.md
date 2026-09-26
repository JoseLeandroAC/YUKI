Yuki — Status Atual do Projeto

Última atualização: 2026-09-26
Status geral: Foundation v0.1 / MVP-0 implementada, auditada e testada; base consolidada para MVP-1
Repositório: "JoseLeandroAC/YUKI"
Branch de referência: "feature/foundation-v0.1" (incorporada em "main")

---

1. Estado atual

A Yuki concluiu sua principal fase inicial de descoberta e definição arquitetural, bem como a implementação, auditoria e consolidação da **Foundation v0.1 / MVP-0**.

O projeto possui atualmente uma arquitetura conceitual extensa documentada em ADRs e uma primeira célula funcional de software implementada em Rust puro (1.98.1), com testes de integração positivos e negativos protegendo os invariantes fundamentais.

A situação atual pode ser resumida como:

ARQUITETURA
████████████████████████████████████████
DEFINIDA E DOCUMENTADA (ADRs 006 a 017)

CONTRATOS DE IMPLEMENTAÇÃO (FOUNDATION v0.1)
████████████████████████████████████████
DEFINIDOS, IMPLEMENTADOS E AUDITADOS

FOUNDATION v0.1 / MVP-0 (SOFTWARE & PIPELINE VERTICAL SLICE)
████████████████████████████████████████
IMPLEMENTADA, AUDITADA E CONSOLIDADA

TESTES DE INTEGRAÇÃO & SEGURANÇA (NEGATIVOS)
████████████████████████████████████████
26/26 TESTES PASSANDO (TESTES A AO I + NEGATIVOS DE VIOLAÇÃO)

PROVEDORES REAIS DE MODELO / RUNTIME ASSÍNCRONO / INTEGRAÇÕES
░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░
PLANEJADOS PARA MVP-1 (NÃO INICIADOS)

PRODUÇÃO
░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░
NÃO INICIADA

---

2. O que a Yuki já possui

A Yuki já possui definições arquiteturais para:

- visão e princípios;
- agentes e tarefas;
- sistema de capacidades;
- Model Router;
- segurança;
- evolução;
- voz e multimodalidade;
- eventos e execução em background;
- infraestrutura;
- integrações;
- Resource Model;
- Integration Model;
- Credential Isolation;
- External Action Verification;
- Physical World Safety;
- Durable Workflow;
- Reconciliation;
- Home/Cloud Federation;
- Personal Goal Reconciliation;
- Privacy & Data Minimization;
- Context & Knowledge;
- External Platform Reuse & Vendor Independence.

Essas definições devem ser tratadas como contratos arquiteturais, não como prova de que a funcionalidade já está implementada.

---

3. ADRs oficiais atuais

A arquitetura atual possui os seguintes ADRs principais:

ADR-006
Yuki Infrastructure Resource Model

ADR-007
Yuki Integration Model & Manifest

ADR-008
Yuki Credential Isolation & Secret Delivery

ADR-009
Yuki External Action Verification

ADR-010
Yuki Physical World Safety & Hardware Interlocks

ADR-011
Yuki Durable Workflow, Saga & Compensation

ADR-012
Yuki Reconciliation Engine

ADR-013
Yuki Home/Cloud Event Store Synchronization & Offline State Federation

ADR-014
Yuki Personal Goal Reconciliation & Human Agency

ADR-015
Yuki Privacy, Data Minimization & Information Boundaries

ADR-016
Yuki Context & Knowledge Architecture

ADR-017
Yuki External Platform Reuse, Runtime Adapters & Vendor Independence

Os ADRs posteriores complementam e refinam os anteriores.

Quando houver conflito entre documentos antigos e decisões posteriores formalizadas, a arquitetura deverá seguir a decisão mais recente e explicitamente aceita.

---

4. Arquitetura versus implementação

Uma distinção fundamental passa a ser adotada neste documento:

PRINCÍPIO
    ↓
ARQUITETURA
    ↓
CONTRATO
    ↓
IMPLEMENTAÇÃO
    ↓
TESTE
    ↓
PRODUÇÃO

A existência de um documento arquitetural não significa que seu componente correspondente exista no software.

Exemplo:

ADR-008 define Credential Isolation
                 ↓
não significa
                 ↓
que o Credential Broker já esteja implementado.

Da mesma forma:

ADR-012 define Reconciliation
                 ↓
não significa
                 ↓
que exista um Reconciliation Engine funcional.

Essa distinção deve permanecer explícita para evitar falsa sensação de progresso.

---

5. Estado da implementação

No estado atual do repositório, a Yuki ainda não possui uma implementação estrutural consolidada do runtime.

Ainda não existe uma base de código que represente integralmente:

Yuki Core
Context
Capability System
Security Controller
Model Router
Execution Layer
Integration Gateway
Verification Engine
Workflow Engine
Memory
Knowledge
Reconciliation
Agent System
Event System
Infrastructure Runtime

Portanto, a próxima fase do projeto não será uma grande refatoração de código existente.

Será a construção da primeira implementação estrutural da Yuki.

---

6. Próxima fase: Foundation

A primeira fase de implementação será chamada de:

Yuki Foundation

Seu objetivo é criar as abstrações e contratos fundamentais sobre os quais os demais componentes poderão ser construídos.

A Foundation não deve tentar implementar toda a Yuki.

Ela deve estabelecer:

Core
Contracts
Types
Configuration
Capability abstraction
Context abstraction
Execution abstraction
Authorization abstraction
Observability foundation
Testing foundation

---

7. Ordem inicial de implementação

A ordem inicial recomendada é:

1. Repository / Documentation Consistency
             ↓
2. Foundation Contracts
             ↓
3. Project Structure
             ↓
4. Core
             ↓
5. Context
             ↓
6. Model Router
             ↓
7. Capability System
             ↓
8. Security / Policy Boundary
             ↓
9. Execution Layer
             ↓
10. Integration Gateway
             ↓
11. Verification Engine
             ↓
12. Workflow
             ↓
13. Memory / Knowledge
             ↓
14. Agents
             ↓
15. Events / Background
             ↓
16. Reconciliation
             ↓
17. Federation
             ↓
18. Evolution

Essa ordem não significa que os componentes posteriores não possam ser prototipados antes.

Ela define a ordem da fundação estrutural, evitando que componentes de alto nível criem contratos incompatíveis com o núcleo.

---

8. Primeiro objetivo funcional

O primeiro objetivo não será criar uma Yuki extremamente inteligente.

Será criar uma Yuki arquiteturalmente correta e funcional em pequena escala.

O primeiro fluxo deverá ser capaz de representar algo semelhante a:

USER
 ↓
YUKI CORE
 ↓
CONTEXT
 ↓
MODEL ROUTER
 ↓
CAPABILITY
 ↓
SECURITY / AUTHORIZATION
 ↓
EXECUTION
 ↓
VERIFICATION
 ↓
RESULT

Mesmo que inicialmente exista apenas uma capacidade simples.

O objetivo é validar a arquitetura através de software real.

---

9. Contratos fundamentais

Antes de expandir o runtime, deverão ser definidos contratos centrais para conceitos como:

TypedContext
Capability
Permission
Authorization
Operation
Attempt
Evidence
VerificationResult
Integration
CredentialReference
Mission
Task
Workflow
Resource
DesiredState
ObservedState
ReconciliationIntent

Esses conceitos não devem ser duplicados de forma incompatível entre módulos.

Cada conceito fundamental deve possuir uma definição canônica ou uma interface claramente estabelecida.

---

10. Regra de implementação

A implementação deverá seguir:

ADR
 ↓
Contract
 ↓
Implementation
 ↓
Tests
 ↓
Review
 ↓
Integration

Nenhum agente de desenvolvimento deve inventar silenciosamente uma nova arquitetura para preencher uma lacuna.

Quando uma implementação exigir uma decisão arquitetural ainda não definida:

Implementação encontra lacuna
        ↓
registra a lacuna
        ↓
proposta arquitetural
        ↓
revisão
        ↓
ADR/decisão, quando necessário
        ↓
implementação

---

11. Google e plataformas externas

A Yuki adotará o princípio:

«Reutilizar tecnologia madura antes de reimplementá-la.»

Porém:

«Reutilizar uma implementação não significa transferir a ela a autoridade arquitetural da Yuki.»

Plataformas externas como:

- Gemini;
- Google ADK;
- Google Agent Platform;
- MCP;
- A2A;
- Cloud Run;
- Vertex AI;
- outros modelos;
- outros runtimes;
- outros provedores;

podem ser utilizadas através das abstrações e adaptadores definidos pela Yuki.

Nenhuma dessas tecnologias deve se tornar uma dependência constitucional do Core sem uma decisão arquitetural explícita.

---

12. Agentes de desenvolvimento

Agentes externos de desenvolvimento poderão atuar como implementadores dentro de um ambiente controlado.

Modelo:

Yuki Architecture
        ↓
Development Specification
        ↓
Development Agent
        ↓
Branch
        ↓
Implementation
        ↓
Tests
        ↓
Security / Architecture Review
        ↓
Human Approval
        ↓
Merge

O agente de desenvolvimento não recebe autoridade automática sobre:

- arquitetura;
- segurança;
- identidade;
- permissões;
- produção;
- credenciais;
- políticas críticas.

---

13. Git como memória de engenharia

O Git é parte fundamental da engenharia da Yuki.

Decisões arquiteturais importantes devem ser preservadas em arquivos versionados.

O estado do projeto não deve depender exclusivamente de conversas.

A documentação deve permitir que um novo agente ou desenvolvedor compreenda:

O que a Yuki é
↓
Por que foi projetada dessa maneira
↓
Quais decisões já foram tomadas
↓
Quais decisões continuam abertas
↓
Como implementar
↓
Como testar

---

14. O que NÃO fazer nesta fase

Não começar simultaneamente a construir:

- todos os agentes;
- memória completa;
- sistema de voz completo;
- automação residencial;
- robótica;
- câmeras;
- finanças;
- dezenas de integrações;
- infraestrutura distribuída completa;
- autoevolução completa.

Isso aumentaria a superfície de complexidade antes de validar a fundação.

Primeiro:

FOUNDATION

Depois:

CAPABILITIES

Depois:

EXECUTION

Depois:

INTELLIGENCE + AGENCY

Depois:

DISTRIBUTED / PHYSICAL / EVOLUTION

---

15. Critério de progresso

O projeto não deve medir progresso apenas pelo número de arquivos ou linhas de código.

O progresso deve ser medido pela quantidade de arquitetura que passou de:

IDEIA
 ↓
DOCUMENTO
 ↓
CONTRATO
 ↓
IMPLEMENTAÇÃO
 ↓
TESTE
 ↓
USO REAL

Uma capacidade só será considerada implementada quando houver implementação e teste correspondente.

---

16. Estado atual resumido

┌──────────────────────────────────────────────┐
│                 YUKI                         │
├──────────────────────────────────────────────┤
│ Visão                       ✅               │
│ Princípios                  ✅               │
│ Arquitetura macro           ✅               │
│ ADRs 006–017                ✅               │
│ Contratos de implementação  🟡              │
│ Foundation                  🔴               │
│ Core                        🔴               │
│ Runtime                     🔴               │
│ Security Runtime            🔴               │
│ Capability Runtime          🔴               │
│ Execution                   🔴               │
│ Verification                🔴               │
│ Workflow                    🔴               │
│ Memory                      🔴               │
│ Agents                      🔴               │
│ Background                  🔴               │
│ Reconciliation              🔴               │
│ Federation                  🔴               │
│ Production                  🔴               │
└──────────────────────────────────────────────┘

Legenda:

✅ Definido/documentado
🟡 Em definição
🔴 Ainda não implementado

---

17. Próximo marco

O próximo marco oficial do projeto é:

YUKI FOUNDATION v0.1

Objetivos:

1. definir contratos fundamentais;
2. definir estrutura inicial do código;
3. escolher a primeira implementação tecnológica sem torná-la constitucional;
4. implementar o Core mínimo;
5. implementar o primeiro fluxo end-to-end;
6. criar testes estruturais;
7. validar os contratos contra os ADRs;
8. preparar a base para os agentes de desenvolvimento.

---

18. Regra de ouro desta fase

«Não construir a Yuki inteira. Construir a fundação sobre a qual a Yuki inteira possa ser construída.»

A Yuki deve crescer por composição:

Foundation
    +
Capabilities
    +
Models
    +
Integrations
    +
Memory
    +
Agents
    +
Knowledge
    +
Workflow
    +
Events
    +
Reconciliation
    +
Evolution

e não por um único bloco monolítico.

---

19. Próximo documento

O próximo documento a ser criado é:

docs/FOUNDATION_CONTRACTS.md

Ele definirá os contratos canônicos dos objetos fundamentais utilizados pela implementação.

Depois dele:

docs/IMPLEMENTATION_ROADMAP.md

Esses documentos serão a ponte entre:

ADRs
  ↓
Arquitetura
  ↓
Código

---

20. Status

Estado: FOUNDATION STARTING

Arquitetura: definida

Implementação: iniciando

Próximo objetivo: Yuki Foundation v0.1

Regra: preservar a arquitetura enquanto transformamos decisões em software testável.