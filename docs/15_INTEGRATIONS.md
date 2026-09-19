YUKI — DOMÍNIO 15

INTEGRATIONS

Versão: v1.0
Status: ACCEPTED
Tipo: Arquitetura Oficial
Escopo: Integrações externas, sistemas externos, conectores, adaptadores, gateways, credenciais, execução externa e verificação de efeitos
Projeto: Yuki
Data: 2026-09-19

---

1. Objetivo

O Domínio 15 define como a Yuki se comunica com sistemas externos, serviços, APIs, plataformas, dispositivos, sistemas construídos pelo usuário e futuras infraestruturas.

O objetivo é permitir que a Yuki seja altamente integrada ao mundo externo sem transformar o Core em um componente diretamente acoplado a APIs, fornecedores, protocolos, credenciais ou implementações específicas.

A arquitetura deve permitir:

- consumir dados externos;
- executar ações externas;
- integrar serviços de terceiros;
- integrar sistemas próprios;
- integrar dispositivos;
- receber eventos;
- executar operações digitais;
- futuramente interagir com sistemas físicos e robóticos;
- trocar tecnologias sem reescrever o Core;
- controlar permissões e credenciais;
- verificar efeitos externos;
- isolar falhas;
- revogar integrações;
- adicionar novos provedores;
- suportar tecnologias ainda desconhecidas.

A integração externa é considerada uma fronteira de confiança da Yuki.

---

2. Princípio Fundamental

A Yuki não deve tratar sistemas externos como extensões confiáveis de seu próprio ambiente.

A regra fundamental é:

«Tudo que vem de fora é dado externo até que seja explicitamente interpretado e autorizado.»

Da mesma forma:

«Nenhum sistema externo recebe autoridade simplesmente porque está integrado à Yuki.»

E:

«Nenhum modelo recebe autoridade simplesmente porque consegue produzir uma instrução válida.»

---

3. Separação Conceitual Fundamental

Os conceitos abaixo são diferentes e não devem ser fundidos.

Capability
     │
     │ representa
     ▼
O QUE a Yuki consegue fazer

Tool
     │
     │ representa
     ▼
INTERFACE INVOCÁVEL usada por modelos/planners

Plugin
     │
     │ representa
     ▼
FORMA DE DISTRIBUIR/INSTALAR EXTENSÕES

Integration
     │
     │ representa
     ▼
CONEXÃO CONFIGURADA COM UM SISTEMA EXTERNO

Connector
     │
     │ representa
     ▼
LÓGICA DE DOMÍNIO DA INTEGRAÇÃO

Adapter
     │
     │ representa
     ▼
TRADUÇÃO ENTRE INTERFACES/PROTOCOLOS

Essas abstrações podem se relacionar, mas não são equivalentes.

---

4. Capability

Uma Capability representa uma habilidade funcional da Yuki.

Exemplos:

github.repository.read
github.issue.create
email.send
calendar.event.create
finance.portfolio.read
home.lights.control
robot.pick_object

Uma Capability define:

- o que pode ser feito;
- entradas;
- saídas;
- pré-condições;
- efeitos esperados;
- requisitos;
- permissões necessárias;
- risco;
- políticas aplicáveis.

A Capability não define necessariamente como o sistema externo será acessado.

---

5. Tool

Tool é uma interface invocável exposta para componentes cognitivos, modelos, planners ou agentes.

Exemplo:

github.create_issue(
    repository,
    title,
    body
)

A existência de uma Tool não significa que sua execução esteja autorizada.

O fluxo correto é:

Modelo
 ↓
Tool Request
 ↓
Capability
 ↓
Security / Policy
 ↓
Authorization
 ↓
Integration
 ↓
External System

Portanto:

«Tool ≠ Permission.»

---

6. Plugin

Plugin é uma unidade instalável ou distribuível que adiciona, implementa ou modifica capacidades da Yuki.

Um plugin pode ser:

- código;
- Wasm;
- container;
- processo isolado;
- extensão declarativa;
- componente remoto;
- componente oficial;
- componente interno;
- componente de terceiros;
- outra forma futura de extensão.

Plugin é uma questão de extensibilidade e distribuição.

Plugin não recebe confiança automaticamente.

---

7. Integration

Uma Integration representa uma conexão operacional concreta entre a Yuki e um sistema externo.

Ela contém ou referencia:

- sistema externo;
- identidade;
- conta;
- endpoints;
- protocolos;
- capacidades suportadas;
- políticas;
- permissões;
- requisitos de runtime;
- credenciais ou referências a credenciais;
- estado;
- configuração;
- limites;
- mecanismos de verificação.

Uma Integration pode suportar diversas Capabilities.

GitHub Integration
 ├── repository.read
 ├── repository.write
 ├── issue.read
 ├── issue.create
 ├── pull_request.read
 └── pull_request.create

