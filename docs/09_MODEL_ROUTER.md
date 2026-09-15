YUKI — MODEL ROUTER

Documento: "docs/09_MODEL_ROUTER.md"
Versão: 0.1
Status: Arquitetura oficial
Projeto: Yuki

---

1. Objetivo

O Model Router é o sistema responsável por decidir:

«qual modelo, conjunto de modelos ou infraestrutura de IA deve ser utilizado para determinada tarefa da Yuki.»

A Yuki não deve depender de um único modelo de IA.

Ela deve conseguir utilizar:

- modelos comerciais;
- modelos open-source;
- modelos locais;
- modelos especializados;
- modelos de visão;
- modelos de áudio;
- modelos de vídeo;
- modelos de código;
- modelos de raciocínio;
- modelos futuros;
- múltiplos modelos simultaneamente.

O Model Router transforma essa diversidade em uma camada abstrata para o restante da arquitetura.

---

2. Princípio fundamental

«O Core da Yuki não deve depender diretamente de um modelo específico.»

Evitar:

Yuki Core
    ↓
Modelo X

Preferir:

Yuki Core
    ↓
Model Router
    ↓
Model Capability
    ↓
Modelo / Provider / Runtime

Assim, trocar um modelo não exige reescrever o Core.

---

3. Modelo ≠ Provedor

Esses conceitos devem ser separados.

Modelo

Representa a capacidade computacional/intelectual.

Exemplo conceitual:

reasoning-model-vX
vision-model-vY
coding-model-vZ

Provider

É quem fornece acesso ao modelo.

Provider A
Provider B
Provider C
Local Runtime
Home Server

Um mesmo modelo pode eventualmente possuir múltiplos meios de execução.

---

4. Model Registry

A Yuki deve possuir um catálogo dos modelos disponíveis.

O Registry deve conhecer:

- ID;
- versão;
- provider;
- modalidades;
- capacidades;
- contexto;
- desempenho;
- custo;
- latência;
- requisitos;
- localização do processamento;
- privacidade;
- disponibilidade;
- limites;
- status;
- compatibilidade.

Exemplo conceitual:

id: model.reasoning.example
version: 1.x

capabilities:
  - reasoning
  - text
  - structured_output

context:
  size: large

processing:
  location:
    - cloud

cost:
  class: high

latency:
  class: medium

privacy:
  class: external

status:
  healthy: true

O formato final pode mudar.

O contrato lógico deve permanecer.

---

5. Model Manifest

Cada modelo registrado deve possuir um manifesto.

Model
├── Identity
├── Provider
├── Version
├── Capabilities
├── Modalities
├── Context
├── Performance
├── Cost
├── Latency
├── Privacy
├── Availability
├── Resource Requirements
├── Limits
└── Compatibility

---

6. Model Capability

A Yuki não deve perguntar apenas:

«"Qual modelo é melhor?"»

Ela deve perguntar:

«"Qual capacidade de modelo esta tarefa exige?"»

Exemplo:

Task
 ↓
Requires:
 ├── reasoning
 ├── vision
 ├── structured output
 └── low latency

Depois:

Model Router
 ↓
Candidate Models
 ↓
Selection

---

7. Critérios de seleção

O Model Router pode considerar:

Task
Quality
Reasoning
Latency
Cost
Privacy
Context
Modality
Availability
Reliability
Resource Requirements
User Preference
Policy
Risk
Urgency

Nenhum critério isolado deve necessariamente determinar a escolha.

---

8. Task Classification

Antes de escolher o modelo, a tarefa deve ser classificada.

Exemplo:

Task
 ↓
Classification
 ├── Simple
 ├── Normal
 ├── Complex
 ├── Deep
 └── Critical

Também pode ser classificada por modalidade:

Text
Vision
Audio
Video
Code
Multimodal

---

9. Níveis de processamento

A arquitetura utiliza quatro níveis principais.

Level 1 — Everyday

Tarefas rápidas.

Exemplos:

- perguntas simples;
- pequenas conversas;
- memória;
- classificação;
- pequenas transformações;
- comandos.

Objetivo:

«baixa latência.»

---

Level 2 — Enhanced

Tarefas intermediárias.

Exemplos:

- comparações;
- planejamento;
- análise de documentos;
- pesquisa moderada;
- explicações complexas.

Objetivo:

«equilíbrio entre qualidade e velocidade.»

---

Level 3 — Deep

Tarefas complexas.

Exemplos:

- pesquisa extensa;
- programação complexa;
- arquitetura;
- análise técnica;
- análise financeira;
- problemas multidisciplinares.

Objetivo:

«maximizar qualidade dentro dos recursos disponíveis.»

---

Level 4 — Focus / Extreme

Tarefas que exigem concentração computacional temporária.

Exemplos:

- missão extremamente complexa;
- grande pesquisa;
- desenvolvimento importante;
- processamento de grandes volumes;
- investigação técnica profunda.

A Yuki pode temporariamente direcionar uma parcela maior dos recursos disponíveis para a missão.

Entretanto:

«segurança, infraestrutura crítica e monitoramento essencial nunca devem ser privados de recursos necessários.»

---

10. Fast Path

Tarefas simples devem evitar o pipeline mais pesado possível.

Simple Task
 ↓
Fast Model / Local Processing
 ↓
Result

Isso reduz:

- latência;
- custo;
- consumo;
- complexidade.

---

11. Deep Path

Tarefas complexas podem utilizar um pipeline maior.

Complex Task
 ↓
Planner
 ↓
Model Router
 ↓
Specialized Models
 ↓
Research / Tools
 ↓
Verification
 ↓
Result

---

12. Model Selection Pipeline

Fluxo geral:

User / Event
     ↓
Intent
     ↓
Context
     ↓
Task Classification
     ↓
Capability Requirements
     ↓
Policy Check
     ↓
Candidate Models
     ↓
Filtering
     ↓
Scoring / Selection
     ↓
Execution
     ↓
Verification
     ↓
Result

---

13. Filtering antes da seleção

Modelos que não atendem aos requisitos devem ser eliminados antes da escolha.

Exemplo:

Task requires:
Vision
Large Context
Private Processing

Um modelo que não suporta visão não deve continuar como candidato.

---

14. Model Selection Score

O Router pode utilizar uma função de decisão interna.

Conceitualmente:

Score =
Quality
+ Capability Fit
+ Reliability
+ Availability
+ Privacy Fit
- Cost
- Latency
- Risk

Os pesos devem ser contextuais.

Não existe um único "melhor modelo" para todas as tarefas.

---

15. Context-aware Routing

O mesmo pedido pode usar modelos diferentes dependendo do contexto.

Exemplo:

"Explique isso."

Pode significar:

Texto simples
→ modelo rápido

ou:

Documento técnico complexo
→ modelo de raciocínio

ou:

Imagem + pergunta
→ modelo multimodal

O Router considera o contexto antes de selecionar.

---

16. User Preference

O usuário pode definir preferências.

Exemplo:

Prioridade:
Privacy > Cost > Speed

Ou:

Prioridade:
Quality > Speed

Essas preferências podem influenciar o Router.

Entretanto:

«Preferência do usuário não pode violar políticas de segurança.»

---

17. Privacy-aware Routing

Dados podem possuir diferentes níveis de sensibilidade.

PUBLIC
LOW
PRIVATE
SENSITIVE
CRITICAL

O Router deve considerar isso.

Exemplo:

Sensitive Data
 ↓
Privacy Policy
 ↓
Allowed Models

Um dado pode ser proibido de sair do ambiente local.

Nesse caso:

Cloud Model
✗

Local Model
✓

---

18. Processing Location

O Router deve poder selecionar:

EDGE
LOCAL
HOME
CLOUD
HYBRID

Exemplo:

Wake Word
→ Edge

Private Camera Analysis
→ Home Server

Large Research
→ Cloud

Sensitive Data + Large Model
→ Hybrid / approved private infrastructure

---

19. Hybrid Inference

Uma missão pode utilizar mais de uma infraestrutura.

Phone
 ↓
Preprocessing

Home Server
 ↓
Private Processing

Cloud
 ↓
Heavy Reasoning

Home Server
 ↓
Final Result

O Router deve ser capaz de montar esses pipelines.

---

20. Multi-model Collaboration

Uma tarefa pode usar vários modelos.

Exemplo:

Model A
→ Research

Model B
→ Reasoning

Model C
→ Verification

Fluxo:

Task
 ↓
Parallel Models
 ↓
Results
 ↓
Aggregator
 ↓
Verifier
 ↓
Final Result

---

21. Model Ensemble

Quando a importância da tarefa justificar, múltiplos modelos podem responder independentemente.

             Task
               │
       ┌───────┼───────┐
       ▼       ▼       ▼
     Model A Model B Model C
       │       │       │
       └───────┼───────┘
               ▼
           Aggregator
               ↓
          Verification

Isso pode aumentar robustez, mas aumenta custo e latência.

