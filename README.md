# Yuki

> Personal AI Platform — Arquitetura, conhecimento, decisões e evolução

## Sobre o projeto

Yuki é uma plataforma pessoal de Inteligência Artificial projetada para funcionar como assistente, agente, secretária digital e sistema pessoal de inteligência de longo prazo.

O objetivo não é criar apenas um chatbot, mas uma plataforma capaz de:

* compreender linguagem natural;
* conversar por voz e texto;
* possuir memória persistente;
* compreender contexto pessoal;
* planejar objetivos e tarefas;
* executar ações através de ferramentas;
* utilizar múltiplos modelos de IA;
* pesquisar informações;
* analisar dados;
* utilizar sistemas externos;
* trabalhar com múltiplos agentes;
* monitorar eventos relevantes;
* evoluir suas capacidades de maneira controlada;
* integrar futuras tecnologias e sistemas.

## Princípios fundamentais

1. Yuki é uma plataforma modular.
2. Capacidade não implica autorização.
3. Quanto mais poderosa uma capacidade, maior deve ser seu controle.
4. Dados externos são considerados não confiáveis por padrão.
5. Segurança é parte da arquitetura, não um complemento.
6. O usuário é a autoridade final sobre decisões críticas.
7. Yuki deve poder evoluir sem exigir reescritas estruturais constantes.
8. Modelos de IA são componentes substituíveis.
9. Memória deve sobreviver à troca de modelos.
10. Sistemas externos devem ser acessados através de contratos e gateways.
11. Alterações importantes devem ser versionadas.
12. Toda alteração relevante deve possuir histórico.
13. Yuki deve ajudar o usuário, não controlar o usuário.

## Estado atual

O projeto encontra-se na fase de:

**Personal Intelligence Foundation & Expansão de Capacidades.**

* **Foundation v0.1 & MVP-1 Runtime:** Concluídos e validados (ADRs 006 a 019).
* **Research v1 (Marcos 1 a 5):** Recuperação governada, SSRF guard, extração e descompressão segura de conteúdo, Brave Search e LangSearch providers, registro de fontes e citações verificáveis (ADR-020).
* **Marco 6 (Expansão de Infraestrutura):** Credential Vault v1 (AES-256-GCM + PBKDF2, zeroize, SecretStore, CompositeCredentialBroker), Native Search Foundation (busca soberana local SQLite FTS5 BM25, indexador com defesa de path traversal, zero rede), e endurecimento de transporte (ADR-008 e ADR-021).
* **Marco 7 (Personal Intelligence Foundation):** Credential Vault Hardened (canary upfront, limites de iteração e payload, mitigação de symlinks e Admin CLI `yuki vault`), Memory v1 (SQLite FTS5 BM25, triggers de sincronização, deduplicação e superseding, sanitização fail-closed de segredos e direito ao esquecimento), Personal Context v1 (orçamento estrito de tokens em 5 camadas priorizadas), e Personality v1 (identidade estável de 3 camadas com salvaguardas ontológicas e perfil "Yuki Default").
* **Marco 7.1 (Endurecimento da Personal Intelligence e Diretrizes de Evolução):**
  - Correção dos 12 itens técnicos identificados na auditoria do Marco 7 (paginação e filtros FTS5 antes de LIMIT/OFFSET, `count()` desacoplado de paginação, eliminação de fallback aleatório, propagação de erros de banco com emissão de auditoria `MemoryRetrievalFailed`, isolamento de memórias fora da instrução de sistema conforme `Data != Instruction`, escopo lógico de dono `owner_id` com isolamento multi-tenant, expansão da política sanitária contra credenciais/chaves/vigilância íntima, orçamento adaptativo proporcional ao prompt, sincronização interprocessos no Vault com `VaultProcessLock` e proteção Windows `share_mode(0)`, inicialização inequívoca e fail-closed de `vault init` com confirmação `RECRIAR`, e alinhamento de documentação de status);
  - Formalização de Propostas e ADRs Fundamentais de Evolução de Longo Prazo:
    - **ADR-022:** Ampliação das Capacidades Humanas e Parceria Intelectual;
    - **ADR-023:** Arquitetura de Memória Cognitiva, Associativa e Expansível;
    - **ADR-024:** Governança de Políticas Configuráveis e Autonomia Proporcional ao Impacto;
    - **ADR-025:** Princípio de Independência Arquitetural e Primazia do Proprietário;
    - **Proposta:** Yuki Ambient Intelligence & Security (`docs/proposals/YUKI_AMBIENT_INTELLIGENCE_AND_SECURITY.md`);
    - **Proposta:** OperatorProfile v1 (`docs/proposals/OPERATOR_PROFILE_V1.md`).
