Yuki — Status Atual do Projeto

Última atualização: 2026-10-11
Status geral: Marco 7.2 (Hardening Final, Integridade e Validação Independente) implementado, testado e validado; 488 testes aprovados no workspace
Repositório: "JoseLeandroAC/YUKI"
Branch de trabalho: "feature/marco-7.2-hardening"
Baselines preservados: "v0.1.0-foundation" (654c181) e "v0.2.0-mvp1" (6a9d070)

---

1. Estado atual

A Yuki concluiu todos os marcos da Research v1 (Marcos 1 a 5), Marco 6 (Expansão de Infraestrutura), Marco 7 (Personal Intelligence Foundation), Marco 7.1 (Endurecimento da Personal Intelligence e Diretrizes de Evolução) e o **Marco 7.2 (Hardening Final, Integridade e Validação Independente)**:
- **Research v1 (Marcos 1 a 5):** Capability `research.search` e `research.fetch`, defesas SSRF, extração HTML, suporte a Content-Encoding (gzip, deflate, br, identity), provedores Brave Search e LangSearch, Registro de Fontes e Citações Verificáveis;
- **Marco 6 (Infraestrutura & Busca Local):** Credential Vault v1 inicial, defesas de streaming e sanitização em rede, e Native Search Foundation (busca soberana local SQLite FTS5 BM25, indexador com defesa de path traversal, zero rede);
- **Marco 7 (Credential Vault Hardened & Admin CLI):** Canary upfront de autenticação, limites estritos de iteração PBKDF2 e payload, mitigação de symlinks, persistência estritamente transacional e interface administrativa de linha de comando (`yuki vault [init|store|replace|list|status|revoke|delete]`);
- **Marco 7 (Memory v1):** Contratos canônicos (`MemoryRecord`, `MemoryCandidate`, `MemoryPolicy`), armazenamento persistente relacional com SQLite FTS5 (`unicode61 remove_diacritics 2`), triggers atômicos de sincronização, detecção de duplicatas e superseding, sanitização fail-closed de credenciais e direito ao esquecimento (`delete_permanently`);
- **Marco 7 (Personal Context v1):** `PersonalContextBuilder` com orçamento estrito de caracteres/tokens e priorização em 5 camadas (Diretrizes, Identidade, Memória de Trabalho, Perfil/Preferências, Fatos/Episódica), auditado via `ContextBuilt`;
- **Marco 7 (Personality v1):** Identidade estável de 3 camadas com salvaguardas ontológicas imutáveis contra simulação de consciência humana (`YUKI_CORE_IDENTITY_DIRECTIVES`), perfil tipado com padrão canônico `Yuki Default` e adaptação contextual dinâmica (`ContextUrgency`);
- **Marco 7.1 (Endurecimento & Correções Técnicas da Personal Intelligence):** 12 correções técnicas (filtros antes de LIMIT/OFFSET, count() independente, zero fallback aleatório, propagação de erros, isolamento Data != Instruction, owner_id, política sanitária expandida, orçamento adaptativo, VaultProcessLock, share_mode(0), vault init inequívoco e ADRs 022 a 025);
- **Marco 7.2 (Hardening Final, Integridade e Validação Independente):**
  1. Bloqueio interprocessos no nível de kernel do SO (`share_mode(0)` no Windows e POSIX `flock` no Unix/Linux), eliminando heurísticas frágeis de tempo e validado contra 5 cenários multiprocesso reais entre processos independentes de SO (exclusão mútua com `ProcessLockBusy`, lock mantido > 6s sem roubo ou expiração indevida, liberação normal, crash abrupto por `abort()` com liberação imediata pelo kernel, e múltiplos escritores concorrentes com recarregamento sob lock preservando integridade);
  2. Proteção contra adoção silenciosa de senha arbitrária na migração de cofres legados vazios (`UnverifiableLegacyVault`), exigindo intervenção administrativa explícita (`initialize_unverifiable_legacy`);
  3. Proteção nativa de permissões de arquivos restritas e verificáveis (`0o600` em Unix com checagem de modo pós-aplicação; e obtenção do SID exato via `whoami /user`, remoção de herança `/inheritance:r` e concessão exclusiva `/grant:r <target>:(F)` com verificação ativa de permissões efetivas no Windows; além de arquivos temporários criados diretamente com `share_mode(0)` e `mode(0o600)`);
  4. Suíte de concorrência com asserção estrita de `ProcessLockBusy` em todas as plataformas;
  5. Propagação estrita de falhas reais de armazenamento de memória (`MemoryError::Storage`) no `YukiCore` com emissão de `MemoryRetrievalFailed` e `YukiError::ExecutionFailed` sem fallback silencioso;
  6. Orçamento integral de prompt (`PromptBudgetManager`) estruturado em 6 tiers ontológicos, integrado diretamente no caminho de produção de `YukiCore::process_input_internal`, protegendo inviolavelmente as Instruções Essenciais de Sistema (Tier 1), descontando delimitadores e avisos de truncamento sem mascaramento por `saturating_sub`, falhando fechado com erros tipados (`UserTurnExceedsBudget`, `EssentialInstructionsExceedBudget`) e registrando auditoria em `PersonalityApplied`;
  7. Defesa ativa contra injeção e quebra de delimitadores em memórias (`sanitize_untrusted_memory_content`), com neutralização insensível a maiúsculas/minúsculas e tolerante a espaços para tags de autoridade (`<system>`, `<developer>`, `<<<SYSTEM>>>`), tokens de controle (`[INST]`, `<|im_start|>`) e delimitadores de envelope (`</operator_personal_context>`).

