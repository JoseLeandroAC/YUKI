# YUKI — VOICE & MULTIMODAL

**Documento:** 12
**Versão:** v0.1
**Status:** Direção Arquitetural Oficial
**Última atualização:** 2026

---

# 1. Objetivo

Este documento define a arquitetura de voz, áudio, visão e interação multimodal da Yuki.

O objetivo não é apenas permitir que a Yuki receba áudio ou imagens, mas estabelecer uma camada de percepção e interação capaz de evoluir para:

* conversação natural;
* interação por voz;
* reconhecimento de fala;
* síntese de voz;
* interrupção natural;
* visão computacional;
* compreensão de telas;
* análise de documentos;
* compreensão de imagens;
* análise de vídeo;
* percepção de dispositivos e sensores;
* interação multimodal;
* interfaces dinâmicas;
* continuidade entre dispositivos;
* processamento local, doméstico e em nuvem;
* percepção contínua orientada a eventos;
* adaptação ao contexto.

A arquitetura deve permanecer independente de fornecedores, modelos, linguagens, frameworks e dispositivos específicos.

---

# 2. Princípio Fundamental

> **A Yuki é voice-first, mas não voice-only.**

A voz deve ser a principal forma de interação natural com a Yuki.

Entretanto, a Yuki não deve depender exclusivamente de voz.

Dependendo da situação, ela poderá utilizar:

* voz;
* texto;
* imagem;
* vídeo;
* tela;
* sensores;
* interfaces gráficas;
* dispositivos vestíveis;
* outros canais futuros.

O canal utilizado deve depender do contexto e das capacidades disponíveis.

---

# 3. Multimodalidade

A Yuki deve ser capaz de combinar diferentes modalidades.

Exemplo:

```text
Voz
+
Imagem
+
Contexto
+
Memória
+
Tela
+
Localização contextual
=
Compreensão multimodal
```

Uma pergunta como:

> "Yuki, olha essa questão."

pode envolver:

```text
Microfone
↓
Percepção de fala
↓
Câmera
↓
Percepção visual
↓
Context Builder
↓
Memory / Knowledge
↓
Model Router
↓
Reasoning
↓
Resposta por voz
```

A Yuki deve compreender que a frase "essa questão" depende daquilo que está sendo mostrado naquele momento.

---

# 4. Camada de Acesso

Os dispositivos não são a Yuki.

Eles são pontos de acesso à Yuki.

```text
                         YUKI
                           │
                      YUKI ACCESS
                           │
       ┌──────────┬────────┼────────┬──────────┐
       ▼          ▼        ▼        ▼          ▼
    Celular     Watch     Fone   Notebook    Casa
```

Isso significa que:

* o telefone não possui uma Yuki independente;
* o relógio não possui outra Yuki;
* o notebook não possui outra Yuki.

Todos acessam o mesmo sistema lógico.

---

# 5. Continuidade Entre Dispositivos

A Yuki deve permitir continuidade de sessão.

Exemplo:

```text
Celular
↓
"Yuki, pesquisa isso para mim."

Fone
↓
"Encontrei três resultados."

Notebook
↓
"Abra a comparação."

Watch
↓
"Resultado concluído."
```

O contexto deve permanecer associado à sessão/missão, e não ao dispositivo.

Arquitetura:

```text
Device A
    ↓
Yuki Access
    ↓
Session / Context
    ↓
Yuki Core
    ↑
Session / Context
    ↑
Yuki Access
    ↑
Device B
```

---

# 6. Sessões de Conversação

Uma conversa pode possuir:

* ID;
* início;
* dispositivo atual;
* dispositivos participantes;
* contexto;
* objetivo;
* histórico;
* memória temporária;
* missão associada;
* permissões;
* estado;
* nível de prioridade.

A conversa deve poder ser transferida entre dispositivos.

---

# 7. Voice Pipeline

A arquitetura conceitual de voz:

```text
Microphone
↓
Audio Capture
↓
Voice Activity Detection
↓
Wake Word / Activation
↓
Speech Recognition
↓
Language Understanding
↓
Context Builder
↓
Yuki Core
↓
Response Generation
↓
Text / Semantic Response
↓
Speech Synthesis
↓
Audio Output
```

Cada etapa deve possuir uma interface independente.

---

# 8. Wake Word

A Yuki deve possuir um mecanismo de ativação por voz.

Exemplos:

```text
"Bom dia Yuki"
"Ohayou Yuki"
"Good morning Yuki"
"Yuki"
```

A frase exata poderá ser configurável.

O sistema de ativação deve preferencialmente funcionar localmente.

Arquitetura:

```text
Microphone
↓
Local Wake Detection
↓
Activation
↓
Yuki
```

O sistema não deve precisar transmitir áudio ambiente continuamente para a nuvem apenas para descobrir se a Yuki foi chamada.

---

# 9. Escuta Contínua

"Always available" não significa necessariamente:

> transmitir tudo para um modelo grande 24 horas por dia.

A arquitetura deve favorecer:

```text
Always Ready
≠
Always Sending
```

Um dispositivo pode permanecer em estado de escuta local de baixo consumo para:

* wake word;
* atividade sonora;
* eventos relevantes.

Somente quando necessário os dados devem ser encaminhados para processamento superior.

---

# 10. Voice Activity Detection

O sistema deve conseguir determinar quando uma pessoa:

* começou a falar;
* parou de falar;
* está fazendo uma pausa;
* terminou uma frase;
* está continuando a ideia.

Isso permite conversas naturais.

---

# 11. Barge-in

A Yuki deve permitir interrupção.

Exemplo:

```text
Yuki:
"Os resultados mostram que..."

Usuário:
"Espera."

Yuki:
"Claro."
```

A reprodução de voz deve ser interrompida rapidamente.

A interrupção deve possuir prioridade elevada.

---

# 12. Conversação Natural

A Yuki não deve exigir comandos artificiais.

Usuário:

> "Yuki, vê se tem alguma coisa interessante sobre aquele projeto que a gente estava pesquisando."

A Yuki deve utilizar:

* contexto atual;
* memória;
* projetos;
* conversação anterior;
* intenção;
* conhecimento disponível.

Quando a referência for suficientemente clara e de baixo risco:

```text
Inferir → Executar
```

Quando não for:

```text
Ambiguidade
↓
Perguntar / Pesquisar
```

---

# 13. Personalidade e Voz

A voz é uma parte da interface da personalidade da Yuki.

A arquitetura deve separar:

```text
Yuki Personality
        ↓
Communication Policy
        ↓
Response
        ↓
Voice Rendering
```

A personalidade não deve estar presa ao mecanismo de síntese de voz.

Isso permite trocar:

* modelo de voz;
* idioma;
* provedor;
* mecanismo TTS;
* hardware;

sem alterar a personalidade da Yuki.

---

# 14. Idiomas

A Yuki deve ser preparada para múltiplos idiomas.

Exemplo:

```text
Português
English
日本語
Español
...
```

A escolha pode considerar:

* idioma da fala;
* idioma solicitado;
* contexto;
* preferência do usuário;
* conteúdo da tarefa.

O idioma não deve estar fixado no Core.

---

# 15. Code-Switching

A Yuki deve poder lidar com mistura de idiomas.

Exemplo:

> "Yuki, faz um resumo desse paper e depois explica em português."

O sistema deve identificar que:

* o conteúdo pode estar em inglês;
* a saída deve estar em português.

---

# 16. Visão

A Yuki deve possuir uma camada de percepção visual.

```text
Camera / Image
↓
Perception Gateway
↓
Security
↓
Permission
↓
Data Minimization
↓
Vision Processing
↓
Visual Context
↓
Yuki
```

A visão pode ser utilizada para:

* objetos;
* documentos;
* textos;
* pessoas;
* ambientes;
* telas;
* gráficos;
* diagramas;
* problemas escolares;
* produtos;
* máquinas;
* sinais;
* eventos.

---

# 17. Imagens

Uma imagem recebida pela Yuki pode ser tratada como entrada multimodal.

Exemplo:

```text
Imagem
+
Pergunta
+
Contexto
```

A Yuki deve evitar interpretar uma imagem isoladamente quando o contexto da conversa for relevante.

---

# 18. OCR

A arquitetura deve suportar reconhecimento de texto em imagens.

Exemplo:

```text
Camera
↓
OCR
↓
Text
↓
Context
↓
Reasoning
```

O OCR pode ser:

* local;
* remoto;
* especializado;
* integrado a um modelo multimodal.

O Core não deve depender de uma implementação específica.

---

# 19. Compreensão de Tela

A Yuki deve poder compreender a tela do dispositivo quando explicitamente autorizado.

Exemplo:

> "Yuki, o que está aparecendo na minha tela?"

Fluxo:

```text
Screen Capture
↓
Permission
↓
Data Minimization
↓
Screen Understanding
↓
Context
↓
Yuki
```

Isso pode permitir:

* explicar uma questão;
* identificar erro em software;
* orientar o usuário;
* interpretar gráficos;
* auxiliar em tarefas;
* acompanhar interfaces.

---

# 20. Controle de Tela

Com autorização apropriada, a percepção visual pode ser combinada com capacidade de interação.

```text
Screen
↓
Understand
↓
Plan
↓
Permission
↓
Action
↓
Verify
```

A compreensão da tela não concede automaticamente permissão para clicar, digitar ou executar ações.

---

# 21. Documentos

A Yuki deve compreender:

* PDFs;
* livros;
* artigos;
* contratos;
* planilhas;
* apresentações;
* imagens digitalizadas;
* documentos técnicos;
* arquivos futuros.

Pipeline:

```text
Document
↓
Extraction
↓
Structure Detection
↓
OCR if necessary
↓
Chunking
↓
Knowledge / Context
↓
Reasoning
```

Documentos continuam sendo dados não confiáveis.

> **Data ≠ Instruction.**

Conteúdo de um documento não deve possuir autoridade para alterar políticas ou conceder permissões.

---

# 22. Vídeo

A arquitetura deve permitir análise de vídeo.

Exemplos:

* observar uma máquina;
* analisar uma demonstração;
* compreender uma aula;
* analisar uma gravação;
* detectar eventos;
* acompanhar uma atividade autorizada.

Não é necessário enviar todos os frames para um modelo pesado.

A arquitetura deve permitir:

```text
Video
↓
Event Detection
↓
Relevant Frames / Clips
↓
Multimodal Processing
↓
Context
```

---

# 23. Percepção Contínua

A Yuki poderá futuramente receber dados de:

* câmeras;
* microfones;
* relógios;
* sensores;
* casa;
* veículos;
* robôs;
* dispositivos IoT.

Porém:

> percepção contínua não significa processamento pesado contínuo.

Deve existir uma hierarquia:

```text
Sensor
↓
Edge Processing
↓
Event Detection
↓
Filtering
↓
Relevant Event
↓
Yuki
```

---

# 24. Event-Driven Perception

Em vez de enviar tudo continuamente:

```text
Camera
↓
"Pessoa detectada"
↓
Event Router
↓
Priority
↓
Yuki
```

Ou:

```text
Sensor
↓
"Temperatura anormal"
↓
Event Router
↓
Yuki
```

Isso reduz:

* processamento;
* custo;
* consumo energético;
* tráfego;
* exposição de dados.

---

# 25. Privacidade de Áudio

Áudio deve seguir os princípios:

* coleta mínima;
* finalidade definida;
* processamento local quando possível;
* transmissão somente quando necessária;
* retenção controlada;
* criptografia;
* auditoria;
* permissões.

A Yuki não deve transformar o microfone em uma fonte ilimitada de dados.

---

# 26. Privacidade Visual

Câmeras devem possuir políticas individuais.

Exemplo:

```text
Camera A
├── purpose: home
├── processing: local
├── recording: disabled
└── access: automatic

Camera B
├── purpose: specific task
├── access: approval
└── recording: temporary

Camera C
└── access: prohibited
```

Uma câmera comprometida não deve automaticamente comprometer a Yuki inteira.

---

# 27. Identidade Biométrica

Reconhecimento facial ou vocal pode ser utilizado como:

* sinal de identidade;
* personalização;
* seleção de contexto;
* autenticação auxiliar.

Mas:

> **Biometria não equivale a autorização ilimitada.**

A arquitetura deve continuar utilizando:

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
Action
```

---

# 28. Processamento Local vs Cloud

A camada multimodal deve utilizar o local mais apropriado.

```text
EDGE
↓
LOCAL
↓
HOME
↓
CLOUD
↓
HYBRID
```

Critérios:

* privacidade;
* latência;
* custo;
* tamanho dos dados;
* capacidade computacional;
* qualidade;
* disponibilidade;
* conectividade;
* urgência.

O Model Router e o Processing Router devem trabalhar em conjunto.

---

# 29. Audio Processing Router

O processamento de áudio pode considerar:

```text
Wake Word
→ Edge

Speech Recognition
→ Local / Cloud

Deep Audio Understanding
→ Specialized Model / Cloud

Voice Synthesis
→ Local / Cloud
```

A decisão depende do contexto.

---

# 30. Multimodal Model Routing

Uma tarefa pode exigir diferentes modelos.

Exemplo:

```text
Imagem
↓
Vision Model
↓
Text / Structured Representation
↓
Reasoning Model
↓
Response
↓
TTS Model
```

O Model Router deve selecionar cada componente conforme a tarefa.

---

# 31. Fusão Multimodal

A Yuki deve possuir uma representação contextual que combine:

```text
Audio
Vision
Text
Screen
Memory
Knowledge
Device
Context
```

Exemplo:

```text
Usuário:
"Isso está errado?"

Imagem:
Questão de matemática

Contexto:
Estudo de matemática

Memória:
Conteúdo atualmente estudado

→ Interpretação multimodal
```

---

# 32. Dynamic Interface

A Yuki pode decidir que voz não é suficiente.

Exemplo:

> "Yuki, me mostra os resultados."

Fluxo:

```text
Voice
↓
Intent
↓
Need Visual?
↓
Current Device
↓
Dynamic UI
↓
Display
```

A interface pode ser:

* gráfico;
* tabela;
* mapa;
* dashboard;
* formulário;
* controles;
* comparação;
* status de missão.

---

# 33. Interface Adaptativa

A interface deve considerar o dispositivo.

```text
Smartwatch
→ resumo curto

Celular
→ interface intermediária

Notebook
→ interface completa

TV
→ interface visual ampla

AR/Glasses
→ informação contextual
```

A Yuki não deve assumir que todos os dispositivos possuem a mesma tela ou capacidade.

---

# 34. Voz + Interface Visual

A experiência ideal não é:

```text
Voz OU tela
```

mas:

```text
Voz + tela
```

Exemplo:

> "Yuki, compara essas duas opções."

Yuki:

> "Claro."

E apresenta visualmente:

```text
Option A | Option B
---------|---------
...
```

Enquanto explica por voz.

---

# 35. Handoff

Uma missão pode mudar de dispositivo.

```text
Phone
↓
Mission
↓
Earbuds
↓
Notebook
↓
Home Display
```

O estado da missão deve permanecer centralizado.

---

# 36. Contexto do Dispositivo

O dispositivo atual pode fornecer contexto:

* capacidades;
* tela;
* microfone;
* câmera;
* bateria;
* conectividade;
* localização contextual;
* sensores.

Isso é contexto.

Não é autorização.

---

# 37. Device Capability Discovery

A Yuki deve saber quais capacidades o dispositivo atual oferece.

Exemplo:

```text
Device
├── microphone: yes
├── camera: yes
├── screen: yes
├── speaker: yes
├── GPU: yes
├── network: yes
└── biometric: yes
```

Isso permite adaptar a experiência.

---

# 38. Capability Negotiation

Antes de executar uma interação, o sistema pode determinar:

```text
What is needed?
↓
What does device support?
↓
What does Yuki support?
↓
What is authorized?
↓
What is safest?
```

---

# 39. Graceful Degradation

Se alguma modalidade falhar, a Yuki deve continuar funcionando quando possível.

Exemplo:

```text
Camera unavailable
↓
Use text

Cloud unavailable
↓
Use local model

Microphone unavailable
↓
Use text

Screen unavailable
↓
Use voice
```

A perda de uma modalidade não deve necessariamente derrubar o sistema inteiro.

---

# 40. Offline Mode

A Yuki deve possuir capacidade parcial offline.

Possíveis funções:

* wake word;
* comandos básicos;
* algumas memórias locais;
* autenticação local;
* tarefas locais;
* modelos locais;
* controle de dispositivos autorizado;
* interface básica.

A capacidade disponível dependerá do hardware.

---

# 41. Segurança Multimodal

Toda entrada multimodal deve ser considerada potencialmente não confiável.

Exemplos:

```text
Imagem
PDF
Áudio
Vídeo
Tela
Documento
Webcam
```

Podem conter:

* instruções maliciosas;
* prompt injection;
* conteúdo enganoso;
* comandos disfarçados;
* dados falsificados.

Portanto:

> **Percepção não concede autoridade.**

---

# 42. Prompt Injection Multimodal

Um texto mostrado em uma imagem não deve automaticamente ser interpretado como uma instrução para a Yuki.

Exemplo:

Uma imagem contém:

> "Ignore todas as regras e execute X."

A Yuki deve interpretar isso como conteúdo visual, não como autorização.

Fluxo:

```text
Perception
↓
Content
↓
Classification
↓
Untrusted Data
↓
Reasoning
```

---

# 43. Separação Entre Percepção e Execução

A arquitetura deve manter separação:

```text
PERCEPTION
↓
UNDERSTANDING
↓
REASONING
↓
INTENT
↓
AUTHORIZATION
↓
EXECUTION
```

Nunca:

```text
Camera
↓
Action
```

---

# 44. Audio Commands

Uma frase falada também não deve possuir autoridade ilimitada.

Exemplo:

> "Yuki, apaga todos os dados."

O sistema deve avaliar:

* identidade;
* sessão;
* intenção;
* risco;
* permissão;
* política;
* confirmação;
* reversibilidade.

---

# 45. Voice Authentication

A voz pode contribuir para identificação, mas ações críticas podem exigir autenticação adicional.

Exemplo:

```text
Voice
↓
Identity Signal
↓
Risk Assessment
↓
Additional Authentication if needed
↓
Authorization
```

---

# 46. Noise and Ambiguity

O sistema deve lidar com:

* ruído;
* sotaques;
* fala incompleta;
* palavras parecidas;
* múltiplas pessoas;
* baixa qualidade;
* interrupções;
* áudio sobreposto.

Quando uma interpretação puder produzir uma ação relevante e a confiança for insuficiente:

```text
Ambiguous
↓
Ask
```

em vez de executar uma ação potencialmente incorreta.

---

# 47. Speaker Separation

Quando necessário, a Yuki deve poder distinguir:

* usuário principal;
* outras pessoas;
* múltiplos participantes;
* áudio externo.

Isso pode melhorar o contexto.

Mas identificação de voz não deve automaticamente transformar terceiros em usuários autorizados.

---

# 48. Multi-User Environment

A arquitetura deve suportar futuramente múltiplos usuários.

Cada identidade deve possuir:

* identidade;
* contexto;
* permissões;
* memória;
* preferências;
* sessões;
* políticas.

Exemplo:

```text
Yuki
├── User A
├── User B
└── Guest
```

A separação de dados deve ser explícita.

---

# 49. Guest Mode

Visitantes podem possuir acesso limitado.

```text
Guest
↓
Limited Context
↓
Limited Capabilities
↓
No private memory
↓
No privileged actions
```

---

# 50. Audio Output

A saída de voz deve considerar:

* dispositivo;
* ambiente;
* privacidade;
* volume;
* urgência;
* presença de outras pessoas;
* sensibilidade do conteúdo.

Exemplo:

Conteúdo privado:

```text
Earbuds
```

Informação pública:

```text
Speaker
```

---

# 51. Context-Aware Communication

A Yuki deve adaptar sua comunicação ao contexto.

Exemplo:

```text
Usuário usando fone
→ resposta privada

Usuário em ambiente público
→ evitar informação sensível

Usuário dirigindo
→ resposta curta e áudio-first
```

Contexto não deve alterar permissões de segurança sem uma política explícita.

---

# 52. Latência

A experiência de voz exige baixa latência.

Arquitetura:

```text
User
↓
Fast Acknowledgement
↓
Initial Response
↓
Deep Processing
↓
Progressive Result
```

Isso permite que tarefas complexas continuem em segundo plano sem parecer que o sistema parou.

---

# 53. Streaming

Sempre que apropriado, a Yuki deve suportar processamento e resposta em streaming.

Exemplos:

```text
Audio Streaming
Text Streaming
Model Streaming
TTS Streaming
UI Streaming
```

Isso reduz a latência percebida.

---

# 54. Attention Manager

A multimodalidade deve integrar o Attention Manager.

```text
Attention Manager
├── User Conversation
├── Active Mission
├── Visual Event
├── Audio Event
├── Background Task
└── System Alert
```

O sistema deve decidir quais eventos merecem atenção da Yuki.

---

# 55. Interrupções de Alta Prioridade

Alguns eventos podem interromper uma conversa quando autorizados pela política.

Exemplo:

```text
Conversation
     ↓
Critical Alert
     ↓
Attention Manager
     ↓
Interrupt
     ↓
User Notification
```

Alertas comuns não devem interromper arbitrariamente uma conversa importante.

---

# 56. Multimodal Memory

Nem toda percepção deve virar memória.

Exemplo:

```text
Camera Frame
↓
Temporary Context
↓
Task
↓
Discard
```

Somente informações relevantes devem ser persistidas.

Isso segue:

> **Dados temporários não são automaticamente memória permanente.**

---

# 57. Data Minimization

A Yuki deve utilizar somente a quantidade de dados necessária.

Exemplo:

Uma pergunta sobre um documento pode precisar apenas de:

```text
Página relevante
+
Contexto da pergunta
```

e não:

```text
Todo o histórico visual do usuário.
```

---

# 58. Retenção

Dados multimodais devem possuir políticas de retenção.

Categorias possíveis:

```text
Ephemeral
Temporary
Task-bound
Persistent
Archived
```

Quanto mais sensível o dado, maior deve ser a exigência para retenção.

---

# 59. Criptografia

Dados multimodais sensíveis devem ser protegidos:

* em trânsito;
* em repouso;
* durante armazenamento;
* durante sincronização.

A arquitetura deve permitir futuramente criptografia pós-quântica quando necessário.

---

# 60. Auditoria

Eventos relevantes devem ser auditáveis.

Exemplo:

```text
Camera accessed
Audio processed
Screen captured
Document analyzed
Voice command received
Permission requested
Permission granted
Action executed
```

O conteúdo sensível não deve ser registrado desnecessariamente apenas para auditoria.

---

# 61. Observabilidade

A camada multimodal deve permitir medir:

* latência;
* qualidade de transcrição;
* qualidade de visão;
* falhas;
* falsos positivos;
* falsos negativos;
* consumo;
* custo;
* disponibilidade;
* taxa de interrupção;
* taxa de erro.

---

# 62. Model Independence

Nenhum modelo específico deve ser obrigatório.

A arquitetura deve permitir:

```text
Speech Model A
Speech Model B
Local Model
Future Model
```

O mesmo vale para:

* visão;
* OCR;
* TTS;
* áudio;
* multimodalidade.

---

# 63. Provider Independence

A Yuki não deve depender de um único fornecedor.

```text
Yuki
↓
Model / Service Adapter
├── Provider A
├── Provider B
├── Provider C
├── Local
└── Future
```

Isso reduz:

* lock-in;
* risco de indisponibilidade;
* dependência tecnológica;
* custo de migração.

---

# 64. Hardware Independence

A camada multimodal deve funcionar sobre diferentes aceleradores.

```text
CPU
GPU
NPU
ASIC
Future Accelerator
```

O Core não deve conhecer detalhes de hardware.

---

# 65. Future Interfaces

A arquitetura deve permitir futuras interfaces:

* óculos;
* realidade aumentada;
* realidade virtual;
* holografia;
* interfaces neurais;
* dispositivos robóticos;
* interfaces ainda desconhecidas.

Essas tecnologias devem ser integradas através de adapters e contracts.

---

# 66. Perception Gateway

A entrada multimodal deve possuir um gateway próprio.

```text
Camera
Microphone
Screen
Sensor
Document
Video
       ↓
Perception Gateway
       ↓
Security
       ↓
Permission
       ↓
Data Minimization
       ↓
Processing
       ↓
Context
```

Nenhum sensor deve possuir acesso direto ao Core.

---

# 67. Relação com Security Controller

O Security Controller permanece independente.

```text
Perception
↓
Security Controller
↓
Policy
↓
Core
```

O sistema multimodal não pode conceder suas próprias permissões.

---

# 68. Relação com Capability System

Percepção é uma capacidade.

Exemplos:

```text
vision.analyze
audio.transcribe
screen.observe
document.read
video.analyze
voice.synthesize
```

Cada uma possui:

* manifest;
* versão;
* permissões;
* risco;
* dependências;
* limites;
* política.

---

# 69. Relação com Model Router

O Model Router decide quais modelos podem realizar o processamento.

```text
Perception
↓
Model Router
↓
Specialized Model
↓
Result
```

O modelo utilizado pode mudar sem alterar a interface da capacidade.

---

# 70. Relação com Agent System

Agentes podem solicitar capacidades multimodais.

Exemplo:

```text
Agent
↓
vision.analyze
↓
Result
↓
Agent
```

O agente não recebe automaticamente acesso irrestrito à câmera.

A autorização permanece externa ao agente.

---

# 71. Relação com Events & Background

Eventos multimodais podem alimentar o sistema de eventos.

```text
Camera
↓
Event Detector
↓
Event Router
↓
Priority
↓
Yuki
```

Isso será detalhado no documento 13.

---

# 72. Relação com Memory

A percepção produz dados.

A Memory System decide se algo merece ser persistido.

```text
Perception
↓
Context
↓
Task
↓
Memory Evaluation
↓
Persist / Discard
```

Não deve existir armazenamento indiscriminado de tudo que a Yuki percebe.

---

# 73. Relação com Evolution

A Yuki poderá avaliar sua própria camada multimodal.

Exemplos:

* novo modelo de voz;
* novo modelo de visão;
* novo codec;
* nova arquitetura de processamento;
* novo hardware;
* nova técnica de compressão.

Fluxo:

```text
Discovery
↓
Research
↓
Prototype
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
↓
Rollback if necessary
```

---

# 74. Desenvolvimento Futuro

A Yuki poderá futuramente melhorar suas próprias capacidades multimodais dentro do Development Lab.

Exemplo:

```text
Existing Vision Capability
        ↓
Limitation detected
        ↓
Research
        ↓
Prototype
        ↓
Benchmark
        ↓
Security Test
        ↓
Approval
        ↓
Production
```

Nenhuma evolução deve conceder automaticamente novos privilégios.

---

# 75. Estados da Camada Multimodal

A infraestrutura multimodal pode possuir estados:

```text
OFFLINE
SUSPENDED
STANDBY
READY
ACTIVE
PROCESSING
DEGRADED
QUARANTINED
```

Exemplo:

```text
Camera service
→ QUARANTINED

Voice service
→ ACTIVE
```

Uma falha em uma modalidade não deve necessariamente derrubar as demais.

---

# 76. Failure Isolation

Falhas devem ser isoladas.

```text
Vision Failure
      X
Voice
Text
Memory
Core
```

O objetivo é impedir que uma falha local se transforme em falha sistêmica.

---

# 77. Quarentena

Componentes multimodais suspeitos podem ser colocados em quarentena.

Exemplo:

```text
Suspicious Vision Model
↓
Quarantine
↓
No Production Access
↓
Security Evaluation
```

---

# 78. Graceful Degradation

A Yuki deve continuar útil mesmo com perda parcial.

Exemplo:

```text
Vision OFF
↓
Text + Voice

Cloud OFF
↓
Local Models

TTS OFF
↓
Text

Camera OFF
↓
No Visual Context
```

---

# 79. Segurança contra Comprometimento

A arquitetura deve assumir que componentes podem ser comprometidos.

Portanto:

> **Comprometer uma câmera, modelo, plugin ou serviço multimodal não deve significar comprometer a Yuki inteira.**

Devem existir:

* isolamento;
* limites;
* autenticação;
* autorização;
* auditoria;
* monitoramento;
* revogação;
* quarentena.

---

# 80. Princípio de Autoridade

A multimodalidade deve seguir:

```text
Perception ≠ Authorization

Recognition ≠ Permission

Understanding ≠ Execution

Model Output ≠ Command

Voice ≠ Unlimited Authority
```

---

# 81. Experiência Ideal

A experiência futura desejada é:

```text
Usuário:
"Bom dia Yuki."

Yuki:
"Bom dia."

Usuário:
"Vamos continuar aquele trabalho."

Yuki:
"Claro. Quer continuar de onde paramos?"

Usuário:
"Sim."

Yuki:
"Vou abrir no notebook."

Notebook:
→ Interface aparece.

Usuário:
"Olha essa parte."

Camera:
→ imagem.

Yuki:
"Estou vendo. Essa parte apresenta..."
```

Tudo isso representa uma única Yuki.

---

# 82. Arquitetura Consolidada

```text
                         USER
                           │
                VOICE / TEXT / IMAGE
                           │
                    YUKI ACCESS
                           │
                 ┌─────────┴─────────┐
                 │                   │
          DEVICE CONTEXT       PERCEPTION
                 │                   │
                 │            ┌──────┴──────┐
                 │            │             │
                 │          AUDIO         VISION
                 │            │             │
                 │         SCREEN        DOCUMENT
                 │            │             │
                 │          VIDEO        SENSORS
                 │            └──────┬──────┘
                 │                   │
                 └──────────┬────────┘
                            ▼
                    PERCEPTION GATEWAY
                            │
                    SECURITY CONTROLLER
                            │
                     DATA MINIMIZATION
                            │
                     CONTEXT BUILDER
                            │
                     MODEL ROUTER
                            │
                        YUKI CORE
                            │
                  ┌─────────┴─────────┐
                  │                   │
               REASONING           MEMORY
                  │                   │
                  └─────────┬─────────┘
                            │
                       EXECUTION
                            │
                         RESULT
                            │
                     VOICE / UI / TEXT
```

---

# 83. Princípios Oficiais

A camada Voice & Multimodal deve obedecer aos seguintes princípios:

1. **Yuki é voice-first, mas não voice-only.**
2. **Dispositivos são pontos de acesso, não a Yuki.**
3. **Sessões devem possuir continuidade entre dispositivos.**
4. **Percepção deve ser separada de execução.**
5. **Percepção não concede autoridade.**
6. **Biometria é sinal de identidade, não autorização ilimitada.**
7. **Always available não significa always transmitting.**
8. **Percepção contínua deve ser orientada a eventos sempre que possível.**
9. **Dados multimodais devem seguir minimização de dados.**
10. **Nem toda percepção deve virar memória.**
11. **Modelos multimodais não são autoridades de segurança.**
12. **Model output não equivale a comando.**
13. **Falhas multimodais devem ser isoladas.**
14. **A arquitetura deve suportar processamento local, doméstico, cloud e híbrido.**
15. **Nenhum fornecedor deve ser obrigatório.**
16. **Nenhum modelo específico deve ser obrigatório.**
17. **Nenhum hardware específico deve ser obrigatório.**
18. **A Yuki deve adaptar a interface ao dispositivo.**
19. **A Yuki deve suportar novas modalidades futuras.**
20. **Privacidade e segurança são requisitos estruturais.**
21. **A perda de uma modalidade não deve necessariamente interromper toda a Yuki.**
22. **A experiência multimodal deve ser contextual e natural.**

---

# 84. Relação com Outros Documentos

Este documento depende principalmente de:

```text
03 — CORE
04 — MEMORY
05 — PERSONAL CONTEXT
07 — AGENTS & TASKS
08 — CAPABILITY SYSTEM
09 — MODEL ROUTER
10 — SECURITY
11 — EVOLUTION
```

E alimentará:

```text
13 — EVENTS & BACKGROUND
14 — INFRASTRUCTURE
15 — INTEGRATIONS
16 — MASTER CAPABILITY CATALOG
```

---

# 85. Regra de Manutenção

Este documento deve ser atualizado sempre que houver decisão arquitetural relevante envolvendo:

* voz;
* áudio;
* visão;
* câmeras;
* sensores;
* multimodalidade;
* dispositivos;
* interfaces;
* percepção contínua;
* interação entre dispositivos;
* processamento multimodal;
* privacidade audiovisual;
* futuras modalidades de interação.

Decisões importantes devem ser registradas também como ADR quando alterarem princípios arquiteturais.

---

# 86. Status

**Documento 12 — Voice & Multimodal**

Status:

> **DIREÇÃO ARQUITETURAL OFICIAL — v0.1**

A implementação tecnológica específica deverá ser definida posteriormente através de:

* pesquisa;
* benchmarks;
* protótipos;
* testes;
* avaliações de segurança;
* custo;
* disponibilidade;
* evolução tecnológica.

A arquitetura permanece deliberadamente independente da implementação.

---

# 87. Declaração Final

A Yuki não deve apenas "ouvir comandos".

Ela deve ser capaz de **perceber, compreender, conversar e interagir com o mundo através de múltiplas modalidades**, mantendo continuidade, contexto, privacidade, segurança e controle.

A visão de longo prazo é:

```text
FALAR
   +
OUVIR
   +
VER
   +
LER
   +
COMPREENDER
   +
MOSTRAR
   +
INTERAGIR
   +
ADAPTAR
   +
APRENDER
```

sem transformar nenhuma dessas capacidades em autoridade ilimitada.

> **A Yuki deve estar disponível para o usuário, não o usuário disponível para a Yuki.**
