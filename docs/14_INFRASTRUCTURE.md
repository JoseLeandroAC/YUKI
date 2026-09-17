Yuki Infrastructure

Documento: "docs/14_INFRASTRUCTURE.md"
Versão: "0.1"
Status: Architectural Direction Approved / Technology Decisions Pending
Projeto: Yuki
Categoria: Infrastructure Architecture

---

1. Objetivo

Este documento define a arquitetura de infraestrutura da Yuki.

A infraestrutura da Yuki não deve ser entendida simplesmente como um conjunto de servidores, máquinas virtuais ou containers.

Ela deve funcionar como uma camada de recursos computacionais, armazenamento, rede, execução, dispositivos e infraestrutura externa, abstraída por contratos que permitam à Yuki evoluir durante décadas sem depender de um hardware, sistema operacional, provedor de nuvem, linguagem, runtime ou tecnologia específica.

O objetivo é permitir que a Yuki:

- opere localmente;
- opere na nuvem;
- funcione offline quando possível;
- utilize diferentes dispositivos;
- distribua processamento;
- utilize diferentes tipos de hardware;
- migre entre plataformas;
- sobreviva a falhas;
- escale quando necessário;
- reduza consumo de recursos quando possível;
- mantenha segurança e isolamento;
- seja reconstruível;
- seja observável;
- possa incorporar tecnologias futuras.

A infraestrutura deve servir à Yuki, e não definir artificialmente aquilo que a Yuki é capaz de fazer.

---

2. Princípio Fundamental

«Infraestrutura não é servidor. Infraestrutura é o conjunto de recursos que torna possível executar, armazenar, comunicar, proteger e operar a Yuki.»

Uma máquina física é apenas uma possível fonte desses recursos.

A arquitetura deve permanecer válida caso:

- um servidor seja substituído;
- uma GPU seja trocada;
- um novo acelerador apareça;
- a memória seja expandida;
- um banco de dados seja migrado;
- um provedor cloud seja substituído;
- um novo runtime seja adotado;
- uma nova arquitetura de CPU seja criada;
- novos tipos de computação sejam disponibilizados.

---

3. Princípios Arquiteturais

3.1 Substrate Independence

A Yuki não deve depender estruturalmente de um substrato computacional específico.

Exemplos de substrato:

- CPU;
- GPU;
- NPU;
- TPU;
- FPGA;
- DSP;
- aceleradores especializados;
- computação fotônica;
- computação quântica;
- futuros aceleradores desconhecidos.

A infraestrutura deve expor capacidades abstratas.

---

3.2 Resource Abstraction

A unidade lógica fundamental da infraestrutura não é:

- servidor;
- VM;
- container;
- GPU;
- máquina física.

A unidade lógica fundamental é um Typed Abstract Resource.

Exemplo conceitual:

ComputeCapability
├── TensorOperations
├── Precision: FP16
├── PerformanceClass: High
├── MemoryRequirement: Large
└── PrivacyTier: Local

Outro workload pode solicitar:

StorageCapability
├── Type: Object
├── Durability: High
├── Encryption: Required
└── Location: Home

A infraestrutura então decide quais recursos físicos ou virtuais podem satisfazer esses requisitos.

---

4. Resource Fabric

A infraestrutura da Yuki será organizada conceitualmente como um Resource Fabric.

YUKI INFRASTRUCTURE FABRIC
│
├── Compute Resources
│   ├── CPU
│   ├── GPU
│   ├── NPU
│   ├── FPGA
│   ├── DSP
│   └── Future Accelerators
│
├── Storage Resources
│   ├── Working Storage
│   ├── Database
│   ├── Vector Storage
│   ├── Object Storage
│   └── Archive
│
├── Network Resources
│   ├── Local Network
│   ├── Overlay Network
│   ├── Secure Tunnels
│   └── External Connectivity
│
├── Runtime Resources
│   ├── Native
│   ├── WebAssembly
│   ├── MicroVM
│   └── Containers
│
├── Device Resources
│   ├── Sensors
│   ├── Cameras
│   ├── Microphones
│   ├── Actuators
│   └── Robots
│
├── Security Resources
│   ├── TPM
│   ├── Secure Enclave / TEE
│   ├── Hardware Keys
│   └── Cryptographic Resources
│
└── External Resources
    ├── APIs
    ├── Cloud Services
    ├── Frontier Models
    └── External Systems

O Fabric não precisa ser implementado como uma única tecnologia ou serviço.

É uma abstração arquitetural.

---

5. Separação entre Cognitive Layer e Infrastructure Layer

A camada cognitiva da Yuki não deve escolher diretamente servidores ou hardware.

A divisão conceitual é:

YUKI CORE
     │
     │ What do I need?
     ▼
PROCESSING ROUTER
     │
     │ Workload Requirements
     ▼
