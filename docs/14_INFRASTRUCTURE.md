# Yuki Infrastructure

**Version:** 1.0
**Status:** Accepted
**Document Type:** Architecture
**Scope:** Infrastructure
**Project:** Yuki
**Last Updated:** 2026-09-18

---

# 1. Objetivo

Este documento define a arquitetura de infraestrutura da Yuki.

A infraestrutura da Yuki não deve ser entendida simplesmente como um conjunto de:

* servidores;
* máquinas virtuais;
* containers;
* GPUs;
* bancos de dados;
* redes;
* provedores de cloud.

Ela deve ser entendida como o conjunto de recursos físicos, virtuais, computacionais, de armazenamento, comunicação, execução e dispositivos que permitem que a Yuki exista e opere.

O objetivo desta arquitetura é permitir que a Yuki:

* funcione em diferentes substratos;
* utilize recursos locais e remotos;
* opere de forma híbrida;
* continue funcionando durante falhas;
* utilize diferentes gerações de hardware;
* incorpore novos aceleradores;
* utilize diferentes provedores;
* execute workloads com diferentes níveis de isolamento;
* opere parcialmente offline;
* escale quando necessário;
* seja observável;
* possa ser reconstruída;
* possa evoluir sem reescrita estrutural.

---

# 2. Princípio fundamental

> **A infraestrutura da Yuki não é definida por servidores. É definida por recursos.**

O servidor é apenas uma possível forma física de fornecer recursos.

A abstração fundamental é definida no:

`ADR-006 — Yuki Infrastructure Resource Model`.

Conceitualmente:

```text
Yuki
  ↓
Infrastructure API
  ↓
Resource Manager
  ↓
Resource Fabric
  ↓
Concrete Infrastructure
```

---

# 3. Infrastructure ≠ Hardware

Hardware é uma implementação física.

Infrastructure é o conjunto de recursos e mecanismos que tornam possível executar a Yuki.

Portanto:

```text
Hardware
    ↓
Resource Adapter
    ↓
Yuki Resource
```

Exemplo:

```text
GPU física
        ↓
GPU Adapter
        ↓
Compute Resource
        ↓
TensorOps / FP16 / etc.
```

O Core não deve depender diretamente da GPU física.

---

# 4. Substrate Independence

A Yuki deverá ser independente do substrato.

O substrato pode mudar.

Exemplos:

```text
CPU
GPU
NPU
FPGA
TPU
CXL Memory
Photonic Accelerator
Quantum Accelerator
Future Hardware
```

Também podem mudar:

```text
Linux
Windows
Future OS
VM
Container
MicroVM
Wasm
Bare Metal
Cloud Runtime
```

A arquitetura deverá permanecer válida.

---

# 5. Princípios arquiteturais

A infraestrutura seguirá os seguintes princípios:

1. **Substrate Independence**
2. **Resource Abstraction**
3. **Hybrid-First**
4. **Local-First quando apropriado**
5. **Graceful Degradation**
6. **Failure Isolation**
7. **Progressive Isolation**
8. **Zero Trust**
9. **Least Privilege**
10. **Reproducibility**
11. **Infrastructure as Code**
12. **Provider Independence**
13. **Hardware Independence**
14. **Observability**
15. **Backup and Recovery**
16. **Offline Capability**
17. **Resource Governance**
18. **Data Minimization**
19. **Human Authority**
20. **Future Capability**
21. **Preserve Before Modify**

---

# 6. Arquitetura geral

```text
                              YUKI
                                │
                    ┌───────────┴───────────┐
                    │                       │
              Control Plane           Cognitive/Data Plane
                    │                       │
                    │                       │
                    └───────────┬───────────┘
                                │
                     Infrastructure API
                                │
                       Resource Manager
                                │
                       Resource Fabric
                                │
             ┌──────────────────┼──────────────────┐
             │                  │                  │
           EDGE               HOME               CLOUD
             │                  │                  │
             └──────────────────┼──────────────────┘
                                │
                         Domain Managers
                                │
                           Schedulers
                                │
                           Adapters
                                │
                      Concrete Infrastructure
```

---

# 7. Control Plane

O Control Plane coordena aspectos administrativos e de controle da Yuki.

Ele deverá incluir ou integrar:

```text
Identity
Authentication
Authorization
Policy
Security
Audit
Resource Management
Configuration
Health
Infrastructure State
```

O Control Plane é uma abstração arquitetural.

Ele não fica permanentemente vinculado a uma linguagem específica.

Uma implementação em Rust poderá ser utilizada, mas isso deverá ser tratado como decisão de implementação/ADR separado.

---

# 8. Cognitive/Data Plane

O Cognitive/Data Plane contém os componentes responsáveis por:

* raciocínio;
* memória;
* conhecimento;
* agentes;
* modelos;
* processamento;
* dados;
* workloads.

Ele não deve precisar conhecer detalhes físicos da infraestrutura.

Exemplo:

```text
Yuki Core
    ↓
Processing Router
    ↓
Workload
    ↓
Resource Manager
```

---

# 9. Execution Plane

O Execution Plane é responsável pela execução concreta.

Pode incluir:

```text
Native Runtime
Wasm Runtime
MicroVM
Container
VM
Remote Execution
Cloud Runtime
```

A escolha depende do workload e de sua classificação de segurança.

---

# 10. Resource Fabric

A infraestrutura será organizada conceitualmente como um Resource Fabric.

```text
Resource Fabric
│
├── Compute
├── Storage
├── Network
├── Runtime
├── Device
├── Accelerator
├── External Resources
└── Future Resources
```

O Fabric pode se expandir sem alterar o modelo fundamental.

---

# 11. Compute Resources

Compute Resources representam recursos utilizados para processamento.

Podem incluir:

```text
CPU
GPU
NPU
TPU
FPGA
DSP
AI Accelerator
Future Accelerator
```

A Yuki deverá trabalhar com propriedades abstratas.

Exemplo:

```text
Compute Resource
├── Tensor Operations
├── Precision Support
├── Memory Capacity
├── Memory Bandwidth
├── Performance Characteristics
├── Locality
├── Power Characteristics
└── Availability
```

---

# 12. Storage Resources

Storage Resources poderão incluir:

```text
RAM
NVMe
SSD
HDD
Object Storage
Distributed Storage
Database Storage
Archive
Future Storage
```

A arquitetura deverá diferenciar:

```text
Hot
Warm
Cold
Archive
```

conforme o uso e a política de retenção.