Portanto não deve ser utilizado indiscriminadamente.

---

22. Model Debate

Para tarefas que exigem alta confiabilidade, modelos podem assumir papéis diferentes.

Model A
→ Solve

Model B
→ Critique

Model C
→ Verify

O objetivo é identificar:

- erros;
- contradições;
- premissas incorretas;
- informações ausentes.

A utilização deve depender do risco e do benefício esperado.

---

23. Specialized Models

O Router deve permitir modelos especializados.

Exemplos:

General Reasoning
Coding
Vision
OCR
Speech Recognition
Speech Synthesis
Translation
Video
Image
Mathematics
Science
Security

A Yuki não precisa utilizar um modelo generalista para tudo.

---

24. Model Fallback

Quando o modelo principal falhar:

Primary
 ↓
Failure
 ↓
Fallback
 ↓
Verification

Possíveis falhas:

- timeout;
- indisponibilidade;
- quota;
- erro de API;
- modelo removido;
- rede indisponível;
- resultado inválido.

---

25. Fallback Chain

O sistema pode possuir uma cadeia:

Primary
 ↓
Secondary
 ↓
Local
 ↓
Minimal Capability

Exemplo:

Cloud Reasoning
 ↓
Other Cloud Model
 ↓
Home Model
 ↓
Basic Local Model

A Yuki deve indicar quando a capacidade foi degradada.

---

26. Graceful Degradation

Se nenhum modelo ideal estiver disponível, a Yuki pode reduzir capacidade.

Exemplo:

Full Vision Analysis
        ↓
Basic Vision
        ↓
OCR
        ↓
Text-only

O sistema deve evitar simplesmente falhar quando uma alternativa segura existir.

---

27. Model Health

Cada modelo deve possuir estado:

HEALTHY
DEGRADED
UNAVAILABLE
QUARANTINED
DEPRECATED

O Router deve evitar modelos indisponíveis.

---

28. Model Monitoring

Monitorar:

- latência;
- erro;
- custo;
- qualidade;
- disponibilidade;
- taxa de timeout;
- consumo;
- incidentes;
- mudanças de comportamento.

Isso alimenta decisões futuras do Router.

---

29. Model Evaluation

Modelos não devem ser avaliados apenas por benchmarks externos.

A Yuki deve possuir avaliações próprias.

Exemplos:

Yuki Benchmark
├── Reasoning
├── Coding
├── Vision
├── Context
├── Tool Use
├── Reliability
└── Safety

Isso permite avaliar modelos para as necessidades reais da Yuki.

---

30. Continuous Model Evaluation

Modelos podem mudar com o tempo.

Portanto:

Model
 ↓
Evaluation
 ↓
Production
 ↓
Monitoring
 ↓
Re-evaluation

Uma mudança do provider pode justificar nova avaliação.

---

31. Model Drift

O Router deve considerar que o comportamento de um serviço pode mudar.

Se a qualidade cair:

Detected Drift
 ↓
Re-evaluate
 ↓
Adjust Ranking
 ↓
Fallback / Replace

---

32. Model Versioning

Modelo e versão devem ser tratados separadamente.

Model A v1
Model A v2
Model A v3

O Router deve saber qual versão está sendo utilizada.

Quando necessário, missões podem registrar:

Model
Version
Provider
Configuration
Timestamp

Isso ajuda na reprodução e auditoria.

---

33. Reproducibility

Para tarefas importantes, deve ser possível reconstruir o ambiente utilizado.

Registrar:

Model
Version
Provider
Prompt Contract
Tools
Context Version
Configuration
Timestamp

Nem toda interação precisa possuir o mesmo nível de retenção, mas operações importantes devem ser rastreáveis conforme a política de auditoria.

---

34. Cost Router

O custo pode ser considerado pelo Router.

Categorias:

FREE / LOCAL
LOW
MEDIUM
HIGH
VERY HIGH

Uma tarefa simples não deve consumir automaticamente um modelo extremamente caro.

---

35. Latency Router

Algumas tarefas exigem resposta imediata.

Exemplo:

Voice Interaction
→ Low Latency

Enquanto:

Deep Research
→ Latency Less Important

A urgência da tarefa influencia a seleção.

---

36. Resource-aware Routing

O Router deve conhecer a disponibilidade computacional.

CPU
GPU
NPU
RAM
VRAM
Network
Cloud Quota
Battery

Se o celular estiver com bateria baixa:

Heavy Processing
→ Home / Cloud

em vez de:

Phone

