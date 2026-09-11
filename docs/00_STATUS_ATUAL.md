# Yuki — Status Atual

**Data:** 11/09/2026

## Estado do projeto

**Fase atual: Arquitetura e especificação**

A implementação do MVP ainda não começou.

O objetivo desta fase é estabelecer uma arquitetura suficientemente sólida para permitir implementação, expansão e evolução futura sem criar dependências estruturais desnecessárias.

---

# 1. O que já foi definido

## Visão

Yuki será uma plataforma pessoal de IA de longo prazo, funcionando como:

* assistente;
* agente;
* secretária digital;
* sistema de memória;
* sistema de planejamento;
* plataforma de integração;
* interface para múltiplos modelos de IA;
* plataforma de capacidades.

---

# 2. Arquitetura

Já foram definidos conceitualmente:

* Yuki Core;
* Memory System;
* Knowledge System;
* Personal Context Engine;
* Executive Brain;
* Reasoning;
* Planning;
* Model Router;
* Capability Registry;
* Security Controller;
* Execution Layer;
* Verification;
* Learning;
* Evolution Manager.

---

# 3. Memória

A arquitetura de memória deverá possuir diferentes tipos de memória:

* Working Memory;
* Episodic Memory;
* Semantic Memory;
* Procedural Memory;
* Preferences;
* Project Memory;
* Goal Memory;
* Relationship Graph;
* External Knowledge.

Também foram definidos conceitos de:

* recuperação;
* ranking;
* consolidação;
* compressão;
* versionamento;
* confiança;
* arquivamento;
* recuperação histórica.

---

# 4. Contexto pessoal

Foi definido o Personal Context Engine.

Seu objetivo é determinar:

> O que está acontecendo agora e quais informações são relevantes para a tarefa atual?

O sistema não deverá enviar todos os dados disponíveis para todos os processos.

Deve utilizar minimização de dados.

---

# 5. Objetivos e planejamento

Yuki deverá possuir:

* objetivos de longo prazo;
* objetivos de médio prazo;
* objetivos de curto prazo;
* tarefas;
* hábitos;
* projetos;
* prioridades;
* dependências;
* métricas;
* prazos;
* conflitos;
* recursos;
* histórico.

Yuki poderá recomendar decisões, mas o usuário permanece como autoridade final.

---

# 6. Sistema de capacidades

Yuki utilizará um Capability Registry.

Cada capacidade deverá possuir informações como:

* ID;
* versão;
* descrição;
* entradas;
* saídas;
* dependências;
* permissões;
* risco;
* recursos;
* efeitos colaterais;
* acesso a dados;
* acesso à rede;
* nível de aprovação.

---

# 7. Multi-modelo

Yuki não deverá depender de um único modelo.

Deverá ser possível utilizar:

* GPT;
* Gemini;
* outros modelos comerciais;
* modelos especializados;
* modelos locais;
* modelos futuros.

O Model Router deverá selecionar o modelo apropriado de acordo com:

* tarefa;
* qualidade;
* latência;
* custo;
* privacidade;
* contexto;
* capacidade;
* disponibilidade.

---

# 8. Segurança

A segurança é considerada um requisito fundamental.

Já foram definidos conceitualmente:

* Security Controller independente;
* identidade;
* autenticação;
* autorização;
* least privilege;
* sandbox;
* Tool Gateway;
* isolamento;
* auditoria;
* logs;
* monitoramento;
* proteção contra prompt injection;
* proteção de segredos;
* backup;
* recuperação;
* kill switch;
* hardware keys;
* classificação de dados;
* estados de contenção.

Princípio:

> A comprometimento de uma ferramenta não deve significar o comprometimento de Yuki.

---

# 9. Evolução

Yuki deverá possuir um Evolution Manager.

Processo conceitual:

```text
Detectar necessidade
        ↓
Pesquisar
        ↓
Propor mudança
        ↓
Criar protótipo
        ↓
Testar
        ↓
Avaliar segurança
        ↓
Aprovar
        ↓
Implementar
        ↓
Monitorar
        ↓
Rollback se necessário
```

A evolução nunca deverá utilizar uma atualização para conceder automaticamente novos privilégios.

---

# 10. Infraestrutura

Estratégia atual:

**Cloud-first.**

No futuro:

**Cloud + Home Server**

Também deverá existir abstração da camada computacional:

```text
Yuki
  ↓
Compute Interface
  ↓
CPU / GPU / QPU / outros
```

---

# 11. Voz e multimodalidade

Arquitetura conceitual:

```text
Wake Word
    ↓
STT
    ↓
Yuki Core
    ↓
Tools / Agents
    ↓
TTS
    ↓
Usuário
```

Também deverá existir suporte futuro a:

* visão;
* imagens;
* vídeo;
* áudio;
* sensores;
* dispositivos externos.

---

# 12. Sistemas externos

Yuki deverá acessar sistemas externos através de gateways e contratos padronizados.

Exemplo:

```text
Yuki
  ↓
System Gateway
  ↓
Sistema externo
```

---

# 13. Estado de desenvolvimento

## Concluído conceitualmente

* [x] Visão geral
* [x] Princípios
* [x] Arquitetura inicial
* [x] Memória
* [x] Contexto
* [x] Objetivos
* [x] Capability Registry
* [x] Model Router
* [x] Segurança conceitual
* [x] Evolução conceitual
* [x] Cloud-first
* [x] Hardware abstraction

## Em aprofundamento

* [ ] Multi-Agent System
* [ ] Task System
* [ ] Guardrails
* [ ] RAG
* [ ] Knowledge Base
* [ ] Knowledge Graph
* [ ] Security Controller detalhado
* [ ] Tool Gateway
* [ ] Permission System
* [ ] Event System
* [ ] Model Router detalhado
* [ ] Evolution Manager detalhado

## Ainda não definido completamente

* arquitetura concreta do MVP;
* stack tecnológica definitiva;
* banco de dados definitivo;
* framework de agentes;
* provedor cloud;
* sistema de autenticação definitivo;
* arquitetura de deployment;
* custos;
* observabilidade;
* interface inicial.

---

# 14. Próxima grande etapa

**Multi-Agent + Task System**

Objetivo:

Definir como Yuki transforma uma solicitação em uma missão, divide o trabalho, executa tarefas em paralelo quando apropriado, acompanha estado, verifica resultados e finaliza a missão.

---

# 15. Regra de atualização

Este documento deverá ser atualizado sempre que houver uma mudança significativa no estado do projeto.

Ele representa o:

**"Save Game" da Yuki.**