INFRASTRUCTURE RESOURCE MANAGER
     │
     │ Where / How?
     ▼
SCHEDULER / PLACEMENT
     │
     ▼
RESOURCE ADAPTERS
     │
     ▼
PHYSICAL / VIRTUAL RESOURCES

Processing Router

Responsável por decisões relacionadas ao processamento necessário para determinada tarefa.

Considera:

- tipo de tarefa;
- modelo;
- latência;
- privacidade;
- qualidade;
- quantidade de dados;
- custo;
- disponibilidade.

Infrastructure Resource Manager

Responsável por transformar requisitos abstratos em recursos reais.

Considera:

- recursos disponíveis;
- localização;
- capacidade;
- carga;
- energia;
- temperatura;
- isolamento;
- disponibilidade;
- custo;
- conectividade;
- políticas de infraestrutura.

Essa separação evita que o Core precise conhecer detalhes de infraestrutura.

---

6. Hybrid-First Architecture

A Yuki deve ser projetada desde o início para operar em múltiplos domínios.

A arquitetura de referência é:

                 YUKI
                  │
        ┌─────────┼─────────┐
        │         │         │
       EDGE      HOME      CLOUD
        │         │         │
    Perception   Core     Overflow
    Interaction Storage   Heavy Compute
    Low latency  Models   External APIs
                 Data     Backup

A Yuki não deve ser:

- exclusivamente cloud;
- exclusivamente local;
- exclusivamente dependente do dispositivo do usuário.

Ela deve poder distribuir tarefas de acordo com contexto e políticas.

---

7. Edge Domain

O Edge representa dispositivos próximos ao usuário.

Exemplos:

- smartphone;
- smartwatch;
- earbuds;
- notebooks;
- sensores;
- câmeras;
- dispositivos IoT.

Funções potenciais:

- wake word;
- percepção básica;
- processamento de áudio;
- processamento inicial de imagem;
- detecção de presença;
- interface;
- autenticação contextual;
- pré-processamento;
- operações de baixa latência.

Sempre que possível, dados que não precisam sair do dispositivo devem permanecer localmente.

---

8. Home Domain

O Home Domain representa a infraestrutura local soberana da Yuki.

Pode incluir:

- servidor doméstico;
- storage;
- rede local;
- aceleradores;
- bancos de dados;
- modelos locais;
- serviços de execução;
- sistemas de automação;
- dispositivos inteligentes.

O Home Domain pode funcionar como:

EDGE
  ↕
HOME CORE
  ↕
CLOUD

Ele deve ser capaz de continuar executando determinadas funções mesmo quando a conectividade externa estiver indisponível.

O Home Domain não precisa manter permanentemente nenhum modelo específico carregado.

A presença de modelos locais deve ser tratada como uma decisão de deployment baseada em:

- capacidade;
- necessidade;
- privacidade;
- latência;
- custo;
- disponibilidade;
- workload.

---

9. Cloud Domain

A nuvem representa capacidade externa elástica.

Funções possíveis:

- modelos frontier;
- processamento pesado;
- grandes workloads paralelos;
- armazenamento externo;
- backup;
- recuperação;
- treinamento/fine-tuning;
- serviços externos;
- APIs;
- processamento geograficamente distribuído.

A cloud deve ser tratada como recurso externo, não como fundamento obrigatório da identidade da Yuki.

A Yuki deve evitar dependência estrutural de um único provedor.

---

10. Infrastructure Resource Manager

O Infrastructure Resource Manager é responsável por administrar o Resource Fabric.

Funções:

- descoberta de recursos;
- registro;
- classificação;
- alocação;
- scheduling;
- placement;
- quotas;
- isolamento;
- monitoramento;
- health checking;
- resource reclamation;
- failover;
- graceful degradation;
- offloading;
- energy-aware scheduling;
- thermal-aware scheduling.

Conceitualmente:

Workload
   │
   ▼
Requirements
   │
   ▼
Resource Manager
   │
   ├── Available Resources
   ├── Policies
   ├── Security
   ├── Cost
   ├── Energy
   ├── Thermal State
   └── Network
   │
   ▼
Placement

---

11. Workload Contract

Toda execução significativa deve poder ser descrita através de requisitos abstratos.

Exemplo conceitual:

Workload
├── Compute
│   ├── CPU
│   ├── GPU
│   └── Accelerator
│
├── Memory
├── Storage
├── Network
├── Privacy
├── Latency
├── Reliability
├── Isolation
├── Energy
├── Cost
└── Location

Isso permite que o mesmo workload seja executado em diferentes infraestruturas.

---

12. Compute Abstraction

A Yuki não deve depender diretamente de uma API específica de hardware.

A arquitetura deve possuir uma camada:

YUKI
 ↓
COMPUTE ABSTRACTION
 ↓
ACCELERATOR ADAPTER
 ↓
HARDWARE