A situação atual pode ser resumida como:

ARQUITETURA
████████████████████████████████████████
DEFINIDA E DOCUMENTADA (ADRs 006 a 025)

CONTRATOS DE IMPLEMENTAÇÃO (FOUNDATION v0.1 & MVP-1)
████████████████████████████████████████
FROZEN / IMUTÁVEIS (v0.1.0-foundation e v0.2.0-mvp1)

RESEARCH V1, MARCOS 6, 7, 7.1 & 7.2
████████████████████████████████████████
IMPLEMENTADOS, HARDENED E VERIFICADOS

TESTES DE INTEGRAÇÃO & SEGURANÇA (NEGATIVOS E ADVERSARIAIS)
████████████████████████████████████████
488 TESTES ATIVOS PASSANDO (0 FALHAS, 4 LIVE OPT-IN IGNORADOS)


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

ADR-018
Yuki Model Gateway & Provider Abstraction

ADR-019
Yuki Local Persistence & Audit Storage (SQLite)

ADR-020
Research v1 Governed Retrieval, Transport & Cloud Boundaries

ADR-021
Yuki Native Search Architecture, Indexing Engine & Hybrid Retrieval

ADR-022
Ampliação das Capacidades Humanas e Parceria Intelectual (Proposto)

ADR-023
Arquitetura de Memória Cognitiva, Associativa e Expansível (Proposto)

ADR-024
Governança de Políticas Configuráveis e Autonomia Proporcional ao Impacto (Proposto)

ADR-025
Princípio de Independência Arquitetural e Primazia do Proprietário (Proposto)

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
│ ADRs 006–021                ✅               │
│ Contratos de implementação  ✅               │
│ Foundation v0.1 / MVP-0     ✅               │
│ Core (governed loop)        ✅               │
│ Context Builder (5 camadas) ✅               │
│ Model Provider (Mock/Gemini)✅               │
│ Capability Registry         ✅               │
│ Security Controller (tokens)✅               │
│ Execution (isolado)         ✅               │
│ Verification (dinâmica)     ✅               │
│ Audit Store (SQLite durável)✅               │
│ CLI & Diagnostics (health)  ✅               │
│ Runtime assíncrono / I/O    ✅               │
│ Credential Vault Hardened   ✅               │
│ Research v1 (Marcos 1-5)    ✅               │
│ Native Search Foundation    ✅               │
│ Memory v1 (SQLite FTS5)     ✅               │
│ Personal Context v1         ✅               │
│ Personality v1 (3 Tiers)    ✅               │
│ Voice v1 & Multimodal       🟡 (Proposta M8) │
│ Missions & Autonomia        🟡 (Proposta M8) │
│ Workflow durável            🔴               │
│ Knowledge avançado          🔴               │
│ Multi-Agentes               🔴               │
│ Background Events           🔴               │
│ Reconciliation              🔴               │
│ Federation                  🔴               │
│ Physical World              🔴               │
│ Produção                    🔴               │
└──────────────────────────────────────────────┘

