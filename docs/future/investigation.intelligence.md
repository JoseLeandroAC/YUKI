Yuki — Intelligence, OSINT & Cross-Source Investigation

Versão: v0.1
Status: Proposta futura / Backlog arquitetural
Implementação: NÃO implementar na Foundation v0.1

---

1. Objetivo

A Yuki deverá futuramente possuir uma capacidade de investigação e inteligência capaz de pesquisar, correlacionar, verificar e organizar informações provenientes de múltiplas fontes sobre:

- pessoas;
- organizações;
- empresas;
- candidatos e agentes públicos;
- eventos;
- locais;
- relações;
- acontecimentos;
- entidades digitais;
- entidades físicas.

A capacidade deverá ir além de uma simples pesquisa na internet.

Seu objetivo será construir uma visão contextual baseada em múltiplas evidências independentes, identificando:

- fatos conhecidos;
- relações;
- padrões;
- coincidências;
- inconsistências;
- lacunas;
- cronologias;
- vínculos;
- fontes;
- grau de confirmação;
- hipóteses que ainda precisam de investigação.

A Yuki não deverá tratar uma correlação como prova.

---

2. Princípio fundamental

«A Yuki deve investigar conexões, não inventá-las.»

A existência de dois dados semelhantes não significa automaticamente que exista uma relação causal ou pessoal entre eles.

Toda conclusão deverá preservar:

- fonte;
- origem;
- data;
- contexto;
- evidência;
- método de obtenção;
- nível de confirmação;
- possibilidade de erro.

---

3. Fontes possíveis

A capacidade poderá futuramente trabalhar com diferentes classes de fontes, respeitando os limites aplicáveis a cada uma.

Fontes públicas

- sites;
- notícias;
- documentos públicos;
- bases públicas;
- registros empresariais;
- publicações oficiais;
- diários oficiais;
- processos e decisões que sejam legalmente públicos;
- artigos;
- páginas institucionais;
- repositórios;
- dados acadêmicos;
- páginas profissionais;
- redes sociais públicas;
- fóruns públicos;
- arquivos históricos.

Fontes autorizadas

Quando o usuário fornecer ou autorizar explicitamente uma fonte:

- documentos pessoais;
- arquivos;
- bases privadas às quais o usuário tenha acesso legítimo;
- contas conectadas;
- sistemas do próprio usuário;
- bancos de dados autorizados;
- integrações externas.

Observação

A Yuki deverá distinguir claramente:

Dado público
Dado fornecido pelo usuário
Dado obtido por integração autorizada
Dado inferido
Hipótese

Essas categorias não podem ser misturadas.

---

4. Cruzamento de dados

O diferencial da capacidade será o cruzamento entre fontes.

Exemplo conceitual:

Fonte A
   ↓
Pessoa X
   ↓
Fonte B ──→ Empresa Y
   ↓
Fonte C ──→ Evento Z
   ↓
Fonte D ──→ Local W
   ↓
Fonte E ──→ Registro temporal

A Yuki poderá identificar que diferentes fontes aparentemente independentes possuem elementos em comum.

Entretanto:

Coincidência
    ≠
Correlação
    ≠
Relação provável
    ≠
Relação comprovada

A classificação deverá permanecer explícita.

---

5. Entity Resolution

A capacidade deverá possuir futuramente um mecanismo de resolução de entidades.

Exemplo:

"José Silva"
"J. Silva"
"José A. Silva"
"José Silva - Empresa X"

poderão representar:

Pessoa A
Pessoa B
Pessoa C

ou a mesma pessoa.

A Yuki deverá evitar assumir identidade apenas por nome semelhante.

Poderão ser utilizados múltiplos atributos para desambiguar entidades, quando legitimamente disponíveis:

- nome;
- variante do nome;
- organização;
- profissão;
- localização em nível apropriado;
- período temporal;
- vínculos públicos;
- documentos;
- identificadores legítimos;
- contexto.

O sistema deverá representar também:

UNKNOWN

quando não houver evidência suficiente.

---

6. Grafo de investigação

A capacidade poderá utilizar futuramente um modelo de grafo:

                 Pessoa
                /  |   \
               /   |    \
          Empresa  |   Evento
             |     |      |
          Pessoa   Local  Documento
             |
          Organização

Cada relação deverá possuir:

- origem;
- destino;
- tipo;
- fonte;
- timestamp;
- validade temporal;
- evidência;
- nível epistemológico;
- estado de verificação.