Portanto:

«Integration não é necessariamente 1:1 com Capability.»

---

8. Integration Definition e Integration Instance

A arquitetura diferencia:

Integration Definition
        │
        ├── regras de conexão
        ├── protocolos
        ├── contratos
        ├── capacidades suportadas
        └── requisitos
                │
                ▼
      Integration Instance

A Definition descreve como uma integração funciona.

A Instance representa uma conexão concreta.

Exemplo:

GitHub Integration Definition
        │
        ├── José / conta pessoal
        ├── Yuki / projeto
        └── outra conta autorizada

Isso permite que uma mesma integração tenha múltiplas instâncias com:

- identidades diferentes;
- credenciais diferentes;
- permissões diferentes;
- políticas diferentes;
- escopos diferentes.

---

9. External System

A arquitetura também diferencia Provider, External System e Resource.

Provider
   ↓
External System
   ↓
Account / Identity
   ↓
Resource

Exemplo:

GitHub
 ↓
GitHub API
 ↓
Conta autorizada
 ↓
Repositório Yuki

Isso evita que o Core dependa diretamente de detalhes específicos do fornecedor.

---

10. Arquitetura Oficial de Integrações

A arquitetura oficial é:

                         YUKI CORE
                             │
                             ▼
                        CAPABILITY
                             │
                             ▼
                    EXECUTION REQUEST
                             │
                             ▼
                 SECURITY / POLICY GATE
                             │
                       autorização
                             │
                             ▼
                  INTEGRATION GATEWAY
                             │
              ┌──────────────┼──────────────┐
              ▼              ▼              ▼
       Credential       Protocol       Event
        Broker          Adapter        Adapter
              │              │              │
              └──────────────┼──────────────┘
                             ▼
                    INTEGRATION RUNTIME
                             │
                    ┌────────┴────────┐
                    ▼                 ▼
                Connector          Adapter
                    │                 │
                    └────────┬────────┘
                             ▼
                      EXTERNAL SYSTEM
                             │
                             ▼
                       VERIFICATION
                             │
                             ▼
                          RESULT

O fluxo principal é:

Core
 ↓
Capability
 ↓
Execution Request
 ↓
Security / Policy
 ↓
Integration Gateway
 ↓
Integration Runtime
 ↓
External System
 ↓
Verification
 ↓
Result

---

11. Integration Gateway

O Integration Gateway é a fronteira arquitetural de mediação entre a Yuki e sistemas externos.

Ele pode realizar funções como:

- roteamento;
- normalização;
- aplicação de contratos;
- seleção de Integration Instance;
- controle de comunicação;
- aplicação de limites;
- observabilidade;
- encaminhamento;
- adaptação;
- coordenação com mecanismos de segurança.

O Gateway não deve se tornar um "God Gateway".

Ele não deve absorver toda a responsabilidade de:

- segurança global;
- gerenciamento de recursos;
- workflow;
- armazenamento de segredos;
- orquestração de agentes;
- auditoria completa;
- verificação completa.

Essas responsabilidades permanecem separadas.

O Gateway é uma fronteira de mediação, não uma garantia absoluta de segurança.

---

12. Security Controller

O Security Controller permanece independente do Integration Gateway.

Responsabilidades incluem:

- identidade;
- autenticação;
- autorização;
- políticas;
- revogação;
- avaliação de risco;
- controle de privilégios;
- contenção.

Fluxo:

Execution Request
        ↓
Security Controller
        ↓
Authorization Decision
        ↓
Integration Gateway

O Gateway não pode simplesmente conceder autorização porque uma integração está registrada.

---

13. Capability ≠ Permission

Uma Capability diz:

«"A Yuki possui esta habilidade."»

Uma Permission diz:

«"Esta execução específica está autorizada neste contexto."»

Exemplo:

Capability:
email.send

Permission:
email.send
→ conta X
→ destinatário Y
→ escopo Z
→ contexto atual
→ validade T

A Capability pode existir permanentemente enquanto a Permission pode ser:

- temporária;
- contextual;
- limitada;
- revogada;
- específica para uma execução.

---

14. Capability Token

Quando necessário, uma autorização pode ser representada por um artefato temporário.

Capability
     +
Permission
     +
Context
     ↓
Capability Token

O token pode carregar restrições como:

- ação;
- integração;
- escopo;
- audiência;
- validade;
- identidade;
- recurso;
- contexto.

O token não substitui o Security Controller.

Ele representa uma autorização já concedida.

---

15. Credenciais e Segredos

Modelos, agentes e prompts não devem receber credenciais brutas.

Regra oficial:

«Raw credentials are never exposed to models.»

Evitar:

Model
 ↓
API_KEY="..."

Preferir:

Model
 ↓
