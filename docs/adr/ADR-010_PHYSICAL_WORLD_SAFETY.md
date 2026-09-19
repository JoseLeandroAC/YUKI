# ADR-010 — Physical World Safety & Hardware Interlocks

**Status:** ACCEPTED  
**Version:** 1.0  
**Date:** 2026-09-19  
**Domain:** Physical World / Safety / Cyber-Physical Systems  
**File:** `docs/ADR-010_PHYSICAL_WORLD_SAFETY.md`

---

## 1. Objective

Este ADR define os princípios arquiteturais pelos quais a Yuki poderá interagir com o mundo físico de forma controlada, verificável, isolada e segura.

O objetivo não é transformar a Yuki em uma autoridade de segurança física. O objetivo é estabelecer uma arquitetura na qual:

- a Yuki possa solicitar ações físicas;
- ações físicas sejam explicitamente autorizadas;
- comandos sejam submetidos às restrições de segurança aplicáveis;
- funções críticas de segurança permaneçam independentes da Yuki;
- limites físicos possam ser impostos por camadas inferiores;
- falhas da Yuki não eliminem as proteções físicas;
- efeitos físicos sejam observáveis e verificáveis;
- perda de rede, software, sensores ou componentes produza comportamento previamente definido;
- tecnologias futuras possam ser incorporadas sem alterar os princípios fundamentais.

Princípio central:

> **A Yuki pode solicitar. A autorização pode permitir. A segurança pode bloquear. O hardware pode impedir.**

---

# 2. Contexto

A evolução da Yuki poderá incluir sistemas físicos:

- robôs;
- veículos;
- braços robóticos;
- dispositivos domésticos;
- máquinas;
- atuadores;
- portas e fechaduras;
- sistemas de energia;
- equipamentos de laboratório;
- sensores;
- câmeras;
- drones;
- sistemas de automação;
- dispositivos vestíveis;
- infraestrutura residencial;
- futuros sistemas ainda desconhecidos.

Uma ação no mundo digital pode produzir consequências físicas.

Exemplos:

```text
"Abra a porta"
"Movimente o braço"
"Ligue o motor"
"Acione o dispositivo"
"Leve este objeto"
"Feche a válvula"
"Desligue o equipamento"
```

Essas ações não devem ser tratadas como simples chamadas de API.

Existe uma diferença fundamental entre:

```text
intenção
→ comando
→ execução
→ efeito físico
→ observação
→ verificação
```

A arquitetura deve preservar essa separação.

---

# 3. Escopo

Este ADR trata principalmente de:

- autoridade física;
- segurança física;
- Safety Controller;
- Physical Gateway;
- Safe Operating Envelope;
- hardware interlocks;
- emergência;
- emergency stop;
- safe state;
- fail-safe;
- fail-operational;
- sensores;
- atuadores;
- limites físicos;
- orçamento de ações físicas;
- perda de comunicação;
- autonomia local;
- verificação de efeitos físicos;
- isolamento entre Yuki e controle físico crítico;
- testes e simulação;
- segurança contra comprometimento da Yuki;
- relação entre segurança digital e segurança física.

Este ADR não define uma implementação específica de:

- ROS/ROS2;
- DDS;
- RTOS;
- MCU;
- PLC;
- FPGA;
- sistema operacional;
- protocolo industrial;
- barramento específico;
- fabricante;
- padrão SIL específico;
- padrão PL específico;
- tecnologia de Emergency Stop específica.

Essas decisões dependem do sistema físico concreto e poderão ser definidas em ADRs posteriores.

---

# 4. Princípios Fundamentais

## 4.1 Cognitive Authority ≠ Physical Authority

A capacidade da Yuki de raciocinar sobre uma ação não concede autoridade física ilimitada.

```text
Yuki Core
    ≠
Physical Safety Authority
```

A Yuki pode:

- interpretar intenção;
- planejar;
- solicitar;
- selecionar uma capacidade;
- acompanhar uma missão;
- analisar sensores;
- interpretar resultados.

Mas funções críticas de segurança não podem depender exclusivamente da decisão do modelo.

---

## 4.2 Capability ≠ Permission ≠ Safety Authorization

Uma capacidade física significa:

> "A Yuki possui uma forma técnica de solicitar determinada operação."

Isso não significa:

> "A Yuki está autorizada a executá-la."

E autorização também não significa:

> "A operação é fisicamente segura."

Portanto:

```text
Capability
    ↓
Permission
    ↓
Authorization
    ↓
Safety Validation
    ↓
Physical Execution
```

Cada etapa possui responsabilidade distinta.

---

## 4.3 Model Output ≠ Safety Authorization

Nenhum modelo de IA pode transformar sua própria saída em autorização de segurança.

```text
Model
  ↓
Intent / Proposal
  ↓
Policy
  ↓
Authorization
  ↓
Safety Validation
  ↓
Execution
```

Isso permanece válido mesmo quando múltiplos modelos concordam.

Concordância entre modelos não constitui, por si só, evidência de segurança física.

---

## 4.4 Segurança Física Independente

Funções críticas de segurança devem permanecer capazes de funcionar independentemente da disponibilidade da Yuki Core.

A perda de:

- internet;
- cloud;
- banco de dados;
- modelo;
- agente;
- memória;
- gateway;
- processo principal;
- interface;

não pode remover automaticamente as proteções físicas fundamentais.

---

# 5. Modelo de Autoridade Física

A arquitetura adota camadas de autoridade:

```text
                         YUKI
                          │
                    HIGH-LEVEL INTENT
                          │
                          ▼
                  PHYSICAL GATEWAY
                          │
                ┌─────────┴─────────┐
                │                   │
          SECURITY POLICY      PHYSICAL POLICY
                │                   │
                ▼                   ▼
          AUTHORIZATION       SAFETY VALIDATION
                │                   │
                └─────────┬─────────┘
                          ▼
                  LOCAL CONTROLLER
                          │
                          ▼
                  SAFETY ENFORCEMENT
                          │
                ┌─────────┴─────────┐
                ▼                   ▼
        HARDWARE INTERLOCK      ACTUATOR
                │                   │
                └─────────┬─────────┘
                          ▼
                    PHYSICAL WORLD
                          │
                          ▼
                       SENSORS
                          │
                          ▼
                  OBSERVATION LAYER
                          │
                          ▼
                 VERIFICATION ENGINE
```

As camadas inferiores não devem assumir que uma instrução recebida da Yuki é segura simplesmente porque veio da Yuki.

---

# 6. Physical Gateway

O **Physical Gateway** é a fronteira entre as intenções da Yuki e os sistemas físicos.

Responsabilidades:

- receber solicitações físicas;
- validar formato;
- validar identidade;
- validar autorização;
- aplicar políticas;
- aplicar limites;
- aplicar rate limits;
- associar operação a uma identidade;
- associar operação a uma missão;
- aplicar Physical Action Budget;
- impedir comandos incompatíveis com as políticas;
- registrar eventos relevantes;
- encaminhar apenas comandos permitidos.

O Physical Gateway não substitui o Safety Controller.

### Physical Gateway

Responsável principalmente por:

- segurança operacional;
- autorização;
- políticas;
- limites de uso;
- runaway prevention;
- controle de frequência;
- contexto;
- escopo.

### Safety Controller

Responsável principalmente por:

- restrições físicas;
- condições de segurança;
- safe state;
- interlocks;
- emergency handling;
- limites críticos;
- resposta a falhas de segurança.

---

# 7. Safety Controller

O **Safety Controller** é uma autoridade física independente da Yuki Core.

Ele pode ser implementado de maneiras diferentes dependendo do sistema.

Sua arquitetura deve permitir que funções críticas permaneçam protegidas mesmo quando:

- a Yuki está comprometida;
- um modelo está comprometido;
- um agente está em loop;
- uma integração está comprometida;
- a rede caiu;
- a cloud está indisponível;
- sensores externos estão comprometidos;
- comandos incorretos foram produzidos.

O Safety Controller não deve receber autoridade adicional simplesmente por ser chamado de "Safety Controller".

Suas funções e nível de garantia devem ser definidos conforme:

- perigo;
- risco;
- função de segurança;
- equipamento;
- ambiente;
- legislação;
- normas aplicáveis;
- consequências da falha.

Não existe um nível universal de garantia aplicável a todos os sistemas físicos.

---

# 8. Safety Assurance

A Yuki não assume que todo sistema físico precisa do mesmo nível de segurança formal.

O nível de assurance deve ser proporcional ao risco.

Um sistema doméstico de baixo risco pode exigir controles diferentes de:

- uma máquina industrial;
- um robô pesado;
- um sistema médico;
- um veículo;
- um equipamento de laboratório;
- um sistema energético.

Portanto:

> **A garantia de segurança deve ser adequada ao perigo e à função de segurança do sistema.**

Tecnologias e padrões específicos poderão ser definidos em ADRs de domínio.

---

# 9. Safe Operating Envelope

Todo sistema físico relevante deve possuir um **Safe Operating Envelope**.

O envelope representa condições dentro das quais uma operação é permitida.

Pode incluir:

- velocidade máxima;
- força máxima;
- torque;
- temperatura;
- corrente;
- tensão;
- posição;
- aceleração;
- distância;
- área permitida;
- carga;
- estado do equipamento;
- presença humana;
- condições ambientais;
- bateria;
- integridade dos sensores;
- condições de comunicação;
- limites temporais.

Exemplo:

```text
Safe Operating Envelope
├── Spatial Limits
├── Velocity Limits
├── Force Limits
├── Torque Limits
├── Thermal Limits
├── Electrical Limits
├── Environmental Limits
├── Human-Presence Constraints
└── Device-State Constraints
```

O envelope deve ser aplicado em uma camada capaz de impedir a violação.

---

# 10. Enforcement na Camada Mais Baixa Apropriada

Uma restrição crítica deve ser aplicada na camada mais baixa que consiga efetivamente impedi-la.

Exemplo:

```text
Yuki Policy
      ↓
Physical Gateway
      ↓
Local Controller
      ↓
Safety Enforcement
      ↓
Hardware
```

Uma regra que precisa ser garantida fisicamente não deve depender apenas de:

- prompt;
- modelo;
- agente;
- memória;
- interface;
- software de alto nível.

---

# 11. Physical Commands

Todo comando físico deve possuir contexto suficiente para avaliação.

Uma operação pode conter:

```text
operation_id
mission_id
device_id
capability
requested_action
parameters
authorization_context
safety_context
constraints
deadline
priority
causation_id
```

A estrutura final permanecerá aberta para decisões futuras.

---

# 12. Physical Effect ≠ Physical Command

O envio de um comando não prova que o mundo físico mudou.

Portanto:

```text
Command
    ≠
Execution
    ≠
Physical Effect
    ≠
Verified Physical Effect
```

Exemplo:

```text
Yuki: "Fechar válvula"
        ↓
comando enviado
        ↓
controlador recebeu
        ↓
atuador acionado
        ↓
posição mudou?
        ↓
sensor confirma?
        ↓
efeito físico verificado
```

Esta distinção integra diretamente o modelo definido no ADR-009.

---

# 13. Physical Effect Verification

A verificação de efeitos físicos deve utilizar evidências adequadas ao sistema.

Possíveis evidências:

- sensor de posição;
- sensor de força;
- encoder;
- câmera;
- telemetria;
- estado do controlador;
- confirmação do dispositivo;
- múltiplos sensores;
- observação externa;
- evento;
- inspeção humana.

A evidência deve ser analisada segundo:

- autenticidade;
- integridade;
- frescor;
- vínculo com a operação;
- origem;
- confiabilidade;
- cobertura;
- independência, quando relevante.

### Evidência não é verdade absoluta

Uma verificação como `CONFIRMED` significa:

> As evidências disponíveis satisfazem os requisitos da política de verificação aplicável.

Não significa conhecimento metafísico de que o mundo físico está absolutamente correto.

---

# 14. Unknown como Estado de Primeira Classe

A arquitetura deve tratar `UNKNOWN` como estado válido.

Exemplos:

```text
COMMAND_SENT
EFFECT_UNKNOWN
```

ou:

```text
EXECUTION_UNKNOWN
```

ou:

```text
SENSOR_UNAVAILABLE
```

Um estado desconhecido não deve ser convertido silenciosamente em sucesso.

Isso é especialmente importante quando:

- o comando pode ter sido executado;
- a comunicação caiu;
- o sensor parou;
- o retorno foi perdido;
- o dispositivo reiniciou;
- existe possibilidade de execução parcial.

---

# 15. Emergency Stop

Sistemas físicos que possam produzir consequências perigosas devem possuir mecanismos de emergência apropriados ao sistema.

Um Emergency Stop deve:

- ser independente da Yuki;
- permanecer disponível durante falhas da Yuki;
- possuir caminho de acionamento adequado;
- possuir comportamento definido;
- possuir recuperação controlada;
- não depender exclusivamente de cloud;
- não depender de um modelo de IA.

O comportamento exato do Emergency Stop depende do equipamento.

Em alguns sistemas o estado seguro pode ser:

- remoção de torque;
- desligamento;
- fechamento de válvula;
- acionamento de freio;
- parada controlada;
- isolamento de energia;
- redução de velocidade;
- manutenção de posição;
- outra estratégia específica.

Não existe um mecanismo universal de parada aplicável a todos os sistemas.

---

# 16. Hardware Interlocks

Hardware interlocks são mecanismos capazes de impedir determinadas condições perigosas independentemente do fluxo normal da Yuki.

Exemplos conceituais:

```text
Door Open
    ↓
Actuator Inhibited
```

```text
Overtemperature
    ↓
Power Path Disabled
```

```text
Emergency Condition
    ↓
Safety Function
    ↓
Actuator Prevented
```

Interlocks críticos não devem depender exclusivamente do modelo.

---

# 17. Hardware Independence

A proteção física deve permanecer funcional quando possível mesmo em cenários como:

- falha de software;
- corrupção de aplicação;
- crash;
- reboot;
- perda de rede;
- indisponibilidade de cloud;
- comprometimento de credenciais;
- prompt injection;
- agente em loop.

Isso não implica que toda função de segurança precise ser fisicamente cabeada.

A implementação deve ser proporcional ao risco e às características do sistema.

---

# 18. Safe State

Cada sistema físico relevante deve definir seu comportamento seguro.

O safe state pode variar.

Exemplos:

```text
Motor
→ torque removido
```

```text
Porta
→ permanecer fechada
```

```text
Veículo
→ parada controlada
```

```text
Braço robótico
→ posição segura / parada definida
```

```text
Sistema térmico
→ desligamento ou modo de proteção
```

A arquitetura não presume que "desligar tudo" seja sempre o comportamento correto.

---

# 19. Fail-Safe e Fail-Operational

Os sistemas devem definir explicitamente como responderão a falhas.

### Fail-Safe

A falha conduz a um estado considerado seguro.

### Fail-Operational

O sistema continua funcionando de forma controlada apesar de determinada falha.

A escolha depende do risco e da função.

Pode existir também:

```text
Normal
→ Degraded
→ Safe Mode
→ Emergency
```

O comportamento deve ser definido por sistema.

---

# 20. Sensores

Sensores são fontes de evidência e podem apresentar:

- falha;
- atraso;
- ruído;
- spoofing;
- perda de comunicação;
- calibração incorreta;
- dados inconsistentes.

Uma entrada de segurança ausente ou inválida não deve ser tratada automaticamente como válida.

Porém, a resposta à perda de um sensor depende do sistema:

```text
Sensor Failure
├── Safe Stop
├── Degraded Mode
├── Redundant Sensor
├── Fallback Strategy
└── Other Defined Safe Response
```

---

# 21. Sensor Fusion

Quando necessário, múltiplas fontes podem ser utilizadas.

Exemplo:

```text
Camera
Encoder
Force Sensor
Telemetry
      ↓
Observation Layer
      ↓
Evidence Evaluation
      ↓
Verification
```

A concordância entre sensores não deve ser considerada automaticamente suficiente.

A independência e confiabilidade das fontes precisam ser avaliadas conforme o sistema.

---

# 22. Atuadores

Atuadores devem possuir limites independentes sempre que necessário.

Possíveis limites:

- velocidade;
- força;
- torque;
- corrente;
- temperatura;
- posição;
- duty cycle;
- frequência;
- tempo;
- energia.

A Yuki não deve depender exclusivamente de software de alto nível para respeitar limites físicos críticos.

---

# 23. Human Presence

Sistemas que interagem com pessoas podem utilizar sinais de presença humana.

Possíveis fontes:

- sensores;
- câmeras;
- wearables;
- presença de dispositivo;
- sensores físicos;
- sistemas de localização.

Esses sinais são contexto de segurança.

Eles não devem ser tratados automaticamente como identidade ou autorização.

---

# 24. Human Authority

A presença humana não significa que toda operação precisa de aprovação manual.

O modelo é:

```text
Risk / Context
      ↓
Authorization Policy
      ↓
Automatic / Pre-authorized / Human Approval
```

Operações podem ser:

- previamente autorizadas;
- automaticamente permitidas dentro de limites;
- submetidas a aprovação;
- bloqueadas.

Emergências podem exigir respostas previamente autorizadas e automáticas.

---

# 25. Physical Action Budget

A Yuki deve possuir mecanismos contra comportamento runaway.

O orçamento físico pode considerar:

- quantidade de ações;
- frequência;
- energia;
- duração;
- distância;
- força;
- área;
- número de dispositivos;
- custo;
- risco;
- impacto acumulado.

Arquitetura:

```text
Yuki / Mission Budget
        ↓
Physical Gateway Budget
        ↓
Safety Controller Limits
        ↓
Hardware Limits
```

As camadas podem possuir limites independentes.

---

# 26. Runaway Prevention

Exemplos de runaway:

```text
Agent Loop
→ Repetir comando
→ Repetir comando
→ Repetir comando
→ Dano
```

ou:

```text
Sensor Bug
→ Falso evento
→ Ação
→ Novo evento
→ Nova ação
→ Loop
```

Mecanismos possíveis:

- rate limiting;
- quotas;
- budgets;
- cooldown;
- deduplicação;
- idempotência;
- circuit breaker;
- maximum duration;
- state validation;
- safety interlock.

---

# 27. Network Loss

A perda de rede deve possuir comportamento definido.

Possibilidades:

```text
Network Loss
├── Continue Locally
├── Degraded Operation
├── Safe Stop
├── Controlled Return
└── Emergency Behavior
```

A escolha depende do sistema físico.

Uma regra geral como "sempre parar" ou "sempre continuar" não é apropriada para todos os equipamentos.

---

# 28. Local Autonomy

Alguns sistemas físicos precisam continuar operando localmente.

A autonomia local pode incluir:

- estabilização;
- controle;
- prevenção de colisão;
- manutenção de estado seguro;
- resposta a sensores;
- recuperação de comunicação.

Essa autonomia deve permanecer dentro de limites previamente definidos.

A autonomia local não deve significar autoridade ilimitada.

---

# 29. Safety State

Safety State deve ser separado de:

- Operational State;
- Control Ownership;
- Authorization State;
- Communication State.

Exemplo:

```text
Operational State:
RUNNING

Safety State:
DEGRADED

Control Ownership:
LOCAL_CONTROLLER

Authorization:
ACTIVE
```

Essas dimensões não devem ser comprimidas em um único status.

---

# 30. Control Ownership

Um sistema físico deve deixar claro quem possui o controle atual.

Possíveis estados:

```text
Yuki
Local Controller
Human
Safety Controller
Emergency System
Maintenance
```

O controle pode ser transferido segundo regras explícitas.

A Yuki não deve assumir controle simplesmente porque possui conectividade.

---

# 31. Security Compromise

Se a Yuki for comprometida, o sistema físico deve continuar protegido.

Threat model:

```text
Attacker
   ↓
Prompt Injection
   ↓
Model
   ↓
Agent
   ↓
Capability
   ↓
Integration
   ↓
Physical Gateway
   ↓
Safety Layer
   ↓
Hardware
```

A arquitetura deve impedir que o comprometimento de uma camada elimine todas as outras.

---

# 32. Prompt Injection → Physical World

Dados externos podem conter instruções maliciosas.

Exemplos:

- páginas web;
- documentos;
- sensores;
- mensagens;
- APIs;
- e-mails;
- câmeras;
- dispositivos.

A regra permanece:

> **Data ≠ Instruction.**

Fluxo:

```text
External Data
    ↓
Typed Representation
    ↓
Policy / Context Evaluation
    ↓
Authorization
    ↓
Safety Validation
    ↓
Physical Action
```

Nenhum conteúdo externo deve ganhar autoridade simplesmente por ser interpretado pelo modelo.

---

# 33. Physical Integration

Uma integração física deve declarar, conforme aplicável:

- dispositivo;
- capacidade;
- atuadores;
- sensores;
- limites;
- unidades;
- estados;
- safe state;
- emergency behavior;
- requisitos de segurança;
- requisitos ambientais;
- modos de operação;
- comportamento em falha;
- métodos de observação;
- métodos de verificação;
- necessidades de autorização.

Essas informações podem fazer parte de um futuro **Physical Capability / Safety Manifest**.

---

# 34. Physical Resource Ownership

Recursos físicos devem possuir um modelo explícito de propriedade e controle.

Exemplo:

```text
Device
→ Owner
→ Current Controller
→ Safety Authority
→ Maintenance Authority
```

A existência de uma conexão técnica não implica autorização de controle.

---

# 35. Physical Sandbox

Sistemas físicos não possuem um sandbox universal equivalente ao software.

A proteção pode ser composta por:

- simulação;
- ambiente virtual;
- hardware-in-the-loop;
- limites físicos;
- áreas restritas;
- baixa potência;
- velocidade limitada;
- carga reduzida;
- atuadores desabilitados;
- dispositivos de teste;
- isolamento mecânico.

A estratégia deve ser adequada ao sistema.

---

# 36. Simulation Before Execution

Quando apropriado, uma ação física pode ser testada previamente em:

- simulação;
- digital twin;
- replay;
- hardware-in-the-loop;
- ambiente de teste.

Fluxo:

```text
Plan
 ↓
Simulate
 ↓
Evaluate
 ↓
Safety Check
 ↓
Authorize
 ↓
Execute
```

Simulação não substitui as proteções físicas reais.

---

# 37. Digital Twin

Um futuro Digital Twin poderá representar:

- estado esperado;
- estado observado;
- configuração;
- limitações;
- geometria;
- ambiente;
- sensores;
- atuadores;
- histórico;
- falhas.

O Digital Twin é uma representação do sistema, não uma garantia de que o mundo físico corresponde perfeitamente ao modelo.

---

# 38. Runtime Safety Assurance

Uma camada de Runtime Safety Assurance pode verificar se comandos ou trajetórias estão dentro do envelope permitido.

Ela pode:

```text
ALLOW
REJECT
MODIFY
SAFE STOP
FALLBACK
```

A resposta depende do sistema.

A camada não deve ser presumida como um algoritmo universal.

Pode assumir diferentes formas:

- limit monitor;
- invariant checker;
- trajectory validator;
- runtime assurance;
- safety supervisor;
- outro mecanismo apropriado.

---

# 39. Firmware e Device Trust

Dispositivos físicos devem possuir mecanismos de confiança apropriados quando necessários.

Possíveis mecanismos:

- identidade de dispositivo;
- secure boot;
- firmware assinado;
- attestation;
- hardware root of trust;
- chave de dispositivo;
- versão conhecida;
- integridade de firmware.

Esses mecanismos fornecem evidências de confiança.

Eles não concedem automaticamente autorização.

---

# 40. Recovery

Recovery deve ser tratado como parte da arquitetura.

Possíveis ações:

```text
Detect Failure
      ↓
Contain
      ↓
Enter Safe State
      ↓
Diagnose
      ↓
Recover
      ↓
Validate
      ↓
Resume
```

A retomada não deve ocorrer automaticamente em todos os casos.

---

# 41. Audit e Forensics

Ações físicas relevantes devem gerar registros suficientes para reconstrução do evento.

Podem incluir:

- operation_id;
- mission_id;
- device;
- command;
- authorization;
- policy;
- safety decision;
- controller;
- timestamps;
- sensor evidence;
- verification result;
- failure;
- recovery;
- human approval, quando aplicável.

Logs devem respeitar:

- privacidade;
- minimização de dados;
- integridade;
- retenção adequada;
- controle de acesso.

---

# 42. Relação com ADR-009

ADR-009 define a separação:

```text
Execution
Effect
Verification
Authorization
Reconciliation
Compensation
```

Para o mundo físico:

```text
Physical Command
      ↓
Execution
      ↓
Observed Physical Effect
      ↓
Evidence
      ↓
Verification
      ↓
Verified Result
```

O ADR-010 adiciona as restrições de segurança física antes e durante essa cadeia.

---

# 43. Relação com ADR-008

ADR-008 define isolamento de credenciais.

No mundo físico:

- modelos não recebem credenciais brutas;
- agentes não recebem autoridade implícita;
- integrações físicas utilizam credenciais e identidades apropriadas;
- possuir credencial não significa possuir autorização física;
- comprometimento de uma integração não deve conceder controle universal.

---

# 44. Relação com ADR-007

ADR-007 define Integration Definition e Integration Instance.

Uma integração física pode representar:

```text
Integration Definition
        ↓
Robot Integration
        ↓
Integration Instance
        ↓
Specific Robot
```

Cada instância pode possuir:

- identidade;
- configuração;
- permissões;
- política;
- estado;
- saúde;
- requisitos;
- limites específicos.

---

# 45. Relação com ADR-006

ADR-006 define recursos abstratos.

Sistemas físicos podem expor recursos como:

```text
Compute
Storage
Network
Device
Sensor
Actuator
Energy
Physical Space
Future Resource
```

O Resource Manager não substitui o Safety Controller.

Um recurso disponível não significa que pode ser utilizado.

```text
Resource Availability
      ≠
Authorization
      ≠
Safety
```

---

# 46. Relação com Capability System

Uma capacidade física pode ser:

```text
robot.move
robot.grasp
door.unlock
valve.close
camera.observe
motor.start
```

A existência da capacidade não concede autorização.

A seleção da capacidade deve passar pelos mecanismos de:

- autorização;
- segurança;
- limites;
- verificação.

---

# 47. Relação com Model Router

O Model Router pode selecionar modelos para:

- interpretação;
- planejamento;
- visão;
- análise;
- previsão;
- verificação de evidências.

Mas:

> **Model Router não é Safety Controller.**

Um modelo escolhido pelo roteador não recebe autoridade física especial.

---

# 48. Relação com Evolution Manager

A evolução de sistemas físicos possui risco superior ao software puramente informacional.

Mudanças em:

- controladores;
- firmware;
- limites;
- políticas;
- Safety Controller;
- interlocks;
- trajetórias;
- drivers;

devem passar por níveis de validação adequados.

A Development Lab deve permitir:

```text
Prototype
→ Simulation
→ Test
→ Hardware-in-the-loop
→ Safety Evaluation
→ Approval
→ Controlled Deployment
```

Mudanças críticas de segurança não devem ser autoimplantadas pela Yuki sem a autoridade exigida pelo sistema.

---

# 49. Physical Safety Invariants

Os seguintes invariantes são fundamentais:

### Invariant 1

> Yuki Core não é autoridade de segurança física.

### Invariant 2

> Funções críticas de segurança devem permanecer independentes da disponibilidade da Yuki.

### Invariant 3

> Modelo de IA não é autorização de segurança.

### Invariant 4

> Comando físico não é efeito físico.

### Invariant 5

> Efeito observado não é automaticamente efeito verificado.

### Invariant 6

> `UNKNOWN` nunca deve ser silenciosamente convertido em sucesso.

### Invariant 7

> Limites físicos críticos devem ser impostos em uma camada capaz de efetivamente impedi-los.

### Invariant 8

> Falha de uma camada não deve remover todas as proteções inferiores.

### Invariant 9

> Emergency mechanisms não dependem exclusivamente da Yuki.

### Invariant 10

> Possuir uma capacidade física não implica autorização para utilizá-la.

### Invariant 11

> Evidência de sensor não é automaticamente verdade absoluta.

### Invariant 12

> Segurança deve ser adequada ao risco e ao sistema físico concreto.

---

# 50. Threat Model

A arquitetura considera, entre outros:

- prompt injection;
- model compromise;
- agent runaway;
- malicious plugin;
- compromised integration;
- stolen credential;
- forged sensor data;
- stale telemetry;
- replay;
- network loss;
- controller failure;
- firmware compromise;
- configuration error;
- sensor failure;
- actuator failure;
- power failure;
- synchronization failure;
- operator error;
- conflicting commands;
- physical tampering.

A proteção deve ser distribuída entre camadas.

---

# 51. Failure Isolation

A arquitetura busca limitar blast radius.

Exemplo:

```text
Compromised Model
      ↓
Compromised Agent
      ↓
Capability
      ↓
Physical Gateway
      X
Safety Controller
      X
Hardware Interlock
```

Nenhuma camada isolada deve possuir poder suficiente para ignorar todas as demais.

---

# 52. Graceful Degradation

Quando componentes falham:

```text
Normal
 ↓
Degraded
 ↓
Restricted
 ↓
Safe
```

A degradação deve preservar:

- segurança;
- controle;
- auditabilidade;
- possibilidade de recuperação.

---

# 53. Physical Multi-System Safety

Quando vários sistemas físicos interagem:

```text
Robot A
Robot B
Door
Camera
Vehicle
Power System
```

cada sistema pode possuir seu próprio Safety Controller.

Uma camada superior pode coordenar, mas não deve remover as proteções locais.

A segurança deve continuar funcionando mesmo se o coordenador superior falhar.

---

# 54. Future Technology Independence

Este ADR não depende constitucionalmente de:

- ROS;
- ROS2;
- DDS;
- MQTT;
- CAN;
- EtherCAT;
- PLC;
- RTOS;
- Linux;
- microcontroladores específicos;
- FPGA;
- NVIDIA;
- qualquer fabricante;
- qualquer protocolo;
- qualquer modelo de IA.

Essas tecnologias podem ser utilizadas como implementações.

O princípio arquitetural é:

> **A Yuki deve conseguir incorporar novas tecnologias físicas sem alterar sua autoridade fundamental e suas invariantes de segurança.**

---

# 55. Proposed Physical Architecture

Arquitetura consolidada:

```text
                         USER
                          │
                          ▼
                         YUKI
                          │
                    HIGH-LEVEL INTENT
                          │
                          ▼
                  PHYSICAL GATEWAY
                          │
                ┌─────────┴─────────┐
                │                   │
          SECURITY POLICY      PHYSICAL POLICY
                │                   │
                ▼                   ▼
          AUTHORIZATION       SAFETY VALIDATION
                │                   │
                └─────────┬─────────┘
                          ▼
                  LOCAL CONTROLLER
                          │
                  SAFETY ENFORCEMENT
                          │
                ┌─────────┴─────────┐
                ▼                   ▼
        HARDWARE INTERLOCK       ACTUATOR
                │                   │
                └─────────┬─────────┘
                          ▼
                    PHYSICAL WORLD
                          │
                          ▼
                    SENSOR LAYER
                          │
                          ▼
                  OBSERVATION LAYER
                          │
                          ▼
                  EVIDENCE VALIDATION
                          │
                          ▼
                 ADR-009 VERIFICATION
                          │
                          ▼
                 RESULT / MEMORY /
                   AUDIT / MISSION
```

Emergency path:

```text
HUMAN / EMERGENCY SYSTEM
          │
          ▼
   EMERGENCY MECHANISM
          │
          ▼
   SAFETY ENFORCEMENT
          │
          ▼
      SAFE STATE
```

Esse caminho não deve depender da Yuki Core.

---

# 56. Closed Decisions

## D010-1 — Physical Safety Authority Independence

A autoridade de segurança física é arquiteturalmente independente da autoridade cognitiva da Yuki.

**Status:** CLOSED

---

## D010-2 — Independent Critical Safety Functions

Funções físicas críticas devem permanecer capazes de atuar independentemente da Yuki Core.

**Status:** CLOSED

---

## D010-3 — Independent Emergency Mechanisms

Mecanismos de emergência não podem depender exclusivamente da disponibilidade da Yuki.

**Status:** CLOSED

---

## D010-4 — Safe Operating Envelope

Safe Operating Envelope é um conceito arquitetural de primeira classe.

**Status:** CLOSED

---

## D010-5 — Lowest Appropriate Enforcement Layer

Restrições críticas devem ser aplicadas na camada mais baixa apropriada capaz de efetivamente impedi-las.

**Status:** CLOSED

---

## D010-6 — Explicit Physical Authorization and Safety Validation

Comandos físicos devem passar por autorização e validação de segurança apropriadas.

**Status:** CLOSED

---

## D010-7 — AI Cannot Override Critical Physical Limits

Limites físicos críticos não podem ser sobrescritos pela Yuki ou por modelos de IA.

**Status:** CLOSED

---

## D010-8 — System-Specific Failure Behavior

O comportamento diante de falhas deve ser definido conforme o sistema físico e sua análise de risco.

**Status:** CLOSED

---

## D010-9 — Physical Action Budgets

Sistemas físicos relevantes devem possuir mecanismos de limitação contra runaway quando apropriado.

**Status:** CLOSED

---

## D010-10 — Independent Emergency and Recovery Authority

Emergency handling e mecanismos críticos de recuperação devem possuir autoridade independente da execução normal da Yuki.

**Status:** CLOSED

---

## D010-11 — Separate Physical State Dimensions

Safety State, Operational State e Control Ownership são dimensões distintas.

**Status:** CLOSED

---

## D010-12 — Command ≠ Effect ≠ Verified Effect

Comando físico, execução, efeito observado e efeito verificado são conceitos distintos.

**Status:** CLOSED

---

## D010-13 — Physical Safety Characteristics

Integrações físicas devem declarar características e restrições de segurança aplicáveis.

**Status:** CLOSED

---

## D010-14 — No Constitutional Safety Technology

Nenhuma tecnologia específica de segurança física é constitucionalmente obrigatória para todos os sistemas.

**Status:** CLOSED

---

## D010-15 — Model Output Is Not Safety Authorization

Nenhuma saída de modelo constitui, por si só, autorização de segurança.

**Status:** CLOSED

---

## D010-16 — Defined Safe Behavior

Todo sistema físico relevante deve possuir comportamento definido para as falhas relevantes.

**Status:** CLOSED

---

# 57. Open Decisions

Os seguintes pontos permanecem deliberadamente abertos:

- formato final do Physical Safety Manifest;
- modelo formal de Safe Operating Envelope;
- taxonomia definitiva de estados físicos;
- Emergency Stop por classe de equipamento;
- Safety Controller implementation;
- Physical Gateway implementation;
- hardware-backed safety;
- device attestation;
- firmware trust;
- Digital Twin;
- simulation framework;
- hardware-in-the-loop;
- physical identity;
- human presence model;
- physical capability schema;
- safety evidence model;
- runtime assurance architecture;
- safety certification strategy;
- industrial protocols;
- robotics middleware;
- device discovery;
- physical fleet management.

---

# 58. Future ADRs

Possíveis ADRs derivados:

### ADR-011 — Durable Workflow, Saga & Compensation

Operações longas, transações distribuídas, compensação e recuperação.

### ADR-012 — Physical Device & Robotics Control

Controle de robôs, dispositivos, atuadores e sensores.

### ADR-013 — Safety Controller & Runtime Assurance

Arquitetura detalhada do Safety Controller e runtime safety assurance.

### ADR-014 — Physical Device Identity & Attestation

Identidade, attestation e confiança de dispositivos físicos.

### ADR-015 — Physical Capability & Safety Manifest

Manifesto formal para capacidades físicas, limites e requisitos de segurança.

### ADR-016 — Physical Simulation, Digital Twin & Safety Testing

Simulação, digital twin, HIL e validação antes da execução física.

---

# 59. Architectural Summary

A arquitetura física da Yuki pode ser resumida em:

```text
Intent
  ↓
Authorization
  ↓
Physical Gateway
  ↓
Safety Validation
  ↓
Local Control
  ↓
Safety Enforcement
  ↓
Hardware
  ↓
Physical World
  ↓
Sensors
  ↓
Evidence
  ↓
Verification
```

Enquanto:

```text
Emergency
   ↓
Independent Safety Path
   ↓
Safe State
```

E:

```text
Security
   ≠
Safety
```

mas ambos devem cooperar.

---

# 60. Core Principle

A regra central deste ADR é:

> **A Yuki nunca deve precisar ser perfeita para que o mundo físico permaneça protegido.**

A arquitetura deve assumir que:

- modelos podem errar;
- agentes podem entrar em loop;
- integrações podem ser comprometidas;
- sensores podem falhar;
- redes podem cair;
- software pode quebrar;
- credenciais podem ser comprometidas;
- dados podem ser maliciosos;
- componentes podem ficar indisponíveis.

Por isso, a segurança física deve existir como uma propriedade distribuída do sistema.

---

# 61. Relation to Yuki Architecture

ADR-010 integra diretamente:

```text
ADR-006
Infrastructure Resource Model
        ↓
ADR-007
Integration Model
        ↓
ADR-008
Credential Isolation
        ↓
ADR-009
Execution / Effect / Verification
        ↓
ADR-010
Physical World Safety
```

A sequência arquitetural torna-se:

```text
RESOURCE
   ↓
INTEGRATION
   ↓
CREDENTIAL / IDENTITY
   ↓
AUTHORIZATION
   ↓
PHYSICAL SAFETY
   ↓
EXECUTION
   ↓
PHYSICAL EFFECT
   ↓
EVIDENCE
   ↓
VERIFICATION
   ↓
MISSION / MEMORY / AUDIT
```

---

# 62. Final Principles

1. **Cognitive Authority ≠ Physical Authority.**
2. **Capability ≠ Permission.**
3. **Permission ≠ Safety Authorization.**
4. **Model Output ≠ Safety Authorization.**
5. **Physical Gateway ≠ Safety Controller.**
6. **Safety ≠ Security.**
7. **Command ≠ Execution.**
8. **Execution ≠ Physical Effect.**
9. **Physical Effect ≠ Verified Physical Effect.**
10. **Evidence ≠ Absolute Truth.**
11. **UNKNOWN is a valid state.**
12. **Critical safety functions remain independent of Yuki Core.**
13. **Emergency mechanisms remain independently available.**
14. **Critical limits cannot be overridden by AI.**
15. **Safe Operating Envelope is first-class.**
16. **Physical action budgets limit runaway behavior.**
17. **Failure behavior is system-specific.**
18. **Safety enforcement belongs at the lowest appropriate layer.**
19. **Physical safety assurance must be proportional to risk.**
20. **No specific technology is constitutionally required.**
21. **The physical world must not depend on the Yuki being infallible.**

---

# 63. Status

**ADR-010 — ACCEPTED**

This ADR establishes the architectural principles for Yuki's interaction with the physical world.

Implementation-specific decisions remain delegated to future ADRs according to the physical system, hazard, risk, applicable requirements and technology.

**Version:** 1.0  
**Date:** 2026-09-19  
**Status:** ACCEPTED