* **Marco 7.2 (Hardening Final, Integridade e Validação Independente):**
  - Implementação de bloqueio de processo no nível de kernel do sistema operacional (`share_mode(0)` no Windows e POSIX `flock` no Unix/Linux), eliminando heurísticas frágeis de tempo e prevenindo roubo de lock;
  - Proteção contra adoção silenciosa de senhas arbitrárias em migração de cofres legados vazios (`UnverifiableLegacyVault`), exigindo intervenção administrativa explícita (`initialize_unverifiable_legacy`);
  - Proteção de permissões de arquivos restritas nativas no Vault (`0o600` em Unix e remoção de herança com concessão exclusiva ao usuário corrente via `icacls` no Windows);
  - Propagação rigorosa de falhas reais de armazenamento de memória (`MemoryError::Storage`) no `YukiCore` com registro de auditoria `MemoryRetrievalFailed` e retorno de `YukiError::ExecutionFailed` sem fallback silencioso;
  - Gerenciamento de orçamento integral de prompt (`PromptBudgetManager`) estruturado em 6 tiers de prioridade ontológica, assegurando preservação inviolável do Tier 1 (Instruções Essenciais de Sistema) e estimativa de tokens conservadora com 10% de margem de segurança;
  - Defesa ativa contra injeção e quebra de delimitadores em memórias de contexto pessoal (`sanitize_untrusted_memory_content`), neutralizando fechamento forçado de envelope e tokens de autoridade (`<system>`, `[INST]`, etc.).
* **Próximo Marco:** Marco 8 — Voice v1 & Missions Foundation (`docs/proposals/MARCO_8_VOICE_AND_MISSIONS.md`).
* **Base de Testes:** 488 testes automatizados aprovados no workspace (0 falhas).

Consulte:

`docs/00_STATUS_ATUAL.md`

## Arquitetura conceitual

```text
                         USER
                          │
                Voice / Text / Image
                          │
                    PERCEPTION
                          │
                  CONTEXT BUILDER
                          │
             ┌────────────┴────────────┐
             ▼                         ▼
          MEMORY                    KNOWLEDGE
             └────────────┬────────────┘
                          ▼
                  EXECUTIVE BRAIN
                          │
       ┌──────────────────┼──────────────────┐
       ▼                  ▼                  ▼
   REASONING           PLANNING          MODEL ROUTER
       │                  │                  │
       └──────────────────┼──────────────────┘
                          ▼
                CAPABILITY SELECTION
                          │
                  SECURITY / POLICY
                          │
                      EXECUTION
                          │
                    VERIFICATION
                          │
                 ┌────────┴────────┐
                 ▼                 ▼
               RESULT           LEARNING
                                     │
                                  EVOLUTION
```

## Documentação

### Arquitetura

* `docs/02_ARQUITETURA.md`
* `docs/03_CORE.md`
* `docs/04_MEMORY.md`
* `docs/05_PERSONAL_CONTEXT.md`
* `docs/06_GOALS_AND_PLANNING.md`
* `docs/07_AGENTS_AND_TASKS.md`

### Capacidades

* `docs/08_CAPABILITY_SYSTEM.md`
* `docs/09_MODEL_ROUTER.md`
* `docs/12_VOICE_AND_MULTIMODAL.md`
* `docs/13_EVENTS_AND_BACKGROUND.md`
* `docs/15_INTEGRATIONS.md`

