YUKI — IMPLEMENTATION SPEC FOUNDATION v0.1

Status: ACTIVE
Versão: 0.1
Data: 2026-09-26
Fase: Foundation / MVP-0
Projeto: Yuki Personal AI Platform

---

1. Objetivo

Este documento transforma a arquitetura definida nos ADRs e documentos fundamentais da Yuki em uma especificação operacional para a primeira implementação executável.

A finalidade desta fase não é implementar toda a arquitetura futura.

A finalidade é construir uma primeira Yuki funcional que:

1. inicialize corretamente;
2. receba uma entrada do usuário;
3. construa um contexto mínimo;
4. processe a solicitação através de uma camada cognitiva;
5. identifique uma capacidade;
6. passe por autorização;
7. execute uma operação segura;
8. verifique o resultado;
9. registre os eventos relevantes;
10. produza uma resposta ao usuário;
11. possua testes automatizados;
12. mantenha fronteiras arquiteturais compatíveis com as próximas fases.

A MVP-0 deve ser pequena, porém estruturalmente correta.

---

2. Princípio da MVP

A Yuki será construída por evolução incremental.

MVP-0
  ↓
Yuki funcional mínima
  ↓
MVP-1
  ↓
mais capacidades
  ↓
MVP-2
  ↓
memória / ferramentas / agentes
  ↓
Runtime avançado
  ↓
Workflows / eventos / background
  ↓
Home / Cloud / dispositivos
  ↓
Evolução controlada
  ↓
Yuki futura

A existência da MVP não significa que sua arquitetura seja descartável.

A implementação inicial deve possuir contratos suficientemente bons para permitir evolução sem reescrever o sistema inteiro.

---

3. O que a MVP-0 NÃO será

A MVP-0 não implementará ainda:

- controle residencial;
- robótica;
- pagamentos;
- movimentações financeiras;
- câmeras;
- automação física;
- execução arbitrária de comandos no sistema operacional;
- auto-modificação irrestrita;
- agentes autônomos irrestritos;
- workflows distribuídos;
- sincronização Home/Cloud;
- reconciliação completa;
- memória semântica avançada;
- knowledge graph;
- infraestrutura distribuída;
- execução em múltiplos dispositivos;
- sistema de segurança físico;
- privilégios administrativos automáticos.

Esses componentes permanecem previstos na arquitetura futura, mas serão adicionados progressivamente.

---

4. Primeiro objetivo funcional

A primeira Yuki deve conseguir realizar um fluxo mínimo completo:

USER
  │
  ▼
INPUT
  │
  ▼
YUKI CORE
  │
  ▼
CONTEXT
  │
  ▼
MODEL / REASONING
  │
  ▼
CAPABILITY PROPOSAL
  │
  ▼
POLICY / AUTHORIZATION
  │
  ▼
EXECUTION
  │
  ▼
VERIFICATION
  │
  ▼
RESULT
  │
  ▼
AUDIT / EVENT
  │
  ▼
USER

O objetivo é provar que a arquitetura funciona como sistema integrado.

---

5. Primeira capacidade

A primeira capacidade oficial da Yuki deve ser deliberadamente segura.

Capability

system.echo

Objetivo

Receber um texto e devolvê-lo através do pipeline oficial da Yuki.

Exemplo:

Usuário:
"Olá Yuki"

Yuki:
"Olá! Estou funcionando."

Ou:

Usuário:
"Repita: teste da Yuki"

Yuki:
"teste da Yuki"

Apesar de simples, essa capacidade deve atravessar as fronteiras arquiteturais oficiais.

Ela não deve ser implementada como:

if input == ...
    print(...)

fora do sistema.

Ela deve demonstrar:

Input
→ Core
→ Context
→ Capability
→ Authorization
→ Execution
→ Verification
→ Result

---

6. Primeiro modelo cognitivo

A arquitetura deve definir uma interface abstrata para modelos.

Exemplo conceitual:

ModelProvider
    ├── generate()
    ├── metadata()
    └── health()

A Yuki Core não deve depender diretamente de:

- Gemini;
- GPT;
- Claude;
- Llama;
- qualquer outro modelo específico.

O modelo deve ser um adapter.

Yuki Core
    │
    ▼
Model Router
    │
    ▼
Model Adapter
    │
    ├── Gemini
    ├── GPT
    ├── Local Model
    └── Future Model

Na primeira implementação pode existir apenas um provider real.

Isso não transforma esse provider em parte constitucional da Yuki.

---

7. Estratégia de implementação do modelo

A implementação deve possuir dois modos.

7.1 Mock Provider

Usado para:

- testes;
- desenvolvimento;
- CI;
- testes determinísticos;
- execução sem internet;
- validação arquitetural.

MockModelProvider

7.2 Provider real

Um adapter separado poderá posteriormente conectar um modelo externo.

Exemplo:

ExternalModelProvider

O provider real nunca deve ser necessário para executar os testes fundamentais da arquitetura.

---

8. Tecnologia inicial

A implementação inicial utilizará uma stack pequena e substituível.

Linguagem principal

Rust

Motivos:

- forte modelagem de contratos;
- segurança de memória;
- bom suporte a concorrência;
- bom desempenho;
- excelente adequação para componentes de infraestrutura;
- possibilidade de executar em Cloud, Home Server, Edge e dispositivos;
- menor dependência de runtime externo;
- boa base para componentes de segurança e execução.

Esta escolha é uma decisão de implementação da MVP.

Ela NÃO altera o princípio de independência tecnológica da Yuki.

Rust não é uma dependência constitucional.

---

9. Dependências iniciais

A implementação deve preferir poucas dependências maduras.

Possíveis componentes:

serde
serde_json
thiserror
uuid
tracing
clap
tokio

A lista final deve ser determinada pelo agente de implementação de acordo com a necessidade real.

Não adicionar dependências apenas por conveniência.

Toda dependência nova deve ter justificativa técnica.

---

10. Estrutura inicial

A estrutura inicial deve seguir aproximadamente:

yuki/
├── Cargo.toml
├── README.md
│
├── docs/
│
├── src/
│   ├── main.rs
│   │
│   ├── core/
│   │   ├── mod.rs
│   │   └── yuki_core.rs
│   │
│   ├── contracts/
│   │   ├── mod.rs
│   │   ├── input.rs
│   │   ├── output.rs
│   │   ├── context.rs
│   │   ├── capability.rs
│   │   ├── authorization.rs
│   │   ├── execution.rs
│   │   ├── verification.rs
│   │   └── events.rs
│   │
│   ├── context/
│   │   ├── mod.rs
│   │   └── builder.rs
│   │
│   ├── capabilities/
│   │   ├── mod.rs
│   │   ├── registry.rs
│   │   └── echo.rs
│   │
│   ├── security/
│   │   ├── mod.rs
│   │   └── authorization.rs
│   │
│   ├── execution/
│   │   ├── mod.rs
│   │   └── executor.rs
│   │
│   ├── verification/
│   │   ├── mod.rs
│   │   └── verifier.rs
│   │
│   └── models/
│       ├── mod.rs
│       ├── provider.rs
│       └── mock.rs
│
└── tests/
    ├── contracts.rs
    ├── capability_echo.rs
    ├── authorization.rs
    ├── execution.rs
    └── verification.rs

A estrutura pode ser adaptada pelo agente se houver uma razão técnica clara.

A arquitetura lógica, porém, deve permanecer.

---

11. Contratos fundamentais

A implementação inicial deve definir tipos explícitos para:

UserInput
ContextRequest
ContextObject
ModelRequest
ModelResponse
CapabilityManifest
CapabilityRequest
AuthorizationRequest
AuthorizationDecision
ExecutionRequest
OperationState
EffectState
VerificationResult
YukiResult
AuditEvent

Esses tipos devem ser separados.

Não criar um objeto universal contendo tudo.

---

12. IDs

As operações devem possuir identificadores próprios.

No mínimo:

request_id
operation_id
attempt_id
correlation_id
causation_id

Quando aplicável:

idempotency_key

Não reutilizar um único ID para representar conceitos diferentes.

---

13. Context Object

O Context Builder deve produzir um contexto tipado.

Exemplo conceitual:

ContextObject {
    context_id
    purpose
    timestamp
    source_refs
    user_input
    relevant_data
    provenance
    freshness
    epistemic_state
}

O contexto deve conter apenas os dados necessários para a tarefa.

Não deve carregar:

- credenciais;
- tokens secretos;
- chaves privadas;
- informações sem finalidade;
- estado global completo da Yuki.

---

14. Model Provider

Criar uma interface abstrata:

trait ModelProvider

Ela deve permitir futuramente:

GeminiProvider
OpenAIProvider
LocalProvider
FutureProvider

A Yuki Core deve depender da abstração.

Não depender de uma implementação específica.

---

15. Capability Registry

Criar um registro mínimo de capabilities.

Primeira entrada:

system.echo

Cada capability deve possuir um manifest mínimo contendo:

id
version
description
input_schema
output_schema
required_permissions
risk_class
side_effects

Exemplo conceitual:

system.echo
version: 0.1
risk: LOW
side_effects: NONE
network: NONE
filesystem: NONE
secrets: NONE

---

16. Capability ≠ Permission

A existência de:

system.echo

não significa que qualquer contexto possa executá-la.

O pipeline deve possuir:

Capability
    ↓
Authorization Request
    ↓
Authorization Decision
    ↓
Execution

A capability descreve o que pode ser feito.

A autorização determina se pode ser feito agora.

---

17. Security Controller inicial

A MVP deve possuir uma implementação mínima de autorização.

Ela deve:

1. receber uma solicitação;
2. verificar a capability;
3. verificar o contexto;
4. verificar a política;
5. produzir uma decisão explícita.

Possíveis decisões:

ALLOW
DENY
REQUIRES_APPROVAL

A MVP pode possuir regras simples.

Porém, a API deve permitir evolução para políticas muito mais complexas.

---

18. Regra crítica

Nenhuma capability deve executar diretamente porque o modelo pediu.

Fluxo obrigatório:

Model
   ↓
Proposal
   ↓
Policy
   ↓
Authorization
   ↓
Execution

Portanto:

Model output ≠ command

---

19. Execution Layer

O executor deve receber apenas operações autorizadas.

Exemplo:

ExecutionRequest

contendo:

operation_id
capability_id
authorized_scope
input

O executor não deve decidir sozinho se uma operação é permitida.

Essa decisão pertence ao mecanismo de autorização.

---

20. Verification

Mesmo a operação "echo" deve atravessar Verification.

Exemplo:

Execution
    ↓
Effect
    ↓
Verification

Para a primeira capability:

EffectState:
MUTATED / NO_MUTATION / UNKNOWN

e:

VerificationState:
VERIFIED_SUCCESS
VERIFIED_FAILURE
UNKNOWN

A arquitetura deve preservar a possibilidade de "UNKNOWN".

---

21. UNKNOWN

A implementação não deve transformar:

timeout

automaticamente em:

failure

nem:

success

Quando não for possível determinar o resultado:

UNKNOWN

deve ser um estado válido.

---

22. Audit / Events

A MVP deve registrar eventos estruturados.

Exemplos:

InputReceived
ContextBuilt
ModelInvoked
CapabilityProposed
AuthorizationRequested
AuthorizationGranted
OperationDispatched
EffectObserved
VerificationCompleted
ResponseProduced

Os eventos devem conter:

event_id
timestamp
event_type
correlation_id
causation_id
payload
provenance

Não armazenar secrets nos eventos.

---

23. Persistência inicial

A primeira versão pode utilizar armazenamento local simples.

A camada de persistência deve ser abstrata.

Exemplo:

EventStore
StateStore

O restante do sistema não deve depender diretamente de SQLite ou qualquer banco específico.

A tecnologia de armazenamento pode mudar futuramente.

---

24. CLI inicial

A MVP deve possuir uma interface de terminal.

Exemplo:

yuki

ou:

yuki "Olá Yuki"

Fluxo esperado:

$ yuki "Olá Yuki"

Yuki:
Olá! Estou funcionando.

Também deve existir uma opção de diagnóstico, por exemplo:

yuki health

Resultado conceitual:

Yuki
Status: OK

Core: OK
Context: OK
Model: OK
Capability Registry: OK
Security: OK
Execution: OK
Verification: OK

---

25. Modo Development

A MVP deve possuir um modo explícito de desenvolvimento/teste.

Esse modo pode habilitar:

- Mock Model;
- logs detalhados;
- inspeção de eventos;
- execução de testes;
- diagnóstico.

O modo Development não deve automaticamente conceder privilégios adicionais ao sistema.

---

26. Primeiro teste arquitetural

O teste mais importante da MVP será:

test_vertical_slice_echo

Ele deve provar:

Input
→ Core
→ Context
→ Model
→ Capability
→ Authorization
→ Execution
→ Verification
→ Result

O teste deve verificar também que cada etapa produziu seus identificadores/eventos esperados.

---

27. Testes de segurança obrigatórios

A MVP deve provar pelo menos:

Teste 1

Uma capability inexistente deve ser rejeitada.

Teste 2

Uma capability não autorizada deve ser rejeitada.

Teste 3

Uma resposta do modelo não deve executar uma capability diretamente.

Teste 4

Dados externos não devem ganhar autoridade automaticamente.

Teste 5

Credenciais não podem aparecer no Context Object.

Teste 6

Credenciais não podem aparecer nos logs.

Teste 7

"UNKNOWN" deve permanecer "UNKNOWN" quando não houver evidência suficiente.

Teste 8

A execução só pode ocorrer depois de uma decisão "ALLOW".

---

28. Testes de isolamento

O sistema deve demonstrar que componentes possuem responsabilidades separadas.

Exemplo:

ModelProvider

não deve conseguir:

authorize()
execute()
grant_permission()

O executor não deve conseguir:

grant_permission()

O Context Builder não deve conseguir:

authorize()
execute()

A arquitetura deve ser reforçada também pela estrutura do código.

---

29. Primeiro limite de segurança

A MVP-0 não deve possuir uma ferramenta genérica como:

shell.execute("qualquer comando")

Também não deve possuir:

filesystem.write_anywhere()

ou:

network.request_anything()

Essas capacidades só poderão ser adicionadas posteriormente através do sistema de Capability + Permission + Security + Execution.

---

30. Autoevolução

A MVP-0 não terá auto-modificação autônoma.

Entretanto, sua arquitetura deve permitir que futuramente exista:

Development Lab
    ↓
Research
    ↓
Proposal
    ↓
Prototype
    ↓
Tests
    ↓
Security Review
    ↓
Human Approval
    ↓
Deployment

A futura Yuki poderá ajudar a desenvolver a própria Yuki.

Porém:

Yuki pode propor mudança
≠
Yuki pode aplicar qualquer mudança

---

31. Git como memória de engenharia

Nenhuma decisão arquitetural importante deverá existir somente na conversa.

Alterações importantes devem ser refletidas em:

docs/
decisions/
ADR/
README

Código deve estar acompanhado de testes e documentação quando necessário.

---

32. Reutilização de tecnologia

A Yuki deve reutilizar componentes maduros quando isso reduzir complexidade.

Entretanto:

external implementation
        ≠
Yuki architecture

Um componente externo deve entrar através de adapter ou boundary apropriada.

Isso preserva:

- substituição;
- independência;
- segurança;
- portabilidade;
- evolução.

---

33. Definition of Done — Foundation v0.1

A Foundation v0.1 foi integralmente cumprida e auditada:

- [x] projeto compila (`cargo check`, `cargo build`, `cargo build --release`);
- [x] testes automatizados executam (26/26 testes passando em `tests/`);
- [x] CLI funciona (`src/main.rs` com comandos de prompt e `health`);
- [x] Core existe (`YukiCore` orquestrando o pipeline de ponta a ponta);
- [x] contratos fundamentais existem (módulos tipados em `src/contracts/`);
- [x] Context Builder existe (com minimização e salvaguarda de segredos);
- [x] ModelProvider abstrato existe (trait em `src/models/provider.rs`);
- [x] Mock Provider funciona (`MockModelProvider` determinístico);
- [x] Capability Registry funciona (`CapabilityRegistry` em `src/capabilities/`);
- [x] "system.echo" funciona (`EchoCapability` e vertical slice comprovado);
- [x] Authorization existe (`SecurityController` e `DefaultFoundationPolicy`);
- [x] Execution existe (`Executor` com validação e consumo atômico de tokens);
- [x] Verification existe (`Verifier` atestando evidências e preservando UNKNOWN);
- [x] eventos estruturados existem (`AuditEvent` em `src/contracts/events.rs`);
- [x] logs não expõem secrets (`InMemoryEventStore` com `assert_no_secrets`);
- [x] testes de segurança básicos passam (rejeição de replay, forjamento e chamada não autorizada);
- [x] vertical slice passa (`test_vertical_slice_echo` e `test_vertical_slice_custom_repeat`);
- [x] documentação permanece consistente e auditada;
- [x] nenhum componente possui autoridade indevida (modelo não executa sem token consumível).

---

34. Definition of Done — MVP-0

A MVP-0 foi concluída e validada através de testes automatizados e execução de terminal:

```bash
$ yuki "Olá Yuki"
Olá! Estou funcionando.
```

A solicitação atravessa o pipeline oficial:

User Input
     ↓
Yuki Core
     ↓
Context
     ↓
Reasoning / Model
     ↓
Capability
     ↓
Authorization
     ↓
Execution
     ↓
Verification
     ↓
Audit
     ↓
Response

com 26 testes automatizados demonstrando a integridade das fronteiras arquiteturais.

---

34.1. Limitações Conhecidas e Deferimentos para o MVP-1

Em conformidade com a auditoria de encerramento da Foundation v0.1, os seguintes itens estão explicitamente documentados como simplificações do MVP-0 ou deferimentos formais para o MVP-1:
1. **Credential Broker:** A Foundation v0.1 utiliza heurística preventiva em texto (`assert_no_secrets`). O Credential Broker completo com referências criptográficas e entrega segura (ADR-008) pertence ao MVP-1.
2. **Verification Strategy Dinâmica:** O `Verifier` atual implementa verificação demonstrativa síncrona para `system.echo`. O despacho polimórfico de estratégias por capability e por evidência (ADR-009) será implementado no MVP-1.
3. **Persistência de Eventos:** O `InMemoryEventStore` é puramente em memória; a persistência durável append-only em disco será construída no MVP-1 (ADR-013).
4. **Entropia de Identificadores:** Identificadores e tokens utilizam contadores monotônicos atômicos, suficientes para o processo local em MVP-0, mas sem finalidade criptográfica (CSPRNG/UUIDv7 deferido para MVP-1).
5. **Garbage Collection de Tokens:** O mapa em memória de tokens do `SecurityController` não possui expurgo de tokens abandonados; o ciclo de vida e eviction serão formalizados no runtime contínuo.
6. **Runtime Síncrono:** O runtime da Foundation é inteiramente síncrono; a adoção de Tokio para integração com LLMs de rede reais será realizada no MVP-1.
7. **Visibilidade de `validate_token`:** A API pública `validate_token` é restrita a inspeção de leitura nos testes e não participa do caminho de execução (o `Executor` utiliza obrigatoriamente `validate_and_consume_token`); sua visibilidade será internalizada no MVP-1.
8. **Acoplamento Semântico do Verifier:** O `Verifier` inspeciona a evidência determinística de echo, sendo a generalização para outras capabilities parte do MVP-1.

---

35. Próxima evolução

Depois da MVP-0:

MVP-0
│
├── real Model Adapter
├── memória básica
├── sessões
├── ferramentas seguras
├── Web/Research
├── Capability Registry avançado
├── Model Router
└── interface de conversa

Depois:

MVP-1
│
├── Memory
├── Knowledge
├── Tools
├── Integrations
├── Agents
└── Voice

Depois:

Runtime
│
├── Durable Workflow
├── Events
├── Background Missions
├── Reconciliation
└── Multi-device

Depois:

Distributed Yuki
│
├── Home
├── Cloud
├── Federation
├── Resource Fabric
└── Offline operation

Depois:

Physical Yuki
│
├── Safety
├── Devices
├── Robotics
├── Home Automation
└── Physical-world verification

Depois:

Evolution
│
├── Development Lab
├── Research
├── Prototyping
├── Benchmarking
├── Security Review
└── Controlled Deployment

E além disso poderão existir novas fases que ainda nem foram imaginadas.

---

36. Regra final

A Yuki MVP não deve ser construída como um protótipo descartável.

Ela deve ser construída como:

«a primeira versão pequena de uma plataforma que pode continuar crescendo por muitos anos.»

O objetivo da Foundation v0.1 não é prever tudo.

É criar uma base que permita que a Yuki descubra, através de desenvolvimento controlado e colaboração humana, quais serão seus próximos componentes.

BUILD SMALL
       ↓
PROVE THE ARCHITECTURE
       ↓
USE YUKI
       ↓
LEARN
       ↓
IMPROVE
       ↓
TEST
       ↓
SECURE
       ↓
EXPAND
       ↓
REPEAT

A Yuki não precisa nascer gigante.

Ela precisa nascer corretamente.