Possíveis implementações futuras podem incluir:

- CPU;
- GPU;
- NPU;
- TPU;
- FPGA;
- DSP;
- aceleradores especializados;
- novos paradigmas computacionais.

Tecnologias como ONNX Runtime, Vulkan Compute, TVM, vLLM ou outros runtimes podem atuar como implementações/adapters.

Nenhuma delas constitui requisito constitucional da arquitetura.

---

13. Storage Architecture

O armazenamento deve ser dividido conceitualmente por função.

STORAGE FABRIC
│
├── Working Data
├── Structured Data
├── Semantic / Vector Data
├── Graph Data
├── Object Data
├── Logs / Telemetry
└── Archive / Backup

A Yuki deve utilizar interfaces de acesso.

Exemplo:

Memory
 ↓
Repository Interface
 ↓
Storage Adapter
 ↓
Database / Vector DB / Object Store

Isso permite substituir implementações sem reescrever o sistema inteiro.

---

14. Database Independence

Um banco específico não deve ser tratado como princípio arquitetural.

PostgreSQL, por exemplo, pode ser uma implementação inicial para dados estruturados.

Outros componentes podem ser utilizados para:

- vetores;
- grafos;
- objetos;
- cache;
- eventos;
- séries temporais.

A arquitetura deve depender de contratos como:

StructuredRepository
VectorRepository
GraphRepository
ObjectStore
EventStore

e não diretamente da tecnologia concreta.

---

15. Network Architecture

A rede deve ser dividida em responsabilidades.

NETWORK
│
├── Connectivity
├── Encryption
├── Overlay / Coordination
├── Workload Identity
└── Application Authentication

Essas funções não devem ser confundidas.

Network Encryption

Pode utilizar tecnologias como WireGuard.

Overlay / Coordination

Pode utilizar soluções como Tailscale, Headscale ou alternativas futuras.

Workload Identity

Pode utilizar SPIFFE/SPIRE ou outra infraestrutura de identidade criptográfica.

Application Authentication

Pode utilizar mTLS ou mecanismos apropriados ao protocolo.

Uma tecnologia específica não deve ser confundida com a responsabilidade arquitetural que ela implementa.

---

16. Zero-Trust Network

A infraestrutura deve seguir o princípio:

«Nenhum recurso deve ser considerado confiável apenas porque está dentro da rede local.»

A confiança deve considerar:

- identidade;
- dispositivo;
- workload;
- sessão;
- credenciais;
- contexto;
- política;
- integridade.

Isso complementa o modelo definido em "docs/10_SECURITY.md".

---

17. Home ↔ Cloud Connectivity

A comunicação entre Home e Cloud deve ser projetada para evitar exposição desnecessária da infraestrutura doméstica.

Sempre que possível:

HOME
  │
  │ outbound secure connection
  ▼
CLOUD

em vez de depender de exposição direta de serviços internos à Internet.

A arquitetura deve suportar:

- reconexão;
- perda de conexão;
- mudança de endereço;
- failover;
- múltiplos provedores;
- operação offline.

---

18. Runtime Isolation

A infraestrutura deve utilizar isolamento progressivo.

Trusted Native
      ↓
Container
      ↓
Wasm Sandbox
      ↓
MicroVM

A escolha depende do risco e dos requisitos.

Native

Para componentes altamente confiáveis e controlados.

Containers

Adequados para isolamento operacional e empacotamento de infraestrutura.

Containers compartilham o kernel do host e não devem ser tratados como isolamento absoluto contra código arbitrariamente não confiável.

WebAssembly

Adequado para componentes com necessidade de portabilidade, sandboxing e contratos bem definidos.

Wasm reduz determinadas classes de riscos através de seu modelo de execução e capacidades, mas não constitui garantia absoluta de segurança.

MicroVM

Adequada para workloads que exigem isolamento mais forte ou execução de código arbitrário.

Nenhum número fixo de overhead ou tempo de inicialização deve ser tratado como garantia arquitetural; esses valores dependem da implementação, hardware e configuração.

---

19. Progressive Isolation

A Yuki deve evitar tanto:

- executar tudo diretamente no host;
- quanto executar absolutamente tudo no isolamento máximo.

O nível de isolamento deve ser proporcional ao risco.

Risk
 ↓
Isolation Requirement
 ↓
Runtime Selection

Essa decisão deve integrar:

- Security Controller;
- Capability System;
- Resource Manager;
- Execution Plane.

---

20. Event and Execution Infrastructure

A infraestrutura deve suportar execução orientada a eventos.

Integração com "docs/13_EVENTS_AND_BACKGROUND.md":

Event
 ↓
Event Router
 ↓
Mission / Task
 ↓
Resource Requirements
 ↓
Infrastructure Resource Manager
 ↓
Execution

A infraestrutura deve suportar:

- filas;
- eventos;
- retries;
- timeouts;
- TTL;
- backpressure;
- deduplicação;
- idempotência;
- persistência;
- pausa;
- retomada;
- cancelamento.

Uma tecnologia específica de mensageria permanece como decisão de implementação.

---

21. Messaging Backbone

Um backbone de eventos/mensagens pode ser utilizado para:

- comunicação entre componentes;
- eventos de infraestrutura;
- tarefas;
- notificações;
- execução assíncrona;
- workflows.

NATS/JetStream é uma implementação candidata.

Não é uma dependência arquitetural definitiva.

A abstração deve representar:

Event Bus
Message Queue
Stream
Task Queue
Persistent Event Store

quando necessário.

---

22. Orchestration

A Yuki não deve assumir Kubernetes como requisito inicial.

A necessidade de um orquestrador deve ser determinada pela escala real.

Inicialmente, alternativas mais simples podem ser suficientes.

Possibilidades futuras incluem:

- systemd;
- container runtimes;
- Compose;
- Nomad;
- Kubernetes/K3s;
- outros sistemas;
- scheduler próprio.

O princípio é:

«Não introduzir complexidade operacional antes que ela seja necessária.»

---

23. Control Plane

O Control Plane da Yuki deve ser independente conceitualmente da linguagem utilizada.

Responsabilidades:

- identidade;
- autorização;
- políticas;
- registro de recursos;
- coordenação;
- estado;
- scheduling;
- infraestrutura;
- auditoria;
- governança.

Rust é atualmente uma candidata de implementação para partes do Control Plane devido às propriedades desejadas de segurança, desempenho e controle de recursos.

Entretanto:

«Rust não é uma decisão constitucional até que um ADR específico seja aprovado.»

O contrato do Control Plane deve sobreviver mesmo que a implementação seja posteriormente migrada para outra linguagem.

---

24. Infrastructure APIs

A infraestrutura deve ser acessível através de APIs/contratos.

Exemplos conceituais:

Resource Discovery API
Resource Allocation API
Compute API
Storage API
Network API
Runtime API
Health API
Telemetry API
Recovery API

O Core não deve acessar diretamente:

/dev/*
host shell
GPU driver
database internals
filesystem internals

sem passar pelas abstrações e políticas apropriadas.

---

25. Infrastructure Security

A infraestrutura integra diretamente com "docs/10_SECURITY.md".

Princípios:

- least privilege;
- least agency;
- zero trust;
- assume breach;
- isolation;
- resource quotas;
- identity;
- audit;
- secrets protection;
- secure bootstrapping;
- supply-chain verification;
- capability attestation;
- network segmentation;
- execution verification.

Nenhum recurso de infraestrutura deve conceder automaticamente autorização para executar uma ação.

---

26. Secrets

Secrets devem ser tratados como recursos protegidos.

Exemplos:

- API keys;
- tokens;
- certificados;
- chaves privadas;
- credenciais de banco;
- credenciais cloud;
- recovery keys.

Secrets não devem ser:

- colocados em código;
- enviados indiscriminadamente para agentes;
- armazenados em logs;
- incluídos em prompts sem necessidade;
- expostos a capabilities que não precisam deles.

---

27. Resource Governance

Todo workload deve estar sujeito a limites.

Possíveis limites:

- CPU;
- memória;
- GPU;
- armazenamento;
- rede;
- duração;
- número de processos;
- número de chamadas;
- tokens;
- custo;
- energia;
- temperatura.

O objetivo é impedir:

Runaway Agent
Runaway Workflow
Resource Exhaustion
Cost Explosion
Event Storm

A infraestrutura deve possuir mecanismos equivalentes a quotas, cgroups, limites de runtime ou mecanismos futuros apropriados.

---

28. Energy-Aware Infrastructure

A Yuki deve considerar energia como recurso.

O scheduler pode considerar:

- consumo;
- carga;
- eficiência;
- horário;
- bateria;
- disponibilidade de energia;
- prioridade da tarefa.

Exemplo:

Task
 ↓
Urgency?
 ↓
Can wait?
 ├── YES → energy-efficient execution
 └── NO  → execute immediately

---

29. Thermal-Aware Infrastructure

Temperatura deve ser considerada uma variável operacional.

A infraestrutura pode:

- reduzir workload;
- migrar processamento;
- utilizar outro acelerador;
- utilizar cloud;
- suspender tarefas;
- alterar prioridade.

Isso evita que performance máxima seja buscada independentemente do estado físico do hardware.

---

30. Graceful Degradation

Falhas de infraestrutura não devem significar necessariamente falha total da Yuki.

Exemplo:

GPU unavailable
      ↓
Local CPU
      ↓
Cloud
      ↓
Reduced capability

Outro exemplo:

Cloud unavailable
      ↓
Home
      ↓
Local Models
      ↓
Deterministic / Cached Functions

Outro:

Home unavailable
      ↓
Cloud
      ↓
Degraded Mode

A Yuki deve preservar o máximo possível de funcionalidade segura.

---

31. Offline Capability

A Yuki deve possuir um modo offline ou parcialmente offline.

Possíveis funções:

- memória local;
- agenda;
- tarefas;
- comandos locais;
- automação doméstica;
- modelos locais;
- reconhecimento de contexto;
- armazenamento;
- processamento determinístico.

Quando a conexão retornar:

Offline Queue
 ↓
Connectivity Restored
 ↓
Synchronization
 ↓
Conflict Resolution

---

32. Failure Isolation

Uma falha em um componente não deve propagar automaticamente para todo o sistema.

Exemplo:

Capability A
   X
Capability B
   ✓
Capability C
   ✓
Core
   ✓
Security Controller
   ✓

Isso exige:

- processos isolados;
- quotas;
- circuit breakers;
- timeouts;
- health checks;
- supervision;
- retry controlado;
- boundaries claros.

---

33. Cloud Provider Independence

A Yuki não deve depender estruturalmente de um único provedor cloud.

A infraestrutura deve utilizar adapters.

YUKI
 ↓
Cloud Interface
 ↓
Provider Adapter
 ├── Provider A
 ├── Provider B
 ├── Provider C
 └── Future Provider

Tecnologias como OpenTofu podem ser utilizadas como implementação de Infrastructure as Code.

A abstração arquitetural continua independente da ferramenta.

---

34. Infrastructure as Code

A infraestrutura deve ser reproduzível através de configuração declarativa sempre que possível.

Objetivos:

- versionamento;
- revisão;
- auditoria;
- reproducibilidade;
- rollback;
- disaster recovery;
- migração;
- automação.

Possíveis tecnologias:

- OpenTofu;
- outras ferramentas IaC;
- sistemas declarativos futuros.

A tecnologia pode mudar sem mudar o princípio.

---

35. Phoenix Bootstrap

A Yuki deve possuir um conceito de reconstrução completa chamado:

Phoenix Bootstrap

Objetivo:

«Ser capaz de reconstruir a infraestrutura essencial da Yuki após uma falha catastrófica.»

Conceitualmente:

Source Code
     +
Declarative Infrastructure
     +
Configuration
     +
Recovery Credentials
     +
Encrypted Backup
     ↓
PHOENIX BOOTSTRAP
     ↓
Infrastructure
     ↓
Core
     ↓
Security
     ↓
Memory / Data
     ↓
YUKI RESTORED

O Phoenix Bootstrap é um conceito arquitetural.

Sua implementação concreta será definida posteriormente.

---

36. Backup

A estratégia deve seguir o princípio 3-2-1 ou uma estratégia futura equivalente:

3 copies
2 different storage media
1 geographically separate copy

Backups devem ser:

- criptografados;
- verificáveis;
- versionados;
- testados;
- protegidos contra corrupção;
- protegidos contra exclusão acidental;
- sujeitos a políticas de retenção.

Backup que nunca foi restaurado/testado não deve ser considerado plenamente confiável.

---

37. Disaster Recovery

A infraestrutura deve possuir objetivos explícitos para recuperação.

Conceitos futuros:

- RPO;
- RTO;
- prioridade de serviços;
- recovery tiers;
- failover;
- restore testing;
- disaster simulation.

Serviços críticos devem possuir prioridade maior.

---

38. Critical Infrastructure

Recursos críticos incluem, conceitualmente:

Security Controller
Control Plane
Core Storage
Identity
Recovery Infrastructure
Event Backbone
Audit

Esses componentes devem possuir proteção contra starvation.

Uma falha ou workload excessivo não deve consumir todos os recursos e impedir a recuperação.

---

39. Observability

Toda infraestrutura relevante deve ser observável.

Categorias:

Metrics
Logs
Traces
Events
Health
Audit
Resource Usage
Security Signals

A observabilidade deve permitir responder:

- o que aconteceu?
- quando?
- onde?
- qual componente?
- qual recurso?
- qual workload?
- qual identidade?
- qual política?
- qual consequência?

OpenTelemetry é uma possível implementação.

---

40. Infrastructure Health

Recursos devem possuir estado de saúde.

Exemplo:

HEALTHY
DEGRADED
OVERLOADED
UNAVAILABLE
SUSPECTED
QUARANTINED
RETIRED

Um recurso suspeito pode ser removido temporariamente do scheduler.

---

41. Resource Discovery

A infraestrutura deve conseguir descobrir recursos disponíveis.

Exemplo:

Resource Discovery
 ↓
Compute
Storage
Network
Runtime
Device
Security
External

Cada recurso deve possuir identidade e metadata suficientes para que o Resource Manager determine sua adequação.

---

42. Resource Lifecycle

Recursos devem possuir ciclo de vida.

DISCOVERED
 ↓
REGISTERED
 ↓
VALIDATED
 ↓
AVAILABLE
 ↓
ALLOCATED
 ↓
MONITORED
 ↓
DEGRADED / QUARANTINED
 ↓
RETIRED

Um recurso novo não deve ser automaticamente considerado confiável apenas porque apareceu na rede.

---

43. Capability Attestation

Quando apropriado, recursos e workloads podem precisar comprovar:

- identidade;
- integridade;
- versão;
- origem;
- configuração;
- estado.

Isso complementa o modelo de attestation definido em "docs/08_CAPABILITY_SYSTEM.md" e "docs/10_SECURITY.md".

---

44. Physical Infrastructure

A arquitetura deve suportar infraestrutura física doméstica e futura expansão.

Possíveis componentes:

- servidor;
- storage;
- UPS/nobreak;
- rede cabeada;
- switches;
- roteadores;
- rack;
- sensores;
- climatização;
- monitoramento;
- energia protegida.

A arquitetura lógica não deve depender de uma configuração física específica.

---

45. Scalability

A Yuki deve escalar horizontal e verticalmente quando necessário.

Vertical:

More CPU
More RAM
More GPU
More Storage

Horizontal:

Node A
Node B
Node C
Node D

Distributed:

Edge
Home
Cloud

A escalabilidade deve ocorrer quando houver necessidade real.

---

46. Resource Scheduling

O scheduler deve considerar múltiplas dimensões:

- prioridade;
- deadline;
- dependências;
- capacidade;
- latência;
- privacidade;
- custo;
- energia;
- temperatura;
- disponibilidade;
- confiabilidade;
- isolamento.

O scheduler não deve considerar apenas “qual máquina está livre”.

---

47. Data Locality

Sempre que possível, o processamento deve ocorrer próximo aos dados quando isso:

- reduzir latência;
- reduzir custo;
- aumentar privacidade;
- reduzir tráfego.

Porém, data locality não deve impedir offloading quando o processamento externo for necessário.

---

48. Privacy-Aware Placement

A localização do processamento pode ser uma propriedade do workload.

Exemplo:

Privacy: Maximum Local
     ↓
Home / Edge

ou:

Privacy: External Allowed
     ↓
Cloud Candidate

ou:

Sensitive Data
     ↓
Eligible Confidential Environment

Confidential Computing pode ser utilizado para workloads sensíveis quando houver suporte adequado.

Não deve ser considerado obrigatório para absolutamente todo processamento cloud.

---

49. Migration

Qualquer componente relevante deve poder ser migrado.

Exemplo:

Implementation A
     ↓
Adapter
     ↓
Contract
     ↓
Implementation B

A migração deve ser facilitada por:

- contratos;
- versionamento;
- adapters;
- testes;
- exportação de dados;
- IaC;
- backups;
- observabilidade.

---

50. Technology Independence

A arquitetura não deve tratar como eternas tecnologias atualmente populares.

Exemplos de tecnologias que podem ser utilizadas sem se tornarem dependências constitucionais:

- Linux;
- Rust;
- Python;
- PostgreSQL;
- CUDA;
- Docker;
- Podman;
- Wasmtime;
- Firecracker;
- NATS;
- Qdrant;
- OpenTofu;
- WireGuard;
- SPIRE;
- vLLM;
- ONNX Runtime.

Essas são implementações possíveis.

Os princípios permanecem acima delas.

---

51. Future Hardware

A infraestrutura deve ser preparada para hardware que ainda não existe.

Possíveis exemplos:

- novos NPUs;
- CXL e memory pooling;
- GPUs futuras;
- aceleradores especializados;
- computação fotônica;
- neuromorphic computing;
- quantum computing;
- novos paradigmas de memória;
- novos sistemas de interconexão.

A arquitetura não deve implementar esses recursos antecipadamente sem necessidade.

Ela deve possuir interfaces que permitam adicioná-los posteriormente.

---

52. Future Capability Principle

«A arquitetura não deve limitar as capacidades futuras da Yuki às tecnologias que conseguimos imaginar hoje.»

Isso significa que novos recursos devem poder aparecer como novos adapters/capabilities.

Exemplo:

Future Accelerator
      ↓
Adapter
      ↓
Compute Resource
      ↓
Resource Fabric
      ↓
Yuki

Não deve ser necessário reconstruir o Core.

---

53. Infrastructure Evolution

A infraestrutura integra-se ao "Evolution Manager".

Fluxo:

Problem
 ↓
Observe
 ↓
Evaluate
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
Benchmark
 ↓
Approval
 ↓
Deploy
 ↓
Monitor
 ↓
Rollback if necessary

A infraestrutura nunca deve evoluir simplesmente por “novidade tecnológica”.

O objetivo é melhorar:

- confiabilidade;
- segurança;
- desempenho;
- custo;
- eficiência;
- capacidade;
- manutenção;
- resiliência.

---

54. Preserve Before Modify

Antes de alterações estruturais:

Current State
 ↓
Backup
 ↓
Snapshot
 ↓
Version
 ↓
Change
 ↓
Validate

A infraestrutura deve ser capaz de retornar a um estado anterior conhecido.

---

55. Infrastructure and Git

Git é parte importante da memória de engenharia da Yuki.

Devem ser versionados:

- arquitetura;
- contratos;
- configuração declarativa;
- infraestrutura como código;
- ADRs;
- migrations;
- documentação;
- decisões;
- políticas;
- testes.

A infraestrutura não deve depender exclusivamente do estado manual de máquinas.

---

56. Security Boundaries

Os principais limites de confiança devem ser explícitos:

USER
 ↓
ACCESS
 ↓
CONTROL PLANE
 ↓
COGNITIVE / DATA PLANE
 ↓
EXECUTION PLANE
 ↓
INFRASTRUCTURE
 ↓
EXTERNAL SYSTEMS

Cada fronteira deve possuir:

- identidade;
- autorização;
- validação;
- observabilidade;
- limites.

---

57. External Systems

Sistemas externos devem ser considerados não confiáveis por padrão.

Exemplos:

- APIs;
- SaaS;
- cloud;
- webhooks;
- serviços de terceiros;
- provedores de modelos.

A Yuki deve validar:

- origem;
- integridade;
- schema;
- resposta;
- autorização;
- impacto.

Dados externos continuam sendo dados, não instruções.

---

58. Infrastructure Failure Model

A arquitetura deve considerar:

Hardware failure

→ failover/offload/degraded mode.

Network failure

→ retry/offline queue/local operation.

Cloud failure

→ alternate provider/home/local fallback.

Storage failure

→ replica/backup/recovery.

Runtime failure

→ restart/retry/isolation.

Capability failure

→ quarantine/fallback.

Event storm

→ backpressure/rate limit/deduplication.

Resource exhaustion

→ quotas/priorities/preemption.

Security incident

→ containment/lockdown.

---

59. Autonomy Boundary

A infraestrutura pode automatizar operações de baixo risco.

Entretanto, a autonomia deve parar antes de alterações críticas.

Exemplos:

- mudança de identidade;
- alteração de chaves de segurança;
- alteração de autorização;
- expansão de privilégios;
- exclusão irreversível;
- gastos acima de limites;
- alteração crítica do Security Controller.

Essas ações exigem o nível de aprovação definido pelo Security Controller e pelas políticas da Yuki.

---

60. Reference Architecture

A arquitetura consolidada é:

                         USER
                          │
                    YUKI ACCESS
                          │
                       YUKI
                        CORE
                          │
                 ┌────────┴────────┐
                 │                 │
          Processing Router    Context/Memory
                 │
                 ▼
        Workload Requirements
                 │
                 ▼
     INFRASTRUCTURE RESOURCE MANAGER
                 │
       ┌─────────┼─────────┐
       │         │         │
    Scheduler  Policy   Security
       │
       ▼
              RESOURCE FABRIC
       │
 ┌─────┼─────┬─────┬─────┬─────┐
 ▼     ▼     ▼     ▼     ▼     ▼
CPU   GPU   Storage Network Runtime Devices
 │     │      │       │       │
 └─────┴──────┴───────┴───────┘
              │
      EDGE / HOME / CLOUD

Transversalmente:

Identity
Authorization
Security
Audit
Observability
Backup
Recovery
Evolution

---

61. Candidate Technology Landscape

As tecnologias abaixo podem ser consideradas para implementação:

Área| Candidatos
Control Plane| Rust, Go, outras
OS| Linux e alternativas futuras
Containers| Podman, containerd, Docker
Wasm| Wasmtime, outros runtimes
MicroVM| Firecracker, alternativas
Messaging| NATS/JetStream, outras
Structured DB| PostgreSQL, outras
Vector DB| pgvector, Qdrant, Milvus, outras
Object Storage| S3-compatible, MinIO, Garage, outras
IaC| OpenTofu, Pulumi, outras
Network encryption| WireGuard, alternativas
Overlay| Tailscale/Headscale, alternativas
Workload identity| SPIFFE/SPIRE, alternativas
Observability| OpenTelemetry, alternativas
Inference| vLLM, ONNX Runtime, llama.cpp, outros

Essa tabela não constitui uma decisão final de tecnologia.

---

62. Decisions Pending

As principais decisões de implementação ainda devem ser formalizadas por ADR.

ADR-001 — Control Plane Implementation

Questão:

«Qual linguagem/runtime deve implementar o Control Plane inicial?»

Rust é candidato atual.

---

ADR-002 — Event and Messaging Backbone

Questão:

«Qual infraestrutura deve implementar Event Bus, Messaging e Durable Streams?»

NATS/JetStream é candidato atual.

---

ADR-003 — Execution Isolation

Questão:

«Qual combinação de Wasm, MicroVM e containers será utilizada e em quais níveis de risco?»

---

ADR-004 — Identity and Network Security

Questão:

«Qual combinação de network encryption, overlay, workload identity e application authentication será utilizada?»

Candidatos:

- WireGuard;
- Tailscale/Headscale;
- SPIFFE/SPIRE;
- mTLS;
- alternativas.

---

ADR-005 — Storage Architecture

Questão:

«Qual implementação concreta será utilizada para structured, vector, graph, object e archive storage?»

---

63. Decisions That Can Be Deferred

Não é necessário decidir imediatamente:

- modelo exato de servidor;
- GPU específica;
- fabricante do hardware;
- provedor cloud definitivo;
- cluster Kubernetes;
- número de nodes;
- capacidade final de storage;
- hardware futuro;
- aceleradores futuros.

Essas decisões devem ser tomadas quando houver requisitos reais.

---

64. What Must Not Become a Hidden Dependency

A Yuki não deve possuir dependência estrutural não documentada em:

- uma GPU específica;
- um fornecedor cloud;
- um modelo;
- uma linguagem;
- um banco;
- um sistema de containers;
- um sistema operacional;
- uma rede específica;
- um runtime específico.

Se uma tecnologia se tornar fundamental, essa decisão deve ser explicitamente documentada.

---

65. Relationship With Other Documents

07_AGENTS_AND_TASKS
        │
        ▼
08_CAPABILITY_SYSTEM
        │
        ▼
09_MODEL_ROUTER
        │
        ▼
10_SECURITY
        │
        ▼
11_EVOLUTION
        │
        ▼
12_VOICE_AND_MULTIMODAL
        │
        ▼
13_EVENTS_AND_BACKGROUND
        │
        ▼
14_INFRASTRUCTURE
        │
        ├── 15_INTEGRATIONS
        │
        └── 16_MASTER_CAPABILITY_CATALOG

"14_INFRASTRUCTURE.md" fornece a base física, virtual e operacional para os sistemas superiores.

---

66. Official Principles

A arquitetura de infraestrutura da Yuki adota oficialmente:

1. Infrastructure is a Resource Fabric, not a server.
2. Substrate Independence.
3. Typed Abstract Resources.
4. Processing Router ≠ Infrastructure Resource Manager.
5. Hybrid-First Architecture.
6. Local capability whenever appropriate.
7. Cloud as elastic external capacity, not mandatory identity.
8. Hardware abstraction.
9. Provider independence.
10. Progressive execution isolation.
11. Zero-Trust infrastructure.
12. Least privilege and least agency.
13. Resource governance.
14. Energy-aware and thermal-aware scheduling.
15. Graceful degradation.
16. Offline capability.
17. Failure isolation.
18. Infrastructure as Code.
19. Reproducible infrastructure.
20. Disaster recovery.
21. Observability.
22. Data locality when beneficial.
23. Privacy-aware placement.
24. Future hardware compatibility.
25. Migration through contracts and adapters.
26. Preserve Before Modify.
27. Git as engineering memory.
28. No hidden technology dependencies.
29. Technology choices require explicit justification.
30. Infrastructure autonomy must remain subordinate to Security Controller and human authority.

---

67. Constitutional Statement

«A infraestrutura da Yuki deve permitir que ela atravesse gerações de hardware, software, modelos, provedores e paradigmas computacionais sem exigir uma reconstrução completa de sua arquitetura.»

«A Yuki deve enxergar infraestrutura como recursos abstratos que podem ser descobertos, alocados, combinados, isolados, monitorados, migrados e substituídos.»

«Nenhum hardware, provedor, runtime ou tecnologia específica deve ser confundido com a própria Yuki.»

---

68. Status

Status: Architectural Direction Approved

Technology Decisions: Pending ADRs

Implementation: Not yet frozen

Next document: "docs/15_INTEGRATIONS.md"

Next architectural focus: integração da Yuki com sistemas externos, APIs, serviços, dispositivos e sistemas desenvolvidos pelo usuário.

---

69. Maintenance Rule

Este documento deve ser atualizado quando:

- um princípio arquitetural mudar;
- um contrato mudar;
- uma decisão de infraestrutura for formalizada;
- uma tecnologia for adotada como implementação oficial;
- uma nova classe de recurso surgir;
- uma ameaça relevante alterar os requisitos;
- a infraestrutura mudar significativamente;
- uma nova geração tecnológica exigir revisão arquitetural.

Alterações de tecnologia que não alterem os princípios podem ser registradas em ADRs sem reescrever a arquitetura inteira.

Versionamento obrigatório: toda alteração arquitetural relevante deve ser registrada no Git.