Authorized Request
 ↓
Credential Broker / Egress
 ↓
Secret
 ↓
External System

Quando tecnicamente possível, o componente que possui a lógica da integração deve permanecer inconsciente do segredo bruto.

O sistema pode utilizar:

- tokens temporários;
- workload identity;
- secret vault;
- credential broker;
- egress proxy;
- hardware-backed secrets;
- mecanismos futuros equivalentes.

Tecnologias específicas permanecem decisões futuras.

---

16. Credential Broker

O Credential Broker é responsável pela entrega controlada de credenciais ou material de autenticação.

Responsabilidades possíveis:

- localizar segredo;
- validar autorização;
- emitir token temporário;
- limitar escopo;
- limitar audiência;
- controlar validade;
- revogar;
- registrar evidências;
- impedir exposição desnecessária.

O Credential Broker não substitui o Security Controller.

---

17. Connector

Connector representa o conhecimento de domínio necessário para interagir com determinado sistema.

Exemplo:

GitHub Connector

pode compreender:

- repositories;
- issues;
- pull requests;
- branches;
- releases;
- usuários;
- permissões específicas do GitHub.

O Connector transforma conceitos da Yuki em conceitos do sistema externo.

---

18. Adapter

Adapter traduz interfaces técnicas.

Exemplos:

REST Adapter
GraphQL Adapter
gRPC Adapter
MQTT Adapter
WebSocket Adapter
SSE Adapter
SOAP Adapter
Custom Protocol Adapter

A arquitetura não depende de um protocolo específico.

---

19. Canonical Data Model

Quando necessário, uma integração pode utilizar um modelo intermediário:

Yuki Domain Model
       ↓
Canonical Data Model
       ↓
Provider Mapping
       ↓
Protocol Adapter
       ↓
External System

Isso evita que detalhes de um fornecedor vazem diretamente para o Core.

O modelo canônico deve ser usado somente quando trouxer benefício real.

Não deve se tornar uma camada obrigatória para todas as integrações.

---

20. External Data

Dados externos devem ser tratados como:

UNTRUSTED_EXTERNAL

até serem devidamente processados.

Exemplos:

- páginas web;
- emails;
- PDFs;
- APIs;
- mensagens;
- issues;
- documentos;
- eventos;
- arquivos;
- sensores;
- respostas de sistemas externos.

Esses dados podem conter:

- instruções maliciosas;
- prompt injection;
- conteúdo enganoso;
- dados incorretos;
- comandos disfarçados;
- payloads malformados.

---

21. Data ≠ Instruction

Uma regra arquitetural fundamental:

«Receber dados externos não concede autoridade aos dados externos.»

Fluxo:

External Data
 ↓
Parsing
 ↓
Schema Validation
 ↓
Provenance
 ↓
Trust Label
 ↓
Instruction/Data Separation
 ↓
Model Exposure Controls
 ↓
Independent Authorization
 ↓
Execution Policy

Mesmo que um modelo interprete um dado externo como instrução, isso não concede permissão para executar a ação.

A autorização continua sendo determinada independentemente.

---

22. Prompt Injection

Sanitização isolada não é considerada defesa suficiente.

A proteção deve ser composta por camadas:

Input
 ↓
Parsing
 ↓
Validation
 ↓
Provenance
 ↓
Trust Classification
 ↓
Context Isolation
 ↓
Model Handling
 ↓
Security Policy
 ↓
Authorization
 ↓
Execution

A representação semântica de dados externos pode utilizar estruturas equivalentes a:

<external_data>
...
</external_data>

mas essa marcação não é considerada uma fronteira de segurança por si só.

---

23. Read vs Write

Integrações devem diferenciar semanticamente:

READ

de:

WRITE

Reads normalmente não produzem efeitos externos significativos.

Writes podem produzir:

- alteração de dados;
- comunicação;
- movimentação financeira;
- alteração de configuração;
- criação de recursos;
- exclusão;
- ação física.

Portanto, writes exigem avaliação adicional.

---

24. Risk

O risco de uma operação não deve depender exclusivamente de seu nome ou tipo.

A avaliação pode considerar:

Action
+
Data
+
Target
+
Account
+
Scope
+
Reversibility
+
Financial Impact
+
Physical Impact
+
User Policy
+
Context

Categorias operacionais iniciais:

R1 — Public Read
R2 — Sensitive Read
R3 — Low-Risk Write / Reversible
R4 — High-Risk Write / Irreversible
R5 — Physical / Critical Financial

Essas categorias são uma taxonomia operacional inicial.

O risco real continua contextual.

---

25. Risk ≠ Permission

Uma operação ser classificada como de baixo risco não significa que esteja automaticamente autorizada.

Da mesma forma, uma operação de alto risco pode ser permitida por uma política explícita e devidamente limitada.