---

# 13. Network Resources

Network Resources representam conectividade.

Podem incluir:

```text
Local Network
Internet
Overlay Network
VPN
Cloud Network
Remote Link
Cellular
Satellite
Future Connectivity
```

Características relevantes:

```text
Latency
Bandwidth
Reliability
Availability
Location
Trust Domain
Cost
Encryption
```

---

# 14. Runtime Resources

Runtime Resources representam ambientes nos quais workloads podem ser executados.

Exemplos:

```text
Native
Container
Wasm
MicroVM
VM
Remote Runtime
```

O runtime escolhido dependerá do nível de isolamento e das necessidades do workload.

---

# 15. Device Resources

A infraestrutura poderá disponibilizar:

```text
Camera
Microphone
Speaker
Sensor
Display
Wearable
Robot
Actuator
Smart Home Device
Vehicle Interface
Future Device
```

Dispositivos são recursos.

Eles não são automaticamente confiáveis.

---

# 16. External Resources

Recursos externos poderão incluir:

```text
Cloud Compute
External APIs
Managed Services
Frontier AI Models
External Storage
Third-party Services
```

Esses recursos deverão ser tratados como dependências externas e submetidos às políticas de segurança, disponibilidade, custo e privacidade.

---

# 17. Future Resources

A arquitetura deverá aceitar tipos ainda desconhecidos.

Exemplo:

```text
Future Accelerator
Quantum Resource
Photonic Resource
Neuromorphic Resource
Future Memory Fabric
Future Network
```

A existência de um novo recurso não deverá exigir reescrever:

```text
Yuki Core
Agent System
Memory System
Model Router
Capability System
```

---

# 18. Workload Model

Workloads deverão declarar necessidades de infraestrutura.

O modelo é definido em conjunto com o ADR-006.

```text
WORKLOAD
│
├── Requirements
├── Constraints
├── Preferences
└── Fallbacks
```

Exemplo:

```text
Requirements:
    TensorOps

Constraints:
    Home Domain

Preferences:
    GPU
    Low Latency

Fallbacks:
    Local CPU
```

---

# 19. Infrastructure Selection

A seleção de infraestrutura seguirá conceitualmente:

```text
Workload
   ↓
Requirements
   ↓
Security / Policy Gate
   ↓
Hard Constraints
   ↓
Compatibility
   ↓
Availability
   ↓
Feasibility
   ↓
Optimization
   ↓
Allocation
```

Performance não pode compensar uma violação de segurança.

Custo não pode compensar uma violação de uma constraint obrigatória.

---

# 20. Processing Router

O Processing Router responde:

> **Que estratégia de processamento é necessária?**

Exemplo:

```text
Local Model
Cloud Model
GPU
CPU
Vision Model
Speech Model
Large Model
Small Model
```

Ele produz requisitos para a infraestrutura.

---

# 21. Resource Manager

O Resource Manager responde:

> **Quais recursos podem satisfazer esses requisitos?**

Responsabilidades:

* discovery;
* inventory;
* matching;
* allocation;
* release;
* resource state;
* health;
* resource availability;
* domain selection;
* integration with schedulers;
* resource accounting.

Ele não deve substituir o Processing Router.

---

# 22. Scheduler

O Scheduler responde:

> **Como e onde o workload será colocado dentro de determinado domínio?**

A Yuki poderá utilizar schedulers especializados.

Exemplos possíveis:

```text
Kubernetes
Nomad
Slurm
Ray
systemd
Cloud Scheduler
Future Scheduler
```

Nenhum deles é uma dependência arquitetural obrigatória.

---

# 23. Domain Manager

A infraestrutura poderá ser dividida em domínios.

```text
Yuki Resource Manager
        │
        ├── Edge Domain Manager
        ├── Home Domain Manager
        └── Cloud Domain Manager
```

Cada domínio pode possuir infraestrutura e scheduler próprios.

---

# 24. Não haverá um Global Super-Scheduler obrigatório

A Yuki não deverá assumir que um scheduler único precisa conhecer todos os recursos físicos.

Preferência arquitetural:

```text
Global Resource Management
        ↓
Domain Management
        ↓
Local Scheduling
```

Isso reduz acoplamento e permite diferentes tecnologias em cada domínio.

---

# 25. Edge Domain

O Edge Domain representa dispositivos próximos ao usuário.

Exemplos:

```text
Phone
Watch
Earbuds
Laptop
IoT
Sensors
Cameras
```

Funções possíveis:

* wake word;
* percepção;
* pré-processamento;
* interface;
* detecção de eventos;
* processamento de baixa latência;
* privacy filtering;
* operação offline parcial.

---

# 26. Home Domain

O Home Domain representa a infraestrutura local soberana da Yuki.

Pode fornecer:

```text
Compute
Storage
Memory
Network
Local Models
Vector Storage
Graph Storage
Event Processing
Home Automation
Local Security
```

Ele poderá funcionar como ponte entre:

```text
Edge
   ↕
Home
   ↕
Cloud
```

O Home Domain não é necessariamente um único servidor.

Ele pode ser um conjunto de recursos locais.

---

# 27. Cloud Domain

Cloud fornece elasticidade.

Pode ser utilizado para:

* modelos grandes;
* processamento pesado;
* workloads massivos;
* pesquisa;
* processamento paralelo;
* armazenamento externo;
* backup;
* serviços externos.

A cloud não deve ser considerada a única infraestrutura da Yuki.

---

# 28. Hybrid-First

A Yuki adotará uma arquitetura:

> **Hybrid-First**

A escolha entre Edge, Home e Cloud será dinâmica.

Critérios:

```text
Privacy
Latency
Cost
Capacity
Availability
Connectivity
Energy
Thermal
Security
Policy
Workload Requirements
```

---

# 29. Local-First

Local-first será uma preferência quando isso for apropriado.

Isso pode reduzir:

* latência;
* dependência de internet;
* exposição de dados;
* custo;
* dependência de fornecedores.

Porém:

> Local-first não significa local-only.

A cloud continua sendo uma extensão legítima da infraestrutura.

---

# 30. Offline Capability

A Yuki deverá possuir um modo de operação offline ou parcialmente offline.

Quando a conectividade falhar:

```text
Cloud unavailable
       ↓
Local resources
       ↓
Local models
       ↓
Local memory
       ↓
Persistent queues
       ↓
Deferred synchronization
```

Workloads que exigirem cloud poderão aguardar ou ser replanejados.

