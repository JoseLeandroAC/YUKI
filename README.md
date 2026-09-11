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

**Arquitetura e especificação conceitual.**

A implementação do MVP ainda será iniciada após a definição suficiente da arquitetura fundamental.

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