### Segurança

* `docs/10_SECURITY.md`

### Evolução

* `docs/11_EVOLUTION.md`

### Infraestrutura

* `docs/14_INFRASTRUCTURE.md`

### Catálogo

* `docs/16_MASTER_CAPABILITY_CATALOG.md`

## Personal Intelligence & Administração

A partir do Marco 7, a Yuki opera com inteligência pessoal soberana ancorada em persistência local durável, governança rigorosa e identidade estável:

### 1. Administração do Credential Vault (`yuki vault`)
Gerenciamento soberano de segredos criptografados em repouso (AES-256-GCM + PBKDF2):
```bash
# Inicializar um novo cofre criptográfico com canary de integridade
yuki vault init

# Armazenar uma nova credencial com mascaramento de senha
yuki vault store <alias> <provedor>

# Substituir o segredo de uma credencial existente
yuki vault replace <alias>

# Listar metadados das credenciais (nunca expõe segredos)
yuki vault list

# Consultar status operacional e integridade do cofre
yuki vault status

# Revogar uma credencial (acesso negado fail-closed)
yuki vault revoke <alias>

# Exclusão física permanente
yuki vault delete <alias>
```

### 2. Memória Persistente Soberana (Memory v1)
- Armazenamento em SQLite com tabela virtual FTS5 e ranking BM25 (`unicode61 remove_diacritics 2`).
- Triggers atômicos de sincronização (`trg_memories_ai`, `trg_memories_au`, `trg_memories_ad`).
- Deduplicação inteligente e versionamento (`superseded_by`).
- Invariante inegociável: `Model Output != Memory != Persistence` — gravações exigem avaliação sanitária e aprovação de política governada (`MemoryPolicy`).
- Sanitização fail-closed contra vazamento de credenciais em memórias.
- Garantia de direito ao esquecimento com exclusão física permanente (`delete_permanently`).

### 3. Contexto Pessoal em 5 Camadas (Personal Context v1)
Priorização governada por orçamento estrito de tokens:
1. **Camada 1 (Crítica):** Diretrizes e restrições imutáveis do operador.
2. **Camada 2 (Identidade):** Perfil do operador e tom relacional.
3. **Camada 3 (Memória de Trabalho):** Fatos imediatos e tarefas em andamento.
4. **Camada 4 (Perfil e Preferências):** Preferências consolidadas de comunicação e ferramentas.
5. **Camada 5 (Episódica e Fatos):** Memórias históricas recuperadas semanticamente/textualmente.

### 4. Personalidade Estável e Salvaguardas Ontológicas (Personality v1)
Arquitetura em 3 níveis:
- **Tier 1 (Core Identity):** Diretrizes ontológicas imutáveis (`YUKI_CORE_IDENTITY_DIRECTIVES`). A Yuki jamais simula sofrimento humano, dor ou consciência biológica.
- **Tier 2 (Personality Profile):** Parâmetros tipados (warmth, formality, humor, expressiveness, proactivity, explanation depth) e estilo conversacional, com o perfil canônico `Yuki Default`.
- **Tier 3 (Context Urgency):** Adaptação contextual dinâmica (Standard, TechnicalDeep, SecurityCritical, CreativeReflective) regulando densidade técnica e sobriedade.

## Desenvolvimento

O desenvolvimento da Yuki deverá seguir uma abordagem incremental:

```text
Arquitetura
    ↓
Fundação
    ↓
MVP
    ↓
Testes
    ↓
Segurança
    ↓
Expansão
    ↓
Evolução contínua
```

## Versionamento

O Git será utilizado como:

* histórico do projeto;
* backup da documentação;
* registro de decisões;
* registro de evolução;
* controle de versões;
* base para futura colaboração entre humanos e agentes de IA.

---

**Projeto:** Yuki
**Tipo:** Personal AI Platform
**Status:** Architecture / Design
**Proprietário:** José