---

# 31. Graceful Degradation

Falhas de infraestrutura não devem necessariamente produzir falha total.

Exemplo:

```text
Home GPU
   ↓
Unavailable
   ↓
Local NPU
   ↓
Unavailable
   ↓
Local CPU
   ↓
Unavailable
   ↓
Cloud
```

Somente alternativas semanticamente válidas e autorizadas poderão ser utilizadas.

---

# 32. Failure Isolation

Uma falha em um recurso não deverá comprometer automaticamente todo o sistema.

Exemplo:

```text
Plugin compromised
      ≠
Yuki compromised
```

e:

```text
GPU failure
      ≠
Core failure
```

e:

```text
Cloud provider failure
      ≠
Yuki unavailable
```

---

# 33. Progressive Isolation

A infraestrutura utilizará diferentes níveis de isolamento.

Conceitualmente:

```text
Trusted Native
      ↓
Container
      ↓
Wasm
      ↓
MicroVM
      ↓
Stronger Isolation / Dedicated Environment
```

A escolha depende do risco e do workload.

---

# 34. Containers

Containers são úteis para:

* serviços;
* infraestrutura;
* aplicações conhecidas;
* deployment;
* empacotamento.

Entretanto:

> Containers compartilham o kernel do host e não devem ser considerados automaticamente suficientes para executar código arbitrário não confiável.

---

# 35. WebAssembly

Wasm poderá ser utilizado como runtime para:

* plugins;
* capabilities;
* workloads leves;
* componentes portáveis;
* execução com menor superfície de acesso.

A arquitetura não assumirá que Wasm fornece segurança absoluta.

Wasm pode reduzir determinadas classes de problemas, mas:

> **Nenhum sandbox deve ser tratado como garantia absoluta de segurança.**

---

# 36. MicroVM

MicroVM poderá ser utilizada quando houver necessidade de isolamento mais forte para workloads arbitrários.

Exemplos:

```text
Untrusted Code
Arbitrary Binary
Compiler
Native Dependency
Unknown Tool
Risky Workload
```

A tecnologia específica de MicroVM será decidida posteriormente.

---

# 37. Native Execution

Native execution continuará existindo.

Não é necessário colocar absolutamente tudo dentro de sandbox.

Componentes altamente confiáveis e controlados poderão utilizar execução nativa quando apropriado.

A decisão deverá considerar:

```text
Trust
Risk
Performance
Isolation
Compatibility
Resource Cost
```

---

# 38. Compute Abstraction

A Yuki deverá possuir uma abstração de compute.

Conceitualmente:

```text
Yuki
 ↓
Compute Interface
 ↓
Compute Adapter
 ↓
CPU / GPU / NPU / FPGA / Future
```

O objetivo é evitar dependência direta do hardware.

Tecnologias como:

```text
ONNX Runtime
vLLM
llama.cpp
TVM
Vulkan Compute
Triton
```

podem ser utilizadas como implementações/adapters, mas não são parte da constituição da arquitetura.

---

# 39. Storage Architecture

A arquitetura de storage deverá suportar diferentes classes.

```text
Hot
 ↓
Warm
 ↓
Cold
 ↓
Archive
```

Exemplo:

```text
Hot:
Working Memory / Cache

Warm:
Database / Vector / Graph

Cold:
Object Storage

Archive:
Long-term Storage / Backup
```

A implementação concreta poderá mudar.

---

# 40. Database Independence

O Core não deverá depender diretamente de um banco específico.

Deverão existir interfaces de acesso:

```text
Repository
Storage Interface
Vector Store Interface
Graph Store Interface
Object Store Interface
```

Isso permite substituir:

```text
PostgreSQL
pgvector
Qdrant
Milvus
Object Storage
Future Database
```

sem alterar a arquitetura superior.

---

# 41. Network Architecture

A rede da Yuki será baseada em:

```text
Encryption
Identity
Authentication
Authorization
Segmentation
Audit
```

Esses conceitos devem permanecer separados.

---

# 42. Network Encryption

Uma tecnologia de transporte criptografado poderá ser utilizada.

Exemplo candidato:

```text
WireGuard
```

Sua função é:

> proteger a comunicação de rede.

Não é, por si só, a identidade completa da workload.

---

# 43. Network Coordination

Uma camada de coordenação poderá facilitar:

* descoberta;
* conectividade;
* routing;
* acesso entre domínios.

Exemplos candidatos:

```text
Tailscale
Headscale
Equivalent Future Systems
```

---

# 44. Workload Identity

A identidade de workloads deve ser tratada separadamente.

Tecnologias candidatas:

```text
SPIFFE
SPIRE
```

Conceitualmente:

```text
Network Encryption
        ↓
WireGuard / equivalent

Workload Identity
        ↓
SPIFFE / equivalent

Application Authentication
        ↓
mTLS / Application Protocol
```

Nenhuma dessas tecnologias deverá ser tratada como obrigação constitucional antes de ADR específico.

---

# 45. Zero Trust

A infraestrutura seguirá o princípio:

> **Never trust by default. Verify explicitly.**

Isso significa que:

```text
Local
≠
Trusted
```

e:

```text
Home
≠
Automatically Authorized
```

e:

```text
Known Device
≠
Unlimited Permission
```

---

# 46. Resource Trust

Recursos deverão possuir sinais de confiança.

Exemplos:

```text
Identity
Attestation
Integrity
Health
Security Properties
Provenance
```

Mas:

```text
Trust
≠
Authorization
```

O Security Controller permanece responsável pela autorização.

---

# 47. Secrets

Secrets deverão ser separados de configuração normal.

Exemplos:

```text
API Keys
Credentials
Encryption Keys
Tokens
Certificates
Recovery Keys
Provider Credentials
```

Secrets não devem ser:

* hardcoded;
* expostos em logs;
* enviados desnecessariamente para workloads;
* armazenados sem proteção apropriada.

---

# 48. Data Minimization

Um workload deve receber apenas os dados necessários.

Exemplo:

```text
Task
 ↓
Required Data
 ↓
Resource
```

não:

```text
Task
 ↓
Entire Yuki Memory
 ↓
Resource
```

Essa regra vale especialmente para cloud e terceiros.

---

# 49. Cloud Data Governance

Antes de enviar dados para cloud, a Yuki deverá considerar:

```text
Data Sensitivity
Privacy
Policy
Security
Provider Trust
Encryption
Location
Retention
Cost
```

Quando apropriado, poderão ser utilizados:

* processamento local;
* anonimização;
* minimização;
* cifragem;
* confidential computing;
* dados derivados em vez de dados originais.

Confidential Computing é uma possibilidade arquitetural para workloads elegíveis, não uma obrigação universal.

---

# 50. Resource Security

Segurança deve existir em profundidade.

```text
Identity
 ↓
Policy
 ↓
Resource Selection
 ↓
Isolation
 ↓
Runtime
 ↓
Network
 ↓
Execution
 ↓
Verification
 ↓
Audit
```

Nenhuma camada deve ser considerada suficiente sozinha.

---

# 51. Resource Quotas

A infraestrutura deverá suportar limites.

Exemplos:

```text
CPU
GPU
Memory
Storage
Network
Concurrency
Cloud Cost
Energy
Execution Time
```

Os limites poderão existir em vários níveis.

---

# 52. Cost Awareness

A Resource Manager deverá poder considerar custo.

Exemplo:

```text
Local GPU
Cost ≈ fixed infrastructure

Cloud GPU
Cost ≈ variable
```

A escolha poderá considerar:

```text
Cost
Performance
Latency
Privacy
Availability
```

Mas custo nunca deve violar constraints obrigatórias.

---

# 53. Energy Awareness

A infraestrutura poderá considerar energia como recurso ou atributo.

Exemplos:

```text
Power Consumption
Energy Budget
Battery Level
Thermal State
```

Isso será especialmente relevante para:

* mobile;
* wearable;
* home server;
* edge devices;
* long-running workloads.

---

# 54. Thermal Awareness

A Yuki poderá monitorar:

```text
CPU Temperature
GPU Temperature
Device Temperature
Thermal Throttling
Cooling Capacity
```

e adaptar workloads quando apropriado.

Exemplo:

```text
Thermal Pressure
      ↓
Reduce Load
      ↓
Move Workload
      ↓
Cloud / Other Resource
```

---

# 55. Resource Accounting

A infraestrutura deverá permitir contabilizar:

```text
Compute Time
Memory
Storage
Network
Energy
Cloud Cost
Resource Allocation
```

Esses dados poderão alimentar:

* planejamento;
* otimização;
* auditoria;
* evolução;
* orçamento.

---

# 56. Observability

Toda infraestrutura importante deverá ser observável.

A arquitetura deverá permitir observar:

```text
Health
Availability
Latency
Utilization
Errors
Resource Allocation
Failures
Cost
Energy
Thermal
Network
Security Events
```

Observabilidade deverá alimentar o Event System e o Evolution Manager.

---

# 57. Health Model

Recursos deverão possuir estados de saúde independentes da alocação.

```text
HEALTHY
DEGRADED
FAILED
QUARANTINED
```

Um recurso degradado pode continuar alocado.

Um recurso saudável pode estar indisponível por estar totalmente alocado.

---

# 58. Resource Lifecycle

O ciclo de vida conceitual:

```text
DISCOVERED
    ↓
REGISTERED
    ↓
AVAILABLE
    ↓
ALLOCATED
    ↓
RELEASED
    ↓
RETIRED
```

Health e allocation permanecem dimensões separadas.

---

# 59. Quarantine

Recursos suspeitos poderão ser isolados.

```text
Resource
 ↓
Anomaly
 ↓
Quarantine
 ↓
Investigation
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

---

# 60. Infrastructure as Code

A infraestrutura deverá ser reproduzível através de configuração declarativa quando apropriado.

Conceitualmente:

```text
Source
+
Declarative Infrastructure
+
Configuration
+
Secrets Recovery
+
Data Backup
=
Reproducible Infrastructure
```

Tecnologias como OpenTofu, Pulumi ou equivalentes poderão ser utilizadas.

Nenhuma ferramenta específica é constitucional neste documento.

---

# 61. Phoenix Bootstrap

A Yuki deverá possuir uma estratégia de reconstrução.

Conceito:

```text
Phoenix Bootstrap
```

Objetivo:

> permitir reconstruir a infraestrutura da Yuki após perda catastrófica.

Fluxo:

```text
Recovery Environment
        ↓
Source
        ↓
Infrastructure Definition
        ↓
Base Infrastructure
        ↓
Security
        ↓
Storage
        ↓
Database
        ↓
Yuki Core
        ↓
Memory / Knowledge Restore
        ↓
Validation
        ↓
Production
```

---

# 62. Backup

A estratégia deverá considerar:

```text
3-2-1
```

com:

* múltiplas cópias;
* diferentes meios;
* cópia offsite;
* criptografia;
* testes de restauração.

Backup sem teste de restore não deve ser considerado confiável.

---

# 63. Disaster Recovery

O sistema deverá possuir planos para:

```text
Disk Failure
Server Failure
GPU Failure
Network Failure
Cloud Failure
Provider Failure
Database Corruption
Security Incident
Configuration Corruption
Complete Infrastructure Loss
```

A prioridade será:

```text
Preserve
Contain
Recover
Verify
Resume
```

---

# 64. Provider Independence

A Yuki não deverá depender estruturalmente de uma única cloud.

Arquitetura:

```text
Yuki
 ↓
Cloud Resource Abstraction
 ↓
Cloud Adapter
 ↓
Provider
```

Isso permite substituir fornecedores.

---

# 65. External Provider Failure

Se um provider falhar:

```text
Provider A
   ↓
Failure
   ↓
Health Detection
   ↓
Alternative Provider
```

quando existir alternativa compatível.

---

# 66. Home Failure

Se o Home Domain falhar:

```text
Home unavailable
      ↓
Edge capabilities
      ↓
Cloud resources
      ↓
Degraded Operation
```

quando permitido pelas políticas e requisitos.

---

# 67. Cloud Failure

Se a cloud falhar:

```text
Cloud unavailable
      ↓
Home
      ↓
Edge
      ↓
Offline Mode
```

quando os recursos locais forem suficientes.

---

# 68. Resource Migration

Workloads poderão ser migrados quando apropriado.

Exemplo:

```text
Home GPU
   ↓
Overload
   ↓
Cloud GPU
```

ou:

```text
Cloud
   ↓
Network failure
   ↓