Legenda:

✅ Implementado, auditado e consolidado
🟡 Em definição / proposta formalizada
🔴 Planejado para fases posteriores

---

17. Próximo marco

O próximo marco oficial do projeto é:

YUKI MARCO 8 — VOICE V1 & MISSIONS FOUNDATION

Objetivos do Marco 8 (ver `docs/proposals/MARCO_8_VOICE_AND_MISSIONS.md`):
1. implementar pipeline de áudio bidirecional STT/TTS (Speech-to-Text governado e Text-to-Speech expressivo);
2. suporte híbrido a modelos de voz locais/edge (Whisper/Piper) e provedores cloud governados via Vault;
3. implementar arquitetura de Missões com decomposição de objetivos em múltiplos passos (`MissionStep`);
4. persistência relacional do ciclo de vida da missão e recuperação de estado após reinicializações;
5. governança estrita com portão humano inegociável (*human-in-the-loop approvals*) para passos mutadores.

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

---

22. Registro de Validação do Marco 7 (Personal Intelligence Foundation)

O Marco 7 concluiu a primeira fundação completa de inteligência pessoal e governança da Yuki:
1. **Credential Vault Hardened & Admin CLI:**
   - Canary upfront de integridade `YUKI_VAULT_CANARY_V1` com rejeição imediata de adulteração ou senha incorreta;
   - Limites criptográficos estritos (10.000–500.000 iterações PBKDF2, até 1 MiB de texto puro, até 10 MiB de texto cifrado);
   - Rejeição estrita de links simbólicos contra ataques de travessia;
   - CLI administrativa completa: `yuki vault [init|store|replace|list|status|revoke|delete]`;
   - Suíte `tests/vault_security.rs` com 11 testes aprovados.
2. **Memory v1 (Armazenamento Relacional & FTS5 Soberano):**
   - Esquema relacional durável em SQLite (`memories`) e tabela virtual FTS5 (`memories_fts`) com tokenizador `unicode61 remove_diacritics 2`;
   - Triggers atômicos de sincronização (`trg_memories_ai`, `trg_memories_au`, `trg_memories_ad`);
   - Detecção ativa de duplicatas e superseding automático dentro de transação atômica;
   - Invariante estrita: `Model Output != Memory != Persistence`;
   - Sanitização fail-closed de credenciais via `MemoryPolicy`;
   - Direito ao esquecimento com exclusão física permanente (`delete_permanently`);
   - Suíte `tests/memory_v1.rs` com 6 testes aprovados.
3. **Personal Context v1 (Orçamento & 5 Camadas de Priorização):**
   - Construção estruturada do contexto pessoal (`PersonalContext`) em 5 camadas (Diretrizes, Identidade, Memória de Trabalho, Perfil/Preferências, Episódica);
   - Limite estrito de orçamento em caracteres/tokens prevenindo poluição de contexto;
   - Integração com `ContextBuilder` e emissão de evento de auditoria `ContextBuilt`;
   - Suíte `tests/personal_context.rs` com 2 testes aprovados.
4. **Personality v1 (Identidade Estável & 3 Tiers):**
   - Tier 1: Diretrizes ontológicas imutáveis contra simulação de consciência humana e dor biológica;
   - Tier 2: Perfil tipado com parâmetros contínuos [0.0..1.0] e padrão canônico `Yuki Default`;
   - Tier 3: Adaptador contextual de urgência (`ContextUrgency`) dinâmico;
   - Suíte `tests/personality_v1.rs` com 3 testes aprovados.
5. **Integração no YukiCore & Preparação para Marco 8:**
   - Conexão do `MemoryController`, `DefaultMemoryRetriever` e `PersonalityProfile` no loop governado;
   - Emissão auditada de `PersonalityApplied` e `MemoryStored`;
   - Adição de eventos tipados em `src/contracts/events.rs` para voz e missões;
   - Elaboração da proposta técnica `docs/proposals/MARCO_8_VOICE_AND_MISSIONS.md`;
   - Suíte `tests/memory_personality_integration.rs` com 4 testes aprovados.

