YUKI — IMPLEMENTATION SPECIFICATION MVP-1

Status: APPROVED
Versão: 1.0
Data: 2026-10-03
Fase: MVP-1 (Runtime, External Model Gateway, Persistence & Reproducibility)
Projeto: Yuki Personal AI Platform

---

1. Categorização Epistemológica e Taxonomia Normativa

Para assegurar clareza e evitar confusão entre princípios imutáveis e detalhes transitórios de engenharia, cada seção desta especificação é rotulada conforme a seguinte taxonomia:

- [ARCHITECTURAL PRINCIPLE]: Invariante constitucional perene do sistema Yuki. Não negociável e independente de implementação.
- [ACCEPTED ADR DECISION]: Decisão formal de design aprovada pelo Owner em ADR oficial (ADR-006 a ADR-019).
- [ENGINEERING RULE]: Norma operacional de disciplina técnica e governança do código e do repositório.
- [MVP-1 IMPLEMENTATION CHOICE]: Escolha técnica concreta selecionada especificamente para o ciclo de desenvolvimento do MVP-1.
- [FUTURE CANDIDATE]: Funcionalidade, padrão ou componente planejado para ciclos posteriores (MVP-2+), fora do escopo do MVP-1.

---

2. Princípios Constitucionais e Invariantes do MVP-1

[ARCHITECTURAL PRINCIPLE]
1. `Capability != Permission != Authorization != Execution`
2. `Model Output != Command != Authorization`
3. `Think != Authorize != Execute`
4. `Data != Instruction`
5. `Verification != Truth`
6. `Storage != Authority`
7. `Audit History != Operational State`
8. `Observability != Audit`
9. `Assume Breach` e `Least Agency`: modelos externos, redes e entradas são domínios de confiança zero.
10. `Devices are access points, not Yuki itself`: A Yuki é soberana e independente de qualquer máquina física específica.

---

3. Escopo Oficial do MVP-1

[ACCEPTED ADR DECISION]
### IN SCOPE
1. **Async Runtime Boundary**: Tokio multi-threaded restrito às fronteiras de I/O, captura de sinais do SO (`SIGINT`/`SIGTERM`) e timeouts de execução.
2. **Model Gateway & Provider Abstraction (ADR-018)**:
   - Trait assíncrona `ModelProvider`.
   - `MockModelProvider` assíncrono para testes de CI sem rede e offline.
   - `GeminiProviderAdapter` para comunicação segura via REST/HTTPS com TLS (rustls).
   - `ModelRouter` para seleção configurável do provedor.
   - Parser tipado convertendo respostas para `CapabilityProposal` (não executável).
   - Taxonomia tipada de erros (`ModelError`).
3. **Credential Broker Mínimo (ADR-008 Level 1)**:
   - Abstração `SecretStore` e `SecretRef`.
   - Implementação `EnvSecretStore`.
   - Injeção efêmera de segredo na camada HTTP, sem vazamento para contexto, prompt, logs ou auditoria.
4. **Dynamic Verification Engine (ADR-009)**:
   - Trait síncrona/pura `VerificationStrategy`.
   - `Trusted Strategy Registry` governado pelo `Verifier`.
   - Estratégias: `EchoVerificationStrategy` e `TimeVerificationStrategy` (`system_time_window`).
   - Estados de verificação tri-state: `VERIFIED`, `FAILED`, `UNKNOWN`.
5. **Persistent EventStore (ADR-019)**:
   - Trait assíncrona `EventStore`.
   - Adaptador `SqliteEventStore` com journaling em modo WAL.
   - Persistência imutável de `AuditEvent` (sem persistir `CapabilityToken` ativo).
6. **Capacidades Iniciais Mínimas**:
   - `system.echo` (sintética para regressão da Foundation).
   - `system.time` (leitura segura de relógio com janela de tolerância temporal configurável).
   - `system.info` (metadados operacionais mínimos de SO/CPU/Yuki, estritamente minimizados sob ADR-015).