Home
```

A migração deverá respeitar:

* segurança;
* estado;
* compatibilidade;
* dados;
* custo;
* política;
* checkpoint;
* disponibilidade.

---

# 69. Durable Execution

Workloads importantes deverão poder sobreviver a interrupções.

Isso requer suporte conceitual para:

```text
Checkpoint
Retry
Resume
Pause
Cancel
Timeout
TTL
Recovery
```

A implementação concreta será definida pelo sistema de Agents/Tasks e infraestrutura.

---

# 70. Infrastructure Events

Infraestrutura deverá gerar eventos.

Exemplos:

```text
ResourceDiscovered
ResourceAvailable
ResourceAllocated
ResourceReleased
ResourceFailed
ResourceDegraded
ResourceQuarantined
ResourceRecovered
NetworkChanged
ProviderUnavailable
ThermalAlert
BudgetExceeded
```

Esses eventos serão integrados ao `13_EVENTS_AND_BACKGROUND.md`.

---

# 71. Backpressure

A infraestrutura deverá impedir que uma avalanche de workloads derrube o sistema.

Mecanismos possíveis:

```text
Queue Limits
Rate Limits
Concurrency Limits
Priority
Backpressure
Load Shedding
Resource Quotas
```

---

# 72. Resource Priority

Recursos e workloads poderão possuir prioridades.

Exemplo:

```text
CRITICAL
HIGH
NORMAL
LOW
BACKGROUND
```

Mas prioridade não substitui segurança.

Um workload crítico não recebe autorização apenas por ser crítico.

---

# 73. Critical Infrastructure

Alguns recursos deverão possuir proteção especial.

Exemplos:

```text
Security Controller
Control Plane
Core Storage
Identity Infrastructure
Audit Infrastructure
Recovery Infrastructure
Event Backbone
```

Esses recursos deverão possuir reservas e mecanismos de proteção contra starvation.

---

# 74. Infrastructure Self-Management

A Yuki poderá monitorar e administrar infraestrutura automaticamente.

Modelo:

```text
Observe
   ↓
Evaluate
   ↓
Plan
   ↓
Act
   ↓
Verify
```

Exemplos:

* mover workloads;
* reiniciar serviços;
* liberar recursos;
* alterar placement;
* escalar;
* colocar recurso em quarentena.

---

# 75. Limites da autonomia

A autonomia de infraestrutura deverá parar antes de:

```text
Identity Changes
Security Key Changes
Authorization Changes
Critical Policy Changes
Irreversible Destruction
Unbounded Spending
Security Boundary Changes
```

Ações críticas exigirão aprovação apropriada.

---

# 76. Infrastructure Evolution

O Evolution Manager poderá propor:

```text
Hardware Upgrade
Runtime Change
Storage Migration
Network Upgrade
Scheduler Change
Provider Change
Architecture Improvement
```

Fluxo:

```text
Observe
 ↓
Identify
 ↓
Research
 ↓
Propose
 ↓
Prototype
 ↓
Test
 ↓
Security Review
 ↓
Approve
 ↓
Deploy
 ↓
Monitor
 ↓
Rollback if necessary
```

---

# 77. Preserve Before Modify

Antes de alterações importantes:

```text
Backup
Snapshot
Version
Document
Validate Recovery
```

A mudança deve ser reversível sempre que possível.

---

# 78. Development Infrastructure

A infraestrutura deverá possuir separação entre:

```text
Development
Testing
Staging
Production
```

quando a escala justificar.

Mudanças experimentais não deverão afetar diretamente a produção.

---

# 79. Yuki Development Lab

O Development Lab será um ambiente controlado para:

* prototipagem;
* testes;
* benchmarks;
* novos adapters;
* novos runtimes;
* novos modelos;
* experimentos de infraestrutura;
* security testing.

Fluxo:

```text
Experiment
 ↓
Prototype
 ↓
Test
 ↓
Benchmark
 ↓
Security Evaluation
 ↓
Approval
 ↓
Production
```

---

# 80. Supply Chain Security

A infraestrutura deverá considerar:

```text
Dependencies
Packages
Images
Binaries
Drivers
Firmware
Adapters
Plugins
Infrastructure Modules
```

Deverão ser consideradas:

* provenance;
* integrity;
* versioning;
* vulnerability scanning;
* signatures;
* dependency monitoring.

---

# 81. Infrastructure Compatibility

Recursos e adapters deverão possuir versões.

```text
Resource Contract
Adapter Version
Runtime Version
Schema Version
Infrastructure Version
```

Mudanças incompatíveis deverão ser detectadas antes da execução.

---

# 82. Graceful Degradation of Infrastructure

A Yuki deverá conseguir operar em diferentes níveis.

```text
FULL
 ↓
REDUCED
 ↓
DEGRADED
 ↓
OFFLINE
 ↓
RECOVERY
```

O sistema deve continuar fornecendo o máximo de funcionalidade possível dentro das limitações existentes.

---

# 83. Minimal Initial Infrastructure

A primeira implementação não deverá tentar construir toda a infraestrutura futura.

Uma implantação inicial poderá possuir:

```text
1 Primary Compute Domain
1 Storage System
1 Network Layer
1 Execution Layer
1 Resource Manager
1 Monitoring Layer
1 Backup Strategy
```

e crescer posteriormente.

---

# 84. Escalabilidade

A infraestrutura deverá poder crescer:

```text
Single Machine
      ↓
Multiple Machines
      ↓
Home Cluster
      ↓
Home + Cloud
      ↓
Multi-Cloud
      ↓
Distributed Fabric
```

A arquitetura deve continuar válida em cada estágio.

---

# 85. Não usar complexidade sem necessidade

A Yuki não deverá adotar tecnologias distribuídas apenas porque são tecnicamente interessantes.

Exemplo:

```text
Small Deployment
→ Simple Runtime

Larger Deployment
→ Specialized Orchestration