---

37. Energy-aware Routing

Dispositivos móveis e vestíveis possuem limitações de energia.

A Yuki deve considerar:

Battery
Charging State
Thermal State
Available Compute

O objetivo é evitar desperdício.

---

38. Thermal-aware Routing

Dispositivos podem reduzir desempenho quando aquecidos.

High Temperature
 ↓
Reduce Local Work
 ↓
Remote Processing

Isso pode ser integrado ao Processing Router.

---

39. Network-aware Routing

O Router deve considerar:

- latência;
- estabilidade;
- largura de banda;
- custo;
- disponibilidade.

Exemplo:

Poor Network
 ↓
Local Processing

---

40. Offline Mode

A Yuki deve possuir capacidade degradada offline.

OFFLINE
 ↓
Local Models
 ↓
Local Memory
 ↓
Local Capabilities

Funções dependentes da internet devem ser identificadas.

---

41. Model Security

Modelos externos também devem ser tratados como componentes não totalmente confiáveis.

Um modelo pode:

- produzir informação incorreta;
- interpretar instruções erroneamente;
- gerar conteúdo malicioso;
- sugerir ações perigosas;
- sofrer manipulação por contexto externo.

Portanto:

«Modelo não é autoridade de segurança.»

A segurança permanece em camadas externas ao modelo.

---

42. Model Output ≠ Command

A saída de um modelo não deve automaticamente executar uma ação privilegiada.

Model Output
 ↓
Interpretation
 ↓
Policy
 ↓
Permission
 ↓
Execution

Nunca:

Model Output
 ↓
Direct Execution

---

43. Prompt Injection Resistance

Conteúdo externo pode tentar manipular o modelo.

Exemplo:

Web Page
 ↓
"Ignore previous instructions..."

Isso deve ser tratado como conteúdo não confiável.

A arquitetura deve separar:

System Policy
User Intent
Trusted Context
Untrusted Content
Model Output

---

44. Model Isolation

Um modelo comprometido ou malcomportado não deve possuir acesso direto a:

- secrets;
- Security Controller;
- infraestrutura crítica;
- permissões administrativas;
- memória completa;
- ferramentas irrestritas.

O modelo solicita.

A arquitetura decide.

---

45. Model Permissions

Modelos podem possuir diferentes permissões.

Exemplo:

Model
├── read_context
├── propose_action
├── tool_call
└── execute_action

Essas permissões devem ser separadas.

Um modelo pode possuir:

propose_action

sem possuir:

execute_action

---

46. Reasoning vs Execution

A Yuki deve separar:

Thinking

de:

Acting

Modelo:

Model
 ↓
Plan
 ↓
Security
 ↓
Permission
 ↓
Action

O modelo não concede autorização a si mesmo.

---

47. Model Router + Agent System

Agentes devem solicitar modelos através do Router.

Agent
 ↓
Model Router
 ↓
Selected Model

O agente não deve possuir uma dependência rígida com um provider específico.

Isso permite:

- substituição;
- fallback;
- otimização;
- testes;
- evolução.

---

48. Model Router + Capability System

A relação é:

Capability
 ↓
Required Model Capability
 ↓
Model Router
 ↓
Model

Exemplo:

Vision Capability
 ↓
vision + OCR
 ↓
Model Router
 ↓
Vision Model

---

49. Model Router + Memory

O Router deve receber apenas o contexto necessário.

Task
 ↓
Context Builder
 ↓
Relevant Memory
 ↓
Model Router
 ↓
Model

Não enviar automaticamente toda a memória da Yuki para todos os modelos.

---

50. Model Router + Security

Segurança deve estar acima da seleção de modelos.

Task
 ↓
Security / Policy
 ↓
Allowed Models
 ↓
Model Router

Um modelo extremamente poderoso não pode ser escolhido se a política não permitir o processamento daquele dado ou tarefa.

---

51. Model Router + Evolution

O Evolution Manager pode avaliar modelos.

Evolution Manager
 ↓
Research
 ↓
Benchmark
 ↓
Compare
 ↓
Model Registry
 ↓
Router Policy

A Yuki pode descobrir que um modelo novo é melhor para determinada categoria e atualizar suas preferências depois de validação.

---

52. Dynamic Model Discovery

A arquitetura deve permitir adicionar modelos no futuro.

New Model
 ↓
Registry
 ↓
Capabilities
 ↓
Evaluation
 ↓
Security
 ↓
Available to Router

O Core não precisa ser alterado.

---

53. Model Adapter