Resultados verificados do Marco 7:
- `cargo test --all-targets`: 409+ testes aprovados (0 falhas, 4 live ignorados);
- `cargo fmt --check`: aprovado sem desvios;
- `cargo clippy --all-targets -- -D warnings`: aprovado com zero warnings;
- `cargo build --release`: aprovado com sucesso.

---

23. Registro de Validação do Marco 7.1 (Endurecimento da Personal Intelligence e Diretrizes de Evolução)

O Marco 7.1 implementou e validou a resolução integral dos 12 itens técnicos e as diretrizes de longo prazo:
1. **Filtros SQL antes de LIMIT/OFFSET:** Construção dinâmica de cláusulas `WHERE` vinculando filtros antes de limitar e deslocar resultados FTS5 e relacionais.
2. **`count()` Desacoplado de Paginação:** Execução de `SELECT COUNT(*)` aplicando exatamente os mesmos filtros sem cláusula `LIMIT/OFFSET`.
3. **Zero Fallback com Fatos Aleatórios:** Busca textual FTS5 que não encontra correspondência retorna `Vec::new()`, eliminando o antigo fallback que injetava memórias aleatórias irrelevantes.
4. **Propagação de Erros de Banco e Auditoria:** Eliminação de silenciamento de erros no SQLite; propagação de `MemoryError::Storage` e emissão do evento `MemoryRetrievalFailed`.
5. **Isolamento de Memórias Fora de Instruções de Sistema (`Data != Instruction`):** Memórias recuperadas são empacotadas no bloco delimitado `<operator_personal_context origin="memory_v1" classification="untrusted_reference_data">` dentro do payload do usuário, mantendo a instrução de sistema livre de injeção.
6. **Escopo Lógico Obrigatório de Dono (`owner_id`):** Introdução de `owner_id` (padrão `operator:primary`), migração transparente do esquema com índice `idx_memories_owner`, e isolamento multi-tenant garantido em deduplicação e buscas.
7. **Política Sanitária Expandida:** `MemoryPolicy` valida nulos (`\0`), rejeita connection strings (`postgres://`, etc.), chaves privadas RSA/SSH, cartões/CVV, seed phrases e proíbe vigilância íntima clandestina (`prohibit_intimate_surveillance`).
8. **Orçamento Adaptativo Proporcional ao Prompt:** `PersonalContextConfig` introduz `total_prompt_budget_chars` e `max_prompt_ratio`, calculando dinamicamente `effective_char_limit()`.
9. **Sincronização Interprocessos no Vault:** Introdução do `VaultProcessLock` com arquivo de bloqueio companion `.lock`, prevenindo condições de corrida entre múltiplos processos CLI e da plataforma.
10. **Proteção Windows `share_mode(0)`:** Escrita atômica em arquivo temporário com acesso exclusivo no kernel Windows via `share_mode(0)` e permissões rigorosas.
11. **Inicialização Inequívoca de `vault init`:** Separação estrita entre `create_new` (falha se arquivo existir), `open` (falha se não existir) e `open_or_create`. O comando CLI `vault init` exige flag `--force` e digitação explícita de `"RECRIAR"` para reinicializar cofre pré-existente.
12. **Documentação e Diretrizes de Evolução de Longo Prazo:**
    - Elaboração de **ADR-022** (Ampliação das Capacidades Humanas e Parceria Intelectual);
    - Elaboração de **ADR-023** (Arquitetura de Memória Cognitiva, Associativa e Expansível);
    - Elaboração de **ADR-024** (Governança de Políticas Configuráveis e Autonomia Proporcional ao Impacto);
    - Elaboração de **ADR-025** (Princípio de Independência Arquitetural e Primazia do Proprietário);
    - Elaboração da proposta `docs/proposals/YUKI_AMBIENT_INTELLIGENCE_AND_SECURITY.md`;
    - Elaboração da proposta `docs/proposals/OPERATOR_PROFILE_V1.md`.

Resultados verificados do Marco 7.1:
- `tests/marco_7_1_validation.rs`: 10/10 testes aprovados cobrindo adversariamente os 12 itens;
- Suíte geral de testes: 419+ testes ativos aprovados (0 falhas);
- `cargo fmt --check`: aprovado sem desvios;
- `cargo clippy --all-targets -- -D warnings`: aprovado com zero advertências;
- `cargo build --release`: aprovado com sucesso.