Very Large Deployment
→ Distributed Scheduler / Cluster
```

A infraestrutura deve crescer junto com a necessidade.

---

# 86. Tecnologias candidatas

As seguintes tecnologias foram identificadas como candidatas durante a pesquisa:

### Compute

```text
ONNX Runtime
vLLM
llama.cpp
TVM
Vulkan Compute
Triton
```

### Runtime / Isolation

```text
Wasm / WASI
Wasmtime
Firecracker
OCI Containers
Podman
containerd
```

### Messaging

```text
NATS
JetStream
Equivalent Future Systems
```

### Storage

```text
PostgreSQL
pgvector
Qdrant
Milvus
S3-compatible Storage
MinIO
Garage
```

### Network

```text
WireGuard
Tailscale
Headscale
SPIFFE
SPIRE
```

### Infrastructure as Code

```text
OpenTofu
Pulumi
Equivalent Systems
```

### Observability

```text
OpenTelemetry
Equivalent Systems
```

Essas tecnologias são **candidatas de implementação**.

Elas não constituem dependências obrigatórias desta arquitetura.

---

# 87. Kubernetes

Kubernetes não será considerado requisito inicial da Yuki.

Pode ser utilizado futuramente caso:

* a escala justifique;
* a complexidade seja compensada;
* os requisitos de scheduling necessitem;
* a operação se beneficie dele.

A arquitetura não depende dele.

---

# 88. Containers

Containers poderão ser utilizados para deployment de infraestrutura.

Eles não serão tratados como único mecanismo de isolamento.

---

# 89. Rust

Rust é uma possível implementação para componentes críticos do Control Plane.

Entretanto:

> **Rust não é requisito arquitetural.**

A interface do Control Plane deve ser definida independentemente da linguagem.

---

# 90. NATS

NATS/JetStream é candidato para infraestrutura de eventos e messaging.

Não deve ser confundido com:

```text
Object Storage
Database
Resource Model
```

Sua função potencial é:

```text
Messaging
Eventing
Streaming
Durable Event Handling
```

---

# 91. Storage e Messaging são conceitos distintos

A arquitetura mantém:

```text
Object Storage
        ≠
Messaging
        ≠
Database
        ≠
Cache
```

Exemplo:

```text
S3-compatible
→ Object Storage

NATS
→ Messaging/Eventing

PostgreSQL
→ Relational Database
```

---

# 92. Infrastructure API

A Yuki deverá possuir uma API conceitual para infraestrutura.

Exemplo:

```text
discover_resources()
get_resource()
query_resources()
allocate_resource()
release_resource()
get_health()
get_capacity()
get_telemetry()
```

Os contratos concretos serão definidos posteriormente.

---

# 93. Resource Adapter API

Adapters deverão fornecer uma interface consistente.

Conceitualmente:

```text
Adapter
├── Discover
├── Describe
├── Validate
├── Allocate
├── Execute
├── Monitor
├── Release
└── Recover
```

Nem todo adapter necessariamente implementará todas as operações.

---

# 94. Infrastructure Health API

A infraestrutura deverá fornecer informações sobre:

```text
Health
Capacity
Availability
Utilization
Temperature
Power
Network
Failures
```

Isso alimentará:

```text
Resource Manager
Event System
Monitoring
Evolution Manager
Security
```

---

# 95. Resource Observability

Cada recurso importante deverá possuir identidade observável.

Exemplo:

```text
Resource ID
Domain
Health
Capacity
Current Load
Allocation
Events
Failures
History
```

Isso permitirá auditoria e troubleshooting.

---

# 96. Reproducibility

A infraestrutura deverá ser reconstruível a partir de:

```text
Source
Configuration
Infrastructure Definitions
Versioned Contracts
Backup
Recovery Material
```

A infraestrutura não deve depender de configurações manuais impossíveis de reproduzir.

---

# 97. Configuration Management

Configurações deverão ser:

* versionadas;
* auditáveis;
* separadas de secrets;
* reproduzíveis;
* validadas.

Mudanças críticas deverão ser registradas.

---

# 98. Infrastructure Audit

Eventos importantes deverão gerar registros auditáveis.

Exemplos:

```text
Resource Added
Resource Removed
Allocation
Deallocation
Migration
Quarantine
Policy Change
Infrastructure Change
Provider Change
Recovery
```

O audit system deverá possuir proteção contra adulteração.

---

# 99. Infrastructure and Human Agency

A infraestrutura deve servir à Yuki e, consequentemente, ao usuário.

Ela não deverá tomar decisões irreversíveis ou críticas apenas porque tecnicamente consegue fazê-lo.

Princípio:

> **Capacidade técnica não implica autoridade.**

---

# 100. Future Hardware Principle

A arquitetura deve assumir que os recursos atuais não são definitivos.

O modelo deverá permitir:

```text
Current Hardware
       ↓
New Hardware
       ↓
Unknown Hardware
       ↓
Future Computational Paradigm
```

sem alterar os contratos superiores.

---

# 101. Future Infrastructure Principle

A arquitetura não deve limitar a infraestrutura futura às tecnologias atualmente conhecidas.

Novas tecnologias poderão ser incorporadas através de:

```text
Resource Definition
+
Adapter
+
Compatibility
+
Security
+
Validation
```

---

# 102. Infrastructure Boundaries

A arquitetura estabelece os seguintes limites:

```text
Yuki Core
→ decides / orchestrates

Processing Router
→ decides processing strategy

Resource Manager
→ manages abstract resources

Scheduler
→ performs placement

Execution Engine
→ executes workload

Adapter
→ translates abstraction

Security Controller
→ authorizes / protects

Evolution Manager
→ evolves infrastructure
```

Nenhum desses componentes deverá absorver completamente as responsabilidades dos demais.

---

# 103. Segurança como camada transversal

Security não é apenas um serviço de infraestrutura.

Ela atravessa:

```text
Identity
Resource Discovery
Resource Selection
Allocation
Network
Runtime
Execution
Storage
Backup
Recovery
Evolution
Audit
```

A arquitetura deve manter o Security Controller independente.

---

# 104. Trust Boundaries

Principais boundaries:

```text
User
 ↓
Access Layer

Access Layer
 ↓
Yuki Core

Yuki Core
 ↓
Control Plane

Control Plane
 ↓
Execution Plane

Execution Plane
 ↓
Sandbox / Runtime

Runtime
 ↓
External System
```

Cada boundary deve ser explicitamente tratada.

---

# 105. Assume Breach

A infraestrutura deve assumir que:

* um plugin pode ser comprometido;
* um workload pode ser malicioso;
* uma dependência pode estar comprometida;
* uma máquina pode falhar;
* uma credencial pode vazar;
* um provider pode falhar;
* um recurso pode mentir sobre suas propriedades;
* um componente pode ser explorado.

A arquitetura deve limitar o impacto.

---

# 106. Regra de isolamento

> **Nenhum componente individual deve possuir poder suficiente para comprometer toda a Yuki.**

Essa regra vem diretamente da arquitetura de segurança.

---

# 107. Resource Fabric como sistema vivo

O Resource Fabric deverá ser dinâmico.

Recursos poderão:

```text
Appear
Disappear
Change
Fail
Recover
Scale
Migrate
Quarantine
Retire
```

A Yuki deverá reagir a essas mudanças através do Event System.

---

# 108. Resource Lifecycle integrado

Fluxo:

```text
Discovery
 ↓