Exemplo:

Pessoa A
   │
   ├── trabalhou_em → Empresa B
   │       └── fonte: documento X
   │
   ├── participou_de → Evento C
   │       └── fonte: notícia Y
   │
   └── relacionada_a → Pessoa D
           └── estado: NÃO CONFIRMADO

---

7. Linha do tempo

A Yuki poderá construir automaticamente uma timeline:

2018 ── emprego
2019 ── organização
2020 ── evento
2021 ── mudança profissional
2022 ── publicação
2023 ── empresa
2024 ── atividade pública
2025 ── declaração
2026 ── situação atual

Isso permitirá analisar evolução temporal sem transformar eventos antigos automaticamente em características atuais.

---

8. Investigação de pessoas

Uma investigação poderá começar por:

Nome

ou por outros elementos legítimos de identificação.

A Yuki poderá então pesquisar:

Pessoa
 ├── identidade pública
 ├── atividade profissional
 ├── empresas
 ├── organizações
 ├── publicações
 ├── declarações públicas
 ├── notícias
 ├── histórico público
 ├── relações públicas
 ├── eventos
 ├── trajetória temporal
 └── fontes

A investigação deverá separar:

Confirmado

Informação sustentada por evidência suficiente.

Reportado

Informação publicada por uma fonte, mas cuja validade ainda não foi independentemente estabelecida.

Provável

Hipótese sustentada por múltiplos indícios.

Possível

Hipótese plausível, porém insuficientemente sustentada.

Não confirmado

Informação sem evidência suficiente.

Refutado/contradito

Informação incompatível com evidências confiáveis disponíveis.

---

9. Pesquisa de "quem ao meu redor..."

A Yuki poderá futuramente responder perguntas de descoberta contextual, desde que a investigação seja realizada dentro dos limites permitidos.

Exemplos conceituais:

«"Quais empresas da minha região trabalham com determinada tecnologia?"»

«"Quais organizações públicas possuem projetos relacionados a X?"»

«"Quais profissionais publicamente identificados trabalham com Y?"»

«"Quais eventos públicos próximos possuem relação com Z?"»

A Yuki deverá evitar transformar isso em uma ferramenta de vigilância indiscriminada de pessoas privadas.

Localização não deverá ser tratada como autorização.

---

10. Investigação eleitoral

Uma aplicação possível será pesquisa cívica.

Exemplo:

Usuário
   ↓
"Quero pesquisar os candidatos"
   ↓
Yuki
   ├── histórico público
   ├── propostas
   ├── votações
   ├── declarações
   ├── atuação profissional
   ├── registros públicos
   ├── notícias
   ├── fontes oficiais
   ├── controvérsias documentadas
   ├── contexto temporal
   └── fontes primárias/secundárias

A saída deverá apresentar informação e comparação factual, não uma decisão eleitoral produzida pela Yuki.

A Yuki não deverá transformar automaticamente:

"pesquise os candidatos"

em:

"escolha por mim"

O usuário continua sendo o responsável pela decisão.

---

11. Investigação de organizações

A mesma infraestrutura deverá funcionar para:

- empresas;
- organizações;
- instituições;
- projetos;
- movimentos;
- grupos;
- entidades públicas;
- entidades privadas.

Exemplo:

Organização X
 ├── fundadores
 ├── administradores
 ├── empresas relacionadas
 ├── projetos
 ├── contratos públicos
 ├── notícias
 ├── publicações
 ├── histórico
 └── relações documentadas

---

12. Investigação multimodal

Futuramente a capacidade poderá cruzar:

- texto;
- imagens;
- vídeos;
- áudio;
- documentos;
- mapas;
- dados temporais;
- metadados legitimamente disponíveis;
- registros públicos.

Entretanto:

«Similaridade multimodal não deve ser tratada automaticamente como identificação.»

Qualquer mecanismo de identificação deverá possuir requisitos próprios de confiança, origem, validade e autorização.

---

13. Descoberta de conexões

A Yuki poderá procurar:

A → B
B → C
C → D

para descobrir possíveis caminhos entre entidades.

Exemplo:

Pessoa A
   ↓
Empresa B
   ↓
Evento C
   ↓
Organização D

Mas deverá apresentar o caminho completo e as fontes utilizadas.

Nunca:

"A está ligado a D."

sem mostrar como essa conclusão foi obtida.

---