O fluxo é:

Risk Assessment
+
Policy
+
Authorization
+
Context
+
Limits
+
Verification

---

26. Human-in-the-Loop

Operações de alto impacto podem exigir aprovação humana.

Entretanto:

«HIGH RISK não significa obrigatoriamente confirmação interativa em todos os casos.»

Uma política previamente autorizada pode permitir determinadas ações automáticas dentro de limites definidos.

Exemplo conceitual:

Leak Detected
 ↓
Water Valve
 ↓
Pre-authorized Safety Policy
 ↓
Automatic Closure

A autonomia deve depender da política previamente estabelecida.

---

27. Physical Systems

Integrações físicas exigem uma separação ainda mais forte.

A arquitetura recomendada é:

Yuki
 ↓
High-Level Command
 ↓
Robotics Controller
 ↓
Safety Controller
 ↓
Low-Level Control Loop
 ↓
Hardware

A Yuki pode dizer:

«"Pegue o objeto."»

O controlador robótico é responsável por:

- posição;
- velocidade;
- torque;
- trajetória;
- colisão;
- estabilidade;
- limites físicos;
- parada de emergência.

Regra:

«Yuki não deve ser o único mecanismo de segurança de um sistema físico.»

---

28. Verification

Executar uma operação não significa que ela teve o efeito esperado.

Portanto:

Execution
 ↓
Verification
 ↓
Result

As estratégias possíveis incluem:

Direct Read
Event Confirmation
Provider Receipt
Independent Observation
Sensor Confirmation
Cryptographic Receipt
Multi-Source Verification
No Direct Verification

Resultado:

VERIFIED
PARTIALLY_VERIFIED
UNVERIFIED
FAILED
UNKNOWN

A estratégia deve ser escolhida de acordo com a natureza da operação.

---

29. Read-After-Write

Read-after-write é uma possível estratégia de verificação, não uma regra universal.

Exemplo:

WRITE
 ↓
READ
 ↓
Compare Expected vs Observed
 ↓
VERIFIED

Mas alguns sistemas podem utilizar:

- eventos;
- receipts;
- callbacks;
- sensores;
- confirmação criptográfica;
- consistência eventual;
- observação independente.

---

30. Idempotência

Operações que podem produzir efeitos externos devem possuir semântica de idempotência quando aplicável.

O modelo conceitual é:

Execution Request
├── operation_id
├── causation_id
├── idempotency_key
└── attempt

A forma de gerar o "idempotency_key" não é constitucionalmente fixada.

Ela pode depender:

- da semântica da operação;
- do provider;
- do sistema;
- da integração;
- do protocolo.

Duas operações com os mesmos parâmetros podem ser operações legítimas diferentes.

Portanto, parâmetros idênticos não significam automaticamente mesma operação.

---

31. Retry

Retries dependem da semântica da operação.

Read

Transient Failure
 ↓
Retry Candidate

Write idempotente

Transient Failure
 ↓
Idempotency Confirmed
 ↓
Retry Candidate

Write não idempotente

Unknown Result
 ↓
DO NOT AUTO-RETRY

O objetivo é evitar:

timeout
 ↓
retry
 ↓
duplicate action

---

32. Workflow e Saga

O Integration Gateway não deve se tornar um Workflow Engine.

Uma Integration pode fornecer:

execute
verify
compensate

quando suportado.

Mas a orquestração de múltiplas operações pertence ao sistema de:

- Agents;
- Tasks;
- Mission;
- Workflow.

Portanto:

Integration
= execution boundary

Agent / Workflow
= orchestration

---

33. Integration Isolation

Toda Integration deve possuir uma fronteira de isolamento proporcional ao risco e aos requisitos técnicos.

A arquitetura não fixa um único mecanismo.

Possíveis níveis:

Low
→ restricted process

Medium
→ container / restricted runtime / Wasm

High
→ stronger sandbox

Critical
→ MicroVM / dedicated execution domain / hardware isolation

A escolha depende de:

- risco;
- linguagem;
- sistema operacional;
- acesso à rede;
- acesso a arquivos;
- privilégios;
- performance;
- hardware;
- sensibilidade;
- impacto potencial.

---

34. Sandbox Abstraction

A arquitetura deve depender de uma abstração:

Sandbox Interface
├── Process Isolation
├── Container
├── WASM
├── MicroVM
├── Dedicated Runtime
└── Future Isolation Technology

Nenhuma tecnologia específica é constitucionalmente obrigatória.

---

35. Integration Failure Isolation

Uma Integration comprometida ou defeituosa não deve comprometer toda a Yuki.

Integration A
     X
     │
     ▼
[Contained Failure]

Integration B
Integration C
Yuki Core
Security Controller
Memory

Falhas devem ser isoláveis.

