Yuki — Status Atual do Projeto

Última atualização: 2026-10-03
Status geral: MVP-1 Marco 4 implementado, testado e validado; suíte adversarial completa e empacotamento OCI pronto para auditoria de fechamento
Repositório: "JoseLeandroAC/YUKI"
Branch de trabalho: "feature/mvp-1-runtime" (PR #3 aberto em draft para "main")
Baseline Foundation: "v0.1.0-foundation" (congelada e preservada em 654c1812f40d6c903e67fd6f86f4b83e06e14b8c)

---

1. Estado atual

A Yuki concluiu todos os quatro marcos de desenvolvimento do **MVP-1 (Runtime Soberano & Modelo Operacional Mínimo)**:
- **Marco 1:** Runtime Assíncrono Tokio, Pipeline Assíncrono com Timeout e Cancelamento;
- **Marco 2:** Credential Broker Nível 1, Model Gateway/Router, Adapter Gemini e Provedor Mock;
- **Marco 3:** Verificação Dinâmica desacoplada, Registry de Estratégias, Persistência Local Durável SQLite com Migrações;
- **Marco 4:** Expansão Segura de Capacidades (`system.time`, `system.info`), Validação de Pré-Autorização Fail-Closed, Integridade Estrita de Parâmetros, Suíte Adversarial Completa (34 testes) e Empacotamento OCI/Docker Multi-Stage.

A situação atual pode ser resumida como:

ARQUITETURA
████████████████████████████████████████
DEFINIDA E DOCUMENTADA (ADRs 006 a 019)

CONTRATOS DE IMPLEMENTAÇÃO (FOUNDATION v0.1)
████████████████████████████████████████
FROZEN / IMUTÁVEIS (v0.1.0-foundation)

MVP-1 (MARCOS 1, 2, 3 E 4)
████████████████████████████████████████
IMPLEMENTADOS E VERIFICADOS

TESTES DE INTEGRAÇÃO & SEGURANÇA (NEGATIVOS E ADVERSARIAIS)
████████████████████████████████████████
120/120 TESTES ATIVOS PASSANDO (1 LIVE GEMINI IGNORADO)

EMPACOTAMENTO OCI / DOCKER
████████████████████████████████████████
CONCLUÍDO (MULTI-STAGE, HARDENED, NON-ROOT)

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

A **Foundation v0.1 / MVP-0** da Yuki foi integralmente implementada em Rust puro (1.98.1), testada (26/26 testes aprovados), auditada por duas rodadas técnicas e consolidada na branch `feature/foundation-v0.1`.

A base de código atual já estabelece estruturalmente os componentes da Foundation:

- **Yuki Core:** orquestrador central do pipeline síncrono do MVP-0;
- **Contracts:** contratos canônicos tipados para identifiers, input, output, context, capability, authorization, execution, verification e events;
- **Context Builder:** construção tipada de contexto com minimização e salvaguarda preventiva de segredos;
- **Model Layer:** abstração desacoplada de provedor cognitivo (`ModelProvider`) e implementação `MockModelProvider`;
- **Capability System:** registro dinâmico (`CapabilityRegistry`) e implementação de referência segura (`system.echo`);
- **Security Controller:** barreira de autorização estrita, desacoplada do modelo, emitindo tokens de uso único com expiração e consumo atômico;
- **Execution Layer:** executor isolado que rejeita qualquer tentativa sem token válido consumível;
- **Verification Engine:** mecanismo de verificação pós-execução que atesta evidências e preserva o estado `UNKNOWN`;
- **Audit Store:** barreira de eventos de auditoria estruturados em memória (`InMemoryEventStore`).

Componentes avançados e de maior autonomia que permanecem explicitamente planejados para fases posteriores (a partir do MVP-1):

- Provedores LLM reais externos (Google Gemini, Vertex AI) e Model Router dinâmico;
- Credential Broker e isolamento criptográfico de segredos (ADR-008);
- Integration Gateway com conectores de rede reais (ADR-007);
- Verification Strategies dinâmicas desacopladas por capability (ADR-009);
- Durable Workflow e Sagas compensatórias (ADR-011);
- Memory semântica e Knowledge Graph (ADR-016);
- Agent System e supervisão autônoma multi-agente;
- Reconciliation Engine declarativo (ADR-012);
- Federação e sincronização de eventos Home/Cloud (ADR-013);
- Physical World Safety e intertravamentos de hardware (ADR-010).

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
│ Contratos de implementação  ✅               │
│ Foundation v0.1 / MVP-0     ✅               │
│ Core (mínimo vertical slice)✅               │
│ Context Builder (mínimo)    ✅               │
│ Model Provider (Mock)       ✅               │
│ Capability Registry (echo)  ✅               │
│ Security Controller (tokens)✅               │
│ Execution (isolado)         ✅               │
│ Verification (básico)       ✅               │
│ Audit Store (in-memory)     ✅               │
│ CLI & Diagnostics (health)  ✅               │
│ Runtime assíncrono / I/O    🔴               │
│ Provedores LLM Reais        🔴               │
│ Credential Broker (ADR-008) 🔴               │
│ Workflow durável            🔴               │
│ Memory                      🔴               │
│ Knowledge                   🔴               │
│ Agents                      🔴               │
│ Background Events           🔴               │
│ Reconciliation              🔴               │
│ Federation                  🔴               │
│ Physical World              🔴               │
│ Produção                    🔴               │
└──────────────────────────────────────────────┘

Legenda:

✅ Implementado, auditado e consolidado
🟡 Em definição
🔴 Planejado para fases posteriores (MVP-1+)

---

17. Próximo marco

O próximo marco oficial do projeto é:

YUKI MVP-1

Objetivos do MVP-1:
1. introduzir runtime assíncrono (Tokio) para I/O de rede sem bloqueio;
2. implementar o primeiro Model Adapter externo real (Google Gemini / Vertex AI);
3. implementar o Credential Broker formal conforme o ADR-008;
4. desacoplar o Verification Engine via estratégias dinâmicas orientadas pelo manifesto da capability (ADR-009);
5. implementar backend de persistência durável para o Event Store (ADR-013);
6. expandir capacidades iniciais seguras e adicionar testes adversariais adicionais.

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

19. Documentos consolidados da Foundation

A ponte entre arquitetura e código está formalizada e consolidada nos seguintes documentos versionados:

- docs/FOUNDATION_CONTRACTS.md (contratos canônicos consolidados);
- docs/IMPLEMENTATION_SPEC_FOUNDATION_V0.1.md (especificação operacional cumprida);
- docs/IMPLEMENTATION_ROADMAP.md (roadmap de engenharia com Foundation concluída).

---

20. Status

Estado: MVP-1 IMPLEMENTATION COMPLETE — READY FOR CLOSURE AUDIT

Arquitetura: definida e aprovada (ADRs 006–019)
Implementação: MVP-1 Marcos 1, 2, 3 e 4 totalmente implementados e testados
Código: 120/120 testes ativos passando (+ 1 teste live Gemini ignorado), clippy limpo (-D warnings), rustfmt verificado
Próximo passo: Auditoria independente de fechamento do Marco 4 e preparação para congelamento oficial da baseline MVP-1 (v0.2.0-mvp1).

---

21. Registro de Validação do MVP-1

O MVP-1 concluiu seus marcos de desenvolvimento:
1. **Marco 1:** Runtime assíncrono Tokio, timeouts e cancelamento cooperativo.
2. **Marco 2:** Credential Broker Nível 1, Model Gateway/Router com fallback resiliente, mock e adapter Gemini seguro.
3. **Marco 3:** Verificação dinâmica desacoplada e persistência local durável SQLite append-only com schema migrations.
4. **Marco 4:** Capacidades seguras adicionais (`system.time`, `system.info`), validação de pré-autorização fail-closed, integridade estrita de parâmetros, empacotamento OCI multi-stage e suíte com 34 testes adversariais.

Resultados verificados:
- `cargo test --all-targets`: 120 ativos aprovados, 1 ignorado (smoke live), 0 falhas;
- `cargo check --all-targets`: aprovado sem erros;
- `cargo fmt --all -- --check`: aprovado sem desvios;
- `cargo clippy --all-targets -- -D warnings`: aprovado com zero warnings;
- `cargo build --release`: aprovado com sucesso.