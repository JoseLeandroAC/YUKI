# ADR-007 — Yuki Integration Model & Manifest

**Projeto:** Yuki
**ADR:** 007
**Título:** Integration Model & Manifest
**Versão:** v1.0
**Status:** ACCEPTED
**Data:** 2026-09-19
**Domínio:** 15 — Integrations
**Decisão:** Accepted

---

# 1. Contexto

A Yuki precisa interagir com sistemas externos de forma extensível, segura e independente de fornecedores.

O Domínio 15 estabeleceu uma separação fundamental entre:

* Capability;
* Tool;
* Plugin;
* Integration;
* Connector;
* Adapter;
* Provider;
* External System.

Sem uma definição formal dessas entidades, existe risco de que futuras implementações criem acoplamento direto entre o Yuki Core e:

* APIs;
* SDKs;
* providers;
* protocolos;
* credenciais;
* contas;
* ambientes;
* runtimes;
* tecnologias específicas.

Também existe o problema de distinguir:

```text
"como uma integração funciona"
```

de:

```text
"qual conexão concreta está sendo utilizada"
```

Por isso, este ADR define o modelo estrutural de Integration e seu Manifest.

---

# 2. Problema

A Yuki precisa responder, de maneira estruturada:

1. O que é uma Integration?
2. Como ela se diferencia de uma Capability?
3. Como ela se diferencia de um Plugin?
4. Como representar uma integração sem amarrar o Core a um provider?
5. Como representar múltiplas contas/conexões da mesma integração?
6. Quais informações pertencem ao Manifest?
7. O que deve permanecer fora do Manifest?
8. Como representar capacidades suportadas?
9. Como representar requisitos de runtime?
10. Como representar protocolos?
11. Como versionar uma Integration?
12. Como distinguir Definition de Instance?
13. Como preparar integrações para futuras tecnologias?
14. Como permitir lifecycle, trust, health e authorization sem misturá-los?

---

# 3. Decisão

A Yuki adotará um modelo explícito baseado em:

```text
Integration Definition
        │
        ├── Manifest
        │
        ├── Contracts
        │
        ├── Supported Capabilities
        │
        ├── Runtime Requirements
        │
        └── Provider/System Mapping
                │
                ▼
       Integration Instance
                │
                ├── Identity
                ├── Configuration
                ├── Policy
                ├── Permissions
                ├── Credential References
                └── Runtime State
```

A **Integration Definition** descreve como uma integração funciona.

A **Integration Instance** representa uma conexão concreta dessa definição.

---

# 4. Definições

## 4.1 Capability

Capability representa uma habilidade funcional da Yuki.

Exemplo:

```text
github.issue.create
```

Capability responde:

> O que a Yuki consegue fazer?

---

## 4.2 Tool

Tool representa uma interface invocável por modelos, planners ou agentes.

Exemplo:

```text
github.create_issue(...)
```

Tool responde:

> Como uma capacidade é apresentada como uma operação invocável?

Tool não concede autorização.

---

## 4.3 Plugin

Plugin é uma unidade instalável ou distribuível que fornece, implementa ou modifica capacidades.

Plugin responde:

> Como uma extensão é distribuída/instalada?

Plugin não é sinônimo de Integration.

---

## 4.4 Integration

Integration representa a abstração que conecta a Yuki a um sistema externo.

Ela responde:

> Como a Yuki se conecta operacionalmente a um sistema externo?

Uma Integration pode suportar várias Capabilities.

```text
GitHub Integration
├── repository.read
├── issue.read
├── issue.create
├── pull_request.read
└── pull_request.create
```

---

## 4.5 Integration Definition

A Definition descreve a integração de forma relativamente estável.

Inclui:

* identidade da integração;
* versão;
* provider;
* external system;
* capabilities suportadas;
* contratos;
* protocolos suportados;
* requisitos de runtime;
* requisitos de rede;
* requisitos de execução;
* lifecycle metadata;
* compatibilidade.

---

## 4.6 Integration Instance

A Instance representa uma conexão concreta.

Exemplo:

```text
GitHub Definition
        │
        ├── Conta A
        ├── Conta B
        └── Conta C
```

Cada Instance pode possuir:

* identidade diferente;
* permissões diferentes;
* políticas diferentes;
* credenciais diferentes;
* escopo diferente;
* configuração diferente;
* estado diferente.

---

# 5. Integration Definition ≠ Integration Instance

Esta separação é obrigatória.

Uma Definition não representa automaticamente uma conta autorizada.

Uma Instance não deve alterar a definição global.

Exemplo:

```text
Definition:
GitHub v1

Instance A:
José / conta pessoal

Instance B:
Projeto Yuki

Instance C:
Conta futura
```

Cada Instance pode possuir autorização independente.

---

# 6. Integration Manifest

Cada Integration Definition deve possuir um Manifest.

O Manifest descreve características relativamente estáveis da integração.

Estrutura conceitual:

```text
Integration Manifest
├── Identity
├── Version
├── Provider
├── External System
├── Supported Capabilities
├── Contracts
├── Protocol Requirements
├── Runtime Requirements
├── Network Requirements
├── Compatibility
└── Lifecycle Metadata
```

O Manifest não deve concentrar todas as informações operacionais da integração.

---

# 7. Identity

Toda Integration Definition deve possuir uma identidade estável.

Exemplo:

```text
integration_id:
github
```

A identidade deve ser:

* única;
* estável;
* independente da URL de um endpoint;
* independente da credencial;
* independente da instância;
* independente da implementação específica.

A identidade não deve depender exclusivamente do provider.

---

# 8. Versioning

Integrações devem possuir versão.

Exemplo:

```text
github
v1.0
v1.1
v2.0
```

A versão deve permitir distinguir mudanças:

* compatíveis;
* incompatíveis;
* comportamentais;
* de contrato;
* de segurança.

O modelo exato de versionamento poderá ser definido posteriormente.

---

# 9. Provider

O Manifest pode identificar o Provider.

Exemplo:

```text
provider:
GitHub
```

Porém:

> Provider não deve se tornar uma dependência estrutural do Core.

A relação é:

```text
Provider
 ↓
External System
 ↓
Integration
```

---

# 10. External System

O Manifest deve permitir identificar o sistema externo ao qual a integração se conecta.

Exemplo:

```text
Provider:
GitHub

External System:
GitHub API
```

Isso permite distinguir:

* organização;
* fornecedor;
* sistema;
* conta;
* recurso.

---

# 11. Supported Capabilities

Uma Integration Definition deve declarar quais Capabilities suporta.

Exemplo:

```text
supported_capabilities:
  - repository.read
  - issue.read
  - issue.create
  - pull_request.read
```

A declaração de uma Capability não significa que todas as Instances possuem autorização para executá-la.

Portanto:

```text
Supported Capability
        ≠
Authorized Capability
```

---

# 12. Capability Binding

Quando necessário, a Integration pode definir como uma Capability é mapeada para o sistema externo.

Exemplo:

```text
Yuki Capability
github.issue.create
        │
        ▼
Integration Mapping
        │
        ▼
External Operation
POST /issues
```

O mapeamento pertence à Integration.

O Core deve trabalhar preferencialmente com a Capability abstrata.

---

# 13. Contracts

A Integration Definition deve declarar os contratos necessários para suas operações.

Esses contratos podem definir:

* input;
* output;
* erros;
* pré-condições;
* pós-condições;
* efeitos;
* requisitos;
* verification strategy;
* idempotency semantics.

A Definition não deve assumir que todo provider possui comportamento idêntico.

---

# 14. Runtime Requirements

A Integration pode declarar requisitos para execução.

Exemplos:

```text
runtime:
  network: required
  filesystem: restricted
  environment: linux-compatible
  isolation: medium
```

Esses requisitos representam necessidades da integração.

Eles não determinam necessariamente qual infraestrutura será usada.

A decisão de alocação pertence ao sistema de Infrastructure/Resource Management definido no Domínio 14.

---

# 15. Network Requirements

Uma Integration pode declarar:

* necessidade de internet;
* endpoints;
* portas;
* protocolos;
* DNS;
* comunicação inbound/outbound;
* necessidade de rede privada;
* requisitos de conectividade.

Exemplo conceitual:

```text
network:
  outbound: required
  inbound: forbidden
  protocols:
    - HTTPS
```

Essas informações são requisitos.

Não são autorização automática.

---

# 16. Protocol Abstraction

A Integration Definition pode declarar protocolos suportados.

Exemplos:

```text
REST
GraphQL
gRPC
SOAP
MQTT
WebSocket
SSE
Custom
```

Nenhum protocolo específico é constitucionalmente obrigatório.

A arquitetura deve permitir:

```text
Integration
 ↓
Protocol Adapter
 ↓
Provider
```

---

# 17. Connector Reference

Uma Definition pode referenciar um Connector.

Exemplo:

```text
integration:
github

connector:
github-connector
v1.x
```

O Connector contém lógica específica de domínio.

A Definition descreve a integração.

O Connector implementa parte dessa integração.

---

# 18. Adapter Reference

Uma Definition também pode declarar adapters necessários.

Exemplo:

```text
protocol:
REST

adapter:
HTTP/REST Adapter
```

O Adapter trata tradução técnica.

O Connector trata conhecimento de domínio.

Quando necessário, uma integração pode utilizar ambos.

---

# 19. Credential References

Credenciais não devem ser armazenadas diretamente no Manifest.

O Manifest pode declarar:

```text
credential_requirements:
  - github.api
```

A Instance pode possuir:

```text
credential_reference:
secret://...
```

A referência não representa o segredo em si.

O segredo permanece sob controle do Credential Broker/Secret Management.

---

# 20. Policy

Policy não deve ser incorporada como parte estática do Manifest.

Uma Definition pode declarar requisitos ou capacidades esperadas.

Mas a autorização efetiva pertence ao:

```text
Security Controller
+
Policy System
```

Isso permite que duas Instances da mesma Definition possuam políticas diferentes.

---

# 21. Permission

Permission também não pertence como regra global ao Manifest.

Exemplo:

```text
Definition:
github.issue.create
```

Uma Instance pode possuir:

```text
Permission:
ALLOW
```

Outra:

```text
Permission:
DENY
```

A Definition descreve possibilidade.

A Security Architecture decide autorização.

---

# 22. Health

Health não deve ser armazenado como propriedade estática do Manifest.

Uma Integration pode estar:

```text
HEALTHY
```

em determinado momento e:

```text
UNHEALTHY
```

posteriormente.

Health é estado dinâmico.

---

# 23. Telemetry

Telemetry também não pertence ao Manifest.

Exemplos:

* latência;
* taxa de erro;
* throughput;
* uso de recursos;
* disponibilidade;
* timeouts;
* retries.

Esses dados pertencem ao sistema de observabilidade.

---

# 24. Allocation

Allocation também permanece separado.

Uma Integration pode necessitar:

```text
CPU
Memory
Network
Runtime
Storage
```

Mas o Manifest não representa a alocação atual.

A regra segue o ADR-006:

> **Manifest ≠ Telemetry ≠ State ≠ Allocation.**

---

# 25. State Model

O estado da Integration deve ser multidimensional.

Não será utilizado um único FSM gigantesco.

## Lifecycle

```text
DISCOVERED
REGISTERED
ACTIVE
SUSPENDED
QUARANTINED
DISABLED
REMOVED
```

## Health

```text
HEALTHY
DEGRADED
UNHEALTHY
UNKNOWN
```

## Trust

```text
TRUSTED
SUSPECTED
UNTRUSTED
```

## Authorization

```text
AUTHORIZED
RESTRICTED
REVOKED
```

## Runtime

```text
RUNNING
STOPPED
FAILED
```

---

# 26. Instance Metadata

Uma Integration Instance pode possuir:

```text
Instance
├── instance_id
├── definition_id
├── identity
├── provider_account
├── configuration
├── credential_reference
├── policy_reference
├── permission_scope
├── environment
├── lifecycle_state
├── health_state
├── trust_state
└── runtime_state
```

O modelo exato poderá ser refinado em ADRs posteriores.

---

# 27. Integration ID

O `integration_id` identifica a Definition.

O `instance_id` identifica uma conexão concreta.

Exemplo:

```text
integration_id:
github

instance_id:
github-jose-personal
```

Essa distinção é obrigatória.

---

# 28. Compatibility

Uma Integration Definition deve declarar informações suficientes para determinar compatibilidade.

Podem ser considerados:

* versão;
* capabilities;
* runtime;
* protocolo;
* arquitetura;
* dependências;
* recursos;
* security requirements.

Compatibilidade não significa autorização.

---

# 29. Dependencies

Uma Definition pode declarar dependências.

Exemplos:

```text
Runtime
Protocol Adapter
Credential Provider
Network
Event Adapter
```

Dependências devem ser identificáveis e versionáveis.

Dependências externas devem passar pelos mecanismos de supply-chain e segurança apropriados.

---

# 30. Security Requirements

O Manifest pode declarar requisitos de segurança.

Exemplo:

```text
security:
  network_isolation: required
  filesystem: restricted
  secret_access: brokered
  minimum_isolation: high
```

Esses requisitos são constraints.

A implementação concreta continua sendo responsabilidade da infraestrutura e dos sistemas de segurança.

---

# 31. Isolation Requirement

A Integration Definition pode declarar um nível mínimo de isolamento.

Exemplo conceitual:

```text
LOW
MEDIUM
HIGH
CRITICAL
```

A implementação pode escolher:

```text
Process
Container
WASM
MicroVM
Dedicated Runtime
```

desde que cumpra o requisito.

Portanto:

> **O Manifest declara o requisito; a Infrastructure escolhe a implementação compatível.**

---

# 32. Data Requirements

A Definition pode declarar quais tipos de dados uma operação necessita.

Exemplo:

```text
data:
  repository_metadata
  issue_content
  account_identity
```

Isso permite aplicar Data Minimization.

Uma Integration não deve receber dados que não sejam necessários para sua operação.

---

# 33. Side Effects

O Manifest deve permitir identificar que operações possuem efeitos externos.

Exemplo:

```text
operation:
issue.create

side_effect:
external_write
```

Isso permite que o sistema determine:

* necessidade de autorização;
* risco;
* idempotency;
* verification;
* audit.

---

# 34. Verification Requirements

Uma operação pode declarar requisitos de verificação.

Exemplo:

```text
verification:
  required: true
  strategies:
    - provider_receipt
    - direct_read
```

A Integration Definition não precisa determinar uma única estratégia.

Ela pode declarar estratégias compatíveis.

O sistema escolhe uma estratégia apropriada ao contexto.

---

# 35. Idempotency Metadata

Uma operação pode declarar:

```text
idempotency:
  supported: true
```

ou:

```text
idempotency:
  supported: false
```

Também pode declarar mecanismos específicos do provider.

A Integration não deve impor um algoritmo universal de geração de idempotency key.

---

# 36. External System Mapping

A Definition deve ser capaz de representar:

```text
Yuki Concept
        ↓
Canonical Concept
        ↓
Provider Concept
        ↓
External Resource
```

Exemplo:

```text
Yuki Issue
 ↓
Canonical Issue
 ↓
GitHub Issue
 ↓
repository/123#42
```

Isso reduz o vazamento de detalhes externos para o Core.

---

# 37. Manifest ≠ Implementation

O Manifest descreve a integração.

Ele não precisa conter:

* código;
* segredos;
* runtime state;
* logs;
* telemetry;
* allocation;
* temporary tokens;
* execution history completo.

Esses elementos pertencem a outros sistemas.

---

# 38. Manifest ≠ Policy

Manifest pode declarar requisitos.

Policy decide o que é permitido.

```text
Manifest
→ "precisa de acesso à rede"

Policy
→ "esta instância pode acessar este endpoint"
```

---

# 39. Manifest ≠ Permission

Manifest:

```text
"suporta issue.create"
```

Permission:

```text
"esta execução pode criar issue no repositório X"
```

São conceitos diferentes.

---

# 40. Manifest ≠ Trust

Uma Integration pode estar registrada sem ser considerada confiável.

```text
REGISTERED
+
UNTRUSTED
```

é um estado válido.

---

# 41. Manifest ≠ Health

Uma Integration pode possuir um Manifest válido enquanto estiver indisponível.

```text
Definition:
VALID

Instance:
UNHEALTHY
```

---

# 42. Lifecycle da Definition

Uma Definition pode passar por:

```text
DISCOVERED
    ↓
ASSESSED
    ↓
REGISTERED
    ↓
VALIDATED
    ↓
AVAILABLE
    ↓
DEPRECATED
    ↓
RETIRED
```

Esse lifecycle representa a Definition.

Não deve ser confundido com o lifecycle de uma Instance.

---

# 43. Lifecycle da Instance

Uma Instance pode passar por:

```text
CREATED
    ↓
CONFIGURED
    ↓
AUTHORIZED
    ↓
ACTIVE
    ↓
SUSPENDED
    ↓
REVOKED
    ↓
REMOVED
```

Esses estados podem coexistir com:

* Health;
* Trust;
* Runtime.

---

# 44. Discovery

Descoberta de uma Integration não implica instalação nem autorização.

Fluxo:

```text
Discovery
 ↓
Assessment
 ↓
Validation
 ↓
Registration
 ↓
Activation
```

Cada etapa possui significado diferente.

---

# 45. Registration

Registration significa que a Yuki conhece a Integration.

Não significa:

```text
trusted
authorized
active
healthy
```

Uma Integration registrada pode permanecer desativada.

---

# 46. Activation

Activation permite que a Integration esteja disponível para execução.

Mesmo ativa, cada operação continua sujeita a:

* authorization;
* policy;
* risk;
* limits;
* verification.

---

# 47. Revocation

A Yuki deve permitir revogação independente de:

* Definition;
* Instance;
* Capability;
* Permission;
* Credential.

Exemplo:

```text
Instance
 ↓
Credential revoked
```

sem necessariamente remover a Definition.

---

# 48. Quarantine

Uma Integration pode ser colocada em quarentena quando:

* integridade estiver comprometida;
* comportamento inesperado for detectado;
* dependência estiver comprometida;
* security signal for suspeito;
* provider estiver comprometido;
* execução apresentar comportamento anômalo.

Quarantine deve reduzir ou bloquear sua capacidade operacional.

---

# 49. Future Compatibility

O modelo deve suportar integrações futuras que ainda não existem.

Isso significa que o Manifest não deve assumir:

* HTTP;
* APIs REST;
* cloud;
* internet;
* containers;
* Wasm;
* um único modelo de identidade;
* um único protocolo.

Essas são possíveis implementações.

---

# 50. Technology Independence

Nenhuma das tecnologias abaixo é uma dependência constitucional deste ADR:

```text
Rust
WASM
WASI
MicroVM
Docker
Kubernetes
NATS
Kafka
SPIFFE
SPIRE
Vault
OpenAPI
gRPC
Protobuf
CloudEvents
OAuth
OIDC
```

Elas poderão ser adotadas por ADRs específicos.

---

# 51. Exemplo Conceitual de Manifest

Um Manifest conceitual pode assumir formato semelhante a:

```yaml
integration:
  id: github
  version: 1.0

provider:
  id: github

external_system:
  id: github_api

capabilities:
  - repository.read
  - issue.read
  - issue.create
  - pull_request.read

protocols:
  - https

runtime:
  network:
    outbound: true
    inbound: false

security:
  secret_access: brokered
  minimum_isolation: medium

operations:
  issue.create:
    side_effect: external_write
    idempotency:
      supported: true
    verification:
      strategies:
        - provider_receipt
        - direct_read
```

Este formato é **ilustrativo**.

O formato definitivo do Manifest será decidido posteriormente.

---

# 52. Exemplo de Instance

Conceitualmente:

```yaml
instance:
  id: github-jose-personal

definition:
  id: github
  version: 1.0

identity:
  account: personal

credentials:
  reference: credential://github/personal

policy:
  reference: policy://github/personal

permissions:
  - repository.read
  - issue.read
  - issue.create
```

Novamente, o formato não é congelado neste ADR.

---

# 53. Fluxo de Execução

A execução de uma operação segue:

```text
Yuki Core
   ↓
Capability
   ↓
Execution Request
   ↓
Security Controller
   ↓
Permission / Policy
   ↓
Integration Definition
   ↓
Integration Instance
   ↓
Integration Gateway
   ↓
Credential Broker
   ↓
Connector / Adapter
   ↓
External System
   ↓
Verification
   ↓
Result
```

---

# 54. Data Minimization

A Integration recebe apenas os dados necessários para sua execução.

Exemplo:

```text
Task Context
      ↓
Required Data Extraction
      ↓
Integration
```

Não:

```text
Entire User Context
        ↓
Integration
```

Isso reduz:

* exposição;
* risco;
* vazamento;
* impacto de comprometimento.

---

# 55. Security Invariants

Este ADR estabelece os seguintes invariantes:

```text
I1.
Definition não concede autorização.

I2.
Registration não concede confiança.

I3.
Capability não concede Permission.

I4.
Instance não altera Definition global.

I5.
Manifest não contém segredos brutos.

I6.
Health não determina Authorization.

I7.
Trust não determina Permission automaticamente.

I8.
Protocol não deve vazar desnecessariamente para o Core.

I9.
Provider-specific details devem permanecer isolados sempre que possível.

I10.
Runtime State não pertence ao Manifest estático.

I11.
Telemetry não pertence ao Manifest.

I12.
Allocation não pertence ao Manifest.

I13.
External Data não ganha autoridade por estar associado a uma Integration.

I14.
Model Output não pode ignorar o Security Controller.

I15.
Uma Integration comprometida deve poder ser isolada.
```

---

# 56. Consequências Positivas

Este modelo permite:

### 56.1 Múltiplas contas

```text
One Definition
→ Multiple Instances
```

### 56.2 Substituição de providers

```text
Provider A
→ Provider B
```

sem reescrever o Core.

### 56.3 Evolução independente

Definition, Connector e Adapter podem evoluir separadamente.

### 56.4 Segurança

Credenciais, autorização e runtime permanecem separados.

### 56.5 Observabilidade

Health e telemetry não contaminam o Manifest estático.

### 56.6 Infraestrutura independente

O Manifest declara requisitos sem determinar hardware.

### 56.7 Futuro

Novos protocolos e tecnologias podem ser adicionados sem alterar o modelo fundamental.

---

# 57. Consequências Negativas

O modelo introduz complexidade adicional.

Será necessário manter:

* Definitions;
* Instances;
* manifests;
* contracts;
* adapters;
* connectors;
* policies;
* credentials;
* states;
* health;
* authorization.

Também existe custo de:

* versionamento;
* compatibilidade;
* migração;
* descoberta;
* validação;
* observabilidade.

Essa complexidade é aceita porque reduz acoplamento e aumenta segurança e evolução.

---

# 58. Decisões Fechadas

Este ADR fecha oficialmente:

### D7-1

**Integration Definition e Integration Instance são entidades distintas.**

### D7-2

**Toda Integration Definition possui identidade própria.**

### D7-3

**Toda Integration Definition possui versão.**

### D7-4

**Integration pode suportar múltiplas Capabilities.**

### D7-5

**Supported Capability não implica autorização.**

### D7-6

**Integration Instance pode possuir políticas e permissões próprias.**

### D7-7

**Manifest representa características relativamente estáveis da Integration Definition.**

### D7-8

**Secrets não pertencem ao Manifest.**

### D7-9

**Policy e Permission não são propriedades globais do Manifest.**

### D7-10

**Health, Telemetry e Allocation permanecem separados do Manifest.**

### D7-11

**Lifecycle, Health, Trust, Authorization e Runtime são dimensões separadas.**

### D7-12

**Provider e External System devem ser representáveis separadamente.**

### D7-13

**Capabilities externas devem ser mapeáveis por contratos da Integration.**

### D7-14

**A Integration deve declarar requisitos sem determinar diretamente a infraestrutura.**

### D7-15

**O formato definitivo do Manifest permanece aberto.**

### D7-16

**O modelo não depende de protocolo, runtime ou tecnologia específica.**

### D7-17

**Uma Integration registrada não é automaticamente confiável ou autorizada.**

### D7-18

**Integration Definition e Instance devem poder evoluir independentemente.**

---

# 59. Decisões Não Fechadas

Este ADR deliberadamente não decide:

* formato final YAML/JSON/Protobuf/etc.;
* Schema Registry;
* sistema definitivo de versionamento;
* semver obrigatório;
* tecnologia do Credential Broker;
* SPIFFE/SPIRE;
* Vault;
* OAuth/OIDC;
* mecanismo de sandbox;
* Wasm;
* MicroVM;
* container;
* formato definitivo de Capability Mapping;
* Canonical Data Model;
* mecanismo definitivo de Discovery;
* mecanismo definitivo de Verification;
* sistema definitivo de Event Integration;
* implementação do Integration Registry.

Esses assuntos permanecem candidatos a ADRs específicos.

---

# 60. ADRs Relacionados

Este ADR depende conceitualmente de:

```text
ADR-006
Yuki Infrastructure Resource Model
```

e se relaciona diretamente com:

```text
07 — Agents & Tasks
08 — Capability System
09 — Model Router
10 — Security
11 — Evolution
13 — Events & Background
14 — Infrastructure
15 — Integrations
```

ADRs futuros relacionados:

```text
ADR-008 — Credential Isolation & Secret Delivery
ADR-009 — External Action Verification
ADR-010 — Integration Execution Isolation
ADR-011 — Integration Identity & Authorization
ADR-012 — External Data / Instruction Boundary
```

---

# 61. Decisão Final

A Yuki adotará **Integration Definition + Integration Instance + Manifest** como modelo oficial para representar integrações externas.

A arquitetura será:

```text
Capability
     │
     ▼
Integration Definition
     │
     ▼
Integration Instance
     │
     ▼
Integration Runtime
     │
     ├── Connector
     ├── Adapter
     └── Credential Broker
     │
     ▼
External System
```

Enquanto:

```text
Security Controller
```

permanece independente e determina a autorização.

E:

```text
Resource Manager
```

determina os recursos de execução.

E:

```text
Agent / Workflow System
```

determina a orquestração.

E:

```text
Audit
```

mantém as evidências.

E:

```text
Verification
```

determina se o efeito esperado realmente ocorreu.

---

# 62. Princípio Final

O modelo é resumido por:

> **Definition descreve. Instance conecta. Capability define o que pode ser feito. Permission autoriza. Connector entende o domínio. Adapter traduz. Gateway media. Security decide. Runtime executa. Verification confirma. Audit registra.**

---

# 63. Status

**ADR-007 — ACCEPTED**

**Versão:** v1.0

A decisão passa a fazer parte da arquitetura oficial da Yuki.

Alterações futuras deverão ocorrer por:

* revisão deste ADR;
* novo ADR;
* ou decisão arquitetural explicitamente documentada.

**Fim do ADR-007**
