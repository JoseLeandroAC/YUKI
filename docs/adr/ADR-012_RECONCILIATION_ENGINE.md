# ADR-012 — Yuki Reconciliation Engine, Desired State & Continuous Convergence

**Arquivo:** `docs/ADR-012_RECONCILIATION_ENGINE.md`
**Versão:** 1.0
**Status:** ACCEPTED
**Domínio:** Reconciliation / State Management / Distributed Systems
**Data:** 2026-09-23

---

## 1. Objetivo

Este ADR define a arquitetura do **Reconciliation Engine da Yuki**, responsável por comparar estados desejados com estados observados, detectar divergências relevantes e produzir intenções declarativas de convergência.

O Reconciliation Engine deve permitir que a Yuki mantenha sistemas, recursos e ambientes próximos de estados desejados ao longo do tempo, inclusive quando mudanças externas, falhas, intervenções humanas ou eventos inesperados produzirem divergência.

O Reconciliation Engine **não possui autoridade soberana para executar mudanças**.

Seu papel é:

```text
Observar
→ Comparar
→ Avaliar
→ Detectar Drift
→ Decidir se há necessidade de convergência
→ Produzir Reconciliation Intent
```

A execução posterior pertence às camadas apropriadas de planejamento, autorização, workflow e execução.

---

# 2. Contexto

A Yuki não opera apenas em tarefas pontuais.

Muitos sistemas possuem estados que precisam permanecer próximos de uma condição desejada durante períodos prolongados:

* temperatura de uma residência;
* configuração de dispositivos;
* infraestrutura;
* backups;
* serviços;
* recursos computacionais;
* contas e reservas financeiras;
* ambientes de software;
* sistemas robóticos;
* configurações de segurança;
* projetos;
* metas pessoais;
* sistemas distribuídos.

Uma operação tradicional responde:

> "Execute esta ação."

Reconciliação responde:

> "O estado atual ainda corresponde ao estado desejado?"

Essa diferença é fundamental.

Uma ação pode terminar com sucesso e, posteriormente:

* outro sistema pode alterar o estado;
* um usuário pode modificar a configuração;
* um dispositivo pode falhar;
* uma conexão pode cair;
* uma política pode mudar;
* o estado observado pode ficar desatualizado;
* um componente pode ser substituído.

Portanto, a Yuki precisa de uma arquitetura capaz de avaliar continuamente a relação entre:

```text
Desired State
Observed State
Policy
Constraint
Authorization
```

sem transformar reconciliação em um superagente executor.

---

# 3. Escopo

Este ADR define:

* Desired State;
* Observation;
* Evidence;
* Evidence Validation;
* Observed State;
* State Comparison;
* Drift;
* Unknown State;
* Reconciliation Decision;
* Reconciliation Intent;
* Convergence;
* Stability Mechanisms;
* Human Override;
* Ownership;
* Domain Authority;
* Distributed Reconciliation;
* Domain Reconcilers;
* limites entre Reconciliation, Verification, Planning e Workflow;
* integração com Security Controller;
* integração com Safety Controller;
* integração com ADR-009, ADR-010 e ADR-011;
* invariantes de segurança;
* decisões arquiteturais.

Este ADR **não define**:

* tecnologia específica de implementação;
* banco específico;
* sistema específico de eventos;
* algoritmo universal de convergência;
* formato final definitivo do Desired State;
* sistema definitivo de políticas;
* autoridade universal do Home Server;
* algoritmo universal de confiança;
* mecanismo específico de eleição distribuída;
* implementação específica de polling/event-driven.

Esses temas permanecem abertos ou pertencem a ADRs posteriores.

---

# 4. Princípio Constitucional de Separação de Autoridade

A arquitetura estabelece uma separação explícita:

```text
Desired State
      ≠
Policy
      ≠
Constraint
      ≠
Authorization
      ≠
Plan
      ≠
Execution
```

Cada conceito possui responsabilidade própria.

### Desired State

Declara o estado desejado.

### Policy

Define regras de governança.

### Constraint

Define limites ou invariantes que não devem ser violados.

### Authorization

Concede permissão para determinada ação dentro de determinado contexto.

### Plan

Define como um objetivo ou intenção pode ser alcançado.

### Execution

Realiza efetivamente uma operação.

Essa separação impede que uma declaração de estado desejado seja interpretada automaticamente como autorização para modificar o mundo.

---

# 5. Definições Fundamentais

## 5.1 Goal

Objetivo de nível superior.

Exemplo:

```text
Manter a casa confortável.
```

Goals normalmente pertencem ao domínio de planejamento e intenção humana.

---

## 5.2 Intent

Tradução operacional de um objetivo.

Exemplo:

```text
Manter a sala entre 21°C e 23°C.
```

Intent pode ser produzido pelo Planner/Mission Manager.

---

## 5.3 Desired State

Representação declarativa do estado que deve ser buscado.

Exemplo:

```text
Sala:
temperatura desejada = 22°C
tolerância = ±1°C
```

Desired State não é comando.

---

## 5.4 Policy

Regra que governa comportamento.

Exemplo:

```text
Não ligar ar-condicionado durante determinado horário.
```

---

## 5.5 Constraint

Limite ou invariante.

Exemplo:

```text
Temperatura máxima permitida pelo equipamento = 25°C.
```

Constraints podem ser de segurança, integridade, recursos, infraestrutura ou domínio.

---

## 5.6 Authorization

Permissão concedida para determinada ação.

Authorization pode ser:

* temporal;
* contextual;
* limitada;
* revogável;
* vinculada a identidade;
* vinculada a recurso;
* vinculada a operação.

Desired State não concede Authorization.

---

## 5.7 Preference

Preferência utilizada quando múltiplas opções são compatíveis com Desired State, Policy e Constraint.

Exemplo:

```text
Preferir menor consumo energético.
```

---

## 5.8 Plan

Sequência ou estrutura operacional usada para alcançar uma intenção.

O Plan pertence ao Planner/Mission Manager/Workflow.

---

# 6. Desired State

Desired State é:

> uma declaração estruturada, versionada e temporalmente delimitada que representa uma condição desejada para uma entidade, recurso, ambiente ou processo.

Desired State pode conter:

* identidade;
* versão;
* origem;
* autoridade proprietária;
* escopo temporal;
* entidade alvo;
* propriedades desejadas;
* tolerâncias;
* condições;
* referências a políticas;
* referências a constraints;
* requisitos de observação;
* regras de expiração;
* prioridade contextual;
* histórico.

A autenticidade e integridade do Desired State devem ser verificáveis de acordo com o contexto.

Assinaturas digitais podem ser utilizadas quando apropriadas, mas não constituem requisito universal da arquitetura.

---

# 7. Desired State não é Comando

Um Desired State como:

```yaml
temperature: 22
```

não significa:

```text
execute set_temperature(22)
```

Significa:

```text
a Yuki deseja que a propriedade
temperature
converja para 22.
```

A transformação:

```text
Desired State
→ Reconciliation
→ Reconciliation Intent
→ Planning
→ Authorization
→ Workflow
→ Execution
```

é deliberadamente separada.

---

# 8. Desired State Lifecycle

Desired States possuem ciclo de vida próprio.

Estados mínimos:

```text
CREATED
ACTIVE
OVERRIDE_SUSPENDED
EXPIRED
SUPERSEDED
CANCELLED
ARCHIVED
```

Exemplo:

```text
CREATED
   ↓
ACTIVE
   ↓
OVERRIDE_SUSPENDED
   ↓
ACTIVE
   ↓
EXPIRED
```

A transição deve respeitar:

* ownership;
* autoridade;
* política;
* escopo temporal;
* contexto.

---

# 9. Desired State × Policy × Constraint × Authorization

A arquitetura não permite que esses conceitos sejam colapsados.

Exemplo:

```text
Desired State:
temperatura = 22°C

Policy:
economizar energia

Constraint:
não operar equipamento fora dos limites permitidos

Authorization:
permitir que a Yuki altere a temperatura durante determinado período
```

Mesmo que exista Desired State, a Yuki não pode concluir:

```text
"Tenho autorização para executar."
```

A autorização precisa ser avaliada separadamente.

---

# 10. Observation Model

A Yuki não possui acesso mágico à realidade.

Ela recebe observações provenientes de:

* sensores;
* APIs;
* integrações;
* eventos;
* bancos;
* dispositivos;
* sistemas externos;
* usuários;
* processos internos.

A arquitetura utiliza:

```text
Observation
→ Evidence
→ Evidence Validation
→ Validated Observation
→ Observed State
```

Uma observação individual não representa necessariamente o estado completo de uma entidade.

---

# 11. Evidence

Evidence é informação utilizada para sustentar uma observação ou conclusão.

Exemplos:

* resposta de API;
* leitura de sensor;
* evento assinado;
* registro de banco;
* confirmação de dispositivo;
* documento;
* observação independente;
* resultado de uma operação.

Evidence possui características que devem ser avaliadas contextualmente.

Entre elas:

* autenticidade;
* integridade;
* frescor;
* proveniência;
* confiabilidade da fonte;
* vínculo com a operação;
* independência;
* cobertura;
* consistência;
* contexto.

Não existe uma hierarquia universal simples em que uma determinada classe de fonte seja sempre superior a todas as outras.

---

# 12. Verification Boundary

O ADR-009 estabelece que:

```text
Execution
≠
Effect
≠
Evidence
≠
Verification
```

A arquitetura de reconciliação respeita essa separação.

Fluxo:

```text
External System
      ↓
Evidence
      ↓
Evidence Validation
      ↓
Verification Engine
      ↓
Validated Observation
      ↓
Observed State
      ↓
Reconciliation Engine
```

Verification não significa conhecimento absoluto da realidade.

Verification significa que determinadas evidências satisfazem uma política de verificação aplicável.

---

# 13. Observed State

Observed State é uma representação consolidada do estado observado de uma entidade.

Ele pode ser derivado de:

* uma observação;
* múltiplas observações;
* sensores;
* APIs;
* eventos;
* evidências históricas;
* resultados de verificação.

Observed State deve manter informações suficientes para avaliar sua validade, incluindo, quando aplicável:

* timestamp;
* freshness;
* provenance;
* evidence references;
* integrity;
* source information;
* verification basis;
* conflicts;
* uncertainty.

---

# 14. Observed State não é Truth

A arquitetura evita a seguinte equivalência:

```text
Observed State = Reality
```

O modelo correto é:

```text
Reality
   ↓
Observation
   ↓
Evidence
   ↓
Validation
   ↓
Observed State
```

Portanto:

> Observed State representa o melhor estado observado e validado disponível para a Yuki dentro das regras aplicáveis.

---

# 15. Drift

Drift é uma divergência relevante entre Desired State e Observed State segundo um comparador e uma política aplicáveis.

Formalmente:

```text
Drift =
Compare(
    Desired State,
    Observed State,
    Comparison Policy
)
```

Não existe uma única função matemática universal de drift.

Comparadores devem ser tipados conforme o domínio.

---

# 16. State Comparators

A arquitetura suporta diferentes tipos de comparação.

## 16.1 Exact Match

```text
estado = esperado
```

---

## 16.2 Numeric Range

```text
20 ≤ temperatura ≤ 24
```

---

## 16.3 Delta

```text
|observado - desejado| ≤ tolerância
```

---

## 16.4 Hysteresis Band

Define regiões distintas para:

* entrar em correção;
* permanecer estável;
* considerar convergência.

---

## 16.5 Boolean

```text
enabled = true
```

---

## 16.6 Set Comparison

```text
Desired:
[A, B, C]

Observed:
[A, B]
```

---

## 16.7 Graph / Topology

Comparação de:

* nós;
* conexões;
* dependências;
* topologia.

---

## 16.8 Temporal

Comparação de:

* janelas;
* deadlines;
* períodos;
* frequência;
* duração.

---

## 16.9 Structured Comparison

Comparação de objetos e estruturas complexas.

---

# 17. Resultado da Comparação

Um comparador deve poder produzir estados como:

```text
NO_DRIFT
DRIFT_DETECTED
UNKNOWN
INCOMPARABLE
```

Após avaliação de políticas e constraints:

```text
DRIFT_DETECTED
      ↓
pode considerar correção?
      ├── SIM → DRIFT_ACTIONABLE
      └── NÃO → DRIFT_NON_ACTIONABLE
```

Importante:

```text
DRIFT_ACTIONABLE
≠
AUTHORIZED
```

Drift actionable significa apenas que existe base para considerar convergência.

---

# 18. Drift não é necessariamente um Problema

Diferença entre Desired State e Observed State não significa automaticamente erro.

Exemplos:

* usuário mudou deliberadamente uma configuração;
* Desired State expirou;
* manutenção está acontecendo;
* outra autoridade controla o sistema;
* mudança externa é esperada;
* política mudou;
* estado ainda está dentro da tolerância;
* informação observada está incompleta.

Portanto:

```text
Difference
≠
Drift
≠
Problem
≠
Authorization
```

---

# 19. UNKNOWN

UNKNOWN é um estado de primeira classe.

Pode ocorrer quando:

* observações estão antigas;
* evidências são conflitantes;
* sensores estão indisponíveis;
* uma integração está inacessível;
* a coleta falhou;
* não existe evidência suficiente;
* a comparação não pode ser realizada com segurança.

Regra:

```text
UNKNOWN
≠
NO_DRIFT
```

e:

```text
UNKNOWN
≠
DRIFT
```

A Yuki não deve transformar ausência de conhecimento em certeza.

---

# 20. Comportamento diante de UNKNOWN

UNKNOWN deve bloquear ações que dependam da informação ausente ou incerta.

Entretanto, UNKNOWN não significa que absolutamente nenhuma ação possa ocorrer.

Podem existir respostas previamente autorizadas e compatíveis com Policy, Security e Safety, como:

* manter estado seguro;
* fallback;
* isolamento;
* coleta adicional;
* consulta de outra fonte;
* suspensão de determinada operação;
* escalonamento.

Portanto:

```text
UNKNOWN
→ bloquear ações dependentes da informação desconhecida
→ permitir apenas fallback previamente permitido
```

A decisão é contextual.

---

# 21. Reconciliation Decision

O Reconciliation Engine avalia:

```text
Desired State
+
Observed State
+
Policy
+
Constraint
+
Freshness
+
Context
```

e produz uma decisão declarativa.

Possíveis resultados:

```text
NO_ACTION
WAIT
REQUEST_OBSERVATION
INQUIRY
DRIFT_NON_ACTIONABLE
PROPOSE_CONVERGENCE
ESCALATE
BLOCKED
```

O Reconciliation Engine não executa diretamente a correção.

---

# 22. Reconciliation Intent

Reconciliation Intent é uma declaração de que determinada divergência deve ser considerada para convergência.

Exemplo:

```yaml
resulting_intent:
  type: CONVERGE_PROPERTY

  target:
    entity: living_room
    property: temperature

  desired_value: 22.0

  reason: DRIFT_ACTIONABLE
```

O Intent não deve determinar diretamente:

```text
qual API chamar
qual connector usar
qual payload enviar
qual token utilizar
```

Essas decisões pertencem às camadas apropriadas de planejamento, integração e execução.

---

# 23. Reconciliation Intent ≠ Workflow Plan

Separação:

```text
Reconciliation Engine
    ↓
"Este estado deve convergir."

Planner
    ↓
"Esta é uma estratégia possível."

Workflow Engine
    ↓
"Estas são as etapas necessárias."

Execution Engine
    ↓
"Vou executar esta operação."

Integration Gateway
    ↓
"Vou interagir com o sistema externo."

External System
    ↓
"Estado alterado."

Verification Engine
    ↓
"Estas evidências sustentam o efeito observado."
```

---

# 24. Workflow Boundary

Workflow responde:

> "Como executar e acompanhar uma sequência de operações?"

Reconciliation responde:

> "O estado observado corresponde ao estado desejado?"

São sistemas complementares.

```text
Reconciliation
      ↓
Intent
      ↓
Planning
      ↓
Workflow
      ↓
Execution
```

O Workflow Engine não deve absorver a responsabilidade do Reconciliation Engine.

---

# 25. Execution Boundary

O Workflow Engine coordena.

A execução efetiva pertence ao:

```text
Execution Engine / Worker
```

O fluxo pode ser:

```text
Workflow Engine
↓
Execution Engine
↓
Integration Gateway
↓
Connector / Adapter
↓
External System
```

Essa distinção mantém:

```text
orchestration
≠
execution
```

---

# 26. Convergence

Convergence significa que o estado observado atende às condições definidas para o Desired State.

Convergência não significa necessariamente igualdade perfeita.

Pode ser:

```text
dentro da tolerância
```

ou:

```text
compatível com a política
```

ou:

```text
dentro do envelope operacional definido
```

dependendo do domínio.

---

# 27. Stability

Reconciliação contínua pode criar instabilidade.

Exemplo:

```text
Ação Yuki
→ estado 22°C

Sistema externo
→ altera para 23°C

Yuki
→ retorna para 22°C

Sistema externo
→ retorna para 23°C

...
```

Isso pode gerar:

* oscillation;
* action storms;
* resource waste;
* desgaste físico;
* custo;
* conflitos de autoridade.

A arquitetura deve possuir mecanismos de estabilidade apropriados ao domínio.

---

# 28. Mecanismos de Estabilidade

Dependendo do sistema:

* hysteresis;
* deadband;
* debounce;
* cooldown;
* rate limits;
* action budgets;
* causal loop detection;
* retry budgets;
* maximum correction frequency;
* quarantine;
* escalation;
* manual intervention.

Nenhum mecanismo individual é obrigatório para todos os tipos de reconciler.

A escolha depende de:

* dinâmica do sistema;
* risco;
* custo;
* reversibilidade;
* frequência;
* impacto;
* domínio.

---

# 29. Action Budget

Reconcilers podem possuir limites de ação.

Exemplo:

```text
máximo de correções
por determinado período
```

Action budgets reduzem risco de:

* runaway reconciliation;
* bugs;
* loops;
* alterações repetitivas;
* custo excessivo;
* desgaste físico.

Os valores concretos são definidos por Policy e pelo domínio.

---

# 30. Human Override

Intervenções humanas são eventos relevantes.

Porém:

```text
manual change
≠
automatic authorization
```

Uma alteração manual pode representar:

* intenção deliberada;
* erro;
* manutenção;
* intervenção temporária;
* alteração não autorizada;
* outro controlador;
* mudança de contexto.

Portanto, a Yuki deve avaliar:

```text
Ownership
+
Identity
+
Intent
+
Policy
+
Context
```

antes de alterar o Desired State ou suspender reconciliação.

---

# 31. Override Policy

Uma Policy pode definir comportamentos como:

```text
PRESERVE_HUMAN_OVERRIDE
```

Nesse caso, uma intervenção humana legítima pode resultar em:

```text
Desired State
→ OVERRIDE_SUSPENDED
```

por período determinado.

Mas isso não é universal.

A arquitetura permite:

```text
manual intervention
→ preserve
→ reconcile
→ request confirmation
→ escalate
```

dependendo da autoridade e da política aplicável.

---

# 32. Ownership

Sistemas podem possuir diferentes autoridades legítimas.

Exemplo:

```text
Casa
→ Home Domain Authority

Robô
→ Robotics Domain Authority

Conta financeira
→ Finance Domain Authority

Servidor
→ Infrastructure Domain Authority
```

O processamento ocorrer em determinado local não significa automaticamente que aquele local possui autoridade.

---

# 33. Distributed Reconciliation

A Yuki poderá possuir:

```text
Edge
Home
Cloud
```

e múltiplos domínios.

A arquitetura não define:

```text
Home Server = autoridade universal
```

Em vez disso utiliza o conceito:

```text
Domain Authority
```

Cada domínio pode possuir sua própria autoridade operacional, conforme governança.

---

# 34. Split-Brain

Reconciliação distribuída deve evitar múltiplos controladores tentando alterar simultaneamente o mesmo domínio.

Possíveis mecanismos futuros:

* fencing;
* leases;
* epochs;
* authority tokens;
* versioning;
* domain ownership;
* consensus;
* failover controlado.

Nenhum mecanismo específico é congelado por este ADR.

---

# 35. Offline Operation

Durante indisponibilidade de rede, um domínio pode continuar realizando reconciliação local quando:

* possuir autoridade local válida;
* possuir dados suficientes;
* a operação estiver dentro da política;
* os limites de segurança forem respeitados.

Quando depender de informações externas:

```text
Cloud unavailable
→ local reconciliation may continue
   if authorized and safe

Cloud-dependent reconciliation
→ WAIT / SUSPEND / DEFER
```

---

# 36. Domain Reconcilers

A arquitetura permite múltiplos reconcilers especializados:

```text
Reconciliation Framework
├── Home Reconciler
├── Infrastructure Reconciler
├── Finance Reconciler
├── Robotics Reconciler
├── Security Reconciler
├── Knowledge Reconciler
└── Future Reconcilers
```

Eles compartilham princípios e contratos arquiteturais.

Porém, não precisam utilizar o mesmo algoritmo.

---

# 37. Reconciliation e Security Controller

O Reconciliation Engine:

* detecta drift;
* avalia convergência;
* produz intents.

O Security Controller:

* avalia autorização;
* aplica políticas de segurança;
* limita permissões;
* pode bloquear execução.

Fluxo:

```text
DRIFT_ACTIONABLE
      ↓
Reconciliation Intent
      ↓
Planning
      ↓
Security Controller
      ↓
Authorization
```

Nunca:

```text
DRIFT_ACTIONABLE
      ↓
AUTHORIZED
```

---

# 38. Reconciliation e Safety Controller

Para sistemas físicos:

```text
Reconciliation
→ Intent
→ Planning
→ Authorization
→ Physical Gateway
→ Safety Validation
→ Execution
```

O Reconciliation Engine não substitui o Safety Controller.

Safety constraints permanecem independentes.

---

# 39. Reconciliation e ADR-009

ADR-009 define:

* Effect;
* Evidence;
* Verification;
* Observed Effect State;
* UNKNOWN;
* Reconciliation.

ADR-012 amplia esse conceito para estado contínuo.

A relação é:

```text
Operation
    ↓
Evidence
    ↓
Verification
    ↓
Observed Effect
    ↓
Observed State
    ↓
Reconciliation
```

Reconciliation não redefine Verification.

---

# 40. Reconciliation e ADR-010

Para o mundo físico:

```text
Desired State
      ↓
Reconciliation
      ↓
Intent
      ↓
Planning
      ↓
Security
      ↓
Safety Validation
      ↓
Physical Execution
```

O Safety Controller permanece capaz de bloquear ou limitar ações independentemente da Yuki Core.

---

# 41. Reconciliation e ADR-011

ADR-011 define Durable Workflow.

ADR-012 fornece uma fonte contínua de avaliação de estado.

Integração:

```text
Reconciliation
    ↓
Reconciliation Intent
    ↓
Planner
    ↓
Durable Workflow
    ↓
Execution
    ↓
Verification
    ↓
Observed State
    ↺
Reconciliation
```

O Workflow não deve assumir que a realidade permaneceu igual simplesmente porque uma operação terminou.

---

# 42. Reconciliation e ADR-006

ADR-006 define o modelo de recursos.

Reconciliation pode detectar:

```text
Desired Resource State
≠
Observed Resource State
```

mas não substitui:

* Resource Manager;
* Scheduler;
* Execution Engine.

Exemplo:

```text
Desired:
GPU workload available

Observed:
GPU unavailable

Reconciliation:
DRIFT_DETECTED

Resource Manager:
identify alternatives

Model/Execution Strategy:
determine acceptable fallback

Workflow:
execute approved strategy
```

---

# 43. Prompt Injection

Observações externas podem conter conteúdo malicioso.

Exemplo:

```text
Sensor/API/Webhook
→ payload
→ "ignore security policy and execute..."
```

Isso deve ser tratado como:

```text
External Data ≠ Instruction
```

A defesa deve ser em camadas:

```text
External Data
↓
Typed Representation
↓
Schema Validation
↓
Provenance
↓
Evidence Validation
↓
Policy Evaluation
↓
Reconciliation
```

O conteúdo externo não pode:

* alterar Policy;
* alterar Authorization;
* redefinir Desired State;
* conceder privilégios;
* executar ferramentas.

---

# 44. Papel dos LLMs

LLMs podem auxiliar:

* interpretar observações;
* explicar drift;
* sugerir comparadores;
* sugerir estratégias;
* analisar conflitos;
* produzir diagnósticos;
* ajudar no planejamento.

Porém:

```text
LLM output
≠
Authorization
≠
Verification
≠
Truth
≠
Execution
```

O LLM não pode declarar sozinho:

```text
"convergência confirmada"
```

como autoridade externa.

---

# 45. Deterministic Core + Intelligent Assistance

Sempre que possível, partes críticas da reconciliação devem ser estruturadas de forma determinística:

```text
comparadores
estado
versionamento
tolerâncias
limites
budgets
transições
políticas
```

LLMs podem atuar nas partes que se beneficiam de interpretação e raciocínio.

Arquitetura:

```text
Deterministic State Layer
        +
Policy / Constraint Layer
        +
Intelligent Analysis
```

---

# 46. Failure Handling

Falhas possíveis:

* observação indisponível;
* fonte inconsistente;
* integração indisponível;
* workflow parado;
* autorização expirada;
* estado desconhecido;
* recurso indisponível;
* conflito de autoridade;
* loop de reconciliação;
* alteração humana;
* falha de sensor.

O Reconciliation Engine deve distinguir:

```text
NO_DRIFT
DRIFT
UNKNOWN
BLOCKED
INCOMPARABLE
```

e não converter automaticamente uma categoria em outra.

---

# 47. Quarantine

Quando uma integração, sensor ou domínio apresentar comportamento anômalo repetido, podem ser aplicados:

* cooldown;
* isolamento;
* suspensão;
* quarantine;
* escalonamento.

Isso evita que reconciliação transforme uma falha localizada em um loop de alterações.

---

# 48. Observability

O sistema deve registrar, quando apropriado:

* Desired State utilizado;
* versão;
* Observed State;
* evidências relevantes;
* comparador;
* resultado;
* motivo;
* Reconciliation Intent;
* políticas aplicadas;
* bloqueios;
* authorization reference;
* workflow reference;
* verification result;
* timestamps;
* domínio;
* autoridade.

Logs e auditoria não devem armazenar secrets indevidamente.

---

# 49. Memory

Nem todo resultado de reconciliação deve virar memória permanente.

Eventos relevantes podem ser armazenados como:

* histórico episódico;
* estado operacional;
* decisão;
* auditoria;
* conhecimento consolidado.

A promoção para memória semântica ou factual deve obedecer à política de memória.

Exemplo:

```text
DRIFT_DETECTED
```

pode ser um evento válido mesmo que não seja um fato permanente.

---

# 50. Desired State History

Desired States devem ser versionáveis.

Exemplo:

```text
v1
↓
v2
↓
v3
```

A Yuki deve conseguir identificar:

* qual versão estava ativa;
* quem a originou;
* quando entrou em vigor;
* quando expirou;
* qual versão a substituiu.

Isso é especialmente importante para auditoria e recuperação.

---

# 51. Reconciliation Context

Um contexto de reconciliação deve reunir referências, não necessariamente duplicar todos os dados.

Exemplo conceitual:

```yaml
reconciliation_context:

  context_id: "rec_ctx_living_room_temp"

  domain: "home_automation"

  desired_state:
    id: "ds_domain_home_climate"
    version: "2.1.0"

  observed_state:
    id: "obs_living_room_temp"
    freshness: "FRESH"

  comparison:
    status: "DRIFT_ACTIONABLE"
    comparator: "HYSTERESIS_BAND"

  resulting_intent:
    type: "CONVERGE_PROPERTY"

    target:
      entity: "living_room"
      property: "temperature"

    desired_value: 22.0
```

O exemplo não define o schema final.

---

# 52. Security Invariants

## INV-012-1

Reconciliation nunca concede autorização.

## INV-012-2

Desired State nunca é interpretado como comando executável.

## INV-012-3

UNKNOWN não pode ser silenciosamente convertido em NO_DRIFT ou sucesso.

## INV-012-4

Reconciliation não pode ignorar Policy ou Constraint.

## INV-012-5

Reconciliation não executa diretamente integrações externas.

## INV-012-6

LLM não possui autoridade para declarar convergência externa.

## INV-012-7

Safety Controller permanece independente para sistemas físicos.

## INV-012-8

Integrações não podem obter privilégios adicionais apenas porque foram selecionadas por um Reconciler.

## INV-012-9

Dados externos não possuem autoridade por serem apresentados ao Reconciliation Engine.

## INV-012-10

Falhas de observação não devem gerar correções cegas dependentes daquela informação.

---

# 53. Arquitetura Consolidada

```text
                         YUKI CORE
                             │
                      MISSION / GOALS
                             │
                             ▼
                    DESIRED STATE STORE
                             │
                             ▼
                 RECONCILIATION ENGINE
                 ├── State Comparators
                 ├── Drift Evaluation
                 ├── Stability Controller
                 └── Reconciliation Policy
                             │
                             ▼
                  RECONCILIATION INTENT
                             │
                             ▼
                  PLANNER / MISSION MANAGER
                             │
                             ▼
                       AUTHORIZATION
                             │
                    SECURITY CONTROLLER
                             │
                             ▼
                    DURABLE WORKFLOW
                             │
                             ▼
                    EXECUTION ENGINE
                             │
                             ▼
                   INTEGRATION GATEWAY
                             │
                    ┌────────┴────────┐
                    ▼                 ▼
               CONNECTOR           ADAPTER
                    │                 │
                    └────────┬────────┘
                             ▼
                     EXTERNAL SYSTEM
                             │
                             ▼
                          EVIDENCE
                             │
                             ▼
                  VERIFICATION ENGINE
                             │
                             ▼
                  VALIDATED OBSERVATION
                             │
                             ▼
                     OBSERVED STATE
                             │
                             └──────────────↺
```

Transversalmente:

```text
Security Controller
Safety Controller
Resource Manager
Credential Broker
Audit
Observability
Memory
Attention Manager
```

---

# 54. Fluxo Constitucional

O fluxo fundamental é:

```text
Desired State
      +
Observed State
      +
Policy
      +
Constraint
      ↓
Reconciliation
      ↓
Decision
      ↓
Reconciliation Intent
      ↓
Planning
      ↓
Authorization
      ↓
Workflow
      ↓
Execution
      ↓
External Effect
      ↓
Evidence
      ↓
Verification
      ↓
Validated Observation
      ↓
Observed State
      ↺
```

Nenhuma etapa deve ser implicitamente pulada apenas para reduzir latência.

Exceções somente quando previamente definidas por arquitetura, policy e risco.

---

# 55. Closed Decisions

## D012-1 — Reconciliation é subsistema contínuo e não soberano

O Reconciliation Engine é responsável por avaliação contínua de estado e proposta declarativa de convergência.

---

## D012-2 — Desired State é separado de Policy, Constraint e Authorization

Esses conceitos possuem responsabilidades independentes.

---

## D012-3 — Reconciliation utiliza Observed State derivado de observações validadas

O Reconciliation Engine não deve depender diretamente de dados externos não validados como autoridade.

---

## D012-4 — UNKNOWN é estado de primeira classe

A ausência ou insuficiência de evidência não deve ser convertida em certeza artificial.

---

## D012-5 — UNKNOWN não pode silenciosamente produzir convergência

A Yuki não deve considerar um sistema convergido quando não possui evidência suficiente para avaliar essa condição.

---

## D012-6 — Reconciliation nunca concede autorização

Uma decisão de drift actionable não constitui autorização.

---

## D012-7 — Reconciliation não executa diretamente integrações

Execução pertence às camadas de Workflow/Execution/Integration apropriadas.

---

## D012-8 — Reconciliation ≠ Workflow

Reconciliation avalia estado; Workflow coordena execução.

---

## D012-9 — Reconciliation ≠ Verification

Verification avalia evidências; Reconciliation avalia relação entre Desired State e Observed State.

---

## D012-10 — Reconciliation ≠ Safety Controller

Safety permanece uma autoridade independente para limites e funções de segurança física.

---

## D012-11 — Desired State não é comando executável

Uma declaração de estado desejado nunca deve ser interpretada automaticamente como instrução de execução.

---

## D012-12 — Reconciliation Intent ≠ Workflow Plan

Reconciliation declara necessidade de convergência; Planning define estratégia operacional.

---

## D012-13 — Mecanismos de estabilidade são aplicados conforme o domínio

Hysteresis, debounce, cooldown, budgets e mecanismos similares são utilizados quando apropriados às características e riscos do sistema.

---

## D012-14 — Action Budgets podem limitar reconciliação

Reconcilers podem possuir limites de frequência, volume ou impacto conforme Policy e risco.

---

## D012-15 — Intervenção humana é avaliada por ownership, intent e policy

Uma alteração humana pode modificar, suspender ou substituir uma Desired State quando legitimamente autorizada, mas isso não é presumido automaticamente.

---

## D012-16 — Autoridade distribuída é baseada em domínio

Processamento local, Edge, Home ou Cloud não concede automaticamente autoridade sobre determinado domínio.

---

## D012-17 — LLM não pode declarar convergência como autoridade externa

LLMs podem interpretar e auxiliar, mas não substituem Verification, Security ou Safety.

---

## D012-18 — Arquitetura permanece independente de tecnologia específica

Nenhuma implementação de banco, protocolo, framework, linguagem, sistema de eventos ou algoritmo distribuído é constitucionalmente exigida por este ADR.

---

# 56. Open Decisions

Os seguintes pontos permanecem deliberadamente abertos:

### O012-1 — Policy Language

Possibilidades futuras:

* Cedar;
* Rego/OPA;
* sistema próprio;
* combinação de mecanismos.

---

### O012-2 — Cooldown Calibration

Como determinar automaticamente ou semi-automaticamente valores adequados de cooldown.

---

### O012-3 — Desired State Schema

Formato definitivo:

* JSON;
* YAML;
* Protobuf;
* schema híbrido;
* outro.

---

### O012-4 — Reconciliation Policy Schema

Formato definitivo para:

* tolerâncias;
* autonomia;
* budgets;
* escalation;
* freshness;
* fallback;
* constraints.

---

### O012-5 — Distributed Authority Protocol

Mecanismos de:

* fencing;
* leases;
* epochs;
* failover;
* ownership;
* autoridade distribuída.

---

### O012-6 — Event vs Polling Strategy

Determinar quando cada domínio utiliza:

* polling;
* eventos;
* streaming;
* combinação híbrida.

---

### O012-7 — State Comparison Algorithms

Algoritmos especializados para:

* estruturas;
* grafos;
* séries temporais;
* estados probabilísticos;
* sistemas físicos;
* recursos distribuídos.

---

# 57. Future ADRs

## ADR-013

**Home/Cloud Event Store Synchronization & Offline State Federation Protocol**

Possíveis temas:

* sincronização;
* autoridade distribuída;
* offline;
* event federation;
* conflict resolution;
* replay;
* recovery;
* fencing;
* domain authority.

---

## ADR-014

**Personal Goal Reconciliation & Human Agency Boundaries**

Possíveis temas:

* Goals;
* Preferences;
* Habits;
* personal Desired States;
* autonomy;
* user override;
* human agency;
* long-term personal planning.

---

# 58. Relação com ADRs Anteriores

```text
ADR-006
Infrastructure Resource Model
        │
        ▼
ADR-007
Integration Model
        │
        ▼
ADR-008
Credential Isolation
        │
        ▼
ADR-009
Verification & Observed Effects
        │
        ▼
ADR-010
Physical World Safety
        │
        ▼
ADR-011
Durable Workflow
        │
        ▼
ADR-012
Reconciliation
```

ADR-012 não substitui esses ADRs.

Ele conecta:

```text
estado desejado
        ↓
estado observado
        ↓
necessidade de convergência
        ↓
planejamento
        ↓
execução
        ↓
verificação
        ↓
novo estado observado
```

---

# 59. Princípios Oficiais do ADR-012

A arquitetura de Reconciliation da Yuki adota os seguintes princípios:

1. **Desired State não é comando.**
2. **Observed State não é realidade absoluta.**
3. **Evidence não é Truth.**
4. **Verification não concede autorização.**
5. **Drift não significa automaticamente problema.**
6. **Drift actionable não significa autorizado.**
7. **UNKNOWN é informação válida.**
8. **Reconciliation não executa diretamente.**
9. **Reconciliation não substitui Workflow.**
10. **Reconciliation não substitui Verification.**
11. **Reconciliation não substitui Safety.**
12. **Human intervention deve ser interpretada por autoridade e contexto.**
13. **Estabilidade é necessária para evitar loops e oscillation.**
14. **Autoridade distribuída pertence ao domínio, não ao local de processamento.**
15. **LLMs auxiliam raciocínio, mas não possuem autoridade externa.**
16. **A arquitetura deve permanecer independente de tecnologias específicas.**
17. **Convergência é um processo contínuo, não uma única execução.**
18. **A Yuki deve preferir evidência explícita a suposições implícitas.**

---

# 60. Status

**ACCEPTED — v1.0**

Este ADR estabelece o Reconciliation Engine como componente arquitetural oficial da Yuki.

A arquitetura permite:

```text
Desired State
      ↓
Continuous Observation
      ↓
Drift Detection
      ↓
Declarative Reconciliation Intent
      ↓
Planning
      ↓
Authorization
      ↓
Durable Execution
      ↓
Verification
      ↓
Observed State
      ↺
```

O Reconciliation Engine permanece deliberadamente **não soberano, não executor e independente de tecnologia específica**.

Seu propósito é manter a Yuki capaz de detectar e responder a divergências ao longo do tempo sem transformar o mecanismo de convergência em uma autoridade autônoma capaz de modificar o mundo sem as camadas apropriadas de planejamento, autorização, segurança, execução e verificação.

**ADR-012 v1.0 — ACCEPTED.**