Uma integração pode ser:

- suspensa;
- revogada;
- isolada;
- colocada em quarentena;
- reiniciada;
- substituída.

---

36. Lifecycle

O lifecycle da integração é separado de outras dimensões.

Lifecycle

DISCOVERED
REGISTERED
ACTIVE
SUSPENDED
QUARANTINED
DISABLED
REMOVED

Health

HEALTHY
DEGRADED
UNHEALTHY
UNKNOWN

Trust

TRUSTED
SUSPECTED
UNTRUSTED

Authorization

AUTHORIZED
RESTRICTED
REVOKED

Runtime

RUNNING
STOPPED
FAILED

Esses estados não devem ser fundidos em uma única máquina de estados gigante.

---

37. Integration Manifest

O Manifest deve representar informações relativamente estáveis.

Exemplo:

Integration Manifest
├── Identity
├── Version
├── Provider
├── Contract
├── Supported Capabilities
├── Runtime Requirements
├── Network Requirements
└── Lifecycle Metadata

Informações dinâmicas permanecem separadas:

Policy
Permissions
Secrets
Health
Telemetry
Runtime State
Allocation

Regra:

«Manifest ≠ Telemetry ≠ State ≠ Allocation.»

---

38. Event Integration

Integrações podem receber e enviar eventos.

Arquitetura:

External System
 ↓
Event Adapter
 ↓
Event Gateway / Event System
 ↓
Event Router
 ↓
Yuki

Eventos externos permanecem sujeitos às mesmas regras de:

- confiança;
- proveniência;
- validação;
- autorização;
- isolamento.

---

39. Protocol Independence

A Yuki não deve assumir que existe um único protocolo universal.

A arquitetura deve permitir:

REST
GraphQL
gRPC
SOAP
MQTT
WebSocket
SSE
Custom Protocols
Future Protocols

através de abstrações e adapters.

Tecnologias específicas permanecem substituíveis.

---

40. Event Envelope

A arquitetura pode utilizar uma abstração:

Event Envelope Interface

Implementações futuras podem utilizar padrões como:

- CloudEvents;
- sistemas próprios;
- brokers específicos;
- protocolos locais.

CloudEvents, NATS e outras tecnologias são candidatas de implementação, não dependências constitucionais.

---

41. Integration Registry

O Integration Registry mantém o catálogo de integrações conhecidas.

Pode armazenar:

- Definition;
- Instances;
- capabilities;
- provider;
- versões;
- requisitos;
- políticas associadas;
- estado;
- health;
- trust;
- compatibilidade;
- runtime;
- histórico.

O Registry não concede autorização automaticamente.

---

42. Integration Discovery

Uma nova integração deve passar por:

Discovery
 ↓
Identity
 ↓
Integrity
 ↓
Trust / Attestation
 ↓
Capability Advertisement
 ↓
Adapter
 ↓
Compatibility
 ↓
Sandbox / Test
 ↓
Policy Review
 ↓
Registration
 ↓
Activation

Descoberta não significa confiança.

Registro não significa autorização ilimitada.

---

43. Provider Independence

O Core não deve depender diretamente de:

- APIs proprietárias;
- SDK específico;
- fornecedor específico;
- protocolo específico;
- linguagem específica.

Arquitetura:

Yuki Core
 ↓
Capability
 ↓
Integration Contract
 ↓
Provider Adapter
 ↓
Provider

Isso permite substituir:

Provider A
→ Provider B

sem reescrever o Core.

---

44. Graceful Degradation

Se uma integração estiver indisponível, a Yuki deve degradar de forma controlada.

Exemplo:

Primary Integration
      ↓
   unavailable
      ↓
Alternative Integration
      ↓
available
      ↓
continue

ou:

No Alternative
 ↓
Partial Result
 ↓
Explain Limitation

A Yuki não deve inventar que uma operação foi executada.

---

45. Observabilidade

Integrações devem produzir evidências suficientes para compreender:

- execução;
- latência;
- erros;
- retries;
- timeouts;
- autorização;
- provider;
- operação;
- verification;
- consumo de recursos;
- estado;
- incidentes.

A observabilidade deve respeitar:

- privacidade;
- minimização de dados;
- proteção de segredos;
- políticas de retenção.

---

46. Audit

A auditoria deve registrar evidências importantes.

Exemplo:

Who
What
When
Why
Which Capability
Which Integration
Which Resource
Which Authorization
Which Policy
Which Result
Which Verification

Segredos não devem ser gravados no log.

O Audit System permanece arquiteturalmente separado do Gateway.

---

47. Rate Limits

Integrações podem possuir:

- limites por segundo;
- limites diários;
- limites de custo;
- limites de concorrência;
- limites de dados;
- limites de operações;
- limites de impacto.