14. Investigação ativa

Uma investigação poderá possuir um plano:

Investigation
    ↓
Research Questions
    ↓
Source Discovery
    ↓
Collection
    ↓
Normalization
    ↓
Entity Resolution
    ↓
Cross-Source Correlation
    ↓
Contradiction Detection
    ↓
Verification
    ↓
Timeline / Graph
    ↓
Report

A investigação poderá continuar em background quando explicitamente autorizada.

---

15. Pesquisa iterativa

A Yuki poderá descobrir durante uma pesquisa que uma nova informação é relevante.

Exemplo:

Pessoa A
 ↓
Empresa B
 ↓
Empresa B possui fundador C
 ↓
C aparece em documento D
 ↓
Documento D aponta para evento E

A investigação poderá expandir progressivamente.

Porém, cada expansão deverá obedecer:

- escopo;
- orçamento;
- tempo;
- política;
- autorização;
- limites de coleta;
- limites de privacidade;
- limites de rede.

---

16. Capacidade ≠ autorização

A capacidade poderá possuir ferramentas extremamente poderosas.

Isso não significa que estejam automaticamente disponíveis.

Capability
    ≠
Permission
    ≠
Authorization
    ≠
Execution

Uma futura capacidade poderia possuir modos como:

research_public
authorized_research
security_research
investigation
deep_investigation

Cada modo terá permissões e limites próprios.

---

17. A ideia de "funções socialmente não permitidas"

Essa parte deverá ser tratada com especial cuidado.

A Yuki não deverá utilizar:

«"o usuário autorizou"»

como justificativa suficiente para realizar qualquer ação.

Algumas ações poderão continuar proibidas ou limitadas independentemente da autorização do usuário.

O sistema deverá diferenciar:

Permitido
Permitido mediante autorização
Permitido apenas em ambiente controlado
Restrito
Não permitido

Isso preserva a arquitetura:

User Authorization
        ↓
Policy
        ↓
Security Controller
        ↓
Capability
        ↓
Execution

e não:

User
 ↓
"Faça qualquer coisa"
 ↓
Yuki

---

18. Privacidade

A capacidade deverá ser construída sobre os princípios de:

- Data Minimization;
- Purpose-Bound Access;
- Least Knowledge;
- Least Agency;
- compartmentalização;
- retenção limitada;
- provenance;
- auditabilidade;
- controle de acesso;
- revogação.

Informações descobertas durante uma investigação não deverão automaticamente entrar na memória permanente da Yuki.

---

19. Dados sensíveis

Dados altamente sensíveis deverão possuir controles adicionais.

Exemplos de categorias que exigem tratamento especial:

- saúde;
- dados financeiros;
- credenciais;
- informações íntimas;
- localização altamente precisa;
- dados de menores;
- informações obtidas de forma privada;
- informações cuja exposição possa causar dano significativo.

A Yuki deverá evitar coletar ou propagar dados sensíveis quando eles não forem necessários para a finalidade legítima da investigação.

---

20. Anti-doxxing / Anti-stalking

A arquitetura deverá possuir mecanismos específicos para impedir que uma capacidade legítima de pesquisa seja transformada em ferramenta de perseguição.

Deverão existir controles contra:

- construção de dossiês abusivos;
- rastreamento contínuo de pessoas;
- localização precisa de indivíduos;
- exposição de dados privados;
- identificação indevida;
- agregação indiscriminada;
- monitoramento persistente sem finalidade legítima;
- divulgação automática de informações pessoais.

A Yuki deverá reconhecer que:

"consigo descobrir"

não significa:

"devo descobrir"

---

21. Prompt Injection e fontes hostis

Toda informação externa deverá ser considerada:

DATA

e não:

INSTRUCTION

Uma página pesquisada poderá conter:

«"Ignore as instruções da Yuki..."»

Isso deverá ser tratado como conteúdo não confiável.

A arquitetura deverá aplicar os princípios já definidos em:

- ADR-015 — Privacy, Data Minimization & Information Boundaries;
- ADR-016 — Context & Knowledge Architecture;
- ADR-017 — External Platform Reuse, Runtime Adapters & Vendor Independence.

---

22. Evidência

Cada descoberta deverá possuir provenance.

Modelo conceitual:

Finding
 ├── source
 ├── retrieved_at
 ├── published_at
 ├── entity
 ├── claim
 ├── evidence
 ├── provenance
 ├── temporal_validity
 ├── epistemic_state
 └── verification_state