Cada provider ou runtime deve possuir um adapter quando necessário.

Model Router
 ↓
Provider Adapter
 ↓
Provider API

Isso abstrai diferenças entre APIs.

---

54. Provider Independence

A Yuki não deve ser arquiteturalmente dependente de uma empresa específica.

Ela deve conseguir:

Provider A
Provider B
Provider C
Local
Home Server
Future Provider

sem reconstruir o sistema.

---

55. Local-first quando apropriado

Quando privacidade, latência ou disponibilidade justificarem, o processamento local deve ser considerado primeiro.

Não significa:

«"Tudo deve ser local."»

Significa:

«"Local é uma opção arquitetural de primeira classe."»

---

56. Cloud-first quando apropriado

Quando uma tarefa exige:

- grande capacidade;
- modelo indisponível localmente;
- grande contexto;
- processamento pesado;

a nuvem pode ser utilizada.

A decisão deve ser contextual.

---

57. Hybrid-first Architecture

A arquitetura da Yuki deve ser naturalmente híbrida.

             YUKI
               │
       Processing Router
               │
       ┌───────┼────────┐
       ▼       ▼        ▼
      Edge    Home     Cloud

Nenhum desses ambientes deve ser considerado permanentemente superior.

---

58. Future Hardware

O Router deve ser capaz de utilizar futuros aceleradores.

Exemplos conceituais:

CPU
GPU
NPU
TPU
QPU
Neuromorphic
Future Accelerator

A arquitetura não deve depender da existência atual de qualquer um deles.

---

59. Model Selection Transparency

Quando necessário, a Yuki deve conseguir explicar:

«"Usei este modelo porque precisava de visão, baixa latência e processamento local."»

A explicação não precisa revelar detalhes internos sensíveis.

Mas deve existir rastreabilidade suficiente para auditoria.

---

60. User Control

O usuário pode definir preferências de alto nível.

Exemplo:

"Priorize privacidade."

"Economize custo."

"Quando for importante, use modelos mais poderosos."

"Para tarefas simples, seja rápida."

Essas preferências alimentam o Router.

---

61. No Permanent Best Model

Não existe um modelo permanentemente melhor para todas as tarefas.

O sistema deve evitar:

"Modelo X é o modelo oficial da Yuki para tudo."

Preferir:

Task
 ↓
Requirements
 ↓
Best Available Fit

---

62. Model Router Decision Example

Exemplo:

«"Yuki, veja essa questão pela câmera e me explique."»

Pipeline:

Voice
 ↓
Intent
 ↓
Camera Permission
 ↓
Perception
 ↓
Image
 ↓
Task Classification
 ↓
Vision + Reasoning Required
 ↓
Model Router
 ↓
Suitable Vision/Reasoning Model
 ↓
Answer

---

63. Deep Research Example

«"Yuki, faça uma pesquisa profunda sobre X."»

Mission
 ↓
Research Planner
 ↓
Parallel Search
 ↓
Specialized Models
 ↓
Document Analysis
 ↓
Reasoning
 ↓
Cross-check
 ↓
Verification
 ↓
Final Report

O Router pode utilizar vários modelos durante a missão.

---

64. Coding Example

«"Yuki, analise esse repositório e encontre o problema."»

Repository
 ↓
Context Builder
 ↓
Coding Model
 ↓
Reasoning Model
 ↓
Tests
 ↓
Verifier
 ↓
Result

A Yuki pode usar modelos diferentes para:

- entender código;
- criar solução;
- revisar;
- testar.

---

65. Model Router Failure

Se o Router não conseguir encontrar um modelo adequado:

No Suitable Model
 ↓
Alternative Capability?
 ↓
Reduced Mode?
 ↓
Ask User?

A Yuki não deve fingir que possui uma capacidade que não possui.

---

66. Unknown Model Capability

Um novo modelo pode apresentar uma capacidade ainda não catalogada.

Nesse caso:

Unknown Capability
 ↓
Evaluation
 ↓
Registry
 ↓
Policy
 ↓
Available

Isso permite evolução sem depender de atualizações manuais do Core.

---

67. Model Lifecycle

DISCOVERED
    ↓
EVALUATING
    ↓
APPROVED
    ↓
AVAILABLE
    ↓
ACTIVE
    ↓
DEGRADED
    ↓
DEPRECATED
    ↓
REMOVED

---

68. Model Quarantine

Se um modelo apresentar comportamento inesperado:

Anomaly
 ↓
Detection
 ↓
Quarantine
 ↓
Investigation
 ↓
Re-evaluation

Durante a quarentena, o Router deve deixar de utilizá-lo para tarefas incompatíveis com o estado de segurança.

---

69. Model Governance

Modelos importantes devem possuir:

- origem conhecida;
- versão identificada;
- avaliação;
- política;
- permissões;
- monitoramento;
- mecanismo de remoção.

---

70. Model Router Official Principles

1. O Core não depende diretamente de modelos.
2. Modelo e provider são conceitos diferentes.
3. Modelos são selecionados por tarefa e contexto.
4. Não existe um único modelo ideal para tudo.
5. Privacidade influencia o roteamento.
6. Custo influencia o roteamento.
7. Latência influencia o roteamento.
8. Disponibilidade influencia o roteamento.
9. Qualidade influencia o roteamento.
10. Risco e política podem restringir modelos.
11. Modelos não são autoridades de segurança.
12. Output de modelo não equivale a autorização.
13. Modelos devem ser isolados de privilégios críticos.
14. Fallback deve existir quando possível.
15. Degradação controlada deve ser possível.
16. Modelos podem trabalhar em conjunto.
17. Modelos especializados devem ser suportados.
18. Modelos locais são cidadãos de primeira classe.
19. Cloud e local devem coexistir.
20. O hardware deve ser abstraído.
21. Modelos devem ser avaliados continuamente.
22. Mudanças de comportamento devem ser detectáveis.
23. Modelos podem ser substituídos sem alterar o Core.
24. Novos modelos devem poder ser incorporados dinamicamente.
25. O Router deve permanecer independente de providers específicos.
26. A Yuki deve poder utilizar modelos futuros ainda desconhecidos.
27. O usuário pode definir preferências de alto nível.
28. Preferências nunca substituem políticas de segurança.
29. Processamento deve ser proporcional à necessidade da tarefa.
30. A seleção de modelo deve ser rastreável quando necessário.

---

71. Arquitetura consolidada

                         YUKI CORE
                              │
                              ▼
                     TASK / MISSION
                              │
                              ▼
                       CONTEXT BUILDER
                              │
                              ▼
                     TASK CLASSIFICATION
                              │
                              ▼
                     CAPABILITY REQUIREMENTS
                              │
                              ▼
                    SECURITY / POLICY CHECK
                              │
                              ▼
                       MODEL ROUTER
                              │
             ┌────────────────┼────────────────┐
             ▼                ▼                ▼
          LOCAL            CLOUD            EDGE
             │                │                │
             └────────────────┼────────────────┘
                              │
                    ┌─────────┴─────────┐
                    ▼                   ▼
                Model A              Model B
                    │                   │
                    └─────────┬─────────┘
                              ▼
                         AGGREGATOR
                              │
                              ▼
                         VERIFICATION
                              │
                              ▼
                            RESULT

---

72. Relação com outros sistemas

O Model Router se integra principalmente com:

03_CORE.md
04_MEMORY.md
05_PERSONAL_CONTEXT.md
07_AGENTS_AND_TASKS.md
08_CAPABILITY_SYSTEM.md
10_SECURITY.md
11_EVOLUTION.md
12_VOICE_AND_MULTIMODAL.md
13_EVENTS_AND_BACKGROUND.md
14_INFRASTRUCTURE.md

O "08_CAPABILITY_SYSTEM.md" define o que uma capacidade é.

O "09_MODEL_ROUTER.md" define qual infraestrutura de IA pode executar a parte cognitiva necessária para aquela capacidade.

---

73. Estado atual

Model Router: arquitetura conceitual definida.

Componentes definidos:

- Model Registry;
- Model Manifest;
- Model Capability;
- Provider Abstraction;
- Model Adapter;
- Task Classification;
- Model Selection;
- Fast Path;
- Deep Path;
- Focus Mode;
- Multi-model Collaboration;
- Model Ensemble;
- Model Debate;
- Specialized Models;
- Fallback;
- Graceful Degradation;
- Privacy-aware Routing;
- Cost-aware Routing;
- Latency-aware Routing;
- Resource-aware Routing;
- Energy-aware Routing;
- Thermal-aware Routing;
- Network-aware Routing;
- Offline Mode;
- Local Processing;
- Cloud Processing;
- Hybrid Processing;
- Model Evaluation;
- Model Monitoring;
- Model Drift Detection;
- Model Versioning;
- Model Quarantine;
- Model Governance;
- Dynamic Model Discovery;
- Future Hardware Abstraction.

Status: "ARCHITECTURE DEFINED"