---

24. Registro de Validação do Marco 7.2 (Hardening Final, Integridade e Validação Independente)

O Marco 7.2 implementou e validou o endurecimento estrito de 7 áreas críticas levantadas pela auditoria e inspeção independente:
1. **Bloqueio de Processo a Nível de Kernel do SO (`src/security/vault/store.rs`):** Substituição de verificações baseadas em idade de arquivo (5s) por travas nativas de SO. No Windows, utiliza `OpenOptionsExt::share_mode(0)` impedindo qualquer outro processo de abrir ou deletar o arquivo companion `.lock`. No Unix/Linux, utiliza POSIX `flock(LOCK_EX | LOCK_NB)` via `libc`. O kernel libera o lock automaticamente na morte do processo, eliminando falsos positivos e impedindo roubo de travas ativas.
2. **Proteção de Migração de Cofre Legado Vazio (`UnverifiableLegacyVault`):** Quando um cofre legado não possui canary e está com zero entradas, não há como atestar a chave. Em vez de adotar silenciosamente qualquer senha fornecida, o cofre agora falha fechado com `VaultError::UnverifiableLegacyVault`. A recuperação requer intervenção administrativa explícita através de `CredentialVault::initialize_unverifiable_legacy(passphrase)`.
3. **Restrição Nativa de Permissões de Arquivo (`restrict_file_permissions`):** No Unix/Linux, aplica modo `0o600`. No Windows, invoca `icacls` para remover herança (`/inheritance:r`) e conceder controle total exclusivamente ao usuário corrente (`/grant:r %USERNAME%:F`). Aplicado a arquivos do cofre, arquivos temporários de commit atômico (`.tmp`) e arquivos de trava (`.lock`).
4. **Validação de Concorrência e Bloqueio Transplataforma (`tests/marco_7_1_validation.rs`):** Atualizada asserção do teste de concorrência para exigir `VaultError::ProcessLockBusy` em todas as plataformas (Windows e Unix).
5. **Propagação Rigorosa de Erros de Memória no Core (`src/core/yuki_core.rs`):** Erros de armazenamento `MemoryError::Storage` no `retriever.retrieve` registram `EventType::MemoryRetrievalFailed` na auditoria durável e imediatamente retornam `Err(YukiError::ExecutionFailed(...))` sem qualquer fallback silencioso para lista vazia.
6. **Orçamento Integral de Prompt Governamental (`src/context/prompt_budget.rs`):** Implementado `PromptBudgetManager` governando o orçamento do modelo entre 6 tiers ontológicos:
   - Tier 1: Instruções Essenciais de Sistema (não truncáveis; falha fechada se orçamento for insuficiente);
   - Tier 2: Diretrizes de Personalidade;
   - Tier 3: Turno do Usuário;
   - Tier 4: Resultados de Ferramentas;
   - Tier 5: Histórico Conversacional;
   - Tier 6: Memórias de Contexto Pessoal.
   Aplica estimativa conservadora (1 token ≈ 3.5 caracteres) com margem de segurança de 10%.
7. **Defesa Ativa contra Injeção de Delimitadores (`src/context/personal_context.rs`):** Sanitização ativa via `sanitize_untrusted_memory_content` que neutraliza tags de escape (`</operator_personal_context>`) e tokens de autoridade (`<system>`, `<developer>`, `[INST]`, etc.) em textos de memória de dados não confiáveis (`Data != Instruction`).

Resultados verificados do Marco 7.2:
- `tests/marco_7_2_hardening.rs`: 9/9 testes aprovados cobrindo os 7 itens de segurança e integridade;
- `tests/marco_7_1_validation.rs`: 10/10 testes aprovados com asserção estrita de `ProcessLockBusy`;
- Suíte geral de testes no workspace: 488 testes ativos aprovados (0 falhas);
- `cargo fmt --all -- --check`: aprovado sem desvios de formatação;
- `cargo clippy --workspace --all-targets -- -D warnings`: aprovado com zero advertências;
- `cargo build --release`: aprovado com sucesso.