Yuki — Multi-Agent & Task System

Versão: 0.1
Status: Arquitetura definida
Data: 2026-09-11

---

1. Objetivo

O Multi-Agent & Task System é responsável por permitir que a Yuki transforme objetivos complexos em tarefas executáveis, organize dependências, distribua trabalho entre agentes especializados, execute tarefas em paralelo quando apropriado, verifique resultados e mantenha o estado de missões de longa duração.

O sistema deve permitir que a Yuki execute desde tarefas simples até missões complexas que podem durar horas, dias ou mais.

A arquitetura deve permanecer modular e independente de modelos específicos de IA.

---

2. Princípios

O sistema segue os princípios fundamentais da arquitetura Yuki:

1. Missão não é tarefa.
2. Tarefa não é agente.
3. Nem toda tarefa precisa de múltiplos agentes.
4. Paralelismo deve ser utilizado quando realmente trouxer benefício.
5. Execução não significa conclusão.
6. Resultados importantes devem ser verificados.
7. Agentes não possuem autorização implícita.
8. O Supervisor coordena, mas não concede permissões.
9. Segurança permanece independente do sistema de agentes.
10. Missões devem poder ser pausadas e retomadas.
11. O contexto necessário deve ser fornecido aos agentes, evitando exposição desnecessária de dados.
12. Resultados temporários não devem automaticamente virar memória permanente.
13. Falhas devem ser tratadas de forma controlada.
14. Nenhuma tentativa pode gerar loops infinitos ou gastos ilimitados.
15. O usuário continua sendo a autoridade final sobre ações relevantes.

---

3. Mission System

Uma Mission representa um objetivo de nível superior.

Exemplo:

«Encontrar o melhor notebook para programação dentro de determinado orçamento.»

A missão pode permanecer ativa durante segundos, minutos, horas ou dias.

Uma missão pode conter várias tarefas relacionadas.

3.1 Estrutura conceitual

MISSION
│
├── Objective
├── Context
├── Constraints
├── Priority
├── Deadline
├── Task Graph
├── Progress
├── State
├── Results
├── Decisions
├── Errors
└── History

3.2 Identificação

Cada missão deve possuir um identificador único:

mission_id

Exemplo:

M-2026-0001

---

4. Mission States

Uma missão pode possuir estados como:

CREATED
READY
RUNNING
PAUSED
BLOCKED
COMPLETED
FAILED
CANCELLED

Fluxo comum:

CREATED
   ↓
READY
   ↓
RUNNING
   ↓
COMPLETED

Possíveis interrupções:

RUNNING
├── PAUSED
├── BLOCKED
├── FAILED
└── CANCELLED

Uma missão pausada deve poder ser retomada posteriormente.

---

5. Task System

Uma Task representa uma unidade de trabalho necessária para completar uma missão.

Exemplo:

MISSION
└── Comprar notebook
    ├── Identificar requisitos
    ├── Pesquisar modelos
    ├── Comparar especificações
    ├── Pesquisar preços
    ├── Verificar lojas
    ├── Avaliar custo-benefício
    └── Produzir recomendação

Uma tarefa deve possuir identidade e estado próprios.

---

6. Task Structure

Cada tarefa deverá possuir, conceitualmente:

Task
├── task_id
├── mission_id
├── description
├── objective
├── assigned_agent
├── state
├── priority
├── dependencies
├── inputs
├── outputs
├── required_tools
├── required_permissions
├── deadline
├── attempts
├── resource_limits
├── cost
├── evidence
├── verification
├── errors
└── history

Nem todos os campos precisam ser obrigatórios em todas as tarefas.

---

7. Task Graph

As tarefas de uma missão serão organizadas através de um grafo de dependências.

Sempre que possível, o sistema deverá utilizar uma estrutura semelhante a um DAG (Directed Acyclic Graph).

