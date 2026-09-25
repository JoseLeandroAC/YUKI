# ADR-015 — Yuki Privacy, Data Minimization & Information Boundaries

**Version:** 1.0
**Status:** ACCEPTED
**Domain:** Privacy / Information Architecture / Security
**Date:** 2026-09-25

---

## 1. Objetivo

Este ADR define a arquitetura de privacidade, minimização de dados e fronteiras de informação da Yuki.

Seu objetivo é permitir que a Yuki utilize o contexto necessário para executar tarefas complexas, mantendo separação entre:

* existência de dados;
* acessibilidade;
* relevância;
* finalidade;
* autorização;
* contexto;
* processamento;
* retenção;
* compartilhamento.

A Yuki deve ser capaz de conhecer informações suficientes para ser útil sem transformar o conhecimento acumulado em acesso indiscriminado.

---

# 2. Princípio Central

## Purpose-Bound Contextual Compartmentalization

O acesso à informação deve ser limitado pela finalidade, domínio, política, autorização, contexto e necessidade da tarefa.

Conceitualmente:

```text
Accessible Data
=
Data Existence
∩
Purpose Scope
∩
Domain Policy
∩
Authorization
∩
Context
∩
Applicable Model/Processing Policy
```

Essa expressão representa uma regra arquitetural e não uma fórmula matemática obrigatória de implementação.

---

# 3. Data ≠ Access

A existência de um dado não implica que ele possa ser acessado.

```text
Data Exists
    ≠
Data Accessible
    ≠
Data Relevant
    ≠
Data Authorized for Use
    ≠
Context Assembled
    ≠
Model Authorized to Receive
```

Exemplo:

Um extrato bancário pode existir na Yuki.

Isso não significa que:

* o Core possa lê-lo diretamente;
* qualquer agente possa recuperá-lo;
* qualquer modelo possa recebê-lo;
* qualquer tarefa possa utilizá-lo;
* qualquer integração possa recebê-lo.

---

# 4. Data, Information, Knowledge e Context

## Data

Representação bruta ou estruturada de valores, registros, bytes ou eventos.

## Information

Dados interpretados ou contextualizados de forma que possuam significado para determinada finalidade.

## Knowledge

Informação persistente e organizada que pode representar fatos, relações, padrões ou conhecimento útil à Yuki.

## Context

Conjunto selecionado de informações relevantes para uma tarefa, finalidade e janela temporal específicas.

Contexto não é sinônimo de memória.

```text
Memory
   ↓
Policy / Retrieval
   ↓
Selection / Projection
   ↓
Context
```

---

# 5. Memory ≠ Context

A memória pode conter muito mais informação do que a necessária para determinada tarefa.

Portanto:

```text
Memory
    ≠
Context
```

O Context Builder deve recuperar apenas o subconjunto apropriado.

---

# 6. Information Boundaries

A Yuki deve possuir fronteiras lógicas entre domínios de informação.

Exemplos:

```text
Personal
├── Education
├── Finance
├── Health
├── Work
├── Family
├── Home
├── Projects
├── Development
├── Robotics
└── Security
```

Esses domínios não precisam corresponder necessariamente a bancos, servidores ou processos físicos separados.

A separação é inicialmente arquitetural e política.

---

# 7. Compartmentalized Memory

A memória da Yuki deve ser logicamente compartimentalizada.

Cada memória, registro ou conhecimento relevante deve poder possuir atributos como:

* domínio;
* sensibilidade;
* finalidade;
* proveniência;
* retenção;
* compartilhamento;
* estado;
* validade;
* políticas aplicáveis.

Um agente não recebe acesso global simplesmente porque possui acesso à memória.

---

# 8. Least Knowledge

Além de:

> Least Privilege

a Yuki adota:

> **Least Knowledge**

Um componente deve receber somente a informação necessária para cumprir sua função, quando tecnicamente possível.

Isso reduz:

* vazamento;
* impacto de comprometimento;
* propagação de dados;
* exposição de modelos;
* contaminação de contexto;
* cross-domain leakage.

---

# 9. Purpose Limitation

Dados devem ser utilizados dentro da finalidade autorizada.

Exemplo:

```text
Finance Data
    ↓
Purpose:
"Analisar orçamento"
```

não implica automaticamente autorização para:

* treinamento;
* publicidade;
* recomendação de compras;
* outro agente;
* outro domínio;
* outro modelo;
* outra integração.

Expansão de finalidade deve passar por política e autorização apropriadas.

---

# 10. Data Access Gateway

A Yuki deve possuir uma camada conceitual de acesso a dados:

```text
Yuki / Agent
      ↓
Data Access Gateway
      ↓
Information Policy
      ↓
Data Source
```

O Data Access Gateway pode:

* aplicar fronteiras;
* verificar finalidade;
* verificar escopo;
* filtrar atributos;
* aplicar projeções;
* aplicar classificação;
* controlar acesso cross-domain;
* impedir acesso desnecessário.

O Gateway não substitui o Security Controller.

---

# 11. Separação de Responsabilidades

## Security Controller

Responsável por:

* identidade;
* autenticação;
* autorização;
* credenciais;
* políticas de segurança;
* emissão/validação de autorização;
* revogação.

## Information Policy

Responsável por:

* finalidade;
* classificação;
* compartimentalização;
* compartilhamento;
* retenção;
* minimização.

## Data Access Gateway

Responsável por enforcement do acesso informacional e geração de projeções.

## Context Builder

Responsável por determinar o contexto necessário para a tarefa.

## Memory / Knowledge

Responsáveis por armazenar e fornecer informação dentro das políticas aplicáveis.

## Model Router

Responsável por selecionar modelos respeitando as políticas de processamento e dados.

Nenhum desses componentes deve possuir autoridade total sobre todos os outros.

---

# 12. Data Projection

Quando possível, a Yuki deve utilizar projeções mínimas.

Exemplo:

```text
Raw Finance Data

R$ 18.392,44
Conta X
Banco Y
Transações...
```

pode ser projetado para:

```text
budget_status = "ABOVE_TARGET"
```

quando isso for suficiente para a tarefa.

Mas:

> Projeção não significa autorização automática.

Uma projeção continua sendo informação e permanece sujeita às políticas aplicáveis.

---

# 13. Cross-Domain Information Flow

Quando uma tarefa cruza domínios:

```text
Finance
   ↓
Purpose-bound Projection
   ↓
Education
```

e não:

```text
Finance
   ↓
Education Agent
   ↓
Entire Finance Database
```

O compartilhamento deve considerar:

* finalidade;
* escopo;
* sensibilidade;
* autorização;
* necessidade;
* transformação;
* retenção;
* destino.

---

# 14. Agent Information Boundaries

Agentes não devem receber automaticamente toda a informação disponível ao Supervisor ou ao Core.

Exemplo:

```text
Yuki Supervisor
    │
    ├── Finance Agent
    │      └── Finance Context
    │
    ├── Education Agent
    │      └── Education Context
    │
    └── Research Agent
           └── Research Context
```

Subagentes devem receber apenas o contexto necessário.

Isso integra com o princípio de privilege attenuation do ADR-014.

---

# 15. Model Data Policy

Antes de enviar dados para um modelo:

```text
Task
 ↓
Data Required
 ↓
Sensitivity
 ↓
Purpose
 ↓
Model Policy
 ↓
Processing Location
 ↓
Trust / Attestation when applicable
 ↓
ALLOW / TRANSFORM / BLOCK
```

O Model Router não deve considerar apenas qualidade e custo do modelo.

Deve também considerar:

* sensibilidade;
* finalidade;
* localização do processamento;
* política do modelo/provedor;
* requisitos de retenção;
* privacidade;
* confiança do ambiente;
* restrições do usuário;
* risco.

---

# 16. Local, Home e Cloud

A Yuki deve suportar:

```text
EDGE
LOCAL DEVICE
HOME
CLOUD
HYBRID
```

Nenhuma localização possui automaticamente maior ou menor confiança apenas por sua posição física.

```text
Home Server
≠
Automatically Trusted
```

O processamento pode ser direcionado conforme:

* privacidade;
* custo;
* latência;
* capacidade;
* disponibilidade;
* política;
* risco;
* sensibilidade.

---

# 17. Confidential Computing

Confidential Computing/TEE pode ser utilizado quando apropriado para proteger dados durante processamento.

Entretanto:

```text
TEE
≠
Authorization
```

e:

```text
TEE
≠
Automatically Safe Data Destination
```

Um ambiente confidencial pode fornecer propriedades adicionais de isolamento e attestation, mas a decisão de liberar determinado dado continua dependendo de política, finalidade, autorização e confiança aplicável.

---

# 18. Data Classification

A Yuki deve suportar classificação de sensibilidade.

Uma taxonomia inicial pode incluir:

```text
PUBLIC
PERSONAL
SENSITIVE
RESTRICTED
SECRET
```

Essa classificação não deve ser a única dimensão.

Outros atributos podem incluir:

```text
Purpose
Domain
Retention
Sharing
Processing Location
Integrity
Freshness
Privacy Requirements
Security Requirements
```

A classificação pode evoluir sem alterar o Core.

---

# 19. Secrets

Secrets permanecem governados pelo ADR-008.

Nunca devem ser tratados como contexto comum.

```text
Secret
→ Credential Broker / Secret Store
```

e não:

```text
Secret
→ Prompt
```

Também não devem aparecer em:

* memória comum;
* logs;
* traces;
* métricas;
* prompts;
* contexto comum.

---

# 20. Inference Privacy

Inferências são informação derivada e devem possuir proveniência própria.

```text
User Declaration
    ≠
Inference
```

Uma inferência deve poder carregar informações como:

* source;
* model;
* evidence;
* timestamp;
* provenance;
* validity;
* uncertainty;
* applicable purpose.

Uma inferência não confirmada não pode automaticamente:

* virar declaração do usuário;
* alterar objetivo;
* alterar delegação;
* alterar autorização;
* alterar política de segurança;
* ganhar status de fato confirmado.

---

# 21. Confidence

A Yuki não deve assumir que um número arbitrário como:

```text
confidence = 0.87
```

possui significado objetivo.

Scores de confiança só devem ser tratados como quantitativos quando houver metodologia/calibração apropriada.

Caso contrário, a Yuki deve preferir:

* evidências;
* proveniência;
* fontes;
* validade;
* incerteza;
* estado de confirmação.

---

# 22. Inference Lifecycle

Inferências podem possuir ciclo de vida próprio:

```text
GENERATED
    ↓
UNCONFIRMED
    ↓
VALIDATED
    ↓
ACTIVE
    ↓
EXPIRED / SUPERSEDED / REJECTED
```

Nem toda inferência precisa ser permanentemente armazenada.

Nem toda inferência precisa ser automaticamente apagada.

A retenção deve depender de política, utilidade, sensibilidade e validade.

---

# 23. External Data

Dados externos são informação, não autoridade.

```text
Website
Email
PDF
API
Message
Sensor
External System
```

podem fornecer:

* contexto;
* evidência;
* observação;
* informação.

Mas não podem automaticamente:

* alterar política;
* conceder permissão;
* criar delegação;
* alterar objetivo;
* autorizar ação.

---

# 24. Prompt Injection

A arquitetura deve assumir que dados externos podem conter instruções maliciosas.

Portanto:

```text
External Data
 ↓
Typed / Isolated Representation
 ↓
Context Boundary
 ↓
Policy Evaluation
 ↓
Authorization
 ↓
Execution
```

Tags ou envelopes semânticos não constituem, sozinhos, uma fronteira de segurança.

O modelo nunca deve ser a única barreira contra exfiltração ou execução indevida.

---

# 25. Observability as Information Boundary

Logs, métricas e traces também são dados.

A Yuki deve aplicar minimização a:

* prompts;
* respostas;
* documentos;
* imagens;
* áudio;
* identificadores;
* dados financeiros;
* dados de saúde;
* credenciais;
* tokens;
* contexto interno.

O fato de um dado ser útil para debugging não concede automaticamente autorização para registrá-lo.

---

# 26. Retention

Dados devem possuir políticas de retenção apropriadas.

Um modelo conceitual pode incluir:

```text
HOT
WARM
COLD
EXPIRED
DELETED
```

mas esses estados não constituem uma obrigação de implementação específica.

Retenção pode depender de:

* finalidade;
* sensibilidade;
* utilidade;
* segurança;
* política do usuário;
* requisitos externos;
* natureza do dado.

---

# 27. Deletion & Forgetting

"Esquecer" é uma operação arquitetural, não apenas um `DELETE`.

Quando aplicável, a Yuki deve considerar:

```text
Primary Data
Indexes
Embeddings
Caches
Summaries
Derived Data
Replicas
Snapshots
Backups
Logs
Exports
External Systems
```

A política de exclusão determina quais camadas devem:

* ser removidas;
* ser invalidadas;
* expirar;
* ser substituídas;
* aguardar retenção;
* receber propagação de exclusão.

---

# 28. Cryptographic Erasure

Crypto-erasure pode ser utilizado quando apropriado.

Porém:

```text
Destroy Key
≠
Guarantee All Copies Are Gone
```

A técnica não substitui:

* controle de réplicas;
* expiração de backups;
* purge de índices;
* purge de embeddings;
* invalidação de caches;
* tratamento de dados derivados;
* políticas de sistemas externos.

---

# 29. Audit Metadata

Uma operação de exclusão pode deixar um registro mínimo de auditoria.

Exemplo:

```text
DATA_PURGE_EXECUTED
project_id
operation_id
timestamp
scope
result
```

sem necessariamente preservar o conteúdo apagado.

A retenção do registro de auditoria segue sua própria política.

---

# 30. Derived Data

Dados derivados continuam sendo dados governados.

Incluem:

* embeddings;
* summaries;
* classifications;
* projections;
* inferred facts;
* generated metadata;
* semantic indexes.

Derivação não remove automaticamente as restrições de privacidade do dado original.

---

# 31. Privacy × Security

Security pergunta:

> "Este componente está autorizado a fazer isso?"

Privacy pergunta:

> "Este componente precisa desta informação para esta finalidade?"

É possível existir:

```text
Technically Authorized
+
Privacy-Inappropriate
```

A arquitetura deve impedir que autorização técnica seja interpretada como autorização irrestrita de uso.

---

# 32. Privacy × Human Agency

O usuário deve poder definir políticas relevantes de informação, dentro das capacidades do sistema.

Exemplos:

* permitir ou proibir determinados provedores;
* permitir processamento cloud;
* restringir determinados domínios;
* limitar retenção;
* definir compartimentos;
* revogar acesso;
* permitir processamento local-only;
* estabelecer políticas de compartilhamento.

Isso integra diretamente com ADR-014.

---

# 33. Privacy × Safety

Privacidade e segurança física podem entrar em conflito.

A Yuki não deve utilizar uma hierarquia universal simplista.

Cada domínio deve possuir políticas apropriadas.

Em sistemas físicos:

```text
Privacy Policy
+
Security Policy
+
Safety Policy
+
Emergency Policy
```

podem determinar o comportamento aplicável.

A Safety Controller continua independente conforme ADR-010.

---

# 34. Background Workflows

Contexto e autorização podem perder validade enquanto uma missão continua executando.

Por isso, workflows de longa duração devem considerar:

* context freshness;
* authorization validity;
* delegation state;
* data retention;
* policy changes;
* revocation;
* revalidation.

Isso integra com ADR-011 e ADR-014.

---

# 35. Reconciliation

O Reconciliation Engine não deve possuir acesso irrestrito a todos os dados apenas por funcionar continuamente.

Ele deve trabalhar com:

* Desired State apropriado;
* Observed State apropriado;
* projections;
* policies;
* domínio;
* finalidade.

Isso integra com ADR-012.

---

# 36. Distributed Privacy

ADR-013 estabelece que:

```text
Storage Location
≠
Authority
```

O mesmo vale para privacidade:

```text
Storage Location
≠
Privacy Classification
```

Um dado no Home Server não é automaticamente privado apenas por estar fisicamente em casa.

Da mesma forma:

```text
Cloud
≠
Automatically Forbidden
```

A decisão depende de política, finalidade, ambiente e risco.

---

# 37. Device Privacy

Cada dispositivo pode possuir:

* contexto próprio;
* dados locais;
* confiança própria;
* políticas próprias;
* memória local;
* restrições de sincronização.

Portanto:

```text
Same Yuki
≠
Same Data Access Everywhere
```

---

# 38. Voice Privacy

Voice-first não significa gravação contínua indiscriminada.

A arquitetura deve privilegiar:

* processamento local quando apropriado;
* wake-word local;
* minimização de áudio;
* retenção limitada;
* identificação como sinal de identidade;
* autorização independente.

```text
Voice Identity
≠
Unlimited Authorization
```

---

# 39. Sensors and Cameras

Sensores devem possuir políticas próprias de acesso.

Quando possível:

```text
Raw Sensor
 ↓
Local Processing
 ↓
Minimal Event
 ↓
Discard Raw Data
```

Exemplo:

```text
Camera
→ Person Detected
```

quando a tarefa não exige armazenamento do vídeo.

Isso é uma estratégia possível, não uma regra universal.

---

# 40. Development Lab

O Development Lab não constitui exceção às políticas de privacidade.

Dados de produção não devem ser copiados indiscriminadamente para testes.

Quando possível, utilizar:

* dados sintéticos;
* dados redigidos;
* pseudonimização;
* projeções;
* datasets controlados;
* isolamento.

---

# 41. Evolution Manager

O Evolution Manager também está sujeito às Information Boundaries.

Ele não recebe acesso global apenas por ser responsável pela evolução da Yuki.

Melhor:

```text
Evolution Task
 ↓
Required Data
 ↓
Minimal Projection
 ↓
Development Lab
```

---

# 42. Failure Handling

Quando mecanismos de privacidade estão indisponíveis, o comportamento deve ser definido por domínio e risco.

Não existe regra universal:

```text
Always Fail Open
```

nem:

```text
Always Fail Closed
```

Cada fluxo deve definir o comportamento seguro apropriado.

---

# 43. Access Decision

Uma decisão conceitual de acesso pode considerar:

```text
Identity
Authentication
Purpose
Data
Sensitivity
Domain
Delegation
Authorization
Policy
Context
Device
Model
Processing Location
Risk
Time
Safety
```

A implementação não deve necessariamente executar uma única função matemática.

A decisão deve aplicar:

```text
Hard Constraints
        ↓
Authorization
        ↓
Information Policy
        ↓
Purpose
        ↓
Minimization
        ↓
Processing / Model Policy
        ↓
Execution
```

---

# 44. Data Access State

Uma autorização de acesso pode possuir:

```text
REQUESTED
AUTHORIZED
ACTIVE
EXPIRED
REVOKED
```

O estado de acesso não deve ser confundido com:

* estado do dado;
* estado da memória;
* estado da missão;
* estado do workflow.

---

# 45. Privacy Invariants

### INV-015-1

Data existence does not imply data access.

### INV-015-2

Access must be purpose-bound.

### INV-015-3

Memory does not imply context.

### INV-015-4

Context does not imply authorization.

### INV-015-5

Projection does not remove authorization requirements.

### INV-015-6

Inference is not equivalent to user declaration.

### INV-015-7

External data is not authority.

### INV-015-8

Secrets never become normal model context.

### INV-015-9

Derived data remains governed data.

### INV-015-10

Observability data is subject to privacy controls.

### INV-015-11

Privacy enforcement cannot independently grant execution authority.

### INV-015-12

No component receives global information access merely because it is internal to Yuki.

---

# 46. Closed Decisions

## D015-1 — Data ≠ Access

Existência de dados não concede acesso.

## D015-2 — Purpose-Bound Access

Uso de dados deve ser associado à finalidade aplicável.

## D015-3 — Compartmentalized Memory

A memória possui fronteiras lógicas de informação.

## D015-4 — Least Knowledge

Componentes recebem apenas informação necessária quando tecnicamente possível.

## D015-5 — Context ≠ Memory

Contexto é uma seleção orientada à tarefa, finalidade e tempo.

## D015-6 — Projection ≠ Authorization

Projeção não substitui autorização.

## D015-7 — Inference ≠ Declaration

Inferências possuem proveniência e estado próprios.

## D015-8 — External Data ≠ Authority

Dados externos não adquirem autoridade automaticamente.

## D015-9 — Model Data Policy

O uso de modelos deve considerar as políticas de dados aplicáveis.

## D015-10 — Privacy ≠ Security

Autorização técnica não implica adequação de finalidade.

## D015-11 — Cross-Domain Flow

Compartilhamento entre domínios deve ser explícito, mediado e minimizado.

## D015-12 — Deletion Is Policy-Driven

Exclusão é uma operação multicamada orientada por política, não um único mecanismo.

## D015-13 — Derived Data Is Governed

Dados derivados continuam sujeitos às políticas aplicáveis.

## D015-14 — Observability Is a Boundary

Logs, traces e métricas são fluxos de informação sujeitos a proteção.

## D015-15 — Privacy Cannot Grant Authority

Nenhum mecanismo de privacidade pode conceder autorização de execução.

## D015-16 — Technology Independence

Nenhuma tecnologia específica de banco, TEE, policy engine ou vector store é constitucional.

---

# 47. Open Decisions

### O015-1

Schema definitivo de classificação.

### O015-2

Schema de Information Policy.

### O015-3

Data Access Gateway implementation.

### O015-4

Policy engine.

### O015-5

Memory ACL / compartment model.

### O015-6

Data Projection contract.

### O015-7

Model Data Policy manifest.

### O015-8

Retention policy engine.

### O015-9

Deletion propagation protocol.

### O015-10

Embedding/vector privacy architecture.

### O015-11

User privacy configuration interface.

### O015-12

Multi-user/shared-domain privacy.

### O015-13

Confidential Computing integration.

### O015-14

Privacy-aware Model Router implementation.

---

# 48. Future ADRs

Possíveis próximos ADRs:

```text
ADR-016
Context & Knowledge Architecture

ADR-017
Memory Architecture

ADR-018
Identity, Session & Multi-Device Access

ADR-019
Observability, Audit & Diagnostics
```

Confidential Computing poderá receber ADR próprio posteriormente caso a complexidade arquitetural justifique.

---

# 49. Relation to Previous ADRs

### ADR-006

Resource Model

Privacy requirements podem influenciar seleção de recursos e processamento.

### ADR-007

Integration Manifest

Integrações devem declarar requisitos relevantes de dados e processamento.

### ADR-008

Credential Isolation

Secrets permanecem separados do contexto comum.

### ADR-009

Verification

Evidence e verification continuam sujeitos a information boundaries.

### ADR-010

Physical Safety

Safety mantém autoridade independente.

### ADR-011

Durable Workflow

Workflows longos devem revalidar políticas e contexto quando necessário.

### ADR-012

Reconciliation

Reconciliation não possui acesso global por padrão.

### ADR-013

Distributed Authority

Storage location não concede autoridade nem classificação de privacidade.

### ADR-014

Human Agency

O usuário controla suas preferências, objetivos e políticas pessoais dentro do modelo de autoridade.

---

# 50. Architecture

```text
                         YUKI CORE
                            │
                       TASK / INTENT
                            │
                            ▼
                     CONTEXT BUILDER
                            │
                     PURPOSE / SCOPE
                            │
                            ▼
                  INFORMATION POLICY
                            │
                            ▼
                 DATA ACCESS GATEWAY
                            │
             ┌──────────────┼──────────────┐
             ▼              ▼              ▼
          MEMORY         KNOWLEDGE      OBSERVATION
             │              │              │
             └──────────────┼──────────────┘
                            ▼
                     DATA PROJECTION
                            │
                     MINIMAL CONTEXT
                            │
                            ▼
                      MODEL ROUTER
                            │
             ┌──────────────┼──────────────┐
             ▼              ▼              ▼
           EDGE           HOME           CLOUD
                                          │
                                Model Data Policy
                                + Trust/Attestation
```

Transversalmente:

```text
SECURITY CONTROLLER
        │
        ├── Identity
        ├── Authorization
        ├── Credentials
        └── Security Policy

PRIVACY / INFORMATION POLICY
        │
        ├── Purpose
        ├── Classification
        ├── Compartment
        ├── Sharing
        └── Retention

AUDIT / OBSERVABILITY
        │
        └── Privacy-aware telemetry
```

---

# 51. Central Architecture Principle

A Yuki não deve perguntar apenas:

> "Posso acessar este dado?"

Ela também deve perguntar:

> "Eu preciso deste dado para realizar esta tarefa?"

E:

> "Qual é a menor representação da informação que permite cumprir a finalidade?"

Portanto:

```text
Least Privilege
        +
Least Knowledge
        +
Purpose Limitation
        +
Data Minimization
        +
Compartmentalization
```

formam a base de privacidade da Yuki.

---

# 52. Status

**ACCEPTED — v1.0**

O ADR estabelece a arquitetura fundamental de privacidade e fronteiras de informação da Yuki.

Detalhes de implementação permanecem deliberadamente abertos para futuros documentos e contratos.