Esses limites podem existir em diferentes camadas:

User Policy
Security
Capability
Integration
Provider
Resource

---

48. Resource Management

O Integration Gateway não deve administrar diretamente toda a infraestrutura necessária para executar integrações.

Quando necessário:

Integration
 ↓
Resource Requirements
 ↓
Resource Manager
 ↓
Execution Runtime

Isso mantém consistência com o ADR-006.

---

49. Security Boundaries

Principais fronteiras:

                ┌─────────────────────┐
                │     YUKI CORE       │
                └──────────┬──────────┘
                           │
                    Capability
                           │
                ┌──────────▼──────────┐
                │ Security Controller │
                └──────────┬──────────┘
                           │
                      Authorized
                           │
                ┌──────────▼──────────┐
                │ Integration Gateway │
                └──────────┬──────────┘
                           │
                ┌──────────▼──────────┐
                │ Integration Runtime │
                └──────────┬──────────┘
                           │
                ┌──────────▼──────────┐
                │ External System     │
                └─────────────────────┘

Nenhum componente isolado deve possuir poder suficiente para comprometer toda a Yuki.

---

50. Core Direct Access Rule

Regra oficial:

«O Yuki Core não deve realizar comunicação direta com sistemas externos.»

Isso inclui evitar:

Core
 ↓
HTTP request
 ↓
External API

O fluxo correto é:

Core
 ↓
Capability
 ↓
Security
 ↓
Integration Gateway
 ↓
Integration Runtime
 ↓
External System

---

51. No Direct Model Authority

Modelos não podem:

- escolher livremente credenciais;
- criar permissões;
- alterar políticas;
- ignorar o Security Controller;
- acessar diretamente APIs protegidas;
- determinar que uma ação foi autorizada;
- elevar seus próprios privilégios.

O modelo pode propor uma ação.

A arquitetura decide se ela pode ser executada.

---

52. Integration Security Principle

A segurança da integração deve seguir:

Least Privilege
+
Least Agency
+
Isolation
+
Explicit Authorization
+
Credential Isolation
+
Data/Instruction Separation
+
Verification
+
Auditability
+
Revocation

---

53. Integration and Evolution

O Evolution Manager pode:

- descobrir novas integrações;
- pesquisar APIs;
- criar protótipos;
- desenvolver adapters;
- testar connectors;
- avaliar segurança;
- comparar alternativas;
- preparar instalação.

Porém:

«Criar uma integração não significa conceder automaticamente autorização para utilizá-la.»

Mudanças críticas devem passar pelo processo de evolução apropriado.

---

54. Development Lab

Novas integrações podem ser desenvolvidas no:

Yuki Development Lab

Fluxo:

Research
 ↓
Prototype
 ↓
Sandbox
 ↓
Tests
 ↓
Security Evaluation
 ↓
Compatibility
 ↓
Approval
 ↓
Registry
 ↓
Production

O ambiente de desenvolvimento deve ser isolado do ambiente de produção.

---

55. Future Technology Independence

A arquitetura deve permitir futuras tecnologias sem reestruturação do Core.

Exemplos futuros:

- novos protocolos;
- novos modelos de identidade;
- novos runtimes;
- novos mecanismos de sandbox;
- novos dispositivos;
- novas redes;
- novos sistemas físicos;
- novos paradigmas computacionais.

A Yuki deve poder adaptar-se através de:

Interfaces
Contracts
Adapters
Gateways
Capability Registry
Integration Runtime

---

56. Anti-Patterns

A arquitetura deve evitar:

56.1 Core → API direta

Core
 ↓
API

56.2 Credenciais em prompts

"Use esta API key..."

56.3 Dados externos como comandos

External Data
 ↓
Model
 ↓
Automatic Execution

56.4 Retry indiscriminado

Unknown Write Result
 ↓
Retry
 ↓
Duplicate Effect

56.5 Gateway como sistema absoluto

Gateway
= Security
= Workflow
= Secrets
= Resources
= Audit
= Everything

56.6 HTTP 200 como prova de sucesso

HTTP 200
≠
Desired Effect Verified

56.7 Integração confiável por padrão

Installed
≠
Trusted
≠
Authorized

---

57. Relação com outros Domínios

Domínio 07 — Agents & Tasks

Responsável por:

- missão;
- workflow;
- tarefas;
- DAG;
- orquestração;
- retries em nível de missão;
- compensações;
- coordenação.

Integration não substitui Agent System.

---

Domínio 08 — Capability System

Define:

- capacidades;
- contratos;
- permissões;
- capability registry.

Integration implementa caminhos concretos para capacidades externas.

---

Domínio 09 — Model Router

Define:

- seleção de modelos;
- colaboração entre modelos;
- execução cognitiva.

Model Router não recebe autorização para executar integrações diretamente.