7. **Configuração e Reprodutibilidade**:
   - Carregamento hierárquico de configurações (`yuki.toml` + variáveis `YUKI_*`).
   - GitHub Actions CI (executando fmt, check, clippy -D warnings, test, build).
   - Dockerfile multi-stage OCI/Docker.
   - Inventário estrito em `docs/deployment/`.

[ACCEPTED ADR DECISION]
### OUT OF SCOPE
1. Provedores concorrentes adicionais em produção (OpenAI, Anthropic, Mistral).
2. Streaming de respostas de LLM (SSE/WebSockets).
3. Shell, processos arbitrários, rede aberta ou acesso irrestrito ao sistema de arquivos.
4. Sincronização distribuída Casa-Nuvem (ADR-013 full sync).
5. Workflows de longa duração com sagas compensatórias (ADR-011).
6. Atuadores do mundo físico e intertravamentos de hardware (ADR-010).
7. Reconciliação de metas de vida (ADR-014).
8. Voz, áudio em tempo real e visão computacional (ADR-012).
9. Multi-agentes autônomos sem supervisão.
10. Persistência de `CapabilityToken` ou políticas de expurgo automático de retenção.

---

4. Fronteiras de Componentes e Interfaces

[MVP-1 IMPLEMENTATION CHOICE]

### A. Async Boundary (Fronteira Assíncrona)
- **Síncrono (Puro/In-Memory)**:
  - `ContextBuilder::build`
  - `ProposalParser::parse`
  - `SecurityController::authorize`
  - `Verifier::verify`
  - `VerificationStrategy::verify`
  - Todas as avaliações de políticas e hashing de argumentos.
- **Assíncrono (I/O & Timers)**:
  - `ModelProvider::generate_proposal` (I/O de rede)
  - `EventStore::record` e `EventStore::query` (I/O de disco)
  - `YukiCore::process_turn_async` (Orquestração assíncrona com timeouts e cancelamento cooperativo)
  - Captura de sinais do sistema operacional (`tokio::signal::ctrl_c`).

### B. Credential Boundary (Fronteira de Credenciais)
```text
[Ambiente / Host]
      │ (YUKI_GEMINI_API_KEY)
      ▼
[EnvSecretStore]
      │ (Apenas sob demanda do CredentialBroker)
      ▼
[CredentialBroker]
      │ (Secret Material efêmero injetado diretamente no header HTTP)
      ▼
[GeminiProviderAdapter] ──► [Endpoint HTTPS Seguro]
```
Nenhum material de segredo bruto trafega no `ContextObject`, no `ModelRequest` ou nos logs.

### C. Capability Token Lifecycle
- `CapabilityToken` é emitido em memória pelo `SecurityController` com UUID v4 e digest SHA-256 dos argumentos autorizados.
- O `Executor` valida o token e o marca imediatamente como consumido para impedir ataques de replay.
- O token **NUNCA** é gravado no banco de dados SQLite. O `EventStore` grava apenas metadados de auditoria imutáveis.

### D. Verification Trust Boundary
- O manifesto da capacidade declara apenas o identificador da estratégia:
  - `system.echo` ➔ `"echo_exact_match"`
  - `system.time` ➔ `"system_time_window"`
  - `system.info` ➔ `"system_info_schema"`
- A resolução da estratégia ocorre no `Trusted Strategy Registry`, pertencente ao subsistema de verificação, garantindo que capacidades não possam autoverificar seu próprio comportamento arbitrariamente.
- A tolerância temporal de `system.time` é uma configuração da estratégia (ex: padrão de 2000 ms), não uma constante arquitetural.

---

5. Regras de Engenharia (Engineering Rules)

[ENGINEERING RULE]
1. **Regra de Inventário Operacional**:
   Toda nova dependência externa (Rust crate no `Cargo.toml`, pacote de sistema, imagem base ou variável de ambiente) deve ser imediatamente registrada em `docs/deployment/00_ENVIRONMENT_BASELINE.md` e `docs/deployment/DEPENDENCIES.md` contendo versão, propósito, justificativa e alternativa de substituição.
