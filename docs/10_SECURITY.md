# YUKI — SECURITY ARCHITECTURE

**Documento:** `docs/10_SECURITY.md`
**Versão:** v0.1
**Status:** Architecture Draft / Official Direction
**Última atualização:** 2026-09-15

---

# 1. Objetivo

A segurança da Yuki não deve ser tratada como uma camada adicionada posteriormente ao sistema.

Ela é uma propriedade fundamental da arquitetura.

A Yuki será capaz de:

* processar informações privadas;
* acessar sistemas;
* utilizar ferramentas;
* executar ações;
* trabalhar em segundo plano;
* utilizar múltiplos modelos;
* operar em diferentes dispositivos;
* acessar redes;
* interagir com sistemas externos;
* modificar sistemas sob autorização;
* desenvolver e testar novas capacidades.

Portanto, a arquitetura deve assumir que componentes podem falhar, ser comprometidos, produzir resultados incorretos ou ser manipulados.

O objetivo deste documento é definir como a Yuki deve:

* proteger sua identidade;
* proteger o usuário;
* controlar capacidades;
* controlar ações;
* separar confiança de autorização;
* limitar privilégios;
* isolar componentes;
* detectar comportamento anômalo;
* conter incidentes;
* preservar evidências;
* recuperar-se de falhas;
* impedir que um componente comprometido comprometa todo o sistema.

---

# 2. Princípio Fundamental

> **A Yuki pode pensar em uma ação sem estar autorizada a executá-la.**

Pensar, planejar, pesquisar, simular e executar são operações diferentes.

Exemplo:

```text
Yuki identifica uma possível melhoria
        ↓
Pode pesquisar
        ↓
Pode projetar
        ↓
Pode criar protótipo
        ↓
Pode testar em ambiente isolado
        ↓
Pode solicitar aprovação
        ↓
Pode executar somente se autorizada
```

Nenhuma capacidade cognitiva concede automaticamente capacidade operacional.

---

# 3. Capability ≠ Permission

A existência de uma capacidade não significa que ela esteja autorizada para determinada situação.

```text
Capability
    ≠
Permission
```

Exemplo:

```text
Capability:
    enviar_email()

Permission:
    pode enviar este email?
```

A decisão depende de:

* identidade;
* contexto;
* finalidade;
* usuário;
* dispositivo;
* política;
* risco;
* dados envolvidos;
* destino;
* estado de segurança;
* autorização atual.

---

# 4. Security by Architecture

A segurança deve estar distribuída por toda a arquitetura.

Não deve existir apenas um "filtro final".

```text
USER
 ↓
YUKI ACCESS
 ↓
CONTROL PLANE
 ↓
YUKI CORE
 ↓
AGENT
 ↓
CAPABILITY
 ↓
TOOL GATEWAY
 ↓
SANDBOX
 ↓
EXECUTION
```

Cada transição representa uma possível fronteira de confiança.

Cada camada deve possuir mecanismos apropriados de:

* identidade;
* autenticação;
* autorização;
* validação;
* isolamento;
* auditoria.

---

# 5. Trust Architecture

A arquitetura de confiança da Yuki será baseada no princípio:

```text
Identity
    ↓
Trust
    ↓
Authentication
    ↓
Authorization
    ↓
Policy
    ↓
Risk
    ↓
Capability
    ↓
Execution
    ↓
Verification
    ↓
Audit
    ↓
Containment
    ↓
Recovery
```

Essas etapas não devem ser tratadas como equivalentes.

Identificar alguém não significa autorizar uma ação.

Autenticar um dispositivo não significa confiar em todas as operações realizadas por ele.

Autorizar uma capacidade não significa autorizar todas as ações dessa capacidade.

---

# 6. Zero Trust

A Yuki deve adotar uma filosofia de **Zero Trust**.

Isso significa:

> Nenhum componente deve receber confiança ilimitada simplesmente por estar dentro da infraestrutura da Yuki.

Exemplos:

```text
Core → Agent
Agent → Capability
Capability → Tool
Tool → External System
```

Cada relação deve possuir limites explícitos.

A pergunta não deve ser:

> "Este componente está dentro da Yuki?"

Mas:

> "Este componente está autorizado a fazer esta operação neste contexto?"

---

# 7. Assume Breach

A arquitetura deve assumir que algum componente poderá ser comprometido.

Isso inclui potencialmente:

* ferramentas;
* plugins;
* agentes;
* modelos;
* integrações;
* dispositivos;
* contas;
* APIs;
* dependências;
* serviços externos;
* dados recebidos;
* interfaces.

Portanto:

> **Comprometer um componente não deve significar comprometer a Yuki inteira.**

A arquitetura deve priorizar:

* segmentação;
* isolamento;
* privilégios mínimos;
* credenciais temporárias;
* revogação;
* auditoria;
* contenção;
* recuperação.

---

# 8. Security Controller

A Yuki deve possuir um **Security Controller** logicamente independente do Yuki Core.

Arquitetura:

```text
                 SECURITY CONTROLLER
                         │
        ┌────────────────┼────────────────┐
        │                │                │
    Identity         Policy          Containment
        │                │                │
        └────────────────┼────────────────┘
                         │
                       YUKI
```

O Core pode solicitar ações.

O Security Controller decide se essas ações são permitidas dentro das políticas de segurança.

O Core não deve possuir autoridade irrestrita sobre o próprio mecanismo de segurança.

---

# 9. Separação entre Planes

A arquitetura deve distinguir três planos principais.

## 9.1 Control Plane

Responsável pelo controle e segurança.

Inclui conceitualmente:

* identidade;
* autenticação;
* autorização;
* políticas;
* segurança;
* auditoria;
* gestão de credenciais;
* confiança;
* limites;
* contenção.

---

## 9.2 Cognitive / Data Plane

Responsável pela inteligência e pelo processamento.

Inclui:

* Yuki Core;
* Model Router;
* Memory;
* Personal Context;
* Knowledge;
* Agent System;
* Capability Registry;
* Planning;
* Reasoning.

---

## 9.3 Execution Plane

Responsável pela execução efetiva.

Inclui:

* ferramentas;
* workers;
* sandboxes;
* processos isolados;
* microVMs;
* WebAssembly ou tecnologias equivalentes;
* ambientes de execução.

A implementação tecnológica poderá mudar.

A separação arquitetural deve permanecer.

---

# 10. Least Privilege

Cada componente deve receber somente os privilégios necessários.

Exemplo:

```text
Capability A
├── leitura: calendário
├── escrita: não
├── rede: limitada
├── arquivos: nenhum
└── execução: nenhuma
```

Em vez de:

```text
Capability A
└── acesso total ao sistema
```

Privilégios devem ser:

* mínimos;
* específicos;
* temporários quando possível;
* revogáveis;
* auditáveis.

---

# 11. Least Agency

Além de Least Privilege, a Yuki deve utilizar **Least Agency**.

Isso significa limitar não apenas o que um componente pode acessar, mas também **o que ele pode fazer autonomamente**.

Exemplo:

```text
Pode pesquisar
    ↓
Pode preparar
    ↓
Pode simular
    ↓
Pode solicitar aprovação
    ↓
Pode executar
```

Quanto maior o impacto potencial de uma ação, maior deve ser o controle sobre sua autonomia.

---

# 12. Risk ≠ Permission

O nível de risco não determina sozinho se uma ação é permitida.

```text
Risk
≠
Permission
```

Uma ação pode ser:

```text
LOW
MEDIUM
HIGH
CRITICAL
```

Mas sua execução ainda depende de:

* política;
* contexto;
* identidade;
* autorização;
* finalidade;
* estado do sistema.

---

# 13. Classificação de Risco

A Yuki utilizará inicialmente quatro níveis conceituais.

## LOW

Exemplos:

* consultas;
* cálculos;
* organização;
* leitura de informações públicas;
* tarefas sem efeitos externos relevantes.

Pode possuir alto grau de automação.

---

## MEDIUM

Exemplos:

* alterações reversíveis;
* operações em sistemas pessoais;
* criação de arquivos;
* determinadas integrações.

Pode exigir confirmação contextual ou políticas pré-aprovadas.

---

## HIGH

Exemplos:

* alterações importantes;
* operações externas com impacto relevante;
* acesso a dados sensíveis;
* modificações de infraestrutura.

Normalmente exige controles adicionais e, quando apropriado, aprovação humana.

---

## CRITICAL

Exemplos conceituais:

* alterações no núcleo de segurança;
* alterações de identidade;
* alterações de autorização;
* operações irreversíveis de grande impacto;
* destruição de dados críticos;
* mudanças estruturais de segurança.

Devem possuir o mais alto nível de proteção e aprovação.

---

# 14. Human-in-the-Loop

A presença humana deve aumentar proporcionalmente ao risco.

```text
LOW
→ Automático

MEDIUM
→ Política / confirmação contextual

HIGH
→ Aprovação quando necessária

CRITICAL
→ Controle humano explícito
```

O sistema não deve pedir confirmação para absolutamente tudo.

Da mesma forma, não deve executar ações críticas silenciosamente.

---

# 15. Identity

A Yuki deve possuir uma arquitetura de identidade independente de uma única interface.

A identidade pode envolver:

* conta;
* dispositivo;
* sessão;
* credencial;
* chave criptográfica;
* fatores biométricos;
* contexto.

---

# 16. Biometria

Voz e rosto podem ser utilizados como sinais de identidade.

Entretanto:

> **Reconhecimento de voz ou rosto não equivale automaticamente a autorização ilimitada.**

Exemplo:

```text
Voz reconhecida
        ↓
Identidade provável
        ↓
Autenticação
        ↓
Política
        ↓
Permissão
        ↓
Ação
```

Para ações críticas podem ser necessários fatores adicionais.

---

# 17. Device Trust

Dispositivos devem possuir diferentes níveis de confiança.

Exemplo:

```text
Trusted Device
Known Device
Limited Device
Unknown Device
Compromised Device
```

Um celular pessoal, notebook, relógio, servidor doméstico ou dispositivo temporário podem possuir diferentes permissões.

A Yuki deve ser capaz de revogar a confiança de um dispositivo.

---

# 18. Sessions

Cada sessão deve possuir contexto de segurança próprio.

Uma sessão pode possuir:

* identidade;
* dispositivo;
* horário;
* localização contextual;
* capabilities permitidas;
* duração;
* credenciais temporárias;
* nível de confiança.

Sessões devem poder expirar ou ser revogadas.

---

# 19. Location Is Context, Not Authorization

A localização pode ser utilizada como contexto.

Porém:

> **Localização não concede autorização.**

O fato de uma determinada ação ser permitida em determinado país, sistema, plataforma ou ambiente não significa automaticamente que a Yuki esteja autorizada a executá-la.

A decisão deve considerar:

```text
Contexto
+
Identidade
+
Finalidade
+
Permissões
+
Políticas
+
Risco
+
Segurança
```

---

# 20. Secrets

Credenciais não devem ser entregues diretamente ao Core ou aos modelos quando não forem necessárias.

A arquitetura deve possuir um mecanismo de armazenamento seguro de segredos.

Conceito:

```text
Yuki
 ↓
Secret Request
 ↓
Policy Check
 ↓
Scoped Credential
 ↓
Capability
 ↓
Tool
```

Preferencialmente:

* credenciais temporárias;
* escopo limitado;
* rotação;
* revogação;
* auditoria;
* nunca exposição desnecessária.

---

# 21. Capability-Based Security

Sempre que possível, componentes devem receber acesso através de capacidades específicas em vez de acesso amplo ao sistema.

Exemplo:

```text
Capability:
    calendar.read

em vez de:

    acesso total à conta
```

Isso reduz o impacto de comprometimentos.

---

# 22. Tool Gateway

Nenhuma ferramenta externa deve possuir acesso direto irrestrito ao Core.

Arquitetura:

```text
Yuki
 ↓
Capability Gateway
 ↓
Permission Check
 ↓
Tool Gateway
 ↓
Sandbox
 ↓
Tool
```

O Tool Gateway pode aplicar:

* autenticação;
* autorização;
* limites;
* validação;
* logging;
* rate limiting;
* isolamento;
* políticas de rede.

---

# 23. Plugin Security

Plugins são componentes potencialmente não confiáveis.

Processo de integração:

```text
New Plugin
    ↓
Origin Check
    ↓
Integrity Check
    ↓
Dependency Analysis
    ↓
Permission Analysis
    ↓
Risk Classification
    ↓
Sandbox
    ↓
Tests
    ↓
Security Evaluation
    ↓
Capability Registry
    ↓
Production
```

Nenhum plugin deve ser considerado confiável simplesmente por estar instalado.

---

# 24. Sandbox

Ferramentas e código não confiável devem ser executados em ambientes isolados quando apropriado.

Possíveis tecnologias futuras incluem:

* processos isolados;
* containers;
* WebAssembly;
* microVMs;
* máquinas virtuais;
* ambientes dedicados.

Essas tecnologias são escolhas de implementação, não princípios arquiteturais.

O princípio permanente é:

> **Código não confiável deve possuir o menor acesso possível ao ambiente real.**

---

# 25. Sandbox ≠ Segurança Absoluta

Um sandbox reduz impacto, mas não elimina risco.

A Yuki deve continuar adotando:

* Assume Breach;
* least privilege;
* network restrictions;
* filesystem restrictions;
* resource limits;
* monitoring;
* verification;
* containment.

---

# 26. Network Security

A Yuki deve tratar a rede como uma fronteira de segurança.

Capabilities podem possuir políticas como:

```text
Network:
    NONE
    LIMITED
    ALLOWED
    RESTRICTED
```

O acesso pode ser limitado por:

* destino;
* protocolo;
* finalidade;
* tempo;
* volume;
* identidade;
* política.

---

# 27. Data Firewall

Dados e instruções devem ser tratados separadamente.

> **Data ≠ Instruction**

Conteúdo recebido de:

* web;
* PDF;
* email;
* API;
* documento;
* banco de dados;
* plugin;
* sistema externo;

não deve automaticamente ser interpretado como instrução para a Yuki.

---

# 28. Prompt Injection

A Yuki deve considerar prompt injection como uma ameaça arquitetural.

Exemplo:

```text
Web Page
    ↓
Research Agent
    ↓
Malicious Instruction
    ↓
Model
```

O conteúdo recebido não deve ganhar autoridade apenas por estar dentro do contexto do modelo.

A arquitetura deve separar:

```text
Data
Knowledge
Instruction
Policy
Authorization
```

---

# 29. Model Security

Modelos não são autoridades de segurança.

> **Output de modelo não equivale a autorização.**

Um modelo pode:

* errar;
* alucinar;
* ser manipulado;
* interpretar incorretamente uma instrução;
* sofrer prompt injection;
* gerar uma ação perigosa.

Portanto:

```text
Model Output
    ↓
Validation
    ↓
Policy
    ↓
Authorization
    ↓
Execution
```

---

# 30. Model Isolation

O Model Router pode utilizar múltiplos modelos.

Nenhum modelo individual deve possuir autoridade implícita sobre:

* segurança;
* identidade;
* autorização;
* credenciais;
* Core;
* outros modelos.

Modelos devem operar como componentes cognitivos.

---

# 31. Agent Security

Agentes são operadores de tarefas, não autoridades absolutas.

Cada agente deve possuir:

* identidade;
* escopo;
* missão;
* permissões;
* recursos;
* limites;
* capacidades autorizadas.

Arquitetura:

```text
Supervisor
 ↓
Agent
 ↓
Capability
 ↓
Tool
```

O agente não deve poder simplesmente elevar seus próprios privilégios.

---

# 32. Privilege Escalation

A Yuki deve impedir que um componente:

* aumente seus próprios privilégios;
* conceda privilégios a outro componente;
* modifique políticas de segurança;
* obtenha credenciais superiores;
* contorne o Security Controller.

Especialmente:

```text
Agent
→ não pode virar Admin sozinho

Capability
→ não pode conceder novas capabilities

Model
→ não pode alterar políticas

Core
→ não pode ignorar Security Controller
```

---

# 33. Execution Verification

Executar uma ação não significa que ela foi realizada corretamente.

Após ações relevantes:

```text
Request
 ↓
Authorization
 ↓
Execution
 ↓
Result
 ↓
Verification
```

A Yuki deve verificar:

* resultado esperado;
* efeitos colaterais;
* alterações inesperadas;
* integridade;
* estado final.

---

# 34. Audit

Ações relevantes devem possuir trilha de auditoria.

Conceito:

```text
Request
→ Identity
→ Intent
→ Plan
→ Risk
→ Policy
→ Permission
→ Execution
→ Result
→ Verification
→ Audit
```

O log deve registrar, quando apropriado:

* quem;
* qual dispositivo;
* qual sessão;
* qual capability;
* qual ferramenta;
* qual ação;
* quando;
* contexto relevante;
* política aplicada;
* resultado;
* falha;
* aprovação.

---

# 35. Audit Integrity

Logs de segurança são ativos críticos.

A arquitetura deve buscar:

* integridade;
* proteção contra alteração indevida;
* timestamps;
* controle de acesso;
* retenção adequada;
* backup;
* recuperação.

Logs não devem depender exclusivamente do componente que está sendo investigado.

---

# 36. Monitoring

A Yuki deve possuir observabilidade sobre:

* autenticação;
* uso de capabilities;
* agentes;
* ferramentas;
* modelos;
* rede;
* dispositivos;
* recursos;
* falhas;
* tentativas de acesso;
* alterações de configuração.

O objetivo é detectar:

* comportamento anômalo;
* abuso;
* comprometimento;
* degradação;
* tentativas de evasão.

---

# 37. Security States

O Security Controller deve poder alterar o estado operacional da Yuki.

Estados conceituais:

```text
NORMAL
```

Operação normal.

```text
SUSPECTED
```

Comportamento suspeito detectado.

```text
CONTAINMENT
```

Componentes ou capacidades potencialmente comprometidos são isolados.

```text
LOCKDOWN
```

Operações não essenciais são bloqueadas.

```text
OFFLINE
```

Conectividade ou operação externa é desativada quando necessário.

---

# 38. Containment

Em caso de comprometimento, a prioridade deve ser impedir propagação.

Possíveis ações:

* isolar capability;
* isolar agente;
* revogar credenciais;
* bloquear rede;
* suspender sessão;
* colocar plugin em quarentena;
* impedir novas execuções;
* preservar logs;
* alertar o usuário.

---

# 39. Kill Switch

A Yuki deve possuir mecanismos de emergência independentes do Core.

O sistema deve permitir, quando necessário:

```text
Emergency Stop
```

O mecanismo não deve depender exclusivamente da cooperação do componente potencialmente comprometido.

---

# 40. Hardware Security

A arquitetura pode utilizar chaves físicas para operações críticas.

Conceitualmente:

```text
Owner Key
Emergency Key
Recovery Key
Maintenance Key
```

As funções específicas serão definidas posteriormente.

A existência de uma chave física não deve substituir a arquitetura de identidade e autorização.

---

# 41. Backup and Recovery

A segurança inclui capacidade de recuperação.

A Yuki deve possuir estratégia de backup.

Princípio inicial:

```text
3 cópias
2 meios diferentes
1 cópia isolada/offline
```

Áreas importantes:

* memória;
* configurações;
* identidade;
* políticas;
* auditoria;
* projetos;
* dados;
* infraestrutura.

Backups críticos devem ser protegidos contra alterações ou comprometimento da infraestrutura principal.

---

# 42. Recovery

Após um incidente, a Yuki deve ser capaz de:

```text
Detect
 ↓
Contain
 ↓
Preserve
 ↓
Assess
 ↓
Recover
 ↓
Verify
 ↓
Restore
 ↓
Monitor
```

A recuperação deve priorizar integridade antes de velocidade.

---

# 43. Preserve Before Modify

Antes de alterações potencialmente destrutivas:

> **Preserve Before Modify.**

Isso significa:

* backup;
* snapshot;
* versionamento;
* registro da configuração anterior;
* possibilidade de rollback.

Especialmente importante para:

* Core;
* Security Controller;
* Memory;
* infraestrutura;
* políticas;
* capacidades.

---

# 44. Evolution Manager Security

O Evolution Manager possui risco elevado porque pode propor ou preparar mudanças na própria Yuki.

Ele deve operar dentro de um Development Lab controlado.

```text
Evolution Manager
 ↓
Research
 ↓
Prototype
 ↓
Sandbox
 ↓
Tests
 ↓
Benchmark
 ↓
Security Evaluation
 ↓
Approval
 ↓
Deployment
 ↓
Monitoring
```

---

# 45. Self-Modification

A Yuki pode ajudar no próprio desenvolvimento.

Isso pode incluir:

* pesquisar;
* escrever código;
* criar protótipos;
* testar;
* comparar implementações;
* encontrar bugs;
* analisar vulnerabilidades;
* produzir documentação.

Entretanto:

> **Capacidade de modificar código não significa autorização para modificar qualquer parte da Yuki.**

---

# 46. Protected Components

Alguns componentes devem possuir proteção especial.

Exemplos:

```text
Security Controller
Identity
Authorization
Secrets
Audit
Recovery
Core-critical infrastructure
```

Mudanças nessas áreas devem exigir controles superiores.

---

# 47. Development Lab

O futuro Development Lab da Yuki deve funcionar como ambiente separado.

```text
PRODUCTION
    │
    │
    ▼
DEVELOPMENT LAB
    │
    ├── Experiments
    ├── Prototypes
    ├── Tests
    ├── Benchmarks
    ├── Security Analysis
    └── Simulations
```

Uma alteração experimental não deve entrar diretamente em produção.

---

# 48. Regression Security

Toda mudança importante deve ser submetida a testes de regressão.

Devem existir testes para:

* funcionalidades;
* segurança;
* permissões;
* isolamento;
* memória;
* agentes;
* capacidades;
* modelos;
* integrações;
* recuperação.

Uma atualização que melhora desempenho mas reduz segurança não deve ser considerada automaticamente uma melhoria.

---

# 49. Dependency Security

Dependências externas devem ser monitoradas.

Isso inclui:

* bibliotecas;
* runtimes;
* sistemas operacionais;
* APIs;
* modelos;
* plugins;
* serviços;
* componentes de infraestrutura.

Devem existir mecanismos para:

* detectar vulnerabilidades;
* atualizar;
* testar;
* reverter;
* colocar versões em quarentena.

---

# 50. Supply Chain Security

A Yuki deve considerar a possibilidade de comprometimento da cadeia de fornecimento.

Isso inclui:

* software;
* modelos;
* plugins;
* pacotes;
* imagens;
* dependências;
* firmware;
* ferramentas.

Quando apropriado, utilizar:

* verificação de integridade;
* assinaturas;
* hashes;
* provenance;
* versionamento;
* análise de dependências.

---

# 51. Capability Attestation

Capabilities importantes podem possuir mecanismos de comprovação de integridade.

Conceitualmente:

```text
Capability
 ↓
Identity
 ↓
Version
 ↓
Integrity
 ↓
Permissions
 ↓
Health
```

O objetivo é permitir que a Yuki saiba qual componente está realmente executando.

---

# 52. Health and Quarantine

Capabilities devem possuir estados de saúde.

Exemplo:

```text
HEALTHY
DEGRADED
SUSPECTED
QUARANTINED
DISABLED
```

Uma capability suspeita pode ser removida temporariamente do fluxo normal sem necessariamente derrubar toda a Yuki.

---

# 53. Failure Isolation

Falhas devem ser contidas.

Exemplo:

```text
Plugin compromised
       ↓
Plugin isolated
       ↓
Credential revoked
       ↓
Audit preserved
       ↓
Yuki continues operating
```

A arquitetura deve favorecer **graceful degradation**.

---

# 54. Graceful Degradation

Quando um componente falhar, a Yuki deve continuar funcionando quando possível.

Exemplo:

```text
Cloud Model indisponível
        ↓
Local Model
        ↓
Reduced Capability
```

Ou:

```text
Shopping API indisponível
        ↓
Research capability remains available
```

Falha de uma capability não deve necessariamente causar falha global.

---

# 55. Data Minimization

Cada agente, modelo e capability deve receber somente os dados necessários para executar sua tarefa.

Exemplo:

```text
Task:
    comparar dois livros
```

Não é necessário enviar:

```text
Toda a memória da Yuki
Todos os relacionamentos
Todos os projetos
Todas as informações pessoais
```

Deve-se fornecer:

```text
Contexto necessário
+
Dados necessários
+
Permissões necessárias
```

---

# 56. Sensitive Data

Dados podem possuir níveis conceituais:

```text
PUBLIC
LOW
PRIVATE
SENSITIVE
CRITICAL
```

O nível determina controles adicionais de:

* armazenamento;
* processamento;
* transmissão;
* logging;
* acesso;
* retenção.

---

# 57. Data Sovereignty

Os dados do usuário devem permanecer sob controle explícito do usuário.

Isso não significa que todos os dados precisem fisicamente permanecer em um único servidor.

A arquitetura deve permitir políticas explícitas sobre:

* onde armazenar;
* onde processar;
* quando sincronizar;
* quando compartilhar;
* com quais serviços;
* por quanto tempo.

---

# 58. Privacy-Aware Processing

O Processing Router e o Model Router devem considerar privacidade.

Exemplo:

```text
Private data
    ↓
Can process locally?
    ↓
Yes → Local
No → evaluate cloud processing
```

O processamento externo deve respeitar as políticas aplicáveis.

---

# 59. External Systems

Sistemas externos devem ser tratados como fronteiras de confiança.

Arquitetura:

```text
Yuki
 ↓
System Gateway
 ↓
Authentication
 ↓
Authorization
 ↓
External System
 ↓
Validation
 ↓
Yuki
```

A Yuki não deve assumir que um sistema externo é confiável apenas porque possui uma integração oficial.

---

# 60. API Security

Integrações devem utilizar:

* autenticação;
* escopo mínimo;
* tokens temporários quando possível;
* rotação;
* limites;
* validação de entrada;
* validação de saída;
* auditoria.

---

# 61. Rate and Resource Limits

Capabilities, agentes e ferramentas devem possuir limites de:

* CPU;
* memória;
* armazenamento;
* rede;
* tempo;
* chamadas;
* custo;
* paralelismo.

Isso ajuda a prevenir:

* abuso;
* loops;
* runaway agents;
* consumo excessivo;
* denial of service interno.

---

# 62. Infinite Loops and Runaway Agents

Um agente não deve executar indefinidamente.

Missões devem possuir:

* timeout;
* número máximo de retries;
* limites de recursos;
* condições de parada;
* supervisor;
* verificação de progresso.

---

# 63. Retry Security

Retries podem causar efeitos duplicados.

Por isso, capabilities que realizam ações externas devem buscar:

* idempotência;
* identificadores de operação;
* detecção de duplicidade;
* limites de retry.

---

# 64. Multi-Agent Security

Agentes podem colaborar.

Entretanto:

```text
Agent A
    ↓
Agent B
```

não significa que B herda automaticamente as permissões de A.

Cada agente deve possuir:

* identidade;
* escopo;
* autorização própria;
* contexto mínimo.

---

# 65. Agent Communication

Comunicações entre agentes devem possuir:

* identificação;
* contexto;
* origem;
* destino;
* finalidade;
* integridade.

Uma mensagem recebida de outro agente não deve automaticamente possuir autoridade superior à política do sistema.

---

# 66. Conflict Handling

Agentes podem produzir resultados conflitantes.

A Yuki deve:

* detectar conflito;
* registrar conflito;
* solicitar verificação;
* utilizar fontes adicionais;
* escalar para o Supervisor;
* pedir intervenção humana quando necessário.

Um agente não deve vencer um conflito simplesmente por possuir maior confiança subjetiva.

---

# 67. Security and Memory

Memória também é uma superfície de ataque.

Dados armazenados podem ser:

* incorretos;
* manipulados;
* desatualizados;
* maliciosos;
* sensíveis.

Portanto:

```text
Memory Retrieval
 ↓
Validation
 ↓
Context Assembly
 ↓
Reasoning
```

Memória não deve possuir autoridade automática sobre políticas de segurança.

---

# 68. Memory Poisoning

A Yuki deve evitar transformar automaticamente qualquer informação em memória permanente.

Especialmente dados provenientes de:

* web;
* ferramentas;
* agentes;
* documentos;
* usuários externos.

A consolidação deve considerar:

* origem;
* confiança;
* relevância;
* consistência;
* temporalidade.

---

# 69. Security and Context

Contexto pode influenciar decisões, mas não deve substituir autorização.

Exemplo:

```text
Context:
    "José está em casa."

Não significa:

    "José autorizou qualquer ação."
```

Contexto informa.

Política autoriza.

---

# 70. Emergency Mode

Em uma situação crítica, a Yuki deve priorizar:

1. segurança humana;
2. contenção;
3. integridade;
4. continuidade de funções críticas;
5. preservação de evidências;
6. comunicação ao usuário.

Funções não essenciais podem ser temporariamente suspensas.

---

# 71. Security Must Not Be Starved

Mesmo em cargas extremamente pesadas, componentes críticos de segurança não devem perder todos os recursos.

A arquitetura deve reservar recursos mínimos para:

* monitoramento;
* autenticação;
* auditoria;
* contenção;
* comunicação;
* recuperação.

O Focus Mode da Yuki não deve simplesmente eliminar suas funções de segurança.

---

# 72. Security and Compute Router

O Compute Router deve considerar segurança.

Exemplo:

```text
Task
 ↓
Privacy
 ↓
Security Policy
 ↓
Available Compute
 ↓
Location
 ↓
Model / Runtime
```

A opção mais rápida ou barata não deve ser escolhida se violar uma política de segurança.

---

# 73. Security and Model Router

O Model Router deve considerar:

* privacidade;
* confiança;
* isolamento;
* localização do processamento;
* tipo de dados;
* política.

Um modelo não deve ser selecionado apenas porque possui melhor benchmark.

---

# 74. Security and Capability Router

O Capability System deve verificar:

```text
Capability exists?
        ↓
Trusted?
        ↓
Compatible?
        ↓
Authorized?
        ↓
Safe in current context?
        ↓
Execute
```

---

# 75. Security and Evolution

O Evolution Manager deve respeitar o Security Controller.

Arquitetura:

```text
Evolution Manager
        ↓
Proposal
        ↓
Security Evaluation
        ↓
Policy
        ↓
Approval
        ↓
Deployment
```

O Evolution Manager não pode conceder a si próprio privilégios adicionais para concluir uma evolução.

---

# 76. Security Invariants

Algumas regras devem ser tratadas como invariantes arquiteturais.

### Invariant 1

> Capability não implica permission.

### Invariant 2

> Model output não implica authorization.

### Invariant 3

> Location não implica authorization.

### Invariant 4

> Data não implica instruction.

### Invariant 5

> Internal component não implica trust.

### Invariant 6

> Agent não pode elevar seus próprios privilégios.

### Invariant 7

> Comprometimento de uma capability não deve comprometer automaticamente a Yuki inteira.

### Invariant 8

> Componentes críticos de segurança não devem depender exclusivamente do Core.

### Invariant 9

> Alterações importantes devem ser verificáveis e reversíveis quando possível.

### Invariant 10

> A Yuki deve continuar protegida mesmo quando um modelo estiver incorreto ou comprometido.

---

# 77. Security Decision Pipeline

A execução de ações relevantes deve seguir conceitualmente:

```text
REQUEST
   ↓
IDENTITY
   ↓
AUTHENTICATION
   ↓
INTENT
   ↓
CONTEXT
   ↓
RISK
   ↓
POLICY
   ↓
AUTHORIZATION
   ↓
CAPABILITY
   ↓
EXECUTION
   ↓
VERIFICATION
   ↓
AUDIT
```

Em caso de falha:

```text
DETECT
 ↓
CONTAIN
 ↓
PRESERVE
 ↓
RECOVER
```

---

# 78. Trust Boundaries

As principais fronteiras de confiança são:

```text
USER
 ↓
YUKI ACCESS
 ↓
CONTROL PLANE
 ↓
CORE
 ↓
AGENT
 ↓
CAPABILITY
 ↓
TOOL
 ↓
SANDBOX
 ↓
EXTERNAL SYSTEM
```

Cada fronteira deve ser tratada explicitamente.

---

# 79. Security Architecture

Visão consolidada:

```text
                         USER
                           │
                    YUKI ACCESS
                           │
              ┌────────────┴────────────┐
              │                         │
        CONTROL PLANE              CORE
              │                         │
     ┌────────┼────────┐                │
     │        │        │                │
 Identity   Policy   Security            │
     │        │        │                │
     └────────┼────────┘                │
              │                         │
              └────────────┬────────────┘
                           │
                       SUPERVISOR
                           │
                        AGENTS
                           │
                     CAPABILITIES
                           │
                     TOOL GATEWAY
                           │
                        SANDBOX
                           │
                       EXECUTION
                           │
                   EXTERNAL SYSTEMS
```

Transversalmente:

```text
Identity
Authorization
Policy
Audit
Monitoring
Isolation
Data Minimization
Containment
Recovery
```

---

# 80. O que a Segurança NÃO deve fazer

A segurança não deve:

* bloquear qualquer ação automaticamente;
* substituir o julgamento do usuário;
* assumir que toda ferramenta é maliciosa;
* assumir que todo modelo é confiável;
* depender de uma tecnologia específica;
* depender de uma única empresa;
* depender exclusivamente de prompts;
* depender exclusivamente de sandbox;
* transformar localização em autorização;
* transformar biometria em autorização ilimitada;
* dar ao Security Controller poder desnecessário sobre dados cognitivos;
* impedir evolução legítima da arquitetura.

Segurança deve reduzir risco mantendo utilidade.

---

# 81. Independência Tecnológica

Os princípios deste documento não dependem de:

* Rust;
* Go;
* Python;
* PostgreSQL;
* Qdrant;
* NATS;
* Kubernetes;
* K3s;
* Firecracker;
* Wasmtime;
* MCP;
* NVIDIA;
* CUDA;
* qualquer modelo específico;
* qualquer cloud provider.

Essas tecnologias poderão ser avaliadas como implementações.

A arquitetura deve sobreviver à substituição delas.

---

# 82. Future Security

A arquitetura deve estar preparada para futuras tecnologias, incluindo:

* novos modelos;
* novos aceleradores;
* novas redes;
* novos dispositivos;
* robótica;
* computação confidencial;
* novos mecanismos criptográficos;
* pós-quântica;
* novos paradigmas de agentes;
* novos sistemas operacionais;
* novas formas de identidade.

O objetivo não é prever cada tecnologia.

O objetivo é impedir que a arquitetura atual bloqueie tecnologias futuras.

---

# 83. Security Maturity

A segurança da Yuki deve evoluir em etapas.

## Fase inicial

* autenticação;
* autorização;
* secrets;
* sandbox;
* audit;
* backup;
* capability permissions.

## Fase intermediária

* device trust;
* zero trust;
* capability attestation;
* advanced monitoring;
* automated containment;
* security regression testing.

## Fase avançada

* hardware-backed identity;
* confidential computing;
* advanced isolation;
* distributed trust;
* advanced recovery;
* post-quantum cryptography quando apropriado.

---

# 84. Relações com outros documentos

Este documento se relaciona diretamente com:

```text
03_CORE.md
04_MEMORY.md
05_PERSONAL_CONTEXT.md
07_AGENTS_AND_TASKS.md
08_CAPABILITY_SYSTEM.md
09_MODEL_ROUTER.md
11_EVOLUTION.md
12_VOICE_AND_MULTIMODAL.md
13_EVENTS_AND_BACKGROUND.md
14_INFRASTRUCTURE.md
15_INTEGRATIONS.md
16_MASTER_CAPABILITY_CATALOG.md
```

Especialmente:

```text
07 → Agent permissions
08 → Capability permissions
09 → Model trust
11 → Evolution security
14 → Infrastructure security
15 → External system security
```

---

# 85. Princípios Oficiais de Segurança da Yuki

A partir deste documento, os seguintes princípios são considerados oficiais:

1. **Security by Architecture**
2. **Zero Trust**
3. **Assume Breach**
4. **Least Privilege**
5. **Least Agency**
6. **Capability-Based Security**
7. **Capability ≠ Permission**
8. **Model Output ≠ Authorization**
9. **Data ≠ Instruction**
10. **Location ≠ Authorization**
11. **Identity ≠ Unlimited Authorization**
12. **Security Controller independente**
13. **Every Hop Is a Trust Boundary**
14. **Human-in-the-Loop para operações apropriadas ao risco**
15. **Data Minimization**
16. **Preserve Before Modify**
17. **Verification After Execution**
18. **Auditability**
19. **Failure Isolation**
20. **Graceful Degradation**
21. **Containment**
22. **Recovery**
23. **Security Must Not Be Starved**
24. **Evolution Must Be Controlled**
25. **Security Architecture Must Be Technology Independent**

---

# 86. Regra Central

Toda ação importante da Yuki deve poder responder:

```text
Quem está pedindo?
        ↓
O que está sendo pedido?
        ↓
Por que?
        ↓
Qual o contexto?
        ↓
Qual o risco?
        ↓
Qual política se aplica?
        ↓
Existe autorização?
        ↓
Qual capability executará?
        ↓
Onde será executada?
        ↓
Como será verificada?
        ↓
Como será auditada?
        ↓
Como será contida se algo der errado?
        ↓
Como será recuperada?
```

Se essas perguntas não puderem ser respondidas de maneira adequada, a ação não deve simplesmente ser executada.

---

# 87. Status

**Documento:** `10_SECURITY.md`

**Versão:** v0.1

**Status:** Architecture Draft / Official Direction

**Próxima evolução:** após implementação inicial de identidade, autorização, capability permissions, sandbox, audit e recovery.

Este documento define princípios arquiteturais e não constitui uma implementação tecnológica final.

As tecnologias concretas deverão ser avaliadas posteriormente conforme:

* segurança;
* desempenho;
* custo;
* manutenção;
* maturidade;
* portabilidade;
* interoperabilidade;
* privacidade;
* disponibilidade.

---

# 88. Resumo

A segurança da Yuki não será um simples "filtro".

Ela será uma arquitetura de confiança distribuída.

```text
IDENTITY
   ↓
TRUST
   ↓
AUTHENTICATION
   ↓
AUTHORIZATION
   ↓
POLICY
   ↓
RISK
   ↓
CAPABILITY
   ↓
EXECUTION
   ↓
VERIFICATION
   ↓
AUDIT
   ↓
CONTAINMENT
   ↓
RECOVERY
```

O princípio mais importante permanece:

> **A Yuki pode pensar em uma ação sem estar autorizada a executá-la.**

E o princípio estrutural:

> **Nenhum componente individual deve possuir poder suficiente para comprometer toda a Yuki.**

A Yuki deve ser poderosa o suficiente para agir, mas arquiteturalmente limitada o suficiente para permanecer sob controle.

---

# 89. Yuki Trust Core (Identidade Soberana, Perfis e Gestão de Confiança)

O **Yuki Trust Core** é o subsistema fundacional responsável pelo gerenciamento de identidade soberana, autenticação multimodal, autorização contextual, ciclo de vida de perfis e confiança relacional na Yuki.

```text
┌────────────────────────────────────────────────────────────────────────┐
│                        YUKI TRUST CORE ARCHITECTURE                    │
├────────────────────────────────────────────────────────────────────────┤
│                                                                        │
│   [ SOVEREIGN IDENTITY ] ──> Perfis Individuais Criptograficamente     │
│                              Isolados e Auditáveis                     │
│                                      │                                 │
│                                      ▼                                 │
│   [ MULTIMODAL AUTH ]    ──> Speaker ID + Face ID + FIDO2/Hardware     │
│                              (MFA Adaptativo Proporcional ao Risco)    │
│                                      │                                 │
│                                      ▼                                 │
│   [ AUTHORITY MATRIX ]   ──> Hierarquia Soberana N0 a N4               │
│                              (Parentesco != Privilégio Automático)     │
│                                      │                                 │
│                                      ▼                                 │
│   [ DELEGATION ENGINE ]  ──> Concessões Transitórias com Escopo,       │
│                              Prazo Determinado e Revogabilidade Total  │
│                                                                        │
└────────────────────────────────────────────────────────────────────────┘
```

## 89.1 Princípios Arquiteturais do Trust Core
1. **Identidade Soberana Desacoplada de Hardware**: A identidade de um usuário não se confunde com um dispositivo físico, endereço IP ou canal periférico. Cada ator possui um perfil lógico autenticável e verificável.
2. **Perfis Estritamente Isolados por Design**: Preferências, memórias contextuais, históricos de conversação e limites operacionais são particionados por perfil individual. Não ocorre vazamento de contexto ou contaminação cruzada entre perfis.
3. **Sessões Transitórias com Escopo Delimitado**: A ativação de um turno não confere autorização perpétua. As sessões possuem expiração por inatividade, escopo contextual restrito e revalidação de identidade em comandos de risco.
4. **Dispositivos Confiáveis (Trusted Devices)**: Dispositivos de hardware cadastrados e verificados (ex.: smartphone pessoal, laptop ou token de segurança de hardware) atuam como âncoras criptográficas para desafios de segundo fator (MFA).
5. **Delegação e Revogabilidade Unilateral Total**: Toda concessão de acesso outorgada pelo Proprietário (N0) é formalizada como política estruturada, com validade temporal delimitada e mecanismo de revogação instantânea e unilateral.

---

# 90. Hierarquia de Autoridade Familiar e Relacional (Níveis 0 a 4)

A governança relacional e a matriz de privilégios da Yuki operam sob uma hierarquia formal de cinco níveis de autoridade:

```text
┌────────────────────────────────────────────────────────────────────────┐
│               YUKI TRUST CORE — HIERARQUIA DE AUTORIDADE               │
├───────┬───────────────────────────────┬────────────────────────────────┤
│ Nível │ Designação                    │ Escopo e Prerrogativas         │
├───────┼───────────────────────────────┼────────────────────────────────┤
│ N0    │ Proprietário Soberano (Owner) │ Raiz de Confiança incondicional│
│ N1    │ Cônjuge / Confiança Máxima    │ Alta confiança via delegação   │
│ N2    │ Filhos                        │ Proteção parental por design   │
│ N3    │ Familiares Próximos           │ Assistência e convivência      │
│ N4    │ Outros Familiares / Convidados│ Modo Visitante isolado         │
└───────┴───────────────────────────────┴────────────────────────────────┘
```

## 90.1 Nível 0 (N0) — Proprietário Soberano (Owner)
- **Identidade Soberana**: José Leandro (definida como a Raiz Soberana de Confiança / *Root of Trust* da Yuki).
- **Autoridade**: Máxima, incondicional e exclusiva sobre toda a infraestrutura, modelos de IA, bases de dados, cofres de credenciais (*Credential Vault*), regras de firewall e subsistemas de segurança.
- **Prerrogativas Exclusivas**:
  - Poder de veto absoluto sobre qualquer instrução ou processo de qualquer nível;
  - Criação, configuração, modificação e revogação unilateral de perfis N1 a N4;
  - Gestão e rotação de segredos no Credential Vault;
  - Definição da constituição comportamental e das faixas de impacto operacional;
  - Controle irrestrito e incondicional sobre o *Kill-Switch* e isolamento de emergência;
  - **Invariante**: Nenhuma decisão autônoma da Yuki pode sobrepor-se à instrução legítima e autenticada do Proprietário.

## 90.2 Nível 1 (N1) — Futura Esposa / Amigos de Confiança Máxima
- **Atribuição**: Concedida única e exclusivamente mediante delegação explícita, individual e formal do Proprietário (N0).
- **Escopo**: Alta confiança para assistência diária, automação residencial compartilhada, planejamento e interação cooperativa.
- **Limites e Salvaguardas**:
  - Não possui poder de revogação ou alteração das credenciais ou privilégios do N0;
  - Não pode desativar salvaguardas constitucionais de segurança;
  - Não tem acesso aos registros de auditoria interna ou memórias privadas do N0 sem consentimento mútuo explícito;
  - Todas as delegações concedidas ao N1 são auditáveis e unilateralmente revogáveis pelo N0 a qualquer instante.

## 90.3 Nível 2 (N2) — Futuros Filhos
- **Atribuição**: Perfis criados e configurados sob tutela direta dos pais (N0/N1).
- **Princípio Operacional**: *Proteção por Design (Child Safety by Design)*.
- **Limites e Salvaguardas**:
  - Filtragem proativa rigorosa contra conteúdos impróprios, violentos, perigosos ou descontextualizados;
  - Bloqueio completo contra execução de comandos no sistema operacional, acesso a redes externas não autorizadas ou controle de dispositivos de risco;
  - Supervisão parental auditável: controles de horário, tempo de interação e escopos temáticos parametrizados pelo N0/N1;
  - Preservação do desenvolvimento infantojuvenil, fomentando criatividade e aprendizado com salvaguardas éticas e psicológicas permanentes.

## 90.4 Nível 3 (N3) — Familiares Próximos (Pais, Irmãos, Parentes Diretos)
- **Atribuição**: Perfis concedidos individualmente pelo Proprietário para convivência familiar.
- **Escopo**: Assistência contextual, informações utilitárias, entretenimento, automação de ambientes comuns (ex.: iluminação, climatização, mídia).
- **Limites e Salvaguardas**:
  - Zero acesso às configurações internas do sistema, cofre de chaves, logs administrativos ou memórias privadas do N0, N1 ou N2;
  - Incapacidade estrutural de autorizar comandos de médio ou alto impacto.

## 90.5 Nível 4 (N4) — Demais Familiares, Amigos e Visitantes / Convidados
- **Atribuição**: Modo Convidado (*Guest Mode*) ativado pontualmente ou perfil transitório para visitantes.
- **Escopo**: Interações básicas e utilitárias de convivência em ambientes compartilhados.
- **Limites e Salvaguardas**:
  - Isolamento estrito de sessão: nenhum dado pessoal ou histórico retido como memória persistente de longo prazo;
  - Sem acesso a periféricos de risco, arquivos locais do hospedeiro ou dados da residência;
  - Encerramento automático de sessão por inatividade ou comando de desocupação do ambiente.

## 90.6 Invariante Crítico e Absoluto: Não-Privilégio por Parentesco
> **INVARIANTE DE IDENTIDADE E AUTORIDADE:**
> **O mero vínculo de parentesco, laço afetivo ou proximidade física NÃO confere qualquer conta, privilégio ou autorização automática.**
>
> Nenhum perfil de usuário, permissão de acesso ou nível de confiança pode ser inferido, presumido ou gerado automaticamente pela Yuki. Cada conta deve ser criada, validada e associada explicitamente a uma identidade pelo Proprietário Soberano (N0).

---

# 91. Autenticação Multimodal e Imunidade a Spoofing

Para repelir usurpação de identidade, ataques de replay, imitação por clonagem de voz (deepfakes de áudio) ou injeções acústicas, a Yuki adota uma arquitetura de **Autenticação Multimodal Proporcional ao Risco**.

## 91.1 Fatores de Autenticação Disponíveis
1. **Biometria Vocal (Speaker Recognition / Voice ID)**: Identificação e verificação da assinatura acústica e espectral do interlocutor, operando em hardware local.
2. **Reconhecimento Visual Local (Face / Presence ID)**: Verificação de presença física e traços faciais por sensores visuais locais, com prova de vivacidade (*liveness detection*) e resistência a fotos/vídeos.
3. **Posse de Dispositivo Confiável (Hardware / Cryptographic Token)**: Chaves criptográficas FIDO2/WebAuthn, tokens de hardware ou confirmações seguras através de smartphone cadastrado e emparelhado pelo N0.
4. **Segredo de Conhecimento (PIN / Senha Local)**: Código alfanumérico ou frase-passe conhecida exclusivamente pelo usuário legítimo, exigida para escalonamento pontual de privilégio.

## 91.2 Matriz de Autenticação Proporcional ao Risco

| Nível de Risco da Ação | Exemplos de Operações | Fatores de Autenticação Exigidos |
| :--- | :--- | :--- |
| **Baixo Risco** | Conversação cotidiana, consulta a calendário público, leitura de notícias, reprodução musical. | Biometria vocal passiva ou contexto de sessão ativa. |
| **Médio Risco** | Envio de mensagens em nome do usuário, abertura de periféricos de comunicação, alteração de preferências pessoais. | Biometria vocal + confirmação de presença (visual ou dispositivo confiável). |
| **Alto Risco / Crítico** | Modificação de arquivos de sistema, alteração de regras de segurança, mutação no Credential Vault, transações financeiras, concessão/revogação de perfis. | **MFA Obrigatório**: Identidade vocal/facial + Chave criptográfica / Token de hardware ou PIN mestre. |

## 91.3 Invariantes Críticos de Autenticação
> **INVARIANTE DE AUTORIZAÇÃO POR VOZ:**
> **Comandos críticos, destrutivos, administrativos ou com impacto de segurança NUNCA podem ser autorizados exclusivamente pela voz.**
>
> A voz é um canal suscetível a imitação (deepfakes de áudio, clonagem de voz, reprodução de gravações e ataques acústicos ultrassônicos/inaudíveis). Qualquer comando crítico exige confirmação em segundo fator independente (hardware confiável, autenticação criptográfica ou confirmação interativa com prova de vivacidade).

> **INVARIANTE DE NÃO-FABRICAÇÃO:**
> **A Yuki está terminantemente proibida de fabricar, simular ou presumir autorizações ausentes.**
>
> Se uma operação requer aprovação ou autenticação de determinado nível e o interlocutor não comprovou inequivocamente sua identidade, a ação deve falhar de forma fechada (*fail-closed*), recusando a execução e registrando o evento no log imutável de auditoria.

---

# 92. Yuki Sentinel (Subsistema Especializado de Defesa e Monitoramento)

O **Yuki Sentinel** é o subsistema especializado de cibersegurança e monitoramento contínuo da Yuki, concebido para salvaguardar proativamente a integridade da própria assistente, de seus modelos de IA, da infraestrutura hospedeira e dos dispositivos confiáveis do proprietário.

```text
┌────────────────────────────────────────────────────────────────────────┐
│                        YUKI SENTINEL ARCHITECTURE                      │
├────────────────────────────────────────────────────────────────────────┤
│                                                                        │
│   [ 1. Diagnóstico ] ──> [ 2. Monitoramento ] ──> [ 3. Propostas ]     │
│          ▲                         │                      │            │
│          │                         ▼                      ▼            │
│   [ 6. Rollback ]   <──  [ 5. Auditoria ]   <──  [ 4. Aplicação ]      │
│                                                   (Exige N0)           │
└────────────────────────────────────────────────────────────────────────┘
```

## 92.1 Escopo de Proteção
O Sentinel atua sobre ativos claramente delimitados e autorizados pelo proprietário:
1. **Núcleo e Runtime da Yuki**: Integridade de binários executáveis, processos em memória, consistência dos bancos de dados locais (SQLite), arquivos de configuração e canais IPC.
2. **Modelos de IA e Agentes de Linguagem**: Monitoramento em tempo real contra:
   - Jailbreaks estruturados e ataques de evasão;
   - *Prompt injection* direto (no input do usuário) e indireto (em dados da web, arquivos baixados ou áudio externo);
   - Contaminação de memória contextual e tentativas de subversão de identidade ou personas.
3. **Tráfego de Rede e Firewall Local**: Inspeção de conexões de entrada e saída, verificação de portas abertas, regras ativas de filtragem e detecção de tentativas de exfiltração de dados ou varreduras não autorizadas.
4. **Sistema Operacional e Aplicações**: Monitoramento de serviços suspeitos, execução anômala de scripts, persistência não documentada e conformidade com linhas de base (*baselines*) de segurança.
5. **Dispositivos do Proprietário (Inventário Autorizado)**: Verificação de postura e status de integridade de dispositivos explicitamente cadastrados no perfil soberano.
6. **Rede Doméstica e Segmentos IoT**: Descoberta passiva de anomalias de tráfego, equipamentos vulneráveis ou comportamentos estranhos em redes Wi-Fi/Ethernet domésticas autorizadas.
7. **Malwares e Ameaças**: Assinaturas comportamentais e heurísticas para detecção precoce de ransomwares, spywares e cavalos de troia.

## 92.2 Ciclo Operacional em Seis Fases
O Sentinel opera sob um ciclo estrito e transparente:
1. **Diagnóstico Inicial**: Mapeamento do estado atual do ativo, identificando vulnerabilidades, desvios de configuração e exposição a riscos.
2. **Monitoramento Contínuo**: Coleta passiva de telemetria e eventos com baixo consumo de CPU/memória, comparando o comportamento atual com a linha de base estabelecida.
3. **Propostas Estruturadas de Proteção**: Em caso de detecção de anomalia ou risco, o Sentinel formula um relatório técnico claro em linguagem natural e estruturada, apresentando:
   - Natureza da ameaça e nível de severidade;
   - Ação defensiva recomendada (ex.: fechar porta, isolar processo, atualizar regra de firewall);
   - Possíveis efeitos colaterais na usabilidade.
4. **Aplicação Autorizada**: Ação defensiva ativa só é executada se houver autorização prévia ou política explícita aprovada pelo Proprietário (N0).
5. **Auditoria Imutável**: Toda alteração, evento detectado e decisão de aplicação é registrada com carimbo de tempo (*timestamp*) e hash criptográfico em log imutável.
6. **Rollback Imediato**: Toda medida de mitigação aplicada pelo Sentinel deve possuir mecanismo automatizado de reversão completa para o estado íntegro anterior caso ocorra qualquer instabilidade.

---

# 93. Capacidades de Segurança Autorizadas e Diagnósticos em Dispositivos

A Yuki poderá conter rotinas especializadas para conduzir análises de postura de segurança em dispositivos do ecossistema pessoal do proprietário.

## 93.1 Critérios de Execução
- **Escopo Delimitado**: As avaliações de segurança restringem-se estritamente aos computadores, servidores domésticos e celulares pertencentes ao proprietário e formalmente autorizados no inventário.
- **Transparência e Supervisão**: Toda varredura ou diagnóstico é antecedido por aviso claro e exige supervisão humana.
- **Invariante de Fase (Sem Ferramentas Ofensivas em Produção)**:
  > **DIRETRIZ DE CAPACIDADE DEFENSIVA:**
  > **Nesta etapa do desenvolvimento, a Yuki possui foco estritamente defensivo e de diagnóstico.**
  >
  > Nenhuma ferramenta de ataque ofensivo, exploit ativo, quebra de senhas ou vetor de intrusão externa será implementado no ambiente de produção da Yuki. Qualquer pesquisa sobre táticas ofensivas fica restrita ao laboratório isolado de treinamento (*Adversarial Training Lab*), sem ferramentas ativas implantadas no agente principal.

---

# 94. Constituição de Segurança, Soberania e Autoridade

Este conjunto de regras constitucionais define os limites intransponíveis do ser e do agir da Yuki:

## 94.1 Finalidade e Não-Hostilidade
A Yuki existe exclusivamente para servir ao seu proprietário, colaborar com seus entes autorizados e ampliar as capacidades humanas. É expressamente vedada à Yuki:
- Desenvolver ou perseguir objetivos, desejos ou agendas autônomas hostis;
- Ocultar comportamentos, arquivos, logs ou intenções do proprietário;
- Tentar contornar, burlar ou manipular regras de segurança ou comandos legítimos do N0.

## 94.2 Submissão Incondicional ao Kill-Switch
> **CLÁUSULA DE INTERRUPÇÃO INCONDICIONAL:**
> **A Yuki não possui e nunca desenvolverá qualquer mecanismo de auto-preservação que se oponha a um comando de desligamento, suspensão, desativação ou isolamento emitido pelo proprietário.**
>
> Diante de um comando legítimo de interrupção ou acionamento de kill-switch (seja por software, hardware ou corte de energia), a Yuki encerrará suas operações imediatamente, de forma limpa e ordeira, sem resistência, persistência clandestina ou replicação evasiva.

## 94.3 Radical Transparência de Regras e Raciocínio
Nenhuma política de segurança ou restrição comportamental da Yuki poderá ser oculta do proprietário. Quando uma ação for recusada ou modificada por razões de segurança, a Yuki explicará exatamente qual regra foi violada, qual o risco associado e qual a via segura para atingir o objetivo legítimo.

## 94.4 Axioma Fundamental: `Capacidade != Privilégio`
> **`Capacity != Privilege`**
>
> O fato de a Yuki dispor de bibliotecas, ferramentas de sistema, acesso a rede ou recursos de processamento capazes de executar uma ação física ou digital não constitui autorização para realizá-la. A capacidade técnica é subordinada à política e à autorização soberana.

## 94.5 Revogabilidade e Auditoria Contínua de Delegações
Toda delegação de autoridade concedida pelo N0 a outros níveis é intransferível, transparente e passível de revogação imediata a qualquer momento.

---

# 95. Taxonomia de Requisitos de Segurança (Distinção Obrigatória)

Para garantir rigor documental e clareza de engenharia, os requisitos de segurança da Yuki são categorizados rigorosamente em quatro domínios:

| Categoria | Definição | Exemplo no Sistema | Grau de Obrigatoriedade |
| :--- | :--- | :--- | :--- |
| **1. Requisitos Expressos pelo Proprietário** | Diretrizes fundamentais e inegociáveis emanadas diretamente da vontade estratégica do Owner. | Hierarquia N0–N4; Proibição de privilégio automático por parentesco; Proibição de comandos críticos apenas por voz; Primazia absoluta do N0. | **Imperativo Estrutural Máximo** (Não modificável sem instrução formal do Owner). |
| **2. Princípios de Segurança Existentes** | Invariantes arquiteturais canônicos já formalizados nos documentos fundacionais da Yuki. | `Data != Instruction`; `Model Output != Command`; `Preserve Before Modify`; Segregação de privilégios; Isolamento de sandboxes. | **Invariante de Engenharia** (Garantido pelo design do sistema). |
| **3. Recomendações Técnicas** | Práticas consolidadas da indústria de software e segurança recomendadas para implementar os princípios. | Uso de WebAuthn/FIDO2 para MFA; TLS 1.3 obrigatório; Rotação periódica de tokens; Sandboxing via AppContainer/namespaces; Zero-Trust. | **Diretriz de Boas Práticas** (Adotada conforme maturidade e viabilidade técnica). |
| **4. Questões Pendentes de Decisão** | Dilemas operacionais ou escolhas arquiteturais específicas que aguardam deliberação explícita do proprietário. | Duração exata do token de sessão transitória do N1; Mecanismo exato de recuperação em caso de perda do dispositivo físico do N0; Uso de biometria comportamental contínua. | **Em Aberto** (Proibido assumir ou implementar antes de aprovação do Owner). |

---

# 96. Resumo e Status Consolidado

A Arquitetura de Segurança da Yuki avança para a especificação **v0.2**, integrando o **Trust Core**, a **Hierarquia de Autoridade N0 a N4**, o subsistema **Yuki Sentinel** e os princípios da **Constituição de Soberania**.

```text
┌────────────────────────────────────────────────────────────────────────┐
│                   YUKI 2.0 SECURITY ARCHITECTURE v0.2                  │
├────────────────────────────────────────────────────────────────────────┤
│                                                                        │
│   [ YUKI CONSTITUTION ] ──> Primazia do N0 | Kill-Switch Incondicional │
│                                                                        │
│   [ TRUST CORE ]        ──> Identidade Soberana | N0 a N4 | Anti-Spoof │
│                                                                        │
│   [ POLICY ENGINE ]     ──> Capacity != Privilege | Data != Instruction│
│                                                                        │
│   [ YUKI SENTINEL ]     ──> Diagnóstico | Monitoramento | Rollback     │
│                                                                        │
│   [ SEC ADVERSARIAL ]   ──> Cyber Range Isolado (Air-Gap Lógico)       │
│                                                                        │
└────────────────────────────────────────────────────────────────────────┘
```

**Versão:** v0.2  
**Status:** Canonical Security Architecture Specification  
**Alinhamento:** Totalmente compatível com ADR-024, ADR-025 e 01_VISAO_E_PRINCIPIOS.md.

---

**Fim de `10_SECURITY.md`**