---

Domínio 10 — Security

Define:

- identidade;
- autorização;
- política;
- isolamento;
- trust;
- auditoria;
- contenção.

Integration permanece subordinada a essas regras.

---

Domínio 11 — Evolution

Define:

- descoberta;
- melhoria;
- prototipagem;
- testes;
- deployment;
- rollback.

---

Domínio 13 — Events & Background

Define:

- eventos;
- watchers;
- background execution;
- scheduling;
- durable execution.

Integrações podem produzir e consumir eventos.

---

Domínio 14 — Infrastructure

Define:

- recursos;
- execução;
- runtimes;
- compute;
- storage;
- network;
- resource management.

Integration declara requisitos, mas não substitui o Resource Manager.

---

58. Decisões Arquiteturais Fechadas

D15-1

Integration é uma abstração própria, distinta de Capability, Tool, Plugin, Connector e Adapter.

D15-2

O Core não comunica diretamente com sistemas externos.

D15-3

Autorização é independente da Integration.

D15-4

Credenciais brutas nunca são expostas a modelos.

D15-5

Dados externos não recebem autoridade simplesmente por serem recebidos.

D15-6

Toda Integration deve possuir uma fronteira de isolamento proporcional ao risco e aos requisitos técnicos.

D15-7

Read e Write possuem semânticas diferentes.

D15-8

Efeitos externos importantes devem possuir estratégia apropriada de verificação.

D15-9

Operações com efeitos externos devem possuir semântica de idempotência quando aplicável.

D15-10

Falha de uma Integration não deve comprometer toda a Yuki.

D15-11

Integration não é Workflow Engine.

D15-12

Integration não é Security Controller.

D15-13

Integration não é Resource Manager.

D15-14

Detalhes específicos de providers não devem vazar para o Core quando isso puder ser evitado.

D15-15

Integrações devem poder ser suspensas, revogadas, isoladas ou colocadas em quarentena.

---

59. Decisões Ainda Abertas

As seguintes decisões não devem ser congeladas neste documento:

- Wasm vs Container vs MicroVM;
- mecanismo definitivo de sandbox;
- NATS vs Kafka vs outro Event Bus;
- SPIFFE/SPIRE;
- Vault ou outro Credential Broker;
- implementação de hardware-backed secrets;
- gRPC;
- Protobuf;
- CloudEvents obrigatório;
- OpenAPI obrigatório;
- algoritmo de risk scoring;
- algoritmo de idempotency key;
- implementação do Verification System;
- modelo canônico definitivo;
- tecnologia definitiva do Integration Gateway;
- tecnologia definitiva do Integration Runtime.

Essas decisões devem ser tratadas em ADRs específicos.

---

60. ADRs Futuros

ADR-007 — Integration Model & Manifest

Definir formalmente:

- Integration Definition;
- Integration Instance;
- Manifest;
- Provider;
- External System;
- versionamento;
- compatibilidade.

---

ADR-008 — Credential Isolation & Secret Delivery

Definir:

- Credential Broker;
- secret lifecycle;
- token exchange;
- secret injection;
- workload identity;
- revocation;
- hardware-backed credentials.

---

ADR-009 — External Action Verification

Definir:

- verification strategies;
- verification levels;
- receipts;
- events;
- read-after-write;
- sensor verification;
- unknown outcomes.

---

ADR-010 — Integration Execution Isolation

Definir:

- sandbox levels;
- process isolation;
- containers;
- Wasm;
- MicroVM;
- dedicated execution;
- runtime trust.

---

ADR-011 — Integration Identity & Authorization

Definir:

- integration identity;
- instance identity;
- capability tokens;
- permissions;
- scopes;
- audience;
- delegation;
- revocation.

---

ADR-012 — External Data / Instruction Boundary

Definir:

- external-data envelope;
- provenance;
- trust labeling;
- prompt injection defenses;
- context isolation;
- model exposure;
- instruction/data separation.

---

61. Arquitetura Consolidada

A arquitetura final do Domínio 15 é:

                         USER
                           │
                           ▼
                        YUKI
                           │
                           ▼
                    ┌─────────────┐
                    │ YUKI CORE   │
                    └──────┬──────┘
                           │
                           ▼
                      CAPABILITY
                           │
                           ▼
                  EXECUTION REQUEST
                           │
                           ▼
               ┌───────────────────────┐
               │ SECURITY CONTROLLER   │
               │ POLICY / AUTHORIZATION│
               └───────────┬───────────┘
                           │
                     authorized
                           │
                           ▼
                ┌──────────────────────┐
                │ INTEGRATION GATEWAY  │
                └──────────┬───────────┘
                           │
             ┌─────────────┼─────────────┐
             ▼             ▼             ▼
       Credential       Protocol       Event
         Broker          Adapter       Adapter
             │             │             │
             └─────────────┼─────────────┘
                           ▼
                 INTEGRATION RUNTIME
                           │
                    ┌──────┴──────┐
                    ▼             ▼
                Connector       Adapter
                    │             │
                    └──────┬──────┘
                           ▼
                   EXTERNAL SYSTEM
                           │
                           ▼
                      VERIFICATION
                           │
                           ▼
                         RESULT
                           │
             ┌─────────────┼─────────────┐
             ▼             ▼             ▼
           Audit         Memory        Events

---

62. Princípios Oficiais do Domínio 15

Integration Independence

Integrações são abstrações próprias.

Security by Isolation

Integrações externas devem possuir isolamento proporcional ao risco.

No Direct Core Access

O Core não acessa sistemas externos diretamente.

Capability ≠ Permission

Ter uma capacidade não significa estar autorizado a executá-la.

Data is Never Instruction

Dados externos não recebem autoridade automaticamente.

No Raw Credentials to Models

Modelos nunca recebem credenciais brutas.

Verified Side Effects

Efeitos externos importantes devem ser verificados adequadamente.

Idempotent When Applicable

Operações com efeitos externos devem possuir semântica de idempotência quando necessário.

Failure Isolation

Falhas externas não devem comprometer toda a Yuki.

Human Authority

Impactos críticos permanecem sob políticas e autoridade humana apropriadas.

Provider Independence

O Core não deve depender de um fornecedor específico.

Protocol Independence

O Core não deve depender de um protocolo específico.

Future Capability

A arquitetura deve permitir tecnologias e sistemas ainda desconhecidos.

---

63. Regras Normativas

As seguintes regras são consideradas oficiais:

1. Core não acessa sistemas externos diretamente.

2. Capability não implica Permission.

3. Tool não implica autorização.

4. Plugin não é automaticamente confiável.

5. Integration não recebe confiança automaticamente.

6. External Data não é Instruction.

7. Model Output não é Authorization.

8. Raw Credentials não chegam ao Model.

9. Writes possuem tratamento diferente de Reads.

10. Efeitos importantes devem possuir Verification Strategy.

11. Writes não idempotentes não devem ser automaticamente repetidos
    quando o resultado da execução é desconhecido.

12. Integration Failure deve ser isolável.

13. Security Controller permanece independente.

14. Integration Gateway não deve se tornar um God Gateway.

15. Integration não substitui Workflow.

16. Integration não substitui Resource Management.

17. Tecnologias de implementação permanecem substituíveis.

18. Integrações podem ser revogadas ou colocadas em quarentena.

19. Sistemas físicos devem possuir mecanismos de segurança
    independentes da Yuki.

20. O sistema deve preservar a autoridade humana sobre efeitos críticos.

---

64. Relação com a Arquitetura Geral da Yuki

O Domínio 15 completa uma parte importante da fronteira entre a Yuki e o mundo externo.

A arquitetura geral passa a ser:

                         USER
                          │
                          ▼
                    YUKI ACCESS
                          │
                          ▼
                    PERCEPTION
                          │
                          ▼
                  CONTEXT BUILDER
                          │
                          ▼
                 ┌────────────────┐
                 │   YUKI CORE    │
                 └───────┬────────┘
                         │
              ┌──────────┼──────────┐
              ▼          ▼          ▼
           MEMORY     REASONING   PLANNING
              │          │          │
              └──────────┼──────────┘
                         ▼
                    CAPABILITIES
                         │
                         ▼
                    AGENT SYSTEM
                         │
                         ▼
                  SECURITY CONTROLLER
                         │
             ┌───────────┼────────────┐
             ▼           ▼            ▼
        MODEL ROUTER  RESOURCE     INTEGRATION
                      MANAGER       GATEWAY
                         │            │
                         ▼            ▼
                    EXECUTION     EXTERNAL
                         │          SYSTEMS
                         │            │
                         └─────┬──────┘
                               ▼
                          VERIFICATION
                               │
                               ▼
                            RESULT
                               │
                    ┌──────────┼──────────┐
                    ▼          ▼          ▼
                 MEMORY      AUDIT      EVENTS
                    │
                    ▼
                EVOLUTION

---

65. Status

Domínio 15 — INTEGRATIONS v1.0

Status: ACCEPTED

A arquitetura conceitual está oficialmente definida.

As escolhas tecnológicas específicas permanecem abertas e deverão ser decididas por ADRs individuais.

A evolução futura deste domínio deve preservar:

- independência do Core;
- separação entre capacidade e autorização;
- isolamento;
- proteção de credenciais;
- separação entre dados e instruções;
- verificação;
- reversibilidade quando possível;
- observabilidade;
- autoridade humana;
- independência tecnológica.

Fim do Documento — Domínio 15