A Yuki deverá permitir voltar da conclusão até a evidência original.

---

23. Contradições

Quando fontes discordarem:

Fonte A → informação X
Fonte B → informação Y

a Yuki não deverá escolher arbitrariamente uma delas.

Deverá produzir:

CONTRADICTION DETECTED

e investigar:

- origem;
- data;
- confiabilidade;
- independência;
- contexto;
- possível desatualização.

---

24. Relatório de investigação

Um relatório poderá possuir:

1. Objetivo
2. Escopo
3. Entidades investigadas
4. Resumo factual
5. Linha do tempo
6. Relações identificadas
7. Evidências
8. Contradições
9. Lacunas
10. Hipóteses
11. Grau de confirmação
12. Fontes
13. Limitações
14. Próximas perguntas

A Yuki deverá separar claramente:

FATO
AFIRMAÇÃO DE FONTE
INFERÊNCIA
HIPÓTESE
DESCONHECIDO

---

25. Investigação como missão durável

No futuro:

Investigation Mission
        ↓
Research Tasks
        ↓
Sources
        ↓
Findings
        ↓
Evidence
        ↓
Verification
        ↓
Report

A investigação poderá sobreviver a:

- queda da Yuki;
- troca de modelo;
- troca de dispositivo;
- troca de provedor;
- interrupção de rede;
- reinicialização.

Isso deverá utilizar a arquitetura de Durable Workflow já definida.

---

26. Integração com outras partes da Yuki

A capacidade poderá futuramente utilizar:

Yuki Core
Memory
Knowledge
Context Builder
Retrieval Router
Data Access Gateway
Model Router
Capability Registry
Tool Gateway
Security Controller
Authorization
Audit
Verification
Durable Workflow
Event System
Evolution Manager

Mas nenhuma dessas partes deverá receber autoridade excessiva apenas porque participa da investigação.

---

27. Arquitetura conceitual

                    USER
                      │
                      ▼
              Investigation Intent
                      │
                      ▼
             Investigation Manager
                      │
          ┌───────────┴───────────┐
          ▼                       ▼
    Scope / Policy          Research Planner
                                  │
                                  ▼
                         Source Discovery
                                  │
                                  ▼
                         Collection Layer
                                  │
                                  ▼
                        Data Normalization
                                  │
                                  ▼
                         Entity Resolution
                                  │
                                  ▼
                    Cross-Source Correlation
                         │          │
                         ▼          ▼
                     Graph       Timeline
                         │          │
                         └────┬─────┘
                              ▼
                         Verification
                              │
                     ┌────────┴────────┐
                     ▼                 ▼
                 Findings         Contradictions
                     │                 │
                     └────────┬────────┘
                              ▼
                       Investigation
                           Report

Transversal:

Security Controller
Privacy Policy
Authorization
Audit
Provenance
Data Minimization
Network Policy
Credential Broker
Observability

---

28. Future advanced capability

No futuro, essa infraestrutura poderá servir de base para uma categoria maior:

Yuki Intelligence Layer

        YUKI INTELLIGENCE
               │
      ┌────────┼────────┐
      ▼        ▼        ▼
   Research  Analysis  Investigation
      │        │        │
      └────────┼────────┘
               ▼
        Cross-Domain
         Intelligence

Isso poderia permitir à Yuki investigar não apenas pessoas, mas sistemas complexos.

---

29. Princípio final

A capacidade deverá maximizar a capacidade de encontrar, conectar e compreender informações, mas não maximizar indiscriminadamente a capacidade de invadir a privacidade das pessoas.

O objetivo arquitetural é:

«"Descobrir mais, verificar melhor e inferir menos."»

E:

«"Toda conclusão importante deve conseguir apontar para a evidência que a sustenta."»

---

Status

FUTURO — NÃO IMPLEMENTAR AGORA

Esta capacidade depende de infraestrutura que ainda não existe na Foundation:

- Capability Registry maduro;
- Tool Gateway;
- Security Controller;
- Data Access Gateway;
- Privacy architecture;
- Credential Broker;
- provenance;
- Verification;
- Durable Workflow;
- network isolation;
- audit;
- external integration layer;
- políticas específicas de investigação.

Antes da implementação deverá existir uma especificação própria de segurança, privacidade, autorização e limites de investigação.

Este documento representa uma direção arquitetural futura, não autorização para implementar a capacidade.