Exemplo:

              Requisitos
                  │
          ┌───────┴───────┐
          ▼               ▼
      Modelos          Orçamento
          │               │
          └───────┬───────┘
                  ▼
             Comparação
                  │
       ┌──────────┼──────────┐
       ▼          ▼          ▼
    Preços       Lojas     Reviews
       │          │          │
       └──────────┼──────────┘
                  ▼
             Avaliação
                  │
                  ▼
             Recomendação

As tarefas independentes poderão ser executadas simultaneamente.

Tarefas dependentes deverão aguardar a conclusão das dependências necessárias.

---

8. Parallelism

A Yuki deve utilizar paralelismo quando ele melhorar:

- velocidade;
- utilização de recursos;
- qualidade;
- disponibilidade;
- eficiência.

Exemplo:

Pesquisa A ──┐
Pesquisa B ──┼──→ Comparação
Pesquisa C ──┘

As três pesquisas podem ocorrer simultaneamente.

Entretanto, a Yuki não deve criar complexidade desnecessária apenas para executar tarefas em paralelo.

---

9. When Not to Use Multi-Agent

Nem toda missão deve utilizar múltiplos agentes.

Para tarefas simples:

User
 ↓
Yuki
 ↓
Capability
 ↓
Result

Para tarefas mais complexas:

User
 ↓
Yuki
 ↓
Supervisor
 ↓
Task Graph
 ↓
Agents
 ↓
Verification
 ↓
Result

A decisão de utilizar múltiplos agentes deverá considerar:

- complexidade;
- quantidade de subtarefas;
- independência entre tarefas;
- especialização necessária;
- custo;
- latência;
- risco;
- qualidade esperada.

---

10. Supervisor

O Supervisor é responsável por coordenar a execução de uma missão.

Suas responsabilidades incluem:

- decompor missões;
- criar tarefas;
- identificar dependências;
- selecionar agentes;
- distribuir tarefas;
- acompanhar execução;
- controlar estado;
- identificar bloqueios;
- evitar trabalho duplicado;
- reunir resultados;
- solicitar verificação;
- determinar se a missão está completa.

Arquitetura:

                    SUPERVISOR
                         │
          ┌──────────────┼──────────────┐
          ▼              ▼              ▼
       Research         Vision         Coding
        Agent            Agent          Agent

---

11. Limites do Supervisor

O Supervisor não é o sistema de segurança.

Ele não pode conceder a si próprio ou aos agentes novas permissões.

Por exemplo:

Supervisor
   │
   └── "Preciso acessar a API X"
                │
                ▼
        Security Controller
                │
          autorização?
           /          \
         SIM           NÃO
          │             │
          ▼             ▼
       executar       bloquear

A autorização permanece sob responsabilidade da camada de segurança e política.

---

12. Agent Registry

A Yuki possuirá um registro central de agentes disponíveis.

Exemplo:

Agent Registry
│
├── Research Agent
├── Coding Agent
├── Vision Agent
├── Finance Agent
├── Academic Agent
├── Planning Agent
├── Security Agent
├── Health Agent
├── Communication Agent
├── Creative Agent
└── General Agent

O registro deverá permitir descobrir quais agentes estão disponíveis e quais tarefas eles conseguem executar.

---

13. Agent Contract

Cada agente deverá possuir um contrato.

Conceitualmente:

Agent
├── ID
├── Version
├── Description
├── Supported Tasks
├── Inputs
├── Outputs
├── Tools
├── Permissions
├── Risk Level
├── Resource Limits
├── Dependencies
└── Capabilities

Isso permite substituir a implementação de um agente sem alterar o Core da Yuki.

---

14. Task ≠ Agent

Uma tarefa não representa necessariamente um agente.

Exemplo:

Task:
Pesquisar preços

Pode ser executada por:

Research Agent
   │
   ├── Web Search
   ├── Shopping Capability
   └── Price Comparison

Uma tarefa também pode, quando necessário, envolver mais de um agente.

O sistema deve escolher a arquitetura que melhor atende à missão.

---

15. Task Board

A Yuki deverá possuir uma representação persistente do estado das tarefas.

Exemplo:

MISSION: Pesquisar notebook

[✓] Identificar requisitos
[✓] Pesquisar modelos
[▶] Comparar hardware
[▶] Pesquisar preços
[ ] Verificar lojas
[ ] Avaliar custo-benefício
[ ] Produzir recomendação

O Task Board funciona como uma visão operacional da missão.

Ele também deverá permitir que o Supervisor saiba:

- o que terminou;
- o que está executando;
- o que está bloqueado;
- o que pode iniciar;
- o que falhou;
- o que precisa ser verificado.

---

16. Task States

A máquina de estados padrão será:

PENDING
   ↓
READY
   ↓
RUNNING
   ↓
VERIFYING
   ↓
COMPLETED

Estados alternativos:

RUNNING
├── FAILED
├── TIMEOUT
├── BLOCKED
└── CANCELLED

---

17. Retry System

Falhas não devem necessariamente encerrar uma tarefa.

Quando apropriado:

FAILED
   ↓
Retry Policy
   │
 ┌─┴──────┐
 ▼        ▼
RETRY    STOP
 │
 ▼
RUNNING

O sistema deverá controlar:

- número máximo de tentativas;
- intervalo entre tentativas;
- custo acumulado;
- tempo acumulado;
- motivo da falha;
- possibilidade de usar outra ferramenta;
- possibilidade de usar outro agente;
- necessidade de intervenção humana.

Retries infinitos não são permitidos.

---

18. Verification

A conclusão de uma execução não significa que o resultado está correto.

Portanto:

Agent
  ↓
Result
  ↓
Verifier
  ↓
VALID?
 /   \
YES   NO
 |     |
 ▼     ▼
DONE  CORRECT/RETRY

A verificação deverá ser proporcional ao risco e à importância da tarefa.

---

19. Verification Examples

Em uma pesquisa de produto, a Yuki poderá verificar:

- existência do produto;
- correspondência do modelo;
- preço;
- data da informação;
- loja;
- confiabilidade da fonte;
- consistência entre fontes.

Em programação:

- compilação;
- testes;
- análise estática;
- comportamento esperado;
- regressões;
- segurança;
- revisão.

Em tarefas críticas:

- múltiplas verificações;
- confirmação de identidade;
- autorização;
- evidências;
- aprovação humana quando necessário.

---

20. Agent Independence and Security

Os agentes nunca devem possuir autoridade irrestrita.

O fluxo deverá ser:

Agent
 ↓
Tool Request
 ↓
Security Controller
 ↓
Policy
 ↓
Permission
 ↓
Execution

O agente não deve conseguir simplesmente declarar:

«"Tenho permissão."»

A permissão precisa ser validada externamente.

---

21. Data Minimization

Cada agente deverá receber somente os dados necessários para realizar sua tarefa.

Exemplo:

Mission Context
      │
      ▼
Context Builder
      │
      ├── Research Agent → somente contexto de pesquisa
      ├── Vision Agent   → somente imagem necessária
      └── Finance Agent  → somente dados financeiros autorizados

O fato de a Yuki possuir acesso a uma informação não significa que todos os agentes devem recebê-la.

---

22. Mission Persistence

Missões de longa duração devem ser persistentes.

Exemplo:

MISSION: M-2026-001

Status: PAUSED
Progress: 67%

Completed:
✓ Pesquisa
✓ Coleta
✓ Comparação

Pending:
□ Verificação
□ Relatório

Quando a Yuki retornar à missão, deverá reconstruir seu estado a partir de dados persistidos.

A recuperação deverá considerar:

Mission
+
Task Graph
+
Task States
+
Context
+
Results
+
Decisions
+
Errors
+
Relevant Memory

---

23. Independence from Context Window

Uma missão não pode depender da janela de contexto de um único modelo.

Isso é fundamental para a visão de longo prazo da Yuki.

Um modelo pode ser:

- substituído;
- atualizado;
- removido;
- temporariamente indisponível.

A missão deve continuar existindo independentemente disso.

---

24. Mission Resume

Ao retomar uma missão:

Persistent Mission
        ↓
State Recovery
        ↓
Context Reconstruction
        ↓
Dependency Analysis
        ↓
Find READY Tasks
        ↓
Resume Execution

A Yuki não deve simplesmente repetir todo o trabalho anterior.

Ela deve aproveitar os resultados já validados.

---

25. Shared Context

Os agentes podem utilizar informações compartilhadas, mas o compartilhamento deve ser controlado.

Arquitetura:

                    Mission Context
                          │
             ┌────────────┼────────────┐
             ▼            ▼            ▼
          Research      Vision       Coding
           Agent         Agent        Agent

Cada agente recebe apenas o subconjunto necessário.

---

26. Agent Communication

Quando agentes precisarem trocar informações:

Agent A
   ↓
Structured Result
   ↓
Task / Context System
   ↓
Agent B

O sistema não deverá depender de conversas livres e ilimitadas entre agentes.

Resultados devem preferencialmente possuir estrutura definida.

Isso facilita:

- validação;
- auditoria;
- persistência;
- recuperação;
- depuração;
- controle de custos.

---

27. Conflict Resolution

Agentes podem produzir resultados diferentes.

Exemplo:

Research Agent A → Produto X
Research Agent B → Produto Y

O Supervisor não deverá simplesmente escolher arbitrariamente.

Deverá:

1. identificar o conflito;
2. verificar evidências;
3. comparar confiabilidade;
4. solicitar nova análise quando necessário;
5. utilizar outro agente/modelo quando apropriado;
6. registrar a decisão.

Quando a incerteza permanecer relevante, a Yuki poderá consultar o usuário.

---

28. Duplicate Work Prevention

O Supervisor deverá manter conhecimento sobre tarefas já executadas.

Antes de criar uma nova tarefa:

New Task
   ↓
Existe tarefa equivalente?
 /        \
YES       NO
 |         |
Reuse     Create
Result    Task

Isso evita desperdício de:

- tempo;
- tokens;
- chamadas de API;
- dinheiro;
- processamento.

---

29. Resource Management

Cada missão e tarefa poderá possuir limites de:

- tempo;
- tokens;
- chamadas de API;
- processamento;
- armazenamento;
- custo financeiro;
- número de agentes;
- número de tentativas.

Exemplo:

Mission Budget
├── Time: 30 min
├── API Cost: R$ X
├── Max Attempts: 3
└── Max Parallel Tasks: 5

Missões que atingirem limites deverão ser:

- interrompidas;
- degradadas;
- replanejadas;
- ou submetidas ao usuário.

---

30. Dynamic Replanning

O plano inicial não precisa permanecer imutável.

Se ocorrer:

Task Failed

o Supervisor poderá replanejar:

Original Plan
     ↓
Failure
     ↓
Analyze
     ↓
Alternative Plan
     ↓
Continue

Porém, alterações relevantes de escopo, custo, risco ou autorização devem respeitar as políticas de segurança e, quando necessário, solicitar aprovação humana.

---

31. Integration with Memory

Os resultados das tarefas não serão automaticamente armazenados como memória permanente.

Fluxo:

Task Result
     ↓
Memory Manager
     ↓
Future Value?
   /       \
 NO        YES
 │           │
 ▼           ▼
Archive    Consolidate
/Discard   into Memory

Podem ser armazenados, quando relevantes:

- decisões;
- descobertas;
- preferências;
- fatos importantes;
- aprendizados;
- resultados reutilizáveis;
- histórico de projetos;
- informações necessárias para continuidade.

---

32. Integration with Knowledge Graph

Missões e tarefas podem gerar relações úteis para o Knowledge Graph.

Exemplo:

Notebook X
   │
   ├── possui CPU Y
   ├── vendido por Loja Z
   ├── pesquisado em Data A
   └── recomendado para Projeto B

Isso permite que futuras missões aproveitem conhecimento anterior.

---

33. Integration with Context Engine

O Task System deverá trabalhar junto ao Personal Context Engine.

Exemplo:

Mission
   ↓
Context Builder
   ↓
Relevant Context
   ↓
Planning
   ↓
Task Graph

O contexto atual pode alterar a estratégia.

Exemplo conceitual:

Agenda cheia
+
Prazo próximo
+
Pouco tempo disponível
        ↓
Plano mais rápido

A Yuki deve considerar o contexto sem transformar o usuário em subordinado às decisões do sistema.

---

34. Human-in-the-Loop

A Yuki poderá solicitar intervenção humana quando:

- houver ambiguidade importante;
- a tarefa for de alto risco;
- a autorização não estiver clara;
- houver conflito relevante;
- houver impacto financeiro significativo;
- houver alteração crítica;
- a confiança estiver abaixo do limite;
- o usuário precisar tomar uma decisão.

Fluxo:

Agent / Supervisor
       ↓
Need Human Decision?
       ↓
      YES
       ↓
     USER
       ↓
 Decision
       ↓
 Continue

---

35. Mission Completion

Uma missão somente deverá ser considerada concluída quando:

1. todas as tarefas obrigatórias estiverem concluídas;
2. resultados relevantes tiverem sido verificados;
3. dependências tiverem sido resolvidas;
4. não existirem bloqueios relevantes;
5. os critérios de sucesso tiverem sido atingidos.

Fluxo:

Tasks Complete
      ↓
Verification
      ↓
Success Criteria
      ↓
Mission Complete
      ↓
Review
      ↓
Memory Consolidation

---

36. Mission Review

Após uma missão importante, a Yuki poderá realizar uma revisão:

Mission
 ↓
Review
 ├── What worked?
 ├── What failed?
 ├── What was expensive?
 ├── What was slow?
 ├── What could improve?
 └── What should be remembered?

Essas informações podem alimentar:

- métricas;
- memória;
- evolução;
- otimização;
- futuras decisões de planejamento.

---

37. Relationship with Evolution System

O Multi-Agent System também poderá gerar dados para o Evolution Manager.

Exemplo:

100 missions
      ↓
Metrics
      ↓
Research Agent frequently fails
      ↓
Evolution Manager
      ↓
Investigate
      ↓
Prototype improvement
      ↓
Test
      ↓
Approve
      ↓
Deploy

A evolução não poderá utilizar falhas como justificativa automática para aumentar privilégios.

---

38. Example — Complex Mission

Usuário:

«"Pesquise o melhor notebook para eu programar, considerando meu orçamento e minhas necessidades."»

A Yuki poderá executar:

MISSION
│
├── 1. Entender requisitos
│
├── 2. Pesquisar modelos
│      │
│      ├── Pesquisa A
│      ├── Pesquisa B
│      └── Pesquisa C
│
├── 3. Comparar hardware
│
├── 4. Pesquisar preços
│      │
│      ├── Loja A
│      ├── Loja B
│      └── Loja C
│
├── 5. Verificar confiabilidade
│
├── 6. Avaliar custo-benefício
│
└── 7. Produzir recomendação

A Yuki poderia utilizar:

Research Agent
Vision Agent
General Agent

e ferramentas de:

Web
Shopping
Product Database
Memory
User Preferences

Depois:

Results
 ↓
Verification
 ↓
Comparison
 ↓
Recommendation
 ↓
User

---

39. Example — Programming Mission

Usuário:

«"Crie uma nova capability para a Yuki."»

A missão poderia ser:

MISSION
│
├── analisar requisitos
├── estudar arquitetura atual
├── criar especificação
├── implementar
├── criar testes
├── executar testes
├── análise de segurança
├── revisão
├── documentação
└── preparar deployment

Algumas tarefas podem ocorrer paralelamente.

Nenhuma alteração crítica deve chegar à produção sem passar pelas políticas de segurança e governança apropriadas.

---

40. Example — Long-Term Mission

Uma missão pode durar meses.

Exemplo:

MISSION
"Construir o sistema Yuki Home"

Durante meses:

Architecture
   ↓
Backend
   ↓
Sensors
   ↓
Automation
   ↓
Security
   ↓
Testing
   ↓
Deployment

A missão poderá ser pausada diversas vezes.

Seu estado permanecerá persistido.

---

41. Architecture Overview

A arquitetura consolidada:

                         USER
                           │
                           ▼
                        YUKI CORE
                           │
                           ▼
                      SUPERVISOR
                           │
                           ▼
                     MISSION MANAGER
                           │
                           ▼
                      TASK GRAPH
                           │
              ┌────────────┼────────────┐
              ▼            ▼            ▼
           AGENT A       AGENT B      AGENT C
              │            │            │
              └────────────┼────────────┘
                           ▼
                      RESULT MANAGER
                           │
                           ▼
                        VERIFIER
                           │
                    ┌──────┴──────┐
                    ▼             ▼
                 RESULT         RETRY
                    │
                    ▼
             MEMORY / KNOWLEDGE
                    │
                    ▼
                  REVIEW

Segurança permanece transversal:

              SECURITY CONTROLLER
                      │
      ┌───────────────┼────────────────┐
      ▼               ▼                ▼
 Identity         Policy          Permissions
      │               │                │
      └───────────────┼────────────────┘
                      ▼
                Tool Execution

---

42. Official Rules

As seguintes regras são consideradas parte da arquitetura oficial:

Rule 01

Uma missão representa um objetivo; tarefas representam trabalho necessário para alcançá-lo.

Rule 02

Uma tarefa não implica a existência de um agente exclusivo.

Rule 03

Multi-agent será utilizado somente quando trouxer benefício real.

Rule 04

Tarefas independentes devem poder ser executadas em paralelo.

Rule 05

Dependências devem controlar a ordem de execução.

Rule 06

Resultados importantes devem ser verificados.

Rule 07

O Supervisor coordena, mas não concede privilégios.

Rule 08

Segurança e autorização permanecem independentes do sistema de agentes.

Rule 09

Missões devem possuir estado persistente.

Rule 10

Missões longas devem poder ser pausadas e retomadas.

Rule 11

Retries devem possuir limites.

Rule 12

Nenhum agente deve possuir acesso a dados além do necessário.

Rule 13

Resultados não devem automaticamente virar memória permanente.

Rule 14

Conflitos entre agentes devem ser identificados e tratados.

Rule 15

O sistema deve evitar trabalho duplicado.

Rule 16

Recursos devem possuir limites controláveis.

Rule 17

Planos podem ser adaptados quando novas informações surgirem.

Rule 18

A Yuki deve pedir intervenção humana quando a decisão exceder sua autorização ou confiança.

Rule 19

O sistema deve permanecer independente de modelos específicos.

Rule 20

O usuário permanece a autoridade final sobre decisões pessoais e ações críticas.

---

43. Future Extensions

O sistema deverá permitir futuramente:

- agentes temporários;
- agentes especializados criados dinamicamente;
- agentes locais;
- agentes remotos;
- agentes baseados em diferentes modelos;
- votação entre agentes;
- ensemble de modelos;
- agentes especializados por projeto;
- agentes de monitoramento contínuo;
- agentes de simulação;
- agentes de teste;
- agentes de segurança;
- agentes de desenvolvimento;
- agentes capazes de utilizar outras IAs.

Essas extensões deverão respeitar os contratos, permissões e políticas da arquitetura.

---

44. Final Architecture Principle

O Multi-Agent System da Yuki não será uma coleção de IAs conversando aleatoriamente.

Ele será um sistema coordenado de execução de missões, no qual:

OBJETIVO
   ↓
MISSÃO
   ↓
PLANEJAMENTO
   ↓
TASK GRAPH
   ↓
AGENTES / CAPABILITIES
   ↓
EXECUÇÃO
   ↓
VERIFICAÇÃO
   ↓
RESULTADO
   ↓
MEMÓRIA / CONHECIMENTO
   ↓
APRENDIZADO / EVOLUÇÃO

Sempre sob:

IDENTIDADE
PERMISSÕES
POLÍTICAS
SEGURANÇA
AUDITORIA
LIMITES

A arquitetura deve permitir que a Yuki se torne progressivamente mais capaz sem transformar capacidade em autoridade automática.