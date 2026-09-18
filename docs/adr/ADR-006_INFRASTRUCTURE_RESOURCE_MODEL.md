# ADR-006 — Yuki Infrastructure Resource Model

**Status:** Accepted
**Version:** 1.0
**Date:** 2026-09-18
**Decision Type:** Architectural
**Scope:** Infrastructure / Resource Management
**Project:** Yuki

---

## 1. Contexto

A Yuki deverá funcionar por muitos anos e atravessar diferentes gerações de hardware, sistemas operacionais, runtimes, provedores de nuvem, modelos de IA, dispositivos e tecnologias de computação.

Por isso, a arquitetura não pode assumir que:

* um servidor específico será permanente;
* uma GPU específica será permanente;
* CPU será sempre a principal unidade de computação;
* um único provedor de nuvem será utilizado;
* um único runtime será utilizado;
* containers serão suficientes para todos os workloads;
* um único scheduler será responsável por toda a infraestrutura;
* uma determinada tecnologia de banco, mensageria ou virtualização permanecerá para sempre.

A infraestrutura da Yuki precisa ser representada por uma abstração própria.

O problema central deste ADR é:

> **Como a Yuki representa, descobre, seleciona, aloca e utiliza recursos de infraestrutura sem ficar acoplada à implementação física ou tecnológica desses recursos?**

---

# 2. Problema arquitetural

Existem diferentes conceitos que não devem ser confundidos.

Uma GPU física, por exemplo, é um recurso.

A capacidade de executar operações FP16 é uma capacidade.

Uma tarefa pode exigir baixa latência como requisito.

Uma política pode proibir o processamento fora de determinado domínio.

Uma permissão pode autorizar determinado componente a utilizar um recurso.

Portanto:

```text
Requirement ≠ Capability ≠ Resource ≠ Permission ≠ Workload
```

Misturar esses conceitos faria componentes superiores da Yuki precisarem conhecer detalhes da infraestrutura.

Isso reduziria a portabilidade e dificultaria a evolução futura.

---

# 3. Decisão

A Yuki adotará um **Infrastructure Resource Model próprio**, baseado no conceito de:

> **Typed Abstract Resource**

Um Typed Abstract Resource é a unidade fundamental de **representação abstrata de recursos de infraestrutura** dentro da Yuki.

Ele representa um recurso independentemente de sua implementação física ou tecnológica.

Isso significa que a Yuki poderá representar, por exemplo:

```text
Compute Resource
Storage Resource
Network Resource
Runtime Resource
Device Resource
Accelerator Resource
External Resource
Future Resource
```

sem transformar essa lista em uma taxonomia fechada ou imutável.

O modelo deverá ser extensível para permitir recursos ainda inexistentes ou não previstos atualmente.

---

# 4. O que é um Resource

Um Resource representa algo que pode ser:

* utilizado;
* alocado;
* reservado;
* compartilhado;
* consumido;
* monitorado;
* limitado;
* isolado;
* combinado;
* substituído;
* ou disponibilizado para execução.

Exemplo:

```text
GPU física
CPU
memória RAM
armazenamento
link de rede
runtime
dispositivo
acelerador
recurso computacional remoto
```

A representação abstrata não precisa revelar toda a implementação concreta.

Exemplo:

```text
Yuki Resource
    ↓
Compute Resource
    ↓
Tensor Operations
    ↓
FP16
    ↓
Performance characteristics
    ↓
Locality
    ↓
Memory characteristics
```

A implementação concreta poderia ser:

```text
GPU A
GPU B
NPU C
Cloud Accelerator D
Future Accelerator E
```

A Yuki não deverá precisar alterar o Core para suportar cada nova implementação.

---

# 5. Resource ≠ Capability

Essa separação é fundamental.

## Resource

Representa:

> **O que existe e pode ser utilizado.**

Exemplo:

```text
GPU-01
8 GB VRAM
FP16
INT8
PCIe
Home Domain
```

## Capability

Representa:

> **O que algo consegue fazer.**

Exemplo:

```text
TensorOps
VisionInference
FP16
INT8
VideoEncode
```

Uma capability pode existir em vários recursos.

Um recurso pode possuir várias capabilities.

```text
RESOURCE
├── Identity
├── Type
├── Capacity
├── Availability
├── Topology
├── State
└── Attributes

CAPABILITIES
├── What it can do
└── How it can do it
```

A Capability System continua responsável por representar as capacidades da Yuki.

O Resource Model representa os recursos que tornam determinadas capacidades executáveis.

---

# 6. Resource ≠ Requirement

Um Requirement representa:

> **O que um workload precisa.**

Exemplo:

```text
FP16
latência < 50 ms
processamento local
4 GB de memória
```

Um Resource representa:

> **O que existe para satisfazer esse requisito.**

Exemplo:

```text
GPU-01
FP16
8 GB
Home Domain
```

Portanto:

```text
WORKLOAD
    ↓
REQUIREMENTS
    ↓
RESOURCE MATCHING
    ↓
RESOURCE
```

---

# 7. Resource ≠ Permission

Um recurso estar disponível não significa que determinado componente pode utilizá-lo.

Exemplo:

```text
GPU disponível
        ≠
agente autorizado a utilizá-la
```

A autorização continua pertencendo ao sistema de segurança.

A Resource Manager poderá identificar recursos compatíveis, mas não poderá conceder privilégios arbitrariamente.

A arquitetura permanece:

```text
Security Controller
        ↓
Authorization / Policy
        ↓
Resource Manager
        ↓
Allocation
```

O Security Controller permanece independente.

---

# 8. Resource ≠ Workload

Um Workload representa algo que precisa ser executado.

Exemplo:

```text
Inference Task
Training Job
Embedding Job
Video Processing
Database Operation
Agent Task
```

Um Resource representa aquilo que executará ou sustentará o workload.

Portanto:

```text
WORKLOAD
├── Requirements
├── Constraints
├── Preferences
└── Fallbacks

RESOURCE
├── Identity
├── Capacity
├── Capabilities
├── Topology
├── Availability
└── Attributes
```

---

# 9. Modelo de Workload

Todo workload que depender de infraestrutura deverá poder declarar suas necessidades.

O modelo deverá distinguir:

### Requirements

O que é necessário.

```text
MUST
```

### Constraints

O que não pode ser violado.

Exemplo:

```text
Must remain in Home Domain
```

### Preferences

O que é desejável.

```text
Prefer local GPU
Prefer low latency
Prefer low cost
```

### Fallbacks

Alternativas aceitáveis.

Exemplo:

```text
GPU
→ NPU
→ CPU
```

quando essas alternativas forem semanticamente válidas.

---

# 10. Semântica de requisitos

A Yuki poderá representar requisitos com diferentes níveis de força:

```text
MUST
SHOULD
PREFER
AVOID
```

Exemplo:

```yaml
requirements:
  - capability: tensor_ops
    precision: fp16

constraints:
  - location: home

preferences:
  - accelerator: gpu
  - latency: low

fallbacks:
  - local_cpu
  - cloud_gpu
```

A sintaxe acima é apenas ilustrativa.

O formato definitivo do contrato será decidido posteriormente.

---

# 11. Fallback não pertence automaticamente ao Resource Manager

O Resource Manager não deverá inventar alterações semânticas no workload.

Por exemplo:

```text
FP16
→ INT8
```

pode alterar:

* qualidade;
* comportamento;
* precisão;
* custo;
* desempenho;
* resultados.

Portanto, uma mudança desse tipo deve ser declarada como uma alternativa aceitável pelo componente responsável pela estratégia de execução.

Em geral:

```text
Model Router / Execution Strategy
        ↓
Acceptable Execution Alternatives
        ↓
Resource Manager
        ↓
Find matching resource
```

O mesmo princípio vale para:

```text
GPU → CPU
Cloud → Home
FP16 → INT8
Large Model → Small Model
```

quando houver impacto semântico.

O Resource Manager deve encontrar recursos compatíveis, não decidir sozinho o significado do workload.

---

# 12. Resource Manager

A Yuki terá um componente lógico denominado:

> **Resource Manager**

Sua responsabilidade será administrar o modelo abstrato de infraestrutura.

Ele deverá:

* descobrir recursos;
* registrar recursos;
* acompanhar disponibilidade;
* consultar capacidade;
* avaliar compatibilidade;
* identificar recursos adequados;
* considerar domínio;
* consultar políticas;
* coordenar alocação;
* liberar recursos;
* acompanhar saúde;
* lidar com degradação;
* trabalhar com adapters;
* integrar-se aos schedulers concretos;
* permitir evolução futura da infraestrutura.

O Resource Manager não será responsável por tudo que ocorre na infraestrutura.

---

# 13. Separação de responsabilidades

A arquitetura adotará explicitamente a seguinte separação:

```text
RESOURCE MODEL
"What is a resource?"

RESOURCE MANAGER
"Which resources exist and which can serve the workload?"

SCHEDULER
"Which resource/domain should receive the workload?"

EXECUTION ENGINE
"How will the workload execute?"

ADAPTER
"How is this translated into concrete technology?"
```

Essa separação é uma decisão arquitetural central deste ADR.

---

# 14. Processing Router ≠ Resource Manager

O Processing Router trabalha principalmente com:

```text
Qual processamento é necessário?
```

O Resource Manager trabalha principalmente com:

```text
Onde e com quais recursos isso pode ser executado?
```

Fluxo:

```text
User / Mission
      ↓
Yuki Core
      ↓
Processing Router
      ↓
Workload Requirements
      ↓
Resource Manager
      ↓
Scheduler / Placement
      ↓
Execution
```

O Processing Router não deverá precisar conhecer detalhes físicos de:

```text
PCIe
NUMA
GPU model
VM implementation
container runtime
cloud instance type
```

O Resource Manager e seus adapters lidam com essa tradução.

---

# 15. Resource Manager ≠ Scheduler

A Yuki não deverá assumir que possuirá um scheduler físico universal.

A arquitetura poderá trabalhar com:

```text
Yuki Resource Manager
        ↓
Domain Manager
        ↓
Local Scheduler
```

Por exemplo:

```text
Yuki Resource Manager
        │
        ├── Edge Domain
        │      └── Local Scheduler
        │
        ├── Home Domain
        │      └── Local Scheduler
        │
        └── Cloud Domain
               └── Cloud Scheduler
```

Um scheduler especializado poderá continuar responsável por decisões de baixo nível.

Exemplo:

```text
Kubernetes
Nomad
Slurm
Ray
systemd
Cloud Scheduler
Future Scheduler
```

Essas tecnologias são implementações possíveis, não dependências arquiteturais.

---

# 16. Resource Matching

A seleção de recursos seguirá conceitualmente uma sequência semelhante a:

```text
1. Security / Policy Gate
2. Hard Constraints
3. Compatibility
4. Resource Availability
5. Feasibility
6. Optimization / Ranking
7. Allocation
```

A regra fundamental é:

> **Um recurso que viola um requisito obrigatório não pode tornar-se válido apenas por possuir melhor desempenho ou menor custo.**

Somente depois que os requisitos obrigatórios forem satisfeitos poderão ser consideradas preferências como:

* custo;
* latência;
* energia;
* temperatura;
* desempenho;
* localização;
* disponibilidade;
* eficiência;
* transferência de dados.

---

# 17. Não existe um score universal obrigatório

A arquitetura não congelará uma fórmula específica de scoring.

Não será definido constitucionalmente algo como:

```text
Score =
Performance
+ Locality
+ Cost
+ Thermal
- Latency
```

Diferentes workloads podem exigir estratégias diferentes.

A arquitetura define o pipeline:

```text
Hard Requirements
        ↓
Feasibility
        ↓
Optimization
        ↓
Ranking
```

O algoritmo de otimização poderá evoluir independentemente do Resource Model.

---

# 18. Resource Manifest

Cada recurso deverá possuir uma representação relativamente estável.

Conceitualmente:

```text
Resource Identity
Resource Description
Resource Type
Resource Capacity
Resource Capabilities
Resource Constraints
Resource Topology
Resource Attributes
```

Informações dinâmicas não deverão ser tratadas como identidade estática.

---

# 19. Resource Telemetry

Informações que mudam constantemente deverão ser tratadas separadamente.

Exemplo:

```text
Resource Telemetry
├── Current Utilization
├── Temperature
├── Power
├── Health
├── Availability
├── Network Latency
├── Memory Pressure
└── Current Load
```

Isso evita transformar o manifesto de um recurso em um documento constantemente mutável.

---

# 20. Estado do recurso

A arquitetura não adotará uma única máquina de estados gigante.

Serão consideradas dimensões independentes.

### Lifecycle

```text
DISCOVERED
REGISTERED
RETIRED
```

### Health

```text
HEALTHY
DEGRADED
FAILED
QUARANTINED
```

### Allocation

```text
AVAILABLE
RESERVED
ALLOCATED
RELEASING
```

Isso evita combinações artificiais como:

```text
DEGRADED_ALLOCATED
FAILED_RESERVED
```

como estados monolíticos.

---

# 21. Descoberta de recursos

Recursos poderão surgir de:

* hardware local;
* servidores;
* dispositivos;
* virtualização;
* cloud;
* serviços externos;
* futuros aceleradores;
* novos domínios de infraestrutura.

O processo conceitual será:

```text
Discovery
    ↓
Identity
    ↓
Trust / Attestation
    ↓
Capability Advertisement
    ↓
Adapter Availability
    ↓
Compatibility
    ↓
Testing / Validation
    ↓
Usable Resource
```

Um recurso desconhecido poderá ser conhecido pela Yuki sem necessariamente ser imediatamente utilizável.

---

# 22. Recursos futuros e desconhecidos

A arquitetura deverá permitir recursos que ainda não existem atualmente.

Exemplos hipotéticos:

```text
GenericAccelerator
PhotonicProcessor
QuantumProcessor
NeuromorphicProcessor
CXL Memory Pool
Future AI Accelerator
```

A existência de um novo tipo não deverá exigir reescrever o Yuki Core.

Porém:

> **Ser descoberto não significa automaticamente ser confiável ou executável.**

Será necessário verificar:

* identidade;
* confiança;
* compatibilidade;
* adapter;
* capacidades anunciadas;
* segurança;
* testes;
* políticas.

---

# 23. Resource Adapters

Adapters traduzem a abstração da Yuki para uma implementação concreta.

Exemplo:

```text
Yuki Compute Resource
        ↓
GPU Adapter
        ↓
Concrete GPU Runtime
```

Ou:

```text
Yuki Storage Resource
        ↓
Object Storage Adapter
        ↓
S3-Compatible Backend
```

Ou:

```text
Yuki Runtime Resource
        ↓
MicroVM Adapter
        ↓
Concrete Runtime
```

A arquitetura não deverá exigir uma tecnologia específica.

---

# 24. Hardware Independence

O Yuki Core não deverá depender diretamente de hardware específico.

O caminho deverá ser:

```text
Yuki
 ↓
Infrastructure API
 ↓
Resource Manager
 ↓
Resource Adapter
 ↓
Concrete Hardware / Runtime
```

Isso permite substituir:

```text
CPU
GPU
NPU
FPGA
TPU
Quantum
Photonic
Future Accelerator
```

sem modificar os contratos superiores da Yuki.

---

# 25. Software e drivers

A abstração não significa que drivers físicos desaparecerão.

Alguns ambientes exigirão drivers:

```text
Host
Kernel
Runtime
Firmware
Device Driver
```

A decisão arquitetural é:

> **O Control Plane e o Yuki Core não devem depender diretamente de detalhes específicos de hardware ou drivers.**

A interação concreta deverá ser encapsulada pela camada apropriada.

---

# 26. Domínios de infraestrutura

A infraestrutura poderá ser organizada em domínios.

Inicialmente:

```text
EDGE
HOME
CLOUD
```

Mas o modelo deverá permitir futuros domínios.

Exemplo:

```text
EDGE
HOME
CLOUD
REMOTE
SATELLITE
ROBOTICS
VEHICLE
LAB
FUTURE
```

O domínio representa contexto de infraestrutura.

Não representa autorização.

---

# 27. Resource Fabric

A infraestrutura da Yuki será conceitualmente tratada como um:

> **Resource Fabric**

Em vez de:

```text
Yuki
→ Server
```

a arquitetura será:

```text
Yuki
    ↓
Infrastructure API
    ↓
Resource Manager
    ↓
Resource Fabric
```

O Fabric poderá conter:

```text
COMPUTE
STORAGE
NETWORK
RUNTIME
DEVICE
ACCELERATOR
SECURITY-RELATED EVIDENCE
EXTERNAL RESOURCES
FUTURE RESOURCES
```

A taxonomia poderá evoluir.

---

# 28. Security Resources e Trust

Segurança não será reduzida simplesmente a um tipo de recurso.

Elementos como:

```text
TPM
TEE
Hardware Key
Attestation
Secure Enclave
```

podem fornecer:

* evidências;
* propriedades;
* capacidades;
* garantias técnicas;
* sinais de confiança.

Eles não concedem automaticamente autorização.

Por exemplo:

```text
TPM presente
    ≠
componente autorizado
```

A avaliação de confiança permanece integrada ao Security Controller.

---

# 29. Resource Allocation

Recursos poderão ser:

* compartilhados;
* dedicados;
* reservados;
* temporariamente alugados;
* particionados;
* migrados;
* liberados.

Nem toda alocação precisa possuir TTL.

Recursos permanentes, como determinados elementos da infraestrutura doméstica, não precisam ser representados como leases temporários.

Para recursos temporários:

```text
Acquire
   ↓
Lease
   ↓
Use
   ↓
Renew / Release
```

---

# 30. Resource Failure

Falhas de recursos não deverão necessariamente interromper uma missão.

A Yuki deverá procurar degradação controlada quando possível.

Exemplo:

```text
Home GPU
   ↓
FAILED
   ↓
Alternative Resource
   ↓
CPU / NPU / Cloud
```

desde que a alternativa esteja previamente autorizada e seja semanticamente válida para o workload.

A estratégia de fallback pertence ao planejamento de execução, enquanto o Resource Manager procura o recurso correspondente.

---

# 31. Resource Health e Quarantine

Um recurso poderá ser removido temporariamente do pool utilizável.

Exemplo:

```text
Resource
   ↓
Health Failure
   ↓
Quarantine
   ↓
Diagnosis
   ↓
Validation
   ↓
Return
```

ou:

```text
Quarantine
   ↓
Retirement
```

A quarentena deverá poder ser acionada por:

* falha;
* comportamento anômalo;
* segurança;
* incompatibilidade;
* integridade;
* manutenção.

---

# 32. Resource Security

O Resource Manager não substitui o Security Controller.

A arquitetura continuará:

```text
Security Controller
        │
        ├── Identity
        ├── Authentication
        ├── Authorization
        ├── Policy
        ├── Trust
        ├── Attestation
        └── Audit
                 │
                 ▼
          Resource Manager
```

Nenhum componente deverá conseguir transformar disponibilidade em autorização.

---

# 33. Resource Governance

Recursos deverão estar sujeitos a limites.

Exemplos:

```text
CPU quota
GPU quota
Memory quota
Storage quota
Network quota
Cost budget
Energy budget
Thermal budget
Concurrency limit
```

Esses limites podem existir em múltiplas camadas:

```text
Yuki Policy
↓
Resource Manager
↓
Scheduler
↓
Runtime
↓
Hardware
```

Defesas em profundidade são preferíveis a uma única barreira.

---

# 34. Runaway Computation

A arquitetura deverá considerar workloads que:

* executam por tempo excessivo;
* consomem recursos inesperadamente;
* geram loops;
* multiplicam tarefas;
* excedem orçamento;
* provocam event storms;
* consomem recursos de outros workloads.

O controle poderá envolver:

```text
Timeouts
Quotas
Budgets
Concurrency Limits
Runtime Limits
Cancellation
Checkpointing
Rate Limits
```

As implementações concretas serão definidas posteriormente.

---

# 35. Resource Accounting

A infraestrutura deverá permitir contabilizar:

```text
Compute Usage
Storage Usage
Network Usage
Cloud Cost
Energy
Time
Capacity
Allocation
```

Isso permitirá ao sistema tomar decisões conscientes sobre:

* custo;
* desempenho;
* energia;
* prioridade;
* disponibilidade;
* privacidade.

---

# 36. Resource Federation

A Yuki poderá administrar múltiplos domínios.

Entretanto, a arquitetura não adotará um único scheduler global que conheça todos os detalhes físicos.

Preferência arquitetural:

```text
Yuki Resource Manager
        ↓
Domain Managers
        ├── Edge
        ├── Home
        └── Cloud
              ↓
        Local Schedulers
```

Isso reduz acoplamento e permite que cada domínio utilize sua própria tecnologia.

---

# 37. Hierarquia de gerenciamento

A arquitetura poderá ser hierárquica:

```text
Yuki
 ↓
Global Resource Management
 ↓
Domain Management
 ↓
Local Scheduling
 ↓
Concrete Infrastructure
```

A Yuki toma decisões de nível apropriado.

O domínio local toma decisões de infraestrutura detalhada.

---

# 38. Infraestrutura e offline operation

A Yuki deverá continuar funcional mesmo quando:

```text
Cloud unavailable
Internet unavailable
Provider unavailable
```

desde que os recursos locais disponíveis sejam suficientes.

O Resource Model deve permitir identificar:

```text
LOCAL
HOME
OFFLINE-CAPABLE
REMOTE
CLOUD
```

e a disponibilidade desses recursos.

---

# 39. Hybrid Infrastructure

A infraestrutura da Yuki adotará uma orientação:

> **Hybrid-First**

com uso local sempre que apropriado.

A arquitetura poderá combinar:

```text
EDGE
   ↓
HOME
   ↓
CLOUD
```

O recurso utilizado dependerá de:

* privacidade;
* latência;
* capacidade;
* disponibilidade;
* custo;
* conectividade;
* energia;
* política;
* requisitos do workload.

---

# 40. Resource Selection não é decisão exclusivamente de performance

A seleção de recursos não deverá buscar apenas:

```text
maximum performance
```

Ela deverá considerar o conjunto de restrições e objetivos relevantes.

Exemplo:

```text
Performance
Privacy
Latency
Cost
Energy
Thermal
Reliability
Availability
Location
Data Transfer
Security
```

Nenhum desses fatores poderá violar um requisito obrigatório de segurança ou política.

---

# 41. Observabilidade

A infraestrutura deverá ser observável.

A arquitetura deverá permitir acompanhar:

```text
Resource Discovery
Resource State
Resource Health
Resource Utilization
Allocation
Failures
Quarantine
Migration
Performance
Cost
Energy
```

A observabilidade deverá integrar-se ao sistema de:

* eventos;
* auditoria;
* monitoramento;
* evolução;
* diagnóstico.

---

# 42. Versionamento e compatibilidade

Resources, adapters e contratos poderão evoluir.

A Yuki deverá considerar:

```text
Resource Schema Version
Capability Version
Adapter Version
Runtime Version
Contract Version
```

Mudanças incompatíveis deverão possuir mecanismos de:

* versionamento;
* migração;
* compatibilidade;
* depreciação.

---

# 43. Tecnologia não é arquitetura

Este ADR não torna obrigatórios:

```text
Rust
Linux
PostgreSQL
NATS
Kubernetes
Nomad
Ray
Docker
Podman
Wasm
Firecracker
OpenTofu
WireGuard
SPIFFE
SPIRE
vLLM
ONNX Runtime
```

Essas tecnologias podem ser avaliadas como implementações.

A arquitetura deve sobreviver à substituição delas.

---

# 44. Decisões explicitamente fechadas

As seguintes decisões são consideradas **DECIDED** nesta versão:

### D1

`Requirement ≠ Capability ≠ Resource`.

### D2

A Yuki possuirá uma abstração própria de infraestrutura.

### D3

O modelo será independente de hardware, fornecedor e provedor.

### D4

Workloads poderão declarar requisitos de infraestrutura.

### D5

Requirements, Constraints, Preferences e Fallbacks serão conceitos distintos.

### D6

O Resource Manager não substituirá automaticamente schedulers especializados.

### D7

Adapters traduzirão a abstração da Yuki para tecnologias concretas.

### D8

O Security Controller permanecerá independente do Resource Manager.

### D9

Alocações temporárias poderão utilizar leases.

### D10

O modelo será extensível para recursos futuros e desconhecidos.

### D11

Edge, Home e Cloud serão tratados como domínios de infraestrutura.

### D12

Hard constraints deverão ser satisfeitas antes de qualquer ranking ou otimização.

### D13

O Resource Model será separado do Resource Manager.

### D14

Resource Manager, Scheduler, Execution Engine e Adapter terão responsabilidades distintas.

### D15

Estado, saúde e alocação serão tratados como dimensões distintas.

### D16

Manifesto relativamente estável será separado de telemetria dinâmica.

### D17

Descoberta de recurso não implica confiança nem autorização.

---

# 45. Decisões propostas

As seguintes decisões são **PROPOSED**, podendo gerar ADRs próprios:

### P1

Definir um Resource Contract formal.

### P2

Definir um Workload Contract formal.

### P3

Definir uma linguagem ou estrutura para:

```text
MUST
SHOULD
PREFER
AVOID
```

### P4

Definir protocolo de Resource Discovery.

### P5

Definir mecanismo de Resource Attestation.

### P6

Definir protocolo de Resource Federation.

### P7

Definir Resource Accounting.

### P8

Definir mecanismos de Resource Migration.

### P9

Definir modelo de Resource Quotas.

### P10

Definir integração entre Resource Manager e Domain Managers.

---

# 46. Decisões em aberto

As seguintes decisões permanecem **OPEN**:

* implementação do Resource Manager;
* linguagem utilizada;
* banco de dados;
* sistema de mensageria;
* scheduler concreto;
* runtime de containers;
* runtime Wasm;
* MicroVM;
* formato dos manifests;
* JSON/YAML/Protobuf ou outro;
* algoritmo de scheduling;
* algoritmo de ranking;
* fractional GPU;
* CXL;
* arquitetura de federation;
* estratégia específica de cloud;
* tecnologia de observabilidade;
* estratégia específica de energy-aware scheduling.

Essas decisões não devem ser antecipadas sem ADR ou investigação adequada quando tiverem impacto arquitetural relevante.

---

# 47. Relação com os demais sistemas da Yuki

## Core

O Core trabalha com intenção, contexto, planejamento e orquestração.

```text
Core
↓
Workload
```

## Agent System

Agentes produzem e executam workloads.

```text
Agent
↓
Task
↓
Workload
```

## Model Router

Define estratégia de modelo/processamento.

```text
Model Router
↓
Execution Requirements
```

## Capability System

Define o que a Yuki é capaz de fazer.

```text
Capability
↓
Execution Need
```

## Resource Model

Representa os recursos disponíveis.

```text
Resource
↓
Infrastructure
```

## Security Controller

Define se determinada operação é autorizada.

```text
Security
↓
Authorization
```

## Events & Background

Transporta eventos e mantém workloads persistentes/background.

```text
Event
↓
Mission / Task
↓
Workload
```

## Evolution Manager

Pode propor mudanças na infraestrutura.

```text
Evolution Manager
↓
Proposal
↓
Development / Test
↓
Approval
↓
Infrastructure Change
```

---

# 48. Arquitetura consolidada

O modelo completo fica:

```text
                         USER
                           │
                           ▼
                       YUKI CORE
                           │
                    Mission / Task
                           │
                           ▼
                    Processing Router
                           │
                           ▼
                       WORKLOAD
                           │
             ┌─────────────┼─────────────┐
             │             │             │
        Requirements   Constraints   Preferences
             │             │             │
             └─────────────┼─────────────┘
                           │
                           ▼
                 RESOURCE MANAGEMENT
                           │
                ┌──────────┴──────────┐
                │                     │
          Security/Policy        Resource Discovery
                │                     │
                └──────────┬──────────┘
                           ▼
                       MATCHING
                           │
                           ▼
                     HARD FILTER
                           │
                           ▼
                       FEASIBILITY
                           │
                           ▼
                      OPTIMIZATION
                           │
                           ▼
                        ALLOCATION
                           │
                           ▼
                     DOMAIN MANAGER
                           │
                           ▼
                       SCHEDULER
                           │
                           ▼
                    RESOURCE ADAPTER
                           │
                           ▼
                 CONCRETE INFRASTRUCTURE
```

---

# 49. Resource Fabric consolidado

```text
                    YUKI
                      │
              Infrastructure API
                      │
               Resource Manager
                      │
              Resource Fabric
                      │
     ┌────────────────┼────────────────┐
     │                │                │
   COMPUTE         STORAGE          NETWORK
     │                │                │
   CPU              DB              LAN
   GPU              Vector          VPN
   NPU              Object          Cloud
   FPGA             Archive         Future
   Future
     │
     ├───────────────┐
     │               │
   RUNTIME         DEVICE
     │               │
   Native          Camera
   Wasm             Mic
   MicroVM          Sensor
   Container        Robot
                    Actuator
```

O Fabric não é limitado a essa estrutura.

---

# 50. Princípio de abstração

A regra arquitetural central será:

> **A Yuki deve pedir recursos por propriedades e requisitos, não por nomes específicos de máquinas.**

Preferível:

```text
"Preciso de Compute Resource com TensorOps FP16,
baixa latência e permanência no Home Domain."
```

em vez de:

```text
"Use a GPU RTX XXXX do servidor Y."
```

A segunda informação pode existir internamente como implementação.

Ela não deverá ser necessária para o nível cognitivo.

---

# 51. Princípio de evolução

Novos recursos deverão poder entrar na Yuki através de:

```text
New Resource
      ↓
Resource Definition
      ↓
Discovery
      ↓
Trust
      ↓
Capabilities
      ↓
Adapter
      ↓
Compatibility
      ↓
Validation
      ↓
Resource Fabric
```

O Core não deverá precisar ser reescrito simplesmente porque surgiu uma nova classe de hardware.

---

# 52. Princípio de degradação

Quando um recurso falhar:

```text
Resource Failure
      ↓
Detect
      ↓
Evaluate
      ↓
Find Valid Alternative
      ↓
Migrate / Restart / Offload
      ↓
Verify
```

A Yuki deverá preferir degradação controlada à falha total quando isso for seguro e semanticamente válido.

---

# 53. Princípio de segurança

Nenhum Resource Manager, Scheduler, Agent ou Adapter deverá possuir autoridade implícita para alterar políticas de segurança.

Especialmente:

```text
Resource availability
        ≠
Permission
```

e:

```text
Resource capability
        ≠
Authorization
```

---

# 54. Princípio de independência tecnológica

A arquitetura deverá sobreviver à substituição de:

```text
Hardware
OS
Runtime
Database
Scheduler
Cloud
Provider
Messaging System
Model Runtime
Virtualization
Programming Language
```

O mecanismo para isso será:

```text
Contracts
Interfaces
Adapters
Versioning
Abstraction
```

---

# 55. Consequências positivas

A decisão permite:

* independência de hardware;
* independência de cloud;
* suporte a múltiplos domínios;
* evolução gradual;
* substituição de tecnologias;
* melhor isolamento;
* melhor observabilidade;
* graceful degradation;
* suporte a workloads heterogêneos;
* integração com schedulers especializados;
* suporte a hardware futuro;
* melhor separação de responsabilidades.

---

# 56. Consequências negativas

A decisão também introduz complexidade.

Será necessário manter:

* contratos;
* adapters;
* resource discovery;
* resource state;
* resource management;
* integração com schedulers;
* compatibilidade;
* observabilidade;
* políticas;
* versionamento.

A abstração não elimina a complexidade da infraestrutura.

Ela a organiza e impede que essa complexidade contamine o restante da Yuki.

---

# 57. Risco arquitetural

Existe um risco importante:

> **Transformar o Resource Manager em um sistema operacional distribuído completo.**

A Yuki não deve tentar reinventar todos os mecanismos existentes.

Quando uma tecnologia especializada resolver bem uma parte do problema, a Yuki deverá coordená-la por meio de adapters e contratos.

A regra é:

```text
Yuki-level decision
        ↓
Specialized infrastructure system
```

e não:

```text
Yuki reinvents everything
```

---

# 58. Regra contra overengineering

Nem todo recurso precisa passar pela infraestrutura distribuída completa.

A implementação deverá escolher o nível adequado.

Exemplo:

```text
Tiny local operation
→ direct/local execution

Medium workload
→ local resource allocation

Heavy workload
→ domain scheduler

Massive workload
→ distributed/cloud infrastructure
```

A abstração deve existir para organizar a complexidade, não para tornar uma operação simples artificialmente complexa.

---

# 59. Relação com o princípio de Substrate Independence

Este ADR formaliza parte do princípio:

> **Yuki não deve depender estruturalmente do substrato físico ou tecnológico que executa suas capacidades.**

O substrato pode mudar.

O contrato permanece.

```text
Yuki Contract
     ↓
Resource Abstraction
     ↓
Adapter
     ↓
Current Substrate
```

---

# 60. Relação com a evolução de décadas

A infraestrutura da Yuki poderá evoluir de:

```text
Cloud VM
```

para:

```text
Home Server
```

para:

```text
GPU Cluster
```

para:

```text
NPU / Accelerator
```

para:

```text
CXL Memory
```

para:

```text
Photonic / Quantum / Neuromorphic
```

ou qualquer combinação futura.

O Resource Model deverá continuar válido.

---

# 61. ADRs derivados possíveis

Este ADR poderá gerar decisões específicas para:

```text
ADR-007 — Resource Contract
ADR-008 — Resource Discovery
ADR-009 — Resource Scheduling
ADR-010 — Resource Federation
ADR-011 — Resource Attestation
ADR-012 — Resource Accounting
```

Os números são apenas sugestões e não devem ser considerados reservados sem confirmação do roadmap oficial.

---

# 62. Regras oficiais resultantes

A partir deste ADR, tornam-se princípios oficiais da Yuki:

> **1. Resource não é Capability.**

> **2. Resource não é Requirement.**

> **3. Resource não é Permission.**

> **4. Resource não é Workload.**

> **5. Typed Abstract Resource é a unidade fundamental de representação de recursos de infraestrutura da Yuki.**

> **6. A abstração de recursos deve ser independente de hardware e fornecedor.**

> **7. Resource Manager não é Scheduler.**

> **8. Scheduler não é Execution Engine.**

> **9. Execution Engine não é Adapter.**

> **10. Disponibilidade de recurso não concede autorização.**

> **11. Hard constraints devem ser satisfeitas antes de ranking.**

> **12. Fallbacks semânticos devem ser definidos pela estratégia de execução, não inventados pelo Resource Manager.**

> **13. Recursos futuros devem poder ser incorporados sem reescrever o Core.**

> **14. Descoberta não implica confiança.**

> **15. Confiança não implica autorização.**

> **16. A infraestrutura deve degradar graciosamente quando possível.**

> **17. A arquitetura não deve depender de um scheduler, runtime, banco, sistema operacional, hardware ou provedor específico.**

---

# 63. Status

**ACCEPTED — v1.0**

Este ADR define o modelo arquitetural de recursos da Yuki.

Ele não define todas as implementações da infraestrutura.

Decisões de implementação que possuam impacto arquitetural significativo deverão ser registradas em ADRs posteriores.

---

# 64. Documentos relacionados

```text
07_AGENTS_AND_TASKS.md
08_CAPABILITY_SYSTEM.md
09_MODEL_ROUTER.md
10_SECURITY.md
11_EVOLUTION.md
12_VOICE_AND_MULTIMODAL.md
13_EVENTS_AND_BACKGROUND.md
14_INFRASTRUCTURE.md
```

Este ADR deverá ser considerado conjuntamente com esses documentos.

---

# 65. Resumo executivo

A Yuki não será construída em torno de servidores.

Ela será construída em torno de **recursos abstratos**.

O Core não perguntará:

> "Qual máquina devo usar?"

Ele deverá trabalhar em um nível superior:

> "Que recursos este workload precisa?"

A infraestrutura então responderá:

```text
Requirement
     ↓
Resource Matching
     ↓
Resource
     ↓
Scheduler
     ↓
Adapter
     ↓
Execution
```

Essa separação permite que a Yuki atravesse gerações de:

* hardware;
* software;
* cloud;
* runtimes;
* modelos;
* sistemas operacionais;
* aceleradores;
* paradigmas computacionais.

Sem exigir que sua arquitetura cognitiva seja reconstruída a cada mudança tecnológica.

**Decisão final:**

> **A Yuki adotará um Infrastructure Resource Model baseado em Typed Abstract Resources, separado de Capability, Requirement, Permission e Workload, com Resource Management, Scheduling, Execution e Adapters tratados como camadas distintas e com independência explícita de hardware, fornecedor e tecnologia de implementação.**