Registration
 ↓
Trust
 ↓
Validation
 ↓
Available
 ↓
Allocation
 ↓
Execution
 ↓
Monitoring
 ↓
Release
 ↓
Retirement
```

Falhas podem desviar para:

```text
Degraded
Quarantine
Recovery
Retirement
```

---

# 109. Resource Fabric e Events

```text
Resource Manager
      ↓
Resource Event
      ↓
Event Router
      ↓
Attention Manager
      ↓
Relevant Consumer
```

Nem todo evento precisa chegar ao Yuki Core.

Eventos de baixa relevância podem ser processados localmente.

---

# 110. Resource Fabric e Evolution

O Evolution Manager poderá analisar:

```text
Resource Utilization
Failures
Costs
Thermals
Capacity
Performance
Provider Dependence
```

e identificar oportunidades de evolução.

---

# 111. Resource Fabric e Model Router

O Model Router determina necessidades de processamento.

```text
Model Router
      ↓
Workload Requirements
      ↓
Resource Manager
      ↓
Compute Resource
```

O Resource Manager não escolhe semanticamente o modelo.

---

# 112. Resource Fabric e Agent System

Agents geram workloads.

```text
Agent
 ↓
Task
 ↓
Workload
 ↓
Resource Manager
 ↓
Execution
```

Isso permite que agentes sejam independentes da infraestrutura física.

---

# 113. Resource Fabric e Capability System

Capabilities representam o que a Yuki consegue fazer.

Resources fornecem meios para executar essas capabilities.

```text
Capability
    ↓
Execution Need
    ↓
Workload
    ↓
Resource
```

---

# 114. Resource Fabric e Memory

Memory poderá utilizar diferentes recursos:

```text
RAM
Database
Vector Storage
Object Storage
Archive
```

Mas Memory continua sendo uma abstração própria.

A existência de um storage resource não define automaticamente a semântica da memória da Yuki.

---

# 115. Resource Fabric e Security Controller

```text
Resource Manager
      │
      ├── Resource Discovery
      ├── Resource Matching
      └── Allocation
               │
               ▼
       Security Controller
               │
         Authorization
               │
               ▼
           Execution
```

A implementação poderá organizar a ordem de verificações de maneira diferente, mas a autoridade de segurança não deverá ser absorvida pelo Resource Manager.

---

# 116. Resource Fabric e Access Layer

Dispositivos são portas de acesso.

```text
Phone
Watch
Earbuds
Notebook
Home Interface
```

não são Yuki separadas.

Eles acessam o mesmo sistema através de:

```text
Yuki Access
```

A infraestrutura fornece os recursos necessários para essa continuidade.

---

# 117. Dynamic Infrastructure

A Yuki deverá poder mudar a infraestrutura disponível sem exigir mudança estrutural no Core.

Exemplo:

```text
Morning
→ Home GPU

Travel
→ Phone NPU

Deep Research
→ Cloud GPU

Offline
→ Local CPU/NPU
```

A decisão é contextual e governada.

---

# 118. Resource Placement

Placement deverá considerar:

```text
Where
What
How
Why
```

Mas:

```text
Location
≠
Permission
```

Um recurso em casa não é automaticamente permitido.

Um recurso em outro país não é automaticamente proibido.

A decisão depende de contexto, política, identidade, finalidade, risco e autorização.

---

# 119. Resource Migration and Privacy

Mover um workload também pode significar mover dados.

Portanto:

```text
Migration
→ Data Transfer
→ Privacy Check
→ Security Check
→ Policy Check
```

Uma migração tecnicamente possível pode ser proibida pela política.

---

# 120. Infrastructure and Cost Control

Workloads poderão possuir budgets.

Exemplo:

```text
Maximum Cost
Maximum Cloud Time
Maximum GPU Time
Maximum Data Transfer
```

A infraestrutura deve interromper ou pedir autorização quando limites forem atingidos, conforme a política.

---

# 121. Infrastructure and Energy

Para workloads não urgentes:

```text
Low Energy Period
      ↓
Background Work
```

Para dispositivos móveis:

```text
Low Battery
      ↓
Reduce Local Compute
      ↓
Defer / Offload
```

A implementação será adaptativa.

---

# 122. Infrastructure and Thermal Control

Quando houver pressão térmica:

```text
Thermal Alert
      ↓
Evaluate Workload
      ↓
Reduce / Pause / Migrate
      ↓
Verify
```

Isso pode ocorrer especialmente em:

* smartphones;
* notebooks;
* home servers;
* edge devices.

---

# 123. Infrastructure and Availability

Availability poderá ser expressa por:

```text
Resource Availability
Domain Availability
Provider Availability
Service Availability
Network Availability
```

A Yuki não deverá confundir:

```text
Available
```

com:

```text
Authorized
```

---

# 124. Infrastructure and Reliability

Workloads importantes poderão utilizar:

```text
Redundancy
Replication
Checkpoint
Failover
Retry
Multi-domain execution
```

conforme seus requisitos.

---

# 125. Infrastructure and Idempotency

Operações de infraestrutura deverão ser idempotentes quando possível.

Exemplo:

```text
Allocate
Release
Restart
Recover
Deploy
```

Isso reduz riscos de:

* retries;
* duplicação;
* inconsistência;
* falhas de rede.

---

# 126. Infrastructure Failure Modes

A arquitetura deverá considerar pelo menos:

```text
Hardware Failure
Software Failure
Network Failure
Storage Failure
Database Failure
Provider Failure
Configuration Failure
Dependency Failure
Security Incident
Resource Exhaustion
Thermal Failure
Power Failure
Human Error
```

---

# 127. Recovery Philosophy

A recuperação deverá seguir:

```text
Detect
 ↓
Contain
 ↓
Preserve
 ↓
Recover
 ↓
Validate
 ↓
Resume
 ↓
Learn
```

O último estágio alimenta o Evolution Manager.

---

# 128. Infrastructure Complexity Budget

Cada nova camada deverá justificar sua complexidade.

Antes de adicionar:

```text
Cluster
Scheduler
Distributed Database
Service Mesh
New Runtime
New Messaging Layer
```

deve existir uma necessidade real.

---

# 129. Architecture vs Deployment

Este documento define arquitetura.

Não define que a implantação inicial deverá possuir:

```text
N servers
N GPUs
Kubernetes
Specific Cloud
Specific Database
Specific OS
```

A topologia concreta será determinada conforme o estágio do projeto.

---

# 130. Initial Deployment Philosophy

A implementação inicial deverá priorizar:

```text
Simplicity
Reliability
Observability
Security
Reproducibility
Extensibility
```

em vez de complexidade prematura.

---

# 131. Future Deployment

Conforme a Yuki evoluir:

```text
Single Host
 ↓
Multi-Service Host
 ↓
Multi-Node
 ↓
Home Fabric
 ↓
Home + Cloud
 ↓
Multi-Domain Fabric
```

A arquitetura permanece.

A implementação cresce.

---

# 132. Architecture Decision Summary

Esta arquitetura estabelece:

```text
Resource Abstraction
        ↓
Resource Fabric
        ↓
Edge / Home / Cloud
        ↓
Domain Managers
        ↓
Schedulers
        ↓
Execution Runtimes
        ↓
Adapters
        ↓
Concrete Infrastructure
```

---

# 133. Decisões fechadas

As seguintes decisões são consideradas oficiais:

### D1

A Yuki utilizará um Infrastructure Resource Model próprio.

### D2

Typed Abstract Resource será a unidade fundamental de representação de recursos.

### D3

A infraestrutura será independente de hardware específico.

### D4

A infraestrutura será independente de cloud provider específico.

### D5

A arquitetura será Hybrid-First.

### D6

Local-first será utilizado quando apropriado.

### D7

Edge, Home e Cloud serão domínios de infraestrutura.

### D8

Resource Manager será separado do Processing Router.

### D9

Resource Manager será separado do Scheduler.

### D10

Scheduler será separado do Execution Engine.

### D11

Execution Engine será separado dos Adapters.

### D12

Security Controller permanecerá independente.

### D13

A infraestrutura deverá possuir mecanismos de graceful degradation.

### D14

A infraestrutura deverá possuir failure isolation.

### D15

A infraestrutura deverá suportar operação offline ou parcialmente offline.

### D16

A infraestrutura deverá possuir observabilidade.

### D17

A infraestrutura deverá possuir backup e recuperação.

### D18

A infraestrutura deverá ser progressivamente isolada conforme o risco.

### D19

A arquitetura deverá suportar recursos futuros desconhecidos.

### D20

A infraestrutura deverá buscar reprodutibilidade.

### D21

A infraestrutura deverá evitar dependência estrutural de tecnologias específicas.

### D22

Hard constraints deverão ser satisfeitas antes de otimização.

---

# 134. Decisões propositalmente não fechadas

Continuam abertas:

```text
Programming Language
Database
Message Bus
Scheduler
Container Runtime
Wasm Runtime
MicroVM Technology
Cloud Provider
Infrastructure-as-Code Tool
Observability Stack
Network Coordination
Workload Identity Implementation
Storage Backend
Vector Database
Graph Database
Compute Runtime
GPU Strategy
Cluster Strategy
Federation Protocol
```

Essas decisões deverão ser tratadas individualmente quando necessário.

---

# 135. ADRs futuros relacionados

Possíveis próximos ADRs:

```text
ADR-007 — Resource Contract
ADR-008 — Resource Discovery
ADR-009 — Resource Scheduling
ADR-010 — Resource Federation
ADR-011 — Resource Attestation
ADR-012 — Infrastructure Runtime Strategy
ADR-013 — Storage Architecture
ADR-014 — Infrastructure Networking
ADR-015 — Infrastructure Observability
ADR-016 — Disaster Recovery
```

A numeração deverá ser ajustada ao roadmap real do repositório.

---

# 136. Relação com documentação existente

Este documento depende conceitualmente de:

```text
07_AGENTS_AND_TASKS.md
08_CAPABILITY_SYSTEM.md
09_MODEL_ROUTER.md
10_SECURITY.md
11_EVOLUTION.md
12_VOICE_AND_MULTIMODAL.md
13_EVENTS_AND_BACKGROUND.md
ADR-006
```

Especialmente:

```text
ADR-006
    ↓
Infrastructure Resource Model
    ↓
14_INFRASTRUCTURE.md
```

---

# 137. Modelo arquitetural final

```text
                                  USER
                                    │
                                    ▼
                               YUKI ACCESS
                                    │
                                    ▼
                                YUKI CORE
                                    │
                         ┌──────────┴──────────┐
                         │                     │
                  Processing Router       Agent System
                         │                     │
                         └──────────┬──────────┘
                                    ▼
                                WORKLOAD
                                    │
                    Requirements / Constraints /
                       Preferences / Fallbacks
                                    │
                                    ▼
                           SECURITY / POLICY
                                    │
                                    ▼
                           RESOURCE MANAGER
                                    │
                     ┌──────────────┼──────────────┐
                     │              │              │
                    EDGE           HOME           CLOUD
                     │              │              │
                     └──────────────┼──────────────┘
                                    │
                           DOMAIN MANAGERS
                                    │
                               SCHEDULERS
                                    │
                            EXECUTION ENGINE
                                    │
                              ADAPTER LAYER
                                    │
                                    ▼
                         CONCRETE RESOURCES
                                    │
       ┌──────────────┬────────────┼────────────┬──────────────┐
       ▼              ▼            ▼            ▼              ▼
    Compute        Storage       Network      Runtime        Devices
       │              │            │            │              │
       └──────────────┴────────────┴────────────┴──────────────┘
                                    │
                                    ▼
                             PHYSICAL / REMOTE
                              INFRASTRUCTURE
```

---

# 138. Princípio final

A infraestrutura da Yuki deve ser poderosa o suficiente para acompanhar sua evolução, mas abstrata o suficiente para não aprisioná-la ao presente.

A Yuki não deve ser construída para um servidor.

Não deve ser construída para uma GPU.

Não deve ser construída para uma cloud.

Não deve ser construída para um runtime.

Não deve ser construída para uma geração específica de computadores.

Ela deve ser construída para utilizar **recursos computacionais presentes e futuros através de contratos, abstrações, adapters, isolamento, governança e observabilidade**.

> **Infrastructure is a resource fabric, not a machine.**

E:

> **O substrato pode mudar. Os contratos da Yuki permanecem.**

---

# 139. Status

**ACCEPTED — v1.0**

Este documento estabelece a arquitetura oficial de infraestrutura da Yuki.

Decisões tecnológicas específicas deverão ser tomadas por ADRs próprios quando atingirem relevância arquitetural suficiente.