2. **Regra de Zero Secrets em Repositório**:
   Nenhum arquivo `.env`, credencial real ou chave de API pode ser adicionado ao repositório Git. Arquivos de exemplo (`.env.example`, `config/yuki.example.toml`) devem conter apenas placeholders.
3. **Regra de Supply Chain no CI**:
   Workflows do GitHub Actions devem empregar permissões mínimas (`permissions: contents: read`) e utilizar apenas Actions oficiais com versões ou SHAs fixos.
4. **Regra de Não-Regressão da Foundation**:
   Todos os testes e invariantes da Foundation v0.1 congelada devem continuar passando. Testes puros e síncronos permanecem síncronos.

---

6. Plano de Marcos de Implementação do MVP-1 (~3 a 5 Semanas)

[MVP-1 IMPLEMENTATION CHOICE]

### Marco 1: CI, Baseline de Ambiente & Async Runtime Core
- Criação do workflow `.github/workflows/ci.yml`.
- Criação dos documentos em `docs/deployment/`.
- Criação dos arquivos de configuração modelo (`.env.example`, `config/yuki.example.toml`).
- Adição da dependência `tokio` (features `rt-multi-thread`, `macros`, `signal`, `time`).
- Inicialização do runtime assíncrono em `src/main.rs` com captura de `Ctrl+C`.
- Introdução de ponto de entrada assíncrono no `YukiCore`.
- Preservação e validação integral de todos os 26 testes da Foundation.

### Marco 2: Credential Broker & Model Gateway
- Formalização de contratos `SecretRef` e `SecretStore` com implementação `EnvSecretStore`.
- Implementação da trait assíncrona `ModelProvider` e modernização assíncrona do `MockModelProvider`.
- Implementação do `GeminiProviderAdapter` (via `reqwest` com `rustls-tls`).
- Implementação do `ModelRouter` e do `ProposalParser`.
- Testes negativos de não-vazamento de segredos no contexto e prompt.

### Marco 3: Dynamic Verification & Persistent EventStore
- Implementação do `Trusted Strategy Registry` e estratégias desacopladas (`EchoVerificationStrategy`, `TimeVerificationStrategy`).
- Implementação do `SqliteEventStore` com `rusqlite` e journaling WAL.
- Migrações transacionais de esquema embutidas.
- Testes de integridade de auditoria e persistência pós-restart.

### Marco 4: Capacidades Reais, Container OCI & Freeze
- Implementação dos contratos e execução de `system.time` e `system.info`.
- Criação do `Dockerfile` multi-stage otimizado.
- Validação do fluxo completo ponta a ponta: Prompt ➔ Gemini ➔ Proposta ➔ Autorização ➔ Execução ➔ Verificação ➔ SQLite.
- Bateria de testes de segurança, exercício de restauração a frio e congelamento da tag `v0.2.0-mvp1`.

---

7. Definition of Done (DoD) do MVP-1

[ENGINEERING RULE]
O MVP-1 será considerado concluído quando:
1. `docs/deployment/RESTORE.md` permitir compilação e teste bem-sucedidos em ambiente limpo a partir de um clone do zero.
2. O workflow do GitHub Actions estiver ativo e com status verde em todas as verificações (`fmt`, `check`, `clippy -D warnings`, `test`, `build`).
3. Uma interação em linguagem natural com o modelo real propor `system.time` ou `system.info`, passando por autorização do Security Controller, execução segura e verificação factual com estado `VERIFIED`.
4. Nenhum segredo ou chave de API aparecer em prompts, logs ou eventos de auditoria gravados.
5. Todos os eventos de auditoria forem persistidos duravelmente no SQLite e sobreviverem ao reinício do processo.
6. A imagem Docker OCI for construída e executada em modo headless com shutdown gracioso.
7. 100% dos testes da Foundation e os novos testes assíncronos/persistentes passarem sem avisos do compilador.
