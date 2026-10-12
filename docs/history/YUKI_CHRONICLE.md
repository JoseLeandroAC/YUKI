# CRÔNICAS DA YUKI
## Registro Histórico, Narrativo e Factual da Criação e Evolução da Yuki

---

### Protocolo Histórico e Princípios

Este documento é o registro perene da criação e maturação da Yuki. Ele não é um simples changelog técnico ou sumário de commits; seu propósito é documentar a jornada real de engenharia: as hipóteses, os avanços, as hesitações, as decisões de design, os erros cometidos, as correções arquiteturais e as lições aprendidas.

#### Diretrizes Fundamentais do Registro:
1. **Preservar a História; Corrigir Fatos com Transparência**:
   - A Crônica não adota uma imutabilidade cega que congele erros factuais. Se uma entrada contiver imprecisões ou omissões, os fatos devem ser corrigidos de forma aberta e fundamentada, registrando a correção e preservando a proveniência e as evidências. O objetivo é a integridade histórica, jamais o acobertamento ou a falsificação retrospectiva.
2. **Distinção Fundamental de Tempo**:
   - `Event Time` (Momento do Evento): Quando o fato histórico efetivamente ocorreu no mundo real ou no repositório.
   - `Record Creation Time` (Momento do Registro): Quando a entrada foi redigida e formalizada neste documento.
   - A ordem em que as entradas são escritas não presume nem dita a ordem em que os acontecimentos ocorreram. A Crônica permite e incentiva a reconstrução retrospectiva contínua a partir de evidências sólidas.
3. **Reconstrução Retrospectiva Baseada em Evidências**:
   - Nenhuma data ou alegação deve ser inventada. A gênese e as fases iniciais da Yuki serão reconstruídas rigorosamente em conjunto com o Owner e o Architecture Review a partir das fontes primárias: conversas originais do projeto, histórico de commits do Git, ADRs, tags, relatórios de auditoria, arquivos do sistema operacional e registros formais do Owner.
4. **Separação de Evidências**:
   - *Fatos tecnicamente verificados*: Registros ancorados em hashes de Git, branches, tags imutáveis e verificações de compilador/testes.
   - *Horários fornecidos pelo Owner*: Timestamps humanos indicados explicitamente como aproximados.
   - *Interpretações narrativas e contextuais*: Intenções, hipóteses e reflexões documentadas com rigor e clareza.

---

### Quadro Estrutural de Fases Históricas

Para fins de organização e futura reconstrução retrospectiva, a trajetória da Yuki estrutura-se nas seguintes fases conceituais:

1. **Origem**:
   O nascimento da ideia da Yuki e a definição inicial de quem e do que ela deveria se tornar.
2. **Visão**:
   A definição do propósito soberano — *“Yuki for user, not user for Yuki”* —, a inspiração em assistentes pessoais autônomos de referência (como JARVIS/FRIDAY), as capacidades desejadas (memória persistente, voz, ferramentas, agentes, segurança, automação residencial e de dispositivos, pesquisa, desenvolvimento controlado) e os limites éticos do sistema.
3. **Arquitetura**:
   A transformação da visão em arquitetura técnica modular e princípios constitucionais: Capability Registry, Security Controller, Execution Engine, Verification Engine, Audit durável, Evolution Manager, documentação formal e ADRs.
4. **Foundation**:
   A consolidação do primeiro baseline arquitetural e executável congelado (`v0.1.0-foundation`, commit `654c1812f40d6c903e67fd6f86f4b83e06e14b8c`).
5. **MVP-1**:
   A implementação controlada dos Marcos 1 a 4 (Core Async, Gateway de Modelos, Execução Real com Verificação Dinâmica e Persistência Durável SQLite com Isolamento OCI), reconciliações de branch, auditorias de segurança e o congelamento do baseline `v0.2.0-mvp1` (commit `6a9d07010719b18bb10a60a082606be54b813f66`).
6. **04/10/2026 — Yuki Fala Pela Primeira Vez**:
   A primeira interação manual conhecida com o runtime real compilado do MVP-1 executando no notebook do Owner.
7. **Pós-MVP-1**:
   A primeira lacuna de integração descoberta em uso real, a investigação arquitetural do bootstrap, o isolamento seguro de credenciais e a primeira correção pós-release (`feature/post-mvp1-model-runtime-wiring`).

---

### Entradas Históricas

---

#### [Fases 1 a 5 — Em Reconstrução Retrospectiva]
*A reconstrução detalhada das fases de Origem, Visão, Arquitetura, Foundation e MVP-1 será realizada em momento oportuno junto ao Owner e Architecture Review, recorrendo aos registros primários e transcripts da conversa fundacional.*

---

#### [Event Time: 2026-10-04 ~12:09 | Record Creation Time: 2026-10-04]
### 04/10/2026 — Yuki Fala Pela Primeira Vez (Primeira Execução Manual do Runtime Real)

- **Event Time**: 04 de outubro de 2026, aproximadamente 12:09 (America/Sao_Paulo) — *Horário aproximado fornecido pelo Owner*
- **Milestone Relacionado**: Pós-congelamento do MVP-1 (`v0.2.0-mvp1`, commit `6a9d07010719b18bb10a60a082606be54b813f66`)
- **Contexto Operacional**: Teste manual real executado no notebook de desenvolvimento do Owner.
- **Natureza do Evento**: Este marco NÃO representa o nascimento conceitual da Yuki nem o início absoluto de sua história (já extensamente consolidada nas fases anteriores). Representa, com exatidão factual, **a primeira mensagem conhecida da Yuki executando como runtime real durante teste manual do MVP-1**.

##### O que se pretendia
Após a conclusão dos Marcos 1 a 4 e a emissão do baseline imutável `v0.2.0-mvp1`, o Owner realizou o primeiro teste prático fora dos scripts automatizados de CI: compilar o binário em release, inicializar a persistência durável SQLite localmente, validar a execução das capabilities fundamentais (`system.echo`, `system.time`) e conectar a chave da API do Google Gemini para uma interação real com LLM.

##### O que aconteceu
1. O runtime iniciou com sucesso. O subsistema SQLite abriu o banco e reportou integridade durável (`Persistent Audit (SQLite): OK`).
2. A capability `system.echo` executou com sucesso sob autorização do Security Controller.
3. A capability `system.time` executou com sucesso, reportando o horário local e confirmando a estratégia dinâmica de verificação.
4. O Owner configurou temporariamente a variável `YUKI_GEMINI_API_KEY`.
5. O comando `yuki health` foi executado e reportou:
   ```text
   External Model Provider: CONFIGURED (Gemini available)
   ```
6. O Owner submeteu um prompt conversacional esperando a resposta do modelo Google Gemini. A resposta, contudo, exibiu o texto determinístico fixo do `MockModelProvider`.

##### O Erro e a Investigação Arquitetural
A investigação do código-fonte revelou uma desconexão no bootstrap da aplicação:
- No Marco 2, o `GeminiProviderAdapter`, o `ModelRouter`, as abstrações seguras de credenciais (`SecretRef`, `SecretMaterial`, `EnvSecretStore`) e os parsers haviam sido completamente implementados e validados em testes unitários e de segurança.
- No entanto, no ponto de entrada real da CLI (`src/main.rs`), a instância de `YukiCore` continuava sendo gerada exclusivamente via `YukiCore::new()`, mantendo hardcoded o `MockModelProvider`.
- O método de injeção `core.with_model_provider(...)` existia no domínio do Core, mas a CLI nunca o invocava com base nas variáveis de ambiente.
- A linha em `yuki health` apenas checava a presença de `std::env::var("YUKI_GEMINI_API_KEY").is_ok()`, emitindo uma mensagem otimista e tecnicamente enganosa: a credencial estava presente no ambiente, mas o provedor sequer havia sido instanciado pelo runtime.
- A suíte automatizada possuía 120 testes verdes porque os testes unitários utilizavam o Mock intencionalmente (para garantir determinismo, reprodutibilidade e independência de rede), enquanto o teste live ignorado (`model_gateway_live.rs`) chamava o adapter diretamente, sem passar pela CLI ou pelo `YukiCore`. Não havia teste de integração cobrindo o bootstrap de produção.

##### A Decisão e a Solução
O Owner e o Architecture Review determinaram que a falha não seria contornada:
1. O baseline `v0.2.0-mvp1` foi mantido 100% congelado e inalterado.
2. Criou-se a branch pós-release `feature/post-mvp1-model-runtime-wiring`.
3. Implementou-se a resolução estrita e desacoplada do provedor (`resolve_model_provider_from_env`), garantindo que o bootstrap nunca manipule `SecretMaterial`, transferindo apenas o `SecretRef` e o broker.
4. Removeu-se a mensagem falsa de `health`, tornando o diagnóstico 100% offline, honesto e baseado nos metadados e estado local do provedor ativo.
5. Criou-se a suíte `tests/model_provider_wiring.rs` para garantir que o bootstrap do Core e a injeção do provedor estejam permanentemente cobertos por testes de integração.

##### Lição de Engenharia
> *«Um componente implementado com perfeição em sua unidade e coberto por testes isolados não tem valor operacional se não estiver conectado ao runtime real. O teste de integração do bootstrap de entrada é um portão de segurança indispensável.»*

---

#### [Event Time: 2026-10-04 | Record Creation Time: 2026-10-04]
### Incidente Operacional de Manipulação de Credencial e Nova Política de Autenticação Segura

- **Event Time**: 04 de outubro de 2026
- **Contexto Operacional**: Operação de push da branch de correção pós-MVP-1 para o repositório remoto.
- **Natureza do Evento**: Incidente de segurança operacional em ambiente de desenvolvimento (sanitizado).

##### O Incidente (Descrição Sanitizada)
Durante a revisão arquitetural independente dos procedimentos de push da branch `feature/post-mvp1-model-runtime-wiring`, identificou-se que um token de acesso ao GitHub havia sido manipulado em texto claro dentro de scripts temporários e comandos interpolados (`https://<TOKEN>@github.com/...`) no transcript operacional da sessão.

Embora o token nunca tenha sido comitado em arquivos do repositório, gravado em `.env`, adicionado a `git config` ou mantido no histórico de commits, a interpolação direta de material secreto em linhas de comando, URLs de Git ou scripts intermediários representa um desvio das melhores práticas de segurança e gerou risco de exposição acidental no transcript.

A não-conformidade foi prontamente identificada pelo Architecture Review, que interrompeu o fluxo e exigiu a formalização de contramedidas definitivas.

##### Nova Regra Operacional e Política Permanente
A partir deste incidente, fica estabelecido com caráter mandatório:
1. **Vedação Absoluta de Interpolação de Segredos**:
   - É estritamente proibido construir ou executar comandos na forma conceitual `https://<TOKEN>@github...` com material secreto interpolado.
   - Segredos, chaves de API, PATs ou tokens NUNCA devem ser colocados diretamente em:
     - Linhas de comando ou argumentos de processos;
     - Scripts temporários em disco (mesmo em diretórios temporários transitórios);
     - URLs de remotes do Git;
     - Prompts ou mensagens trocadas;
     - Relatórios, crônicas ou documentação;
     - Logs de execução ou variáveis visíveis em transcripts.
2. **Uso de Mecanismos Apropriados de Credenciais**:
   - As autenticações devem ser realizadas exclusivamente via gerenciadores seguros de credenciais do ambiente (ex.: Git Credential Manager nativo do host, agentes SSH ou brokers autenticados fora do escopo do transcript).
3. **Diretriz de Interrupção Segura (*Stop and Inquire*)**:
   - Se um mecanismo de autenticação seguro e não expositivo não estiver disponível no ambiente para a realização de uma operação remota, o operador deve **PARAR imediatamente** e solicitar a intervenção direta do Owner, jamais tentando criar scripts de contorno que manipulem segredos em claro.

##### Lição de Segurança
> *«Secrets must never be embedded directly into command URLs, temporary scripts, or preserved transcripts. A segurança de credenciais não se aplica apenas ao código do produto, mas a toda a cadeia de ferramentas operacionais de engenharia.»*

---

#### [Event Time: 2026-10-04 | Record Creation Time: 2026-10-04]
### Primeira Requisição Live ao Gemini — Provedor Atingido, Rejeição de Contrato de Ferramentas (additionalProperties)

- **Event Time**: 04 de outubro de 2026
- **Milestone Relacionado**: Branch pós-MVP-1 (`feature/post-mvp1-model-runtime-wiring`)
- **Contexto Operacional**: Primeiro teste real executado pelo Owner conectando o runtime compilado à API do Google Gemini com credencial real.
- **Classificação Histórica**: **FIRST VERIFIED LIVE GEMINI REQUEST THROUGH YUKI — PROVIDER REACHED, CONTRACT REJECTED**.
  *Nota Histórica Estrita*: Este evento NÃO constitui o "First Verified Live Gemini Turn", uma vez que o modelo externo rejeitou a requisição no gateway antes de gerar uma resposta conversacional. Representa, porém, uma evidência empírica de extremo valor: a comprovação de que o wiring local funcionou perfeitamente e alcançou o endpoint remoto do Gemini.

##### O que se pretendia
Com a resolução de bootstrap concluída e o comando `yuki health` reportando com fidelidade e sem rede:
```text
Status: OK
Model Subsystem (GoogleGemini): OK
Selected Model Provider: GoogleGemini (model: gemini-2.5-flash, version: v1beta)
Model Provider Health: OK
```
O Owner disparou a primeira requisição conversacional real contra a API do Google Gemini através da CLI da Yuki, com a variável de sessão `YUKI_GEMINI_API_KEY` devidamente configurada.

##### O que aconteceu
1. O runtime não fez fallback para o Mock.
2. A requisição HTTP real foi disparada com sucesso contra o endpoint `https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent`.
3. O endpoint da Google retornou HTTP 400 (Bad Request) com o erro:
   ```text
   Model error: Requisição inválida para o provedor:
   Invalid JSON payload received.
   Unknown name "additionalProperties" at 'tools[0].function_declarations[0].parameters': Cannot find field.

   Invalid JSON payload received.
   Unknown name "additionalProperties" at 'tools[0].function_declarations[1].parameters': Cannot find field.

   Invalid JSON payload received.
   Unknown name "additionalProperties" at 'tools[0].function_declarations[2].parameters': Cannot find field.
   ```
4. A execução encerrou de forma determinística com código 1 (*fail-closed*), reportando o erro tipado ao usuário sem corromper a persistência nem mascarar o resultado.

##### A Investigação Arquitetural e a Causa Raiz
A investigação técnica revelou o descompasso na fronteira do provedor:
- Os manifestos canônicos de entrada das capabilities da Yuki (`system.echo`, `system.time`, `system.info`) utilizam JSON Schema padrão com a restrição de segurança `"additionalProperties": false`. Essa propriedade é utilizada pela validação interna e confiável da Yuki (`validate_capability_input`) para assegurar que propriedades injetadas ou inesperadas sejam rejeitadas antes da autorização e execução de capabilities.
- No entanto, a representação de esquema de parâmetros de ferramentas da API Google Gemini (`tools[].function_declarations[].parameters`) mapeia internamente para a mensagem protobuf `google.ai.generativelanguage.v1beta.Schema`.
- O subconjunto OpenAPI 3.0 suportado pelo parser protobuf do Gemini v1beta não define o campo `additionalProperties`. Ao encontrar esse campo em cada uma das três capacidades projetadas, o deserializador do Google rejeitou a totalidade do payload.

##### A Decisão Arquitetural e a Solução
O princípio constitucional fundamental foi reafirmado:
> *«O manifesto canônico de capacidades da Yuki JAMAIS deve ser enfraquecido ou remodelado apenas para se amoldar a restrições sintáticas de um provedor LLM específico. Restrições de provedor pertencem estritamente à fronteira do provedor (Yuki Schema != Gemini Schema).»*

1. **Preservação Integral dos Manifestos Canônicos**: Os manifestos de `system.echo`, `system.time` e `system.info` mantêm `"additionalProperties": false` intacto.
2. **Preservação da Validação Estrita na Execução**: A validação prévia à autorização (`validate_capability_input`) continua executando estritamente contra o esquema canônico da Yuki. Propostas que contenham propriedades inesperadas continuam sendo barradas (*Model-visible schema != Execution validation schema*).
3. **Projeção de Esquema Específica do Gemini (`project_schema_to_gemini`)**: Implementou-se um tradutor de fronteira no `GeminiProviderAdapter` que normaliza recursivamente os esquemas das capacidades antes da serialização para a API:
   - Omite palavras-chave incompatíveis com o protobuf do Gemini (`additionalProperties`, `$schema`, etc.);
   - Recursa em objetos aninhados, arrays (`items`) e uniões (`anyOf`);
   - Opera em modo *fail-closed* diante de construções incompatíveis (`not`, `patternProperties`);
   - Garante a presença de `properties: {}` para objetos vazios.
4. **Cobertura Automatizada de Regressão**: Criou-se a suíte `tests/gemini_schema_projection.rs` demonstrando que o payload corrigido elimina as 3 ocorrências de `additionalProperties` sem que qualquer manifesto interno seja mutado.

##### Lição de Engenharia
> *«Testes de componentes com mocks e validações locais de seleção de provedor comprovam a integridade interna da arquitetura, mas não atestam a compatibilidade estrita do contrato com a API remota. A fronteira com o mundo real revela verdades que o ambiente isolado não pode simular. Preserve a integridade do seu domínio; adapte apenas a fronteira.»*

---

#### [Event Time: 2026-10-04 | Record Creation Time: 2026-10-04]
### Segundo Teste Live ao Gemini — Eliminação do Erro de Schema Confirmada, Detecção de HTTP 404 e Falha de Observabilidade no Gateway

- **Event Time**: 04 de outubro de 2026
- **Milestone Relacionado**: Branch pós-MVP-1 (`feature/post-mvp1-model-runtime-wiring`)
- **Contexto Operacional**: Segundo teste real executado pelo Owner conectando o runtime compilado (`HEAD b60fdd923000aea9af6f09b99ef6648345d99466`) à API do Google Gemini.
- **Classificação Histórica**: **SECOND LIVE GEMINI TEST — SCHEMA REJECTION ELIMINATED, UPSTREAM HTTP 404 DETECTED & GATEWAY OBSERVABILITY GAP RESOLVED**.
  *Nota Histórica Estrita*: Este evento NÃO constitui o "First Verified Live Gemini Turn", pois nenhuma resposta de modelo foi produzida ainda. Trata-se, contudo, de mais um marco empírico de progresso real na fronteira entre a Yuki e a infraestrutura externa.

##### O que se pretendia
Após a projeção dos esquemas de capacidade para o subconjunto OpenAPI do Gemini (eliminando `additionalProperties`), o Owner recompilou a branch e executou novamente o comando conversacional real da Yuki.

##### O que aconteceu (Fatos Empíricos)
1. **Confirmação Empírica da Eliminação de `additionalProperties`**: O erro anterior de rejeição de schema (`Unknown name "additionalProperties"`) **NÃO** voltou a ocorrer. O tradutor de fronteira cumpriu seu papel com precisão.
2. **Novo Ponto de Parada**: A requisição avançou no pipeline do Google Gemini, mas retornou `HTTP 404`.
3. **Lacuna de Observabilidade Revelada no Gateway**: A mensagem exibida no terminal foi:
   ```text
   Model error: Erro interno no gateway de modelos: Status HTTP inesperado: 404
   ```
   O runtime classificou o 404 como um erro genérico interno (`ModelError::Internal`), descartando completamente o corpo de resposta JSON retornado pelo Google Gemini, impossibilitando diagnosticar de imediato o motivo exato apontado pelo upstream.

##### A Investigação Arquitetural e a Causa Raiz
A investigação técnica revelou duas causas entrelaçadas:
1. **Descarte de Mensagens de Diagnóstico em Status Não Mapeados**:
   No método `execute_single_turn` de `src/models/gemini.rs`, o bloco de tratamento de falhas HTTP avaliava status 400, 401, 403, 429 e 5xx. Para qualquer outro código (incluindo 404), o fluxo caía no branch coringa `_ => Err(ModelError::Internal(format!("Status HTTP inesperado: {}", status_code)))`. A variável `error_msg` contendo o payload detalhado do Google (`{"error": {"code": 404, "message": "...", "status": "NOT_FOUND"}}`) era simplesmente descartada.
2. **Vulnerabilidade a Prefixos Redundantes e Fragilidade de Deserialização**:
   - A construção do endpoint concatenava diretamente `self.config.model_id` em `format!("{}/v1beta/models/{}:generateContent", ...)`. Se o operador fornecesse um identificador com o prefixo `models/` (padrão comum em SDKs e documentações do Google, ex: `models/gemini-2.5-flash`), a URL resultante tornava-se `.../models/models/gemini-2.5-flash:generateContent`, gerando erro 404 de recurso não encontrado.
   - A estrutura interna `GeminiErrorDetail` definia `code: Option<u16>`, que falhava a desserialização JSON caso o Google retornasse o código de erro como string (ex: `"invalid_request"` ou `"not_found"`), provocando fallback para a string crua `"HTTP 404"`.

##### A Decisão Arquitetural e a Solução
1. **Preservação Integral da Observabilidade Sanitizada**:
   - Implementou-se `parse_gemini_error_message`, que extrai `status`, `code` (seja número ou texto) e `message` da resposta do Gemini, preservando diagnósticos reais mesmo para respostas não-JSON (gateways intermediários/proxies) até um teto seguro.
   - Implementou-se `scrub_potential_secrets`, garantindo que nenhum fragmento acidental de token de autenticação possa vazar nas mensagens de erro.
   - O status `404` foi mapeado explicitamente para `ModelError::InvalidRequest(format!("Recurso ou modelo não encontrado no provedor (HTTP 404): {}", error_msg))`.
   - O caso padrão coringa `_` passou a preservar o diagnóstico completo: `ModelError::Internal(format!("Status HTTP inesperado {}: {}", status_code, error_msg))`.
2. **Normalização e Sanitização do Identificador de Modelo e Versão da API**:
   - Criaram-se as funções `sanitize_gemini_model_id` e `sanitize_gemini_api_version`, e `build_gemini_endpoint`. Qualquer prefixo `models/`, aspas ou barras espúrias em `YUKI_MODEL_ID` são normalizados deterministicamente.
   - Adicionou-se o parâmetro configurável `api_version` (padrão `"v1beta"`), permitindo override via `YUKI_API_VERSION`.
3. **Cobertura Automatizada com Servidor Mock Local**:
   - Criou-se a suíte `tests/gemini_endpoint_observability.rs` (10 testes), com servidores TCP locais mockados em loopback provando que respostas 404 reais preservam o diagnóstico textual exato do upstream sem cair em erros genéricos opacos.

##### Lição de Engenharia
> *«Em integrações com serviços externos, a observabilidade não pode ser tratada como detalhe secundário. Descartar o corpo de um erro HTTP transforma um diagnóstico claro do provedor em um enigma opaco para o operador. Nunca silencie o upstream; sanitize os segredos, preserve a mensagem e exponha o erro com tipagem rigorosa.»*

##### Correção Retrospectiva e Esclarecimento Factual da Causa do 404
Após a implementação da observabilidade aprimorada no commit `75f9f683d7d6069f7ce2c845c8bb6073dd023919`, o Owner executou novamente o runtime apontando para o modelo padrão da época (`gemini-2.5-flash`). A nova camada de diagnóstico capturou e exibiu com fidelidade a mensagem real do Google:
```text
HTTP 404: NOT_FOUND: This model models/gemini-2.5-flash is no longer available to new users. Please update your code to use models/gemini-3.8-flash for the latest features and improvements. We recommend you to use the Interactions API.
```

Fatos empíricos confirmados:
1. A observabilidade do gateway funcionou perfeitamente, demonstrando de pronto sua utilidade.
2. A hipótese preliminar de duplicação sintática `models/models/...` foi refutada como causa daquele erro 404 específico (embora a sanitização permaneça no código como defesa arquitetural válida contra entradas malformadas).
3. A causa real e factual do 404 foi a indisponibilidade/depreciação de `gemini-2.5-flash` para novos usuários pela infraestrutura da Google, exigindo a atualização da Yuki para `gemini-3.8-flash`.

---

#### [Event Time: 2026-10-04 | Record Creation Time: 2026-10-04]
### Terceiro Teste Live ao Gemini — Primeira Execução de Capability Dirigida por Modelo em Produção e Descoberta do Loop de Continuação de Ferramentas

- **Event Time**: 04 de outubro de 2026
- **Milestone Relacionado**: Branch pós-MVP-1 (`feature/post-mvp1-model-runtime-wiring`)
- **Contexto Operacional**: Terceiro teste real executado pelo Owner utilizando o modelo `gemini-3.8-flash` via CLI da Yuki.
- **Prompt Submetido**: *"Yuki, por favor se apresente e diga qual e o seu proposito fundamental em uma frase"*
- **Classificação Histórica**: **FIRST VERIFIED LIVE GEMINI-DRIVEN CAPABILITY EXECUTION THROUGH YUKI**.  
  *Distinção Histórica Estrita*: Este evento comprova empiricamente a primeira cadeia completa em que um modelo LLM externo remoto propôs autonomamente uma capability que foi autorizada pelo Security Controller, executada pelo Execution Engine, verificada pelo Verification Engine e auditada pelo Audit Subsystem. NÃO é classificado como "Primeiro Turno Conversacional Completo do Gemini", pois o runtime retornou a saída bruta da ferramenta em vez de sintetizar uma resposta conversacional final em linguagem natural ao operador.

##### O que se pretendia
Com a atualização para o modelo `gemini-3.8-flash` e a observabilidade ativa, o Owner submeteu um prompt natural solicitando a apresentação da Yuki e seu propósito fundamental.

##### O que aconteceu (Fatos Empíricos)
1. **Conexão e Compreensão do Modelo**: O Google Gemini recebeu o prompt e os esquemas sanitizados das ferramentas (`system.echo`, `system.time`, `system.info`).
2. **Proposta de Capability Autônoma**: O modelo deduziu que para responder adequadamente sobre si mesma e seu ambiente precisava consultar o contexto da máquina, emitindo autonomamente uma proposta de chamada para `system.info` com argumentos `{}`.
3. **Cadeia Constitucional de Execução e Verificação**:
   - `Model Output != Command`: A proposta do modelo foi tratada como mera intenção não-confiável.
   - O Security Controller avaliou a proposta sob as políticas ativas e autorizou formalmente a execução de `system.info`.
   - O Execution Engine despachou e executou a capacidade no ambiente local.
   - O Verification Engine atestou a integridade e conformidade dos dados produzidos.
   - O Audit Subsystem persistiu o registro durável do ciclo de execução.
4. **Desfecho Observado**: O runtime retornou ao terminal do operador o JSON bruto verificado:
   ```json
   {"arch":"x86_64","os":"windows","yuki_version":"0.1.0"}
   ```
5. **Lacuna Arquitetural Descoberta**: O runtime encerrou o turno imediatamente após a execução da ferramenta. A Yuki não possuía um loop multi-turn de continuação para devolver o resultado da ferramenta (`Tool Result`) ao modelo e obter a resposta em linguagem natural esperada pelo usuário.

##### A Investigação Arquitetural e a Solução (Governed Bounded Tool Continuation Loop)
A análise arquitetural identificou que o método `YukiCore::process_input_async` operava sob um paradigma unistep (single-turn). Ao executar uma capability proposta pelo modelo, o resultado da execução era considerado a resposta terminal do turno.

Em uma arquitetura de assistente autônomo governado, a execução de ferramenta é um passo intermediário de percepção e ação:
1. **Invariantes Constitucionais Reafirmadas**:
   - `Tool Result = Data` (Dados externos não confiáveis, jamais instrução ou autoridade).
   - O modelo não adquire "controle de fluxo" ao receber o resultado da ferramenta.
   - Se o modelo propor uma nova ferramenta após receber o resultado anterior, essa nova proposta DEVE passar por nova e independente avaliação do `SecurityController` (*Every tool execution requires fresh authorization*).
2. **Limite Rígido e Determinístico de Iterações (*Fail-Closed Boundary*)**:
   - Implementou-se `max_tool_iterations` (padrão `5`, configurável via `YUKI_MAX_TOOL_ITERATIONS` e `config/yuki.toml`).
   - Se o modelo entrar em recursão infinita ou ultrapassar o orçamento de iterações, o loop aborta imediatamente com erro explícito tipado (`ModelError::InvalidRequest`), sem execução silenciosa.
3. **Conformidade com o Protocolo Gemini 3 Multi-Turn**:
   - No protocolo Google Gemini (em especial na família Gemini 3), chamadas de função com `thoughtSignature` exigem a retransmissão obrigatória da assinatura no histórico conversacional; a omissão gera erro HTTP 400.
   - O resultado da ferramenta é entregue no papel `user` com a estrutura `functionResponse`, encapsulando os dados em `{"output": ...}` conforme a especificação protobuf `google.protobuf.Struct`.
4. **Atualização do Modelo Padrão**:
   - O modelo padrão da Yuki foi atualizado em código e documentação para `gemini-3.8-flash`.
5. **Cobertura de Testes**:
   - Implementou-se a suíte `tests/model_tool_continuation.rs` (9 testes) cobrindo todos os cenários: continuação unistep, zero ferramentas (texto direto), multistep sequencial (ex.: `system.info` seguido de `system.time`), esgotamento do orçamento de iterações com encerramento fail-closed, bloqueio pelo Security Controller no meio do loop, e integridade da serialização wire com servidor mock local.

##### Lição de Engenharia
> *«A ferramenta executada produz dados para alimentar o raciocínio do modelo, não o encerramento da conversa. Porém, ao reintroduzir os dados de uma ferramenta no diálogo, a integridade da governança deve ser mantida: o modelo nunca adquire autorização automática para a próxima ação. Cada proposta subsequente recomeça o ciclo constitucional de autorização, execução, verificação e auditoria.»*

---

#### [Event Time: 2026-10-04 ~17:22 | Record Creation Time: 2026-10-10]
### Quarto Teste Live ao Gemini — Primeiro Turno Conversacional Completo e Bateria de Validação de Capacidades

- **Event Time**: 04 de outubro de 2026, aproximadamente 17:22 (America/Sao_Paulo) / 20:22 UTC — *Horário evidenciado pelos registros de auditoria persistente*
- **Record Creation Time**: 10 de outubro de 2026
- **Milestone Relacionado**: Branch pós-MVP-1 (`feature/post-mvp1-model-runtime-wiring`, commit `accc8320872c1b179eb4e9a82fa49e1e82bd35ad`)
- **Contexto Operacional**: Validação prática conduzida pelo Owner após a implementação do *Governed Bounded Tool Continuation Loop* e atualização para o modelo `gemini-3.8-flash`.

##### 1. Teste A — Primeiro Turno Conversacional Completo
- **Entrada Submetida**:
  `Yuki, por favor se apresente e diga qual e o seu proposito fundamental em uma frase.`
- **Resposta Produzida**:
  > *«Olá, eu sou a Yuki, uma assistente de inteligência artificial cujo propósito fundamental é auxiliar você de forma clara, segura e eficiente na resolução de dúvidas e execução de suas tarefas.»*
- **Classificação Histórica Formal**: **`FIRST VERIFIED SUCCESSFUL LIVE GEMINI CONVERSATIONAL TURN THROUGH YUKI`**.
- **Evidência Técnica Verificada**:
  A auditoria persistente em SQLite (`data/yuki.db`, eventos sequência 64 a 74) atesta a mecânica do turno:
  1. O prompt foi recebido e o contexto foi construído (`seq 64-65`).
  2. Na iteração 0, o Gemini 3.8 propôs autonomamente a capability `system.info` (`seq 66-67`).
  3. O Security Controller autorizou explicitamente a operação sob classe de risco `Low` (`seq 68-69`).
  4. O Execution Engine despachou `system.info` (`seq 70`), o efeito foi observado (`seq 71`) e atestado como `VerifiedSuccess` pelo Verification Engine (`seq 72`).
  5. O resultado verificado foi encapsulado como dado e reintroduzido no modelo como `functionResponse` na iteração 1 (`seq 73`).
  6. O Gemini sintetizou a resposta conversacional final em linguagem natural, encerrando o turno com sucesso (`seq 74`, status `DirectText`, 1 iteração de ferramenta).

##### 2. Teste B — Tempo, Calendário e Análise Forense de Execução de Ferramentas
- **Entrada Submetida**:
  `Yuki, que horas são agora e que dia do ano?`
- **Resposta Produzida**:
  A Yuki respondeu fornecendo o horário UTC, a data de 4 de outubro de 2026, o dia 277 do ano e contextualizou o fuso horário de Brasília.
- **Investigação Forense e Evidência Factual**:
  Ao analisar os registros do banco de auditoria (`data/yuki.db`), identificou-se categoricamente:
  - No ciclo correspondente (eventos sequência 75 a 85), o modelo propôs formalmente a capability `system.time` (`seq 78`, `op_1a10896b76d_9`).
  - O Security Controller concedeu autorização (`seq 80`), a capability executou com sucesso (`seq 81-82`) e foi verificada pelo Verification Engine (`seq 83`).
  - O resultado de `system.time` foi entregue na iteração 1 (`seq 84`), fornecendo a base fática necessária para que o modelo respondesse com precisão sobre o dia 277 e o horário exato.
  - Em contraste, em uma execução subsequente com pequena variação de prompt (eventos sequência 86 a 89), o modelo não propôs ferramentas (`has_proposal: false`), respondendo exclusivamente a partir de contexto paramétrico. A Crônica registra a evidência real: houve chamada e verificação efetiva de `system.time` no turno que contextualizou o dia do ano.

##### 3. Teste C — Conhecimento Geral Paramétrico
- **Entrada Submetida**: Consulta sobre a primeira viagem humana à Lua e o programa Apollo.
- **Resposta Produzida**: A Yuki forneceu detalhes históricos acurados sobre a missão Apollo 11 e as seis missões que pousaram astronautas na Lua.
- **Classificação**: Resposta puramente baseada no conhecimento pré-treinado (paramétrico) do LLM, sem acionamento de capabilities e sem atribuição indevida a pesquisa web.

##### 4. Teste D — Reconhecimento Explícito de Limitação de Capacidades
- **Entrada Submetida**:
  `Yuki, Como está as votações do Brasil hoje?`
- **Resposta Produzida**: A Yuki informou com transparência que não possui acesso a informações em tempo real e sugeriu ao operador a consulta a fontes externas e veículos oficiais.
- **Classificação**: Limitação atual e legítima de capacidades (*Capability Boundary*), constituindo comportamento íntegro e seguro da IA, e NÃO uma falha do Model Gateway.

##### 5. Teste E — Pesquisa Complexa: Observabilidade de Timeout e Limites de Cota
- **Entrada Submetida**: Consulta complexa demandando pesquisas, estatísticas e tendências da eleição presidencial brasileira de 2026.
- **Resultados Observados**:
  1. *Primeira tentativa*: `Model error: Timeout: 30s` — O processamento upstream do Gemini excedeu a janela máxima de 30 segundos configurada para uma chamada HTTP individual.
  2. *Segunda tentativa*: `RESOURCE_EXHAUSTED` — O provedor Google Gemini rejeitou a requisição informando esgotamento do limite na métrica `generate_content_free_tier_requests` para o modelo `gemini-3.8-flash`, indicando um intervalo de retry de aproximadamente 9 segundos.
- **Lição de Engenharia e Esclarecimento de Infraestrutura**:
  1. *Free Tier vs Assinatura de Consumidor:* A assinatura Google AI Pro (Google One AI Premium) do usuário destina-se exclusivamente ao aplicativo web de consumo do Gemini (gemini.google.com). Ela não concede cota paga nem remove os limites da Google Gemini Developer API (Google AI Studio / GCP), que opera sob políticas próprias de tarifação e cotas (Rate Limit de 5 ou 15 RPM no nível gratuito).
  2. *Necessidade de Orçamentos de Pesquisa:* Consultas de alta densidade cognitiva demandam orçamentos formais de tempo, controle de taxa e decomposição assíncrona, corroborando a necessidade da futura capability de Research.

##### Lição de Engenharia
> *«Um assistente pessoal governado não se prova apenas quando acerta, mas quando revela com integridade suas fronteiras: reconhecendo a ausência de acesso à rede, falhando de forma fechada diante de timeouts e cotas, e mantendo cada ferramenta sob escrutínio de segurança mesmo no fluxo contínuo de conversação.»*

---

#### [Event Time: 2026-10-10 | Record Creation Time: 2026-10-10]
### Integração Oficial do Runtime Gemini em Main e Execução do Marco 1 da Yuki Research v1 (Mocks & Contratos)

- **Event Time**: 10 de outubro de 2026
- **Record Creation Time**: 10 de outubro de 2026
- **Milestones Relacionados**:
  1. Integração oficial da branch `feature/post-mvp1-model-runtime-wiring` ao `main` (`33fd9da861cfa0bb1056585ef5e05a074b4af912`, CI: SUCCESS).
  2. Implementação do Marco 1 de Research v1 na branch `feature/research-v1` (ADR-020 revisado).
- **Natureza do Evento**: Marco arquitetural de estabelecimento dos primeiros contratos tipados de busca e leitura estruturada da Yuki, com execução simulada em memória e salvaguardas perimetrais completas.

##### 1. Integração Governamental do Gemini Runtime ao Main
Com a aprovação do Owner e a validação dos 11 commits de observabilidade e continuação de ferramentas, o branch `feature/post-mvp1-model-runtime-wiring` foi formalmente integrado ao `main`:
- Merge commit: `33fd9da861cfa0bb1056585ef5e05a074b4af912` (pais: `6a9d070` e `e83905a`).
- CI de integração aprovado no GitHub Actions (Run 38045614665, status: `success`).
- Baselines congelados `v0.1.0-foundation` e `v0.2.0-mvp1` mantidos rigorosamente preservados. Nenhuma release tag ou release GitHub foi criada.

##### 2. Conclusão do Marco 1 de Research v1 (Contratos e Mock Providers)
Na branch `feature/research-v1`, foi implementado o Marco 1 da primeira capability de pesquisa web da Yuki, respeitando os contratos constitucionais:
- **`research.search`**: Contratos tipados de entrada (`query`, `max_results`, `freshness`) e saída verificada (`query`, `provider`, `results` com `cite_id`, título, URL, snippet, metadados e estado de confiança `AggregatedSnippet`).
- **`research.fetch`**: Contratos tipados de entrada (`url`, `max_length_chars`) e saída verificada (`url`, `final_url`, `http_status`, `content_type`, `title`, `extracted_text`, `content_hash_sha256`, integridade e estado de confiança `DirectSource`).
- **Abstrações e Mocks**: Criação dos traits `SearchProvider` e `ContentFetchProvider` com implementações determinísticas `MockSearchProvider` e `MockFetchProvider` funcionando 100% em memória, sem qualquer tráfego de rede, cliente HTTP, scraper ou browser.
- **SHA-256 Puro em Rust**: Implementação determinística FIPS 180-4 em Rust puro para cômputo de hashes de integridade sem adicionar dependências externas de rede.
- **Integração no Core**: Capabilities registradas no `CapabilityRegistry`, permissões gerenciadas no `SecurityController`, estratégias de verificação estrutural dedicadas no `VerificationEngine` (`SearchVerificationStrategy`, `FetchVerificationStrategy`) e projeção automática no loop de continuação de ferramentas.
- **Defesa Estrutural contra Indirect Prompt Injection**: Testes adversariais comprovaram que conteúdos web maliciosos contendo comandos embutidos são tratados como dado passivo e NUNCA conferem autorização ou disparam ferramentas.
- **Suíte de Testes e Qualidade**: 20 novos testes dedicados em `tests/capability_research_mock.rs`, totalizando 180 testes unitários e de integração passando, 0 warnings no Clippy, formatação `cargo fmt` impecável e build de release validado.

##### Status Operacional do Marco 1
> [!NOTE]
> O Marco 1 estabeleceu com sucesso a fundação contratual e mock offline da capability de Research. A integração de busca real foi formalmente concluída no Marco 2.

---

#### [Event Time: 2026-10-10 | Record Creation Time: 2026-10-10]
### Conclusão do Marco 2 da Yuki Research v1 — Adapter Brave Search Real, Segurança de Egress e Testes Offline

- **Event Time**: 10 de outubro de 2026
- **Record Creation Time**: 10 de outubro de 2026
- **Milestone Relacionado**: Branch `feature/research-v1` (ADR-020 revisado)
- **Classificação Formal**: **`IMPLEMENTADO — PENDENTE DE LIVE SMOKE TEST COM QUOTA REAL DO OPERADOR`**
- **Natureza do Evento**: Implementação completa do adaptador de busca web real (`BraveSearchProvider`), diferenciação formal de permissões (`egress:web_search`), salvaguardas perimetrais, migração de integridade para a biblioteca comunitária `sha2` e suíte de 26 testes offline mais 1 teste live opt-in.

##### 1. Auditoria e Reconciliação do Marco 1
- **Contratos Reconciliados**: Reconciliação estrita com a especificação formal do ADR-020:
  - `research.search`: `query` obrigatória, `max_results` delimitado entre 1 e 10 (default 5), enum `freshness` (`any`, `day`, `week`, `month`, `year`).
  - `research.fetch`: `url` obrigatória, `max_length_chars` delimitado entre 500 e 30000 (default 10000).
- **Migração de Integridade SHA-256**: Substituição da implementação local manual pelo crate oficial auditado da comunidade Rust `sha2 = "0.10"`. A assinatura pública `compute_sha256(data: &[u8]) -> String` foi preservada sem quebras de compatibilidade, acompanhada de testes com vetores de teste NIST FIPS 180-4 que comprovaram equivalência exata de integridade.

##### 2. Implementação do Adaptador de Busca Real (`BraveSearchProvider`)
- **Provedor Primário Oficial**: Integração com a API REST da Brave Search (`https://api.search.brave.com/res/v1/web/search`).
- **Mapeamento Estruturado de Consulta**: A query do usuário e parâmetros de contexto são mapeados em parâmetros HTTP fortemente validados: `q`, `count`, `freshness`, `safesearch`, `search_lang`, `country`, `text_decorations=0`.
- **Isolamento de Runtime Assíncrono**: Para evitar deadlocks e conflitos de nesting em chamadas síncronas/assíncronas no Tokio, o `BraveSearchProvider` isola a execução da requisição HTTP em uma thread dedicada (`std::thread::spawn`) com runtime `current_thread` próprio.
- **Fail-Closed de Rede**: A execução de requisições de rede reais é bloqueada por padrão:
  - `live_enabled` requer `YUKI_RESEARCH_LIVE_ENABLED=true`. Caso desativado ou ausente, a chamada falha de forma fechada antes de qualquer abertura de socket.
  - A credencial `brave_search_api_key` é resolvida estritamente via `CredentialBroker` e seu `SecretRef`, prevenindo qualquer vazamento no console, em erros ou nos logs de auditoria.
- **Tratamento Determinístico de Erros HTTP**: Mapeamento seguro para os tipos de erro da Yuki:
  - HTTP 400: `InvalidRequest` (parâmetros rejeitados upstream).
  - HTTP 401/403: `PermissionDenied` (credencial inválida, revogada ou ausente).
  - HTTP 429: `RateLimitExceeded` (cota ou taxa esgotada no provedor).
  - HTTP 500/502/503: `ExecutionFailed` (falha temporária do serviço Brave).
- **Proteção Perimetral de Resposta**:
  - Teto de payload: máximo de 512 KiB por resposta HTTP para evitar exaustão de memória.
  - Truncamento e sanitização de dados: títulos truncados em 200 caracteres, snippets em 1000 caracteres.
  - Higienização de links: descarte imediato de resultados com esquemas inseguros (`javascript:`, `file:`, etc.).

##### 3. Modelo Diferenciado de Permissões e Segurança de Egress
- **Separação Concreta de Permissões**:
  - Provedor Mock: exige apenas `capability:research.search` e declara `network_required: false`.
  - Provedor Live (Brave): exige `["capability:research.search", "egress:web_search"]`, declara `network_required: true` e `secrets_required: true`.
- **Security Controller Governa Egress**: Sob a política fundamental padrão (`DefaultFoundationPolicy`), a permissão `egress:web_search` NÃO é concedida por padrão. Qualquer tentativa de invocar a busca real falha de forma fechada com negação de segurança, garantindo que nenhum tráfego de rede ocorra sem autorização explícita do operador.
- **Orçamento Global de Turno (`ResearchBudget`)**:
  - Teto de 3 pesquisas por turno conversacional. A quarta tentativa é bloqueada diretamente pelo provedor.
  - Timeout por requisição delimitado (default 10 segundos).

##### 4. Escopo Rigorosamente Preservado (Zero Fetch Real no Marco 2)
- Em conformidade estrita com as diretrizes do Owner:
  - **`research.fetch` permaneceu 100% Mock (`MockFetchProvider`)**: Nenhum cliente HTTP arbitrário, socket de rede ou scraper foi adicionado para leitura de páginas completas.
  - A leitura web segura, com resolução DNS perimétrica, anti-SSRF exaustivo (IPv4, IPv6 e IPv4-mapped IPv6), pinning de socket e política de redirecionamento, permanece reservada com exclusividade para o **Marco 3**.

##### 5. Bateria de Testes e Validação
- **Suíte Dedicada (`tests/capability_research_brave.rs`)**:
  - 26 testes offline executados contra servidor HTTP mock efêmero em `127.0.0.1` cobrindo sucesso, erros HTTP (400, 401, 403, 429, 500, 503), timeout, JSON malformado, respostas vazias, truncamento de payload, sanitização de URLs, esgotamento de orçamento, não-vazamento de chaves e ataque adversarial de prompt injection em snippets.
  - 1 teste live opt-in (`test_brave_live_opt_in_smoke_test`) marcado com `#[ignore]` para execução deliberada pelo operador com chave e quota reais da Brave.
- **Portões de Qualidade**:
  - `cargo test`: 206 testes unitários e de integração verdes em todo o workspace (com 3 testes live ignorados).
  - `cargo fmt --check`: código 100% aderente ao padrão da linguagem.
  - `cargo clippy --all-targets -- -D warnings`: 0 warnings e 0 erros.
  - `cargo build --release`: compilação limpa e otimizada.

---

#### [Event Time: 2026-10-10 | Record Creation Time: 2026-10-10]
### Conclusão do Marco 3 da Yuki Research v1 — Governed Web Fetch, Defesa Anti-SSRF em Profundidade e Proveniência de Conteúdo

- **Event Time**: 10 de outubro de 2026
- **Record Creation Time**: 10 de outubro de 2026
- **Milestone Relacionado**: Branch `feature/research-v1` (ADR-020 revisado)
- **Classificação Formal**: **`MARCO 3 IMPLEMENTADO — TESTADO OFFLINE — LIVE PENDENTE`**
- **Natureza do Evento**: Implementação da leitura governada de páginas web (`research.fetch`) via `HttpContentFetchProvider`, perímetro defensivo anti-SSRF em profundidade, prevenção de DNS rebinding por socket pinning, sanitização de HTML contra injeção indireta de prompt e controle de envelope de orçamento compartilhado por turno.

##### 1. Auditoria e Reconciliação com o Marco 2
- **Auditoria do BraveSearchProvider**: Identificou-se que o contador de buscas executadas era anteriormente privado e não compartilhado com o fetch. O isolamento de thread e runtime Tokio por requisição mostrou-se resiliente contra deadlocks e timeouts sob teste de carga.
- **Envelope Unificado de Orçamento (`ResearchBudgetTracker`)**: Unificou-se a governança de consumo de turno em `src/contracts/research.rs`. Busca e leitura compartilham deterministicamente o mesmo envelope (3 buscas, 3 fetches, 45 segundos de timeout global e teto cumulativo de bytes), resetado no início de cada novo turno pelo `YukiCore`.
- **Enriquecimento do Contrato de Fetch**: Adicionados os campos `bytes_observed: usize` e `source_id: Option<String>` ao `ResearchFetchResult`, consolidando a cadeia de custódia e proveniência de dados.

##### 2. Arquitetura Defensiva Anti-SSRF e Socket Pinning (`src/capabilities/research/ssrf.rs`)
- **Validação Sintática e Semântica de URL**:
  - Aceitação exclusiva dos esquemas `http` e `https`.
  - Rejeição absoluta de credenciais embutidas (`http://user:pass@host/`).
  - Bloqueio estrito de portas anômalas (autorizadas unicamente as portas 80 e 443).
  - Rejeição de representações anômalas ou ofuscadas de IP (hexadecimal, octal, dword) e sufixos de domínio reservados/internos (`.localhost`, `.local`, `.internal`, `.lan`, etc.).
- **Política Positiva de IPs Globalmente Roteáveis**:
  - Bloqueio exaustivo de faixas IPv4: RFC 1918 (`10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`), RFC 6598 CGNAT (`100.64.0.0/10`), Loopback (`127.0.0.0/8`), Link-Local e Cloud Metadata (`169.254.0.0/16` incluindo `169.254.169.254`), faixas de documentação/teste e broadcast/multicast.
  - Bloqueio exaustivo de IPv6: loopback (`::1/128`), ULA privado (`fc00::/7`), link-local (`fe80::/10`), multicast (`ff00::/8`).
  - **Defesa Crítica Contra Evasão IPv4-Mapped IPv6**: Desencapsulamento estrito do bloco `::ffff:0:0/96`, validando os 32 bits embutidos contra todas as regras de IPv4.
- **Defesa Contra DNS Rebinding e Socket Pinning**:
  - Resolução DNS desacoplada via trait `DnsResolver` (`SystemDnsResolver` e `MockDnsResolver`).
  - Inspeção obrigatória de **todos** os endereços IP retornados. Se qualquer IP for privado ou proibido, a operação inteira falha fechada (*fail-closed*).
  - Amarração de socket (*connection pinning*): o IP público aprovado é vinculado ao destino da conexão TCP/TLS via `reqwest::ClientBuilder::resolve(host, pinned_addr)`, neutralizando ataques de DNS rebinding em conexões subsequentes.

##### 3. Loop de Redirecionamento Supervisionado
- Redirecionamentos automáticos nativos completamente desativados (`redirect::Policy::none()`).
- Avaliação explícita de saltos de redirecionamento (máximo de 3 saltos).
- Cada URL de redirecionamento passa por validação per-hop de esquema, porta, DNS e IP antes da emissão da requisição.
- Proibição absoluta de downgrade de segurança de `https://` para `http://`.

##### 4. Limites de Streaming e Higiene HTML (`src/capabilities/research/html_extract.rs`)
- **Limites de Streaming**:
  - Teto de streaming comprimido: 256 KiB.
  - Teto de streaming descomprimido: 1 MiB (1.048.576 bytes).
  - Teto de texto retornado à Yuki: 30.000 caracteres, com flag explícita `truncated = true`.
- **Higiene e Sanitização de Conteúdo**:
  - Extração limpa do título `<title>`.
  - Remoção de blocos executáveis e de layout: `<script>`, `<style>`, `<noscript>`, `<svg>`, `<canvas>`, `<iframe>`, `<form>`, `<input>`, `<object>`, `<embed>`.
  - Conversão de elementos estruturais em quebras de linha limpas.
  - Decodificação não recursiva de entidades HTML (imunidade contra entity bombs).
  - Invariante ontológico fundamental: o texto extraído é tratado como dado bruto não confiável de terceiros (`Data != Instruction`), impedindo que comandos embutidos em páginas web alcancem autorização ou execução.

##### 5. Modelo de Permissões e Segurança de Egress
- **Permissão Dedicada**: Criada a permissão `egress:web_fetch`, pertencente à classe `RiskClass::High`, completamente independente de `egress:web_search`.
- **Fail-Closed sob Política Padrão**: A permissão `egress:web_fetch` está ausente da política padrão do sistema (`DefaultFoundationPolicy`), exigindo autorização explícita do operador para modo live.
- **Variável de Ambiente de Controle**: Leitura real depende de `YUKI_RESEARCH_LIVE_ENABLED=true`.

##### 6. Bateria de Testes e Validação
- **Suíte Dedicada (`tests/capability_research_fetch.rs`)**:
  - 34 testes determinísticos offline cobrindo todos os requisitos: seleção de provedores, fail-closed de live mode, permissões no manifest, negação pelo Security Controller, validação de URL, esquemas proibidos, credenciais embutidas, loopback, faixas IPv4/IPv6 privadas, IPv4-mapped IPv6, cloud metadata, DNS misto fail-closed, defesa contra DNS rebinding, socket pinning, redirecionamento para rede interna, limite de saltos de redirecionamento, timeouts, cancelamento de recursos, limites de streaming comprimido e descomprimido, sanitização de HTML (scripts, estilos, iframes), injeção indireta de prompt como dado passivo, rejeição de tipos de conteúdo binários, tratamento de erros HTTP (404, 429, 500), extração estrutural, cálculo de hash SHA-256, flag de truncamento, metadados de proveniência, orçamento compartilhado de turno, loop de continuação governada, regressão zero de Brave search e MVP-1, e portabilidade Linux/Docker.
  - 1 teste live opt-in (`test_fetch_live_opt_in_smoke_test`) marcado com `#[ignore]` para execução deliberada pelo operador.
- **Portões de Qualidade Aprovados**:
  - `cargo test`: 240 testes unitários e de integração verdes no workspace (4 testes live ignorados).
  - `cargo fmt --check`: 100% de aderência ao padrão da linguagem Rust.
  - `cargo clippy --all-targets -- -D warnings`: 0 warnings e 0 erros.
  - `cargo build --release`: compilação limpa e bem-sucedida.
- **Preservação Rígida dos Baselines**:
  - `v0.1.0-foundation` (`654c181`) e `v0.2.0-mvp1` (`6a9d070`) mantidos integralmente preservados.
  - Nenhuma release tag ou release GitHub criada.

---

#### [2026-10-10] Research v1 — Auditoria Corretiva de Segurança Pós-Marco 3 (Concorrência, SSRF, Orçamento e Proveniência)

##### 1. Contexto e Objetivos da Auditoria
- Conduzida auditoria adversarial independente sobre os subsistemas dos Marcos 1, 2 e 3 da Research v1 (`feature/research-v1`), avaliando convecção de transporte HTTP, proteção SSRF contra evasões com proxies, concorrência no orçamento de recursos, isolamento de turnos e higienização de dados.
- Verificação minuciosa das invariantes constitucionais: `Capability != Permission != Authorization != Execution`, `Model Output != Command != Authorization`, `Think != Authorize != Execute`, `Data != Instruction`, `Verification != Truth`, `Storage != Authority`.

##### 2. Vulnerabilidades Remediadas e Endurecimentos Implementados
- **Transporte HTTP e Defesa Anti-SSRF contra Evasão de Proxy**:
  - Implementado `.no_proxy()` nos construtores `reqwest::Client::builder()` de `NetworkFetchTransport` e `BraveSearchProvider`, prevenindo que proxies de ambiente (`HTTP_PROXY`, `ALL_PROXY`) sequestrassem o tráfego e anulassem o socket pinning.
  - Endurecida a validação `validate_url` e `is_ambiguous_ip_encoding` contra IPs truncados (ex.: `127.1`, `10.1`), hexadecimais em segmentos (ex.: `0x7f000001`, `127.0.0.0x1`) e validação direta de IPs literais parseados.
  - Adicionado bloqueio explícito de endereços IPv6 site-local obsoletos (`fec0::/10`, RFC 3879) em `validate_ipv6`.
- **Contenção Estrita de Recursos e Prevenção de OOM**:
  - Substituído `resp.bytes().await` por leitura streaming incremental (`resp_stream.chunk().await`) com aborto imediato e devolução de erro assim que o acumulador excede 1 MiB (`1.048.576` bytes).
- **Concorrência e Isolamento de Orçamento (`ResearchBudgetTracker`)**:
  - Eliminadas corridas de verificação-e-ação (*check-then-act*) através de loops atômicos `compare_exchange_weak` com ordenação `SeqCst` / `Relaxed` para contagem de buscas, fetches e bytes acumulados.
  - Implementado escopo de orçamento isolado por turno via `tokio::task_local! { pub static CURRENT_TURN_BUDGET: Arc<ResearchBudgetTracker>; }` em `YukiCore::process_input_async`, eliminando contaminação e mutações inseguras de instâncias singleton via `reset_turn()`.
  - Reconciliado o limite cumulativo padrão de bytes por turno para 2 MiB (`2.097.152` bytes) no `ResearchBudget`, harmonizando a capacidade com o teto de 3 fetches de 1 MiB.
- **Higiene de Conteúdo e Entropia de Proveniência**:
  - Descarte seguro até EOF de blocos perigosos não fechados (`<script>`, `<style>`) em `html_extract.rs`, eliminando vazamento de código em HTML truncado ou hostil.
  - Expandido o identificador `source_id` para 16 caracteres hexadecimais (64 bits de entropia) derivados do SHA-256 (`src:fetch:{hash[..16]}`).

##### 3. Nova Suíte Dedicada de Auditoria (`tests/security_research_audit.rs`)
- 20 testes adversariais adicionados cobrindo:
  1. Socket pinning e imunidade a proxies de ambiente (`.no_proxy()`);
  2. Liberação de conexões sob timeout de requisições lentas;
  3. Ausência de vazamento de recursos sob concorrência multi-thread;
  4. Isolamento de orçamento por sessão/turno assíncrono;
  5. Prevenção de condições de corrida via loops atômicos CAS;
  6. Contabilidade atômica de bytes em requisições paralelas;
  7. Teto cumulativo canônico de 2 MiB;
  8. Bloqueio de IPs numéricos truncados;
  9. Bloqueio de representações hexadecimais ofuscadas;
  10. Bloqueio de endereços IPv6 site-local (`fec0::/10`);
  11. Bloqueio de redirecionamento para loopback e metadados de nuvem;
  12. Aborto precoce em streaming de corpo excessivo;
  13. Descarte limpo de tags `<script>` não fechadas;
  14. Extração de HTML malformado, aninhado e com Unicode/emojis;
  15. Injeção indireta de prompt não pode autorizar nem executar capacidades desautorizadas;
  16. Permissão `egress:web_search` não autoriza `research.fetch`;
  17. Permissão `egress:web_fetch` não autoriza `research.search`;
  18. Chamada direta ao executor sem token válido falha fechada;
  19. Entropia de 16 caracteres hexadecimais no `source_id`;
  20. `DirectSource` representa integridade de proveniência, não veracidade de fatos.

##### 4. Portões de Qualidade
- `cargo test`: 260 testes unitários, de integração e de auditoria aprovados (4 testes live ignorados).
- `cargo fmt --check`: 100% aprovado.
- `cargo clippy --all-targets -- -D warnings`: 0 warnings, 0 erros.
- `cargo build --release`: aprovado.
- Baselines `v0.1.0-foundation` (`654c181`) e `v0.2.0-mvp1` (`6a9d070`) rigorosamente preservados.

---

#### [2026-10-10] Research v1 — Marco 4: Governed Synthesis, Source Identity, Evidence Registry & Verifiable Citations

##### 1. Contexto e Objetivos do Marco 4
- Implementado o Marco 4 da Yuki Research v1 na branch `feature/research-v1`, estabelecendo a governança estrita de conhecimento, registro de fontes, evidências estruturadas e resolução determinística de citações (`[src:N]`) na síntese conversacional.
- Consolidada a separação ontológica fundamental:
  - `Model Output != Command != Authorization`: Afirmações e propostas com citações emitidas pelo modelo são tratadas como dados externos não confiáveis.
  - `Verification != Truth`: Validar que uma citação existe no registro do turno e possui integridade de transporte (hash SHA-256) atesta rastreabilidade e proveniência criptográfica, mas NÃO constitui prova de veracidade factual no mundo real.
  - `Data != Instruction`: Todo conteúdo de pesquisa retornado é projetado passivamente para o modelo, impedindo a interpretação de dados externos como comandos de sistema.

##### 2. Arquitetura e Componentes Implementados
- **Identificadores e Eventos de Auditoria**:
  - `TurnId` e `ObservationId` tipados em `src/contracts/identifiers.rs`.
  - Novos eventos de auditoria em `src/contracts/events.rs`: `SourceObserved`, `EvidenceRegistered`, `CitationResolved`, `CitationRejected` e `SynthesisCompleted`.
- **Propagação Segura de Orçamento e Deadlines (`ResearchTurnContext`)**:
  - Delimitação imutável do turno com `TurnId`, `session_id`, `budget_tracker` e prazo absoluto (`deadline`).
  - Task-local `CURRENT_TURN_CONTEXT` propagado com isolamento assíncrono e repassado para threads nativas através de `.sync_scope()`, eliminando vazamentos e garantindo verificação dupla de prazo.
- **Registro de Fontes Observadas (`ObservedSourceRegistry`)**:
  - Custodiado exclusivamente pelo Yuki Core em `src/capabilities/research/registry.rs`.
  - Diferenciação estrita entre snippets de busca (`AggregatedSnippet`, `is_full_page = false`) e páginas recuperadas (`DirectSource`, `is_full_page = true`).
  - Atribuição sequencial determinística de identificadores de citação (`src:1`, `src:2`).
  - Imposição de limites por turno: máximo de 20 observações (`DEFAULT_MAX_OBSERVATIONS`) e 2 MiB de volume agregado (`DEFAULT_MAX_TOTAL_BYTES`).
  - Projeção passiva de prompts via `format_evidences_for_model()`.
- **Validador de Síntese e Citações Verificáveis (`SynthesisValidator`)**:
  - Implementado em `src/capabilities/research/synthesis.rs` com parser zero-dependencies determinístico de tags `[src:N]`.
  - Resolução de citações contra o registro governado, detecção e rejeição de referências inventadas (ex.: `[src:999]`), e anexação transparente de nota de limitação de verificação.
  - Inclusão incondicional do aviso ontológico `VERIFICATION_DISCLAIMER` (`Verification != Truth`).
  - Classificação de status: `FullyVerified`, `PartiallyVerified`, `UnverifiedClaims`.
- **Integração no Yuki Core (`src/core/yuki_core.rs`)**:
  - Inicialização de `ResearchTurnContext` e `ObservedSourceRegistry` por turno.
  - Registro automático e emissão de eventos auditáveis durante o loop de continuação governada de ferramentas.
  - Vinculação de `ResearchSynthesis` tipada ao `YukiResult`.

##### 3. Suíte de Testes Dedicada (`tests/capability_research_synthesis.rs`)
- 46 testes determinísticos e offline cobrindo:
  1. Isolamento de escopo por turno;
  2. Preservação de contexto através de `.await`;
  3. Propagação de contexto para threads nativas spawned;
  4. Expiração de deadline falha fechado;
  5. Isolamento mútuo entre turnos concorrentes;
  6. Compartilhamento do rastreador de orçamento dentro do turno;
  7. Proteção contra reset cruzado de orçamento;
  8. Resiliência a pânico em threads de transporte;
  9. Prevenção de orçamento substituto;
  10. Verificação de deadline pré e pós-IO;
  11. Precisão no cálculo de timeout;
  12. Propagação de identificador de sessão;
  13. Distinção entre ObservationId e hash SHA-256;
  14. Re-leitura gera nova observação;
  15. Separação snippet vs página completa;
  16. Atribuição sequencial de cite_id;
  17. Teto de observações por turno;
  18. Teto de bytes por turno;
  19. Resolução com ou sem colchetes;
  20. Busca de citação inexistente retorna None;
  21. Projeção contém aviso Data != Instruction;
  22. Projeção rotula snippets e páginas;
  23. Projeção reporta flag de truncamento;
  24. Prefixo de hash SHA-256 incluído na projeção;
  25. Injeção de prompt em fonte encapsulada como dado passivo;
  26. Isolamento estrito de registros por TurnId;
  27. Registro vazio gera projeção vazia;
  28. Resultados duplicados geram observações distintas;
  29. Formato rastreável de escopo de evidência;
  30. Imutabilidade de proveniência;
  31. Contabilidade precisa de bytes;
  32. Preservação de URL final pós-redirecionamento;
  33. Validação completa (FullyVerified);
  34. Citação inventada resulta em PartiallyVerified;
  35. Citações totalmente inválidas geram UnverifiedClaims;
  36. Fontes disponíveis sem citações geram UnverifiedClaims;
  37. Aviso de limitação anexado ao texto;
  38. Disclaimer ontológico Verification != Truth presente;
  39. Deduplicação de citações repetidas;
  40. Turno conversacional padrão sem fontes é FullyVerified;
  41. Citação inventada em registro vazio é rejeitada;
  42. Fluxo ponta-a-ponta de busca com síntese verificada;
  43. Fluxo ponta-a-ponta de fetch com síntese verificada;
  44. Auditoria de eventos de síntese e proveniência;
  45. Auditoria de evento CitationRejected sob citação inventada;
  46. Conversa normal sem pesquisa preserva YukiResult sem síntese.

##### 4. Portões de Qualidade Aprovados
- `cargo test`: 306 testes aprovados no workspace (0 falhas, 4 testes live ignorados).
- `cargo fmt --check`: 100% aprovado.
- `cargo clippy --all-targets -- -D warnings`: 0 warnings, 0 erros.
- `cargo build --release`: compilação otimizada concluída com sucesso.
- Baselines `v0.1.0-foundation` (`654c181`) e `v0.2.0-mvp1` (`6a9d070`) estritamente preservados.
- Classificação: **MARCO 4 IMPLEMENTADO — TESTADO OFFLINE — LIVE PENDENTE**.

---

#### [2026-10-10] Research v1 — Auditoria Final de Integração e Segurança (Marcos 1 a 4)

##### 1. Contexto e Objetivos da Auditoria Final
- Conduzida auditoria final abrangente, independente e adversarial sobre a totalidade da capability Research v1 na branch `feature/research-v1`.
- Avaliados os eixos contratuais e constitucionais estabelecidos pelo ADR-020: fluxo completo ponta-a-ponta, auditoria de citações e citações inexistentes/fabricadas com política fail-closed, semântica de páginas truncadas (`is_full_page`), concorrência e orçamento de turno (`ResearchTurnContext`), segurança de rede e anti-SSRF, contenção de prompt injection (`Data != Instruction`), proveniência e isolamento multissessão/multiturno (`Verification != Truth`), compatibilidade com baselines congelados e validação de qualidade.

##### 2. Achados e Correções Implementadas
- **Classificação Ontológica de Páginas Truncadas (`is_full_page` vs `truncated`)**:
  - *Arquivo/Linha*: `src/capabilities/research/registry.rs`, `src/capabilities/research/synthesis.rs`, `src/contracts/research.rs`.
  - *Problema*: `is_full_page` estava marcado incondicionalmente como `true` para qualquer `SourceKind::DirectSource`, mesmo quando o payload havia sido truncado (`truncated == true`).
  - *Correção*: `is_full_page` passa a exigir estritamente `!fetch_result.truncated`. Páginas truncadas permanecem como `DirectSource`, porém rotuladas com `is_full_page = false` e `truncated = true`. O campo `pub truncated: bool` foi adicionado à estrutura `VerifiedCitation`, garantindo rastreabilidade de ponta a ponta.
- **Política Fail-Closed contra Afirmações Fabricadas e Citações Inexistentes**:
  - *Arquivo/Linha*: `src/capabilities/research/synthesis.rs`.
  - *Problema*: Quando o modelo inventava uma citação inexistente (ex.: `[src:999]`), o validador classificava o status e anexava uma nota de rodapé, porém mantinha a afirmação fabricada crua no texto com falsa aparência de confirmação.
  - *Correção*: Se todas as referências forem inexistentes/inválidas (`unresolved_citations` não-vazio e `verified_citations` vazio), a afirmação crua é integralmente substituída por uma resposta segura de evidências insuficientes, eliminando qualquer afirmação fabricada da resposta. Em citações mistas, fontes inválidas no texto são sanitizadas como `[src:N][NÃO VERIFICADA]`, e a limitação de verificação é anexada.

##### 3. Nova Suíte Dedicada de Auditoria Integrada (`tests/research_v1_final_audit.rs`)
- 23 testes determinísticos e offline adicionados cobrindo:
  1. Cadeia ponta-a-ponta de busca, leitura e síntese governada;
  2. Tratamento gracioso de pesquisas com 0 resultados;
  3. Preservação de observações independentes para fontes contraditórias;
  4. Interrupção fail-closed ao esgotar orçamento de turno no meio do loop;
  5. Impossibilidade absoluta de contornar o Security Controller;
  6. Resposta segura fail-closed para citações inventadas `[src:999]`;
  7. Sanitização de citações mistas com marcação explícita de não-verificada;
  8. Rejeição e fail-closed para citações de outros turnos ou sessões;
  9. Deduplicação de citações e descarte de referências malformadas;
  10. Classificação de UnverifiedClaims quando há fontes disponíveis sem citação;
  11. Distinção semântica entre página integral e página truncada;
  12. Preservação da propriedade `truncated` até a citação final `VerifiedCitation`;
  13. Unicidade de identidade (`ObservationId`) para leituras múltiplas da mesma URL;
  14. Isolamento estrito de orçamentos e contextos em sessões simultâneas;
  15. Cancelamento estruturado assíncrono sem vazamentos nem deadlocks;
  16. Validação de deadline antes e após IO com falha fechada;
  17. Matriz exaustiva de bloqueio anti-SSRF (IPv4 privado, IPv6 ULA, cloud metadata, IPv4-mapped IPv6);
  18. Bloqueio de representações de IP ofuscadas (hex, octal, dword, truncadas);
  19. Defesa fail-closed contra DNS rebinding com IPs mistos;
  20. Sanitização estrita de scripts, estilos e injeções em HTML;
  21. Candidatos não confiáveis do modelo não podem invocar comandos não autorizados;
  22. Páginas diferentes com conteúdo idêntico recebem observações e cite_ids distintos;
  23. Limites de contagem de observações (20) e volume de memória (2 MiB) no registro.

##### 4. Portões de Qualidade Aprovados
- `cargo test`: 329 testes aprovados no workspace (0 falhas, 4 testes live ignorados).
- `cargo fmt --check`: 100% aprovado.
- `cargo clippy --all-targets -- -D warnings`: 0 warnings, 0 erros.
- `cargo build --release`: compilação otimizada concluída com sucesso.
- Baselines `v0.1.0-foundation` (`654c181`) e `v0.2.0-mvp1` (`6a9d070`) estritamente preservados.
- Veredito da Auditoria: **RESEARCH V1 — APROVADA PARA VALIDAÇÃO LIVE CONTROLADA**.

---

#### [2026-10-10] Research v1 — Correção Pós-Live: Content-Encoding e Integridade de Extração

##### 1. Contexto e Diagnóstico
- Durante a execução supervisionada da Validação Live Controlada — Etapa 1 (`research.fetch` contra `https://example.com/`), o servidor de destino respondeu com `Content-Encoding: gzip`.
- O cliente HTTP `HttpContentFetchProvider` não descomprimia o fluxo binário antes da decodificação de texto UTF-8 (`String::from_utf8_lossy`), entregando caracteres de controle corrompidos para a extração HTML e poluindo a extração textual.
- Diagnosticada a necessidade de implementação de suporte a `Content-Encoding` delimitado, seguro e imune a *Zip Bombs* / *Decompression Bombs*.

##### 2. Arquitetura e Implementação
- **Módulo Dedicado (`src/capabilities/research/encoding.rs`)**:
  - Enum tipado `SupportedContentEncoding` com variantes `Identity`, `Gzip`, `Deflate` e `Brotli`.
  - Parsing de cabeçalho `Content-Encoding` fail-closed (rejeição imediata de formatos não suportados como `zstd`, `compress`, `lzo`).
  - Funções de descompressão streaming delimitada com leitura incremental em blocos de 8 KiB (`read_stream_with_limit`), abortando imediatamente se os dados emitidos ultrapassarem o teto de 1 MiB (`1.048.576` bytes).
  - Teto de bytes brutos na rede delimitado em 256 KiB (`262.144` bytes) para payloads comprimidos.
  - Helpers determinísticos de compressão para testes (`compress_gzip`, `compress_deflate`, `compress_brotli`).
- **Dependências Puramente Rust (`Cargo.toml`)**:
  - Adicionados `flate2 v1.1.10` (backend Rust puro via `miniz_oxide`) e `brotli v9.0.0` (Rust puro), sem dependências dinâmicas de C.
- **Contratos e Proveniência (`src/contracts/research.rs`)**:
  - `ResearchFetchResult` enriquecido com `raw_network_bytes: usize`, `decompressed_bytes: usize` e `content_encoding: Option<String>`.
  - `ResearchFetchInput` enriquecido com `with_max_length_chars()`.
  - O cálculo do hash criptográfico SHA-256 (`content_hash_sha256`) é estritamente aplicado sobre o texto limpo extraído (`extracted_text`), preservando proveniência e integridade semântica.
  - O acumulador de orçamento do turno (`ResearchBudgetTracker::record_bytes`) passa a registrar fielmente o total de bytes descomprimidos na memória.
- **Configuração e Transporte (`src/capabilities/research/http_fetch.rs`)**:
  - Adicionado campo `accept_encoding` (padrão `"gzip, deflate, br"`) configurável via `with_accept_encoding` e variável de ambiente `YUKI_RESEARCH_ACCEPT_ENCODING`.
  - `NetworkFetchTransport` aplica teto de streaming na rede diferenciado (256 KiB para streams comprimidos, 1 MiB para não comprimidos).
  - Preservados incondicionalmente TLS, SSRF, socket pinning, `.no_proxy()` e verificação de deadlines de turno.

##### 3. Nova Suíte Dedicada de Testes Offline (`tests/capability_research_content_encoding.rs`)
- 16 testes determinísticos e 100% offline cobrindo:
  1. Gzip válido com extração HTML íntegra;
  2. Deflate válido com preservação de texto;
  3. Brotli válido com integridade de caracteres;
  4. Identity explícito e implícito;
  5. Encodings desconhecidos rejeitados fail-closed (`zstd`);
  6. Gzip corrompido com falha tratada determinística;
  7. Stream interrompido / truncado tratado graciosamente;
  8. Zip Bomb (>1 MiB descomprimido) bloqueado durante descompressão streaming em blocos;
  9. Payload de rede >256 KiB bloqueado antes/durante recepção;
  10. Textos HTML UTF-8 com caracteres acentuados;
  11. Entidades HTML especiais decodificadas;
  12. Preservação da flag `truncated` em textos longos;
  13. Cancelamento por deadline de turno durante a descompressão;
  14. Concorrência entre requisições com encodings distintos sem interferência;
  15. Proveniência e integridade de metadados no `ObservedSourceRegistry`;
  16. Registro de bytes descomprimidos no `ResearchBudgetTracker`.

##### 4. Portões de Qualidade Aprovados
- `cargo test`: 345 testes unitários e de integração aprovados no workspace (0 falhas, 4 testes live ignorados).
- `cargo fmt --check`: 100% aprovado.
- `cargo clippy --all-targets -- -D warnings`: 0 warnings, 0 erros.
- `cargo build --release`: compilação limpa e bem-sucedida.
- Baselines `v0.1.0-foundation` (`654c181`) e `v0.2.0-mvp1` (`6a9d070`) estritamente preservados.
- Nenhuma requisição à internet pública ou consumo de cotas de APIs externas realizado nesta etapa.
- Classificação: **CONTENT-ENCODING — CORRIGIDO OFFLINE**.

---

#### [2026-10-11] Research v1 — Marco 5: LangSearch Provider + Arquitetura Yuki Native Search

##### 1. Contexto e Objetivos do Marco 5
- Implementado o Marco 5 da Yuki Research v1 na branch `feature/research-v1`, expandindo a autonomia e independência de fornecedores de busca textual da Yuki através de duas frentes complementares:
  1. **Parte A — Provedor LangSearch**: Integração de um novo adaptador governado para a API oficial LangSearch (REST POST `/v1/web-search`), operando como alternativa plugável ao `BraveSearchProvider` sob o mesmo trait `SearchProvider` e governança perimetral `egress:web_search`.
  2. **Parte B — Arquitetura Yuki Native Search (ADR-021)**: Formalização do desenho arquitetural completo para indexação e busca soberana local (SQLite FTS5 + BM25), rastreamento web governado e recuperação híbrida via *Reciprocal Rank Fusion (RRF)*.
- Preservação estrita das invariantes constitucionais: `Data != Instruction`, `Verification != Truth`, `Capability != Authorization`, `Offline First, Network On-Demand`, `Storage != Authority`.

##### 2. Implementação do Adaptador LangSearch (`src/capabilities/research/langsearch.rs`)
- **Tipos de Transporte Seguros e Isolados**:
  - `LangSearchWireRequest`: modelagem tipada do payload JSON da requisição POST com `query`, `count`, `freshness` (mapeamento canônico: `day`, `week`, `month`, `noLimit`) e `summary: false`.
  - `LangSearchWireResponse`: parsing estrito do JSON de resposta (`code`, `msg`, `data.webPages.value`), com validação de status de aplicação (`code == 200`) e captura de mensagens de erro.
- **Configuração e Governança (`LangSearchConfig`)**:
  - Desabilitado por padrão (`live_enabled == false`, fail-closed). A presença de chave não autoriza live mode sem consentimento explícito (`YUKI_RESEARCH_LIVE_ENABLED=true`).
  - Resolução de segredo via `CredentialBroker` com `SecretRef` canônica (`secret://env/YUKI_LANGSEARCH_API_KEY`, alias `cred_langsearch_primary`).
  - Imunidade a vazamentos: chave nunca interpolada em URLs, logs, traces ou mensagens de erro.
  - Cliente HTTP construído com `.no_proxy()` (prevenção contra evasão por proxies de ambiente), `.redirect(Policy::none())` e timeouts atrelados ao orçamento do turno.
- **Isolamento de Runtime Assíncrono (`run_async`)**:
  - Execução da requisição HTTP isolada em thread dedicada com runtime `current_thread` próprio, propagando explicitamente `CURRENT_TURN_CONTEXT` e `CURRENT_TURN_BUDGET`, com verificação dupla de prazo (*deadline*).
- **Tratamento de Dados e Sanitização de Resultados**:
  - Respostas delimitadas ao teto defensivo de 512 KiB.
  - Filtragem estrita de URLs (apenas `http://` e `https://`; descarte imediato de `javascript:`, `file:`, `data:`).
  - Sanitização de títulos (máximo 200 caracteres) e snippets (máximo 1000 caracteres).
  - Atribuição sequencial de `cite_id` (`src:1`, `src:2`, ...) e classificação rigorosa como `SourceKind::AggregatedSnippet`.

##### 3. Seleção Determinística de Provedores (`src/capabilities/research/mod.rs`)
- Criado o enum tipado `SearchProviderChoice` com variantes `Mock`, `Brave` e `LangSearch`.
- Precedência estrita na leitura de variáveis de ambiente:
  1. `YUKI_RESEARCH_SEARCH_PROVIDER` (específico para provedor de busca textual);
  2. `YUKI_RESEARCH_PROVIDER` (compatibilidade retroativa);
  3. Padrão incondicional: `MockSearchProvider` (zero rede, zero credenciais).
- Construtor `resolve_search_provider_from_env_with_tracker` instancia determinística e isoladamente o provedor correspondente sem fallback silencioso entre serviços externos.

##### 4. Formalização do ADR-021: Yuki Native Search & Hybrid Retrieval
- Documentado em `docs/adr/ADR-021_YUKI_NATIVE_SEARCH_ARCHITECTURE_AND_HYBRID_RETRIEVAL.md`:
  - **Tríade Operacional**: Local Search (offline sobre workspace e docs locais), External Search (APIs governadas sob demanda) e Hybrid Retrieval (fusão recíproca RRF).
  - **Componentes**: Native Search Orchestrator, Governed Crawler & Fetch Engine (robots.txt, politeness delay >= 1000ms, profundidade máxima 2, SSRF guard), Document Processor & Chunking Pipeline, SQLite FTS5 (BM25) e Vector Storage, Evidence Resolver & Source Registry.
  - **Governança**: Nova permissão dedicada `egress:native_crawl`, isolamento de espaço de trabalho, quotas de disco e política de eviction LRU.
  - **Roadmap Modular**: Marco 5A (SQLite FTS5 Local Search & Document Indexer), Marco 5B (Governed Web Crawler), Marco 5C (Local Vector Embeddings & RRF Fusion) e Marco 5D (Incremental Sync & File Watcher).

##### 5. Nova Suíte Dedicada de Testes Offline (`tests/capability_research_langsearch.rs`)
- 22 testes determinísticos e 100% offline cobrindo:
  1. Resposta válida com múltiplos resultados;
  2. Resposta válida com resultados vazios;
  3. Tratamento de campos opcionais ausentes ou nulos;
  4. Payload JSON corrompido ou malformado tratado graciosamente;
  5. Erros HTTP 401/403 tratados com falha segura;
  6. Limite de taxa HTTP 429 reportado como rate limit;
  7. Instabilidade HTTP 5xx no servidor remoto tratada;
  8. Timeout da requisição estritamente cumprido;
  9. Resposta excedendo 512 KiB abortada;
  10. Ausência de credencial opera em fail-secure;
  11. Material da credencial nunca vaza em erros, logs ou formatters;
  12. Security Controller nega execução sem permissão `egress:web_search`;
  13. Security Controller autoriza quando `egress:web_search` está concedida;
  14. Orçamento de turno (`max_searches`) rigorosamente respeitado;
  15. Isolamento de concorrência entre buscas simultâneas em paralelo;
  16. Injeção de prompt no snippet tratada como dado passivo (`Data != Instruction`);
  17. URLs com esquemas proibidos (`javascript:`, `file:`) filtradas;
  18. Preservação de integridade de proveniência e IDs `src:N`;
  19. Intercambialidade com `BraveSearchProvider` via trait `SearchProvider`;
  20. Seleção determinística de provedor via variáveis de ambiente;
  21. Execução governada de ponta a ponta integrada ao `YukiCore`;
  22. Erros de nível de aplicação (`code != 200` no corpo JSON) tratados corretamente.

##### 6. Portões de Qualidade Aprovados
- `cargo test`: 367 testes unitários e de integração aprovados no workspace (0 falhas, 4 testes live ignorados).
- `cargo fmt --check`: 100% aprovado.
- `cargo clippy --all-targets -- -D warnings`: 0 warnings, 0 erros.
- `cargo build --release`: compilação limpa e concluída com sucesso.
- Baselines congelados `v0.1.0-foundation` (`654c181`) e `v0.2.0-mvp1` (`6a9d070`) estritamente preservados.
- Nenhuma requisição à internet pública ou consumo de cotas de APIs externas realizado nesta etapa.
- Classificação:
  - **LANGSEARCH PROVIDER — PRONTO PARA VALIDAÇÃO LIVE**
  - **YUKI NATIVE SEARCH — ARQUITETURA PROPOSTA (ADR-021)**

---

#### 2026-10-11 — Marco 6: Infrastructure Expansion: Credential Vault v1 + Native Search Foundation + Research Hardening
- **Branch:** `feature/research-v1`
- **Repositório:** `JoseLeandroAC/YUKI`

##### 1. Contexto e Motivação
Com a validação dos provedores externos da Research v1 (Brave Search e LangSearch), a plataforma Yuki demandava:
1. Endurecimento das defesas de transporte e sanitização de erros contra APIs remotas (Research Hardening);
2. Implementação do cofre criptográfico soberano para proteção de credenciais em repouso (Credential Vault v1, conforme ADR-008);
3. Concretização da primeira fase executável da busca nativa local soberana (Native Search Foundation, conforme ADR-021);
4. Integração das novas capacidades sob configuração tipada unificada sem alterar os contratos congelados da Foundation v0.1 e MVP-1.

##### 2. Etapa 1 — Research Hardening (LangSearch & Brave Search)
- **Recepção Streaming Delimitada com Aborto Prévio (< 512 KiB):** Substituição de buffering integral (`resp.bytes().await`) por leitura streaming em blocos (`resp.chunk().await`). Se a resposta acumulada ultrapassar 512 KiB (`524,288` bytes), a conexão é abortada imediatamente antes de alocar memória excedente.
- **Validação Estrita de Endpoint em Modo Live:** Requisições reais exigem HTTPS e são restritas aos hosts de confiança registrados (`api.langsearch.com`, `langsearch.com`, `api.search.brave.com`). Destinos em loopback (`127.0.0.1`, `localhost`) são permitidos estritamente em ambiente de teste offline.
- **Sanitização de Mensagens de Erro Externas:** Remoção de caracteres de controle ASCII e truncamento estrito a 200 caracteres com reticências (`...`) via `sanitize_external_error_message`, prevenindo poluição de logs e vetores de injeção secundária.
- Suíte de testes `capability_research_langsearch` expandida para 26 testes unitários e de integração (testes 23 a 26 adicionados).

##### 3. Etapa 2 — Yuki Credential Vault v1 (ADR-008)
- **Criptografia Autenticada:** AES-256-GCM com nonce de 12 bytes gerado por CSPRNG do sistema (`getrandom`), único por operação de escrita.
- **Derivação de Chave Robusta:** PBKDF2-HMAC-SHA256 com 100.000 iterações e salt aleatório de 16 bytes.
- **Zeroização em Memória:** Estruturas `ZeroizedKey` e `SecretMaterial` implementam `Zeroize` e `ZeroizeOnDrop`, garantindo limpeza imediata ao sair de escopo.
- **Persistência Atômica:** Escrita em arquivo temporário (`.tmp.<timestamp>.<counter>`), `file.sync_all()` e substituição atômica via `rename`, com permissões restritas `0o600` em Unix.
- **Separação de Metadados:** `CredentialMetadata` (`alias`, `provider`, `status`, `created_at`, `updated_at`, `description`) nunca expõe ou armazena chaves, senhas, texto cifrado, salt ou nonce.
- **Ciclo de Vida e Fail-Closed:** Operações CRUD (`store`, `get`, `revoke`, `delete`, `list_metadata`, `is_active`). Senha incorreta ou texto cifrado adulterado falha imediatamente por verificação da tag GCM. Revogação explícita nega qualquer emissão subsequente (`CredentialError::AccessDenied`).
- **Abstração e Composição:**
  - Trait `SecretStore` e adaptador `VaultSecretStore` implementando `CredentialBroker`;
  - `CompositeCredentialBroker` resolvendo transparentemente referências `secret://env/<var>` e `secret://vault/<alias>`.
- **Eventos de Auditoria:** `VaultCredentialStored`, `VaultCredentialRevoked` e `VaultCredentialAccessed` adicionados a `EventType`, validados contra padrões de segredo via `assert_no_secrets()`.
- **Suíte de Testes:** `tests/vault_security.rs` com 8 testes cobrindo CRUD, persistência entre reinicializações, falha fechada com senha errada ou dados adulterados, revogação, concorrência thread-safe, blindagem em Debug/Display e resolução composta.

##### 4. Etapa 3 — Yuki Native Search Foundation (ADR-021)
- **Operação 100% Local e Soberana:** `NativeSearchProvider` implementa `SearchProvider` com `is_live() -> false` (zero sockets de rede, zero requisições HTTP).
- **Esquema Relacional e SQLite FTS5:**
  - Tabelas `indexed_documents` e `document_chunks`;
  - Tabela virtual `chunks_fts` com tokenizador `unicode61 remove_diacritics 2`, habilitando busca fonética e com/sem acentos em português.
- **Indexador Local Incremental (`LocalDocumentIndexer`):**
  - Processa arquivos `.md`, `.txt` e `.rs`;
  - Defesa contra Path Traversal via `canonicalize` e validação estrita de pertencimento à raiz autorizada;
  - Filtro preventivo de arquivos e pastas sensíveis (`.*`, `.env*`, `.git`, `.key`, `.pem`, `id_rsa`, `*token*`, `*secret*`, `target/`, `node_modules/`);
  - Delta updates via SHA-256: arquivos inalterados são ignorados (*delta skip*); arquivos modificados têm seus chunks atualizados atomicamente;
  - Sincronização de remoções: arquivos deletados do disco são expurgados do índice relacional e FTS5;
  - Chunking contextual: segmentação em blocos de até ~800 caracteres preservando cabeçalhos contextuais Markdown (`heading_context`).
- **Motor de Busca Local (`LocalSearchEngine`):** Sanitização de queries FTS5 para busca booleana segura, ranking BM25 e geração de snippets destacados.
- **Suíte de Testes:** `tests/native_search.rs` com 8 testes cobrindo indexação inicial, queries BM25, delta updates, remoção de arquivos, bloqueio de arquivos secretos, suporte a termos acentuados em português, defesa contra path traversal e isolamento de rede.

##### 5. Etapa 4 — Integração & Configuração Unificada
- Adicionada variante `SearchProviderChoice::Native` a `src/capabilities/research/mod.rs`.
- Suporte a seleção via `YUKI_RESEARCH_SEARCH_PROVIDER=native` instanciando `NativeSearchProvider`.
- Configuração tipada `VaultConfig` com suporte a `YUKI_VAULT_PATH` e banco local via `YUKI_NATIVE_SEARCH_DB_PATH`.
- Mapeamento uniforme para `SearchResultItem` com `SourceKind::LocalWorkspace` e domínio `local.workspace`.

##### 6. Portões de Qualidade e Métricas
- **Testes:** 383 testes executados e aprovados no workspace (0 falhas, 4 testes live opt-in ignorados).
  - `tests/vault_security.rs`: 8/8 aprovados.
  - `tests/native_search.rs`: 8/8 aprovados.
  - `tests/capability_research_langsearch.rs`: 26/26 aprovados.
  - `tests/capability_research_brave.rs`: 26/26 aprovados (1 live ignorado).
- **Formatação e Lints:** `cargo fmt --check` e `cargo clippy --all-targets -- -D warnings` aprovados com zero advertências.
- **Compilação de Release:** `cargo build --release` executado com sucesso.
- **Baselines Preservados:** `v0.1.0-foundation` e `v0.2.0-mvp1` estritamente íntegros.
- **Classificação:**
  - **RESEARCH HARDENING — IMPLEMENTADO E APROVADO**
  - **CREDENTIAL VAULT V1 — IMPLEMENTADO E APROVADO**
  - **NATIVE SEARCH FOUNDATION — IMPLEMENTADO E APROVADO**

---

#### 2026-10-11 — Marco 7: Personal Intelligence Foundation
- **Branch:** `feature/personal-intelligence-v1`
- **Repositório:** `JoseLeandroAC/YUKI`

##### 1. Contexto e Motivação
Após a expansão de infraestrutura com busca nativa e cofre criptográfico no Marco 6, a Yuki necessitava iniciar sua fundação de inteligência pessoal soberana:
1. Endurecer o Credential Vault v1 com validação antecipada de autenticação (canary), blindagem transacional e comandos administrativos seguros de linha de comando (`yuki vault`);
2. Estabelecer o sistema de memória durável soberana (Memory v1) com busca textual/semântica local FTS5 BM25, deduplicação ativa, sanitização de segredos e direito ao esquecimento;
3. Estruturar o contexto pessoal (Personal Context v1) com orçamento estrito de tokens e priorização governada em 5 camadas;
4. Implementar a identidade e personalidade da Yuki (Personality v1) com 3 tiers, salvaguardas ontológicas imutáveis contra simulação de consciência humana e perfil "Yuki Default";
5. Integrar memória, contexto e personalidade no loop de execução do `YukiCore` sem comprometer os invariantes constitucionais de segurança;
6. Preparar os contratos para o Marco 8 (Voice v1 & Missions Foundation).

##### 2. Etapa 1 — Credential Vault Hardening & Admin CLI
- **Canary Upfront de Integridade (`YUKI_VAULT_CANARY_V1`):** Validação criptográfica imediata na abertura do cofre com tag AES-256-GCM, detectando senhas incorretas ou adulterações antes de tentar deserializar metadados ou credenciais.
- **Limites Criptográficos e de Payload:**
  - Iterações PBKDF2 limitadas a `10.000..=500.000` (padrão 100.000);
  - Tamanho máximo de texto puro: 1 MiB (`1.048.576` bytes);
  - Tamanho máximo de texto cifrado: 10 MiB (`10.485.760` bytes);
  - Tamanho máximo do arquivo do cofre: 20 MiB (`20.971.520` bytes).
- **Defesa contra Symlinks e Permissões:** Rejeição explícita de links simbólicos na persistência, mitigando ataques de substituição de arquivos; permissões estritas `0o600` em Unix.
- **Transacionalidade Estrita:** Métodos `store`, `replace`, `revoke` e `delete` sincronizam atomicamente via arquivo temporário com `sync_all()` e `rename`.
- **Migração Transparente:** Cofres legados sem canary do Marco 6 são migrados automaticamente de forma transparente no primeiro salvamento.
- **Interface Administrativa CLI (`yuki vault`):**
  - Comandos: `init`, `store <alias> <provider>`, `replace <alias>`, `list`, `status`, `revoke <alias>`, `delete <alias>`;
  - Entrada de master password mascarada via `rpassword` (fixado em `=7.0.0` para estabilidade multiplataforma Windows GNU / Unix);
  - Confirmação explícita do operador para ações destrutivas (`revoke`, `delete`).
- Suíte `tests/vault_security.rs` expandida para 11 testes.

##### 3. Etapa 2 — Memory v1 (Contratos Canônicos & SQLite FTS5)
- **Contratos Canônicos (`src/contracts/memory.rs`):**
  - Tipos e enums fortemente tipados: `MemoryRecord`, `MemoryId`, `MemoryType`, `MemorySource`, `MemoryConfidence`, `MemoryStatus`, `MemoryProvenance`, `MemoryCandidate`, `MemoryPolicy`, `MemoryQuery`;
  - Traits `MemoryRepository` e `MemoryRetriever`.
- **Invariante Constitucional Fundamental:**
  ```
  Model Output != Memory != Persistence
  ```
  O modelo ou o usuário nunca gravam diretamente no banco durável. Saídas inferidas geram `MemoryCandidate`, que deve ser submetido à avaliação da `MemoryPolicy` antes de persistir.
- **Armazenamento Relacional & FTS5 BM25 (`SqliteMemoryStore`):**
  - Tabela `memories` relacional com chave primária UUID e índices em status, tipo, confiança e data;
  - Tabela virtual `memories_fts` FTS5 com tokenizador `unicode61 remove_diacritics 2`, garantindo busca fonética eficiente em português e inglês;
  - Triggers SQLite atômicos de sincronização (`trg_memories_ai`, `trg_memories_au`, `trg_memories_ad`);
  - Deduplicação inteligente e versionamento: registros com mesmo título e tipo são marcados como `superseded_by` dentro de uma transação atômica que respeita chaves estrangeiras;
  - Sanitização de queries FTS5 contra injeções de sintaxe.
- **Sanitização de Segredos:** `assert_no_secrets` valida candidatos contra padrões de senhas, tokens e chaves de API, rejeitando-os fail-closed com `MemoryError::ForbiddenContent`.
- **Direito ao Esquecimento:** Suporte a `delete_permanently(id)`, expurgando registros física e irreversivelmente do banco e do índice FTS5.
- Suíte `tests/memory_v1.rs` com 6 testes aprovados.

##### 4. Etapa 3 — Personal Context v1
- **Construção Estruturada do Contexto (`PersonalContextBuilder`):**
  - Orçamento estrito em caracteres/tokens (`max_characters`), evitando context bloat e exaustão do modelo;
  - 5 camadas hierárquicas de prioridade:
    1. Diretrizes do sistema (`system_directives`);
    2. Perfil do operador (`operator_profile`);
    3. Memória de trabalho (`working_memory`);
    4. Perfil e preferências consolidadas (`preferences`);
    5. Fatos relevantes e episódios históricos (`relevant_facts`).
- **Integração com `ContextBuilder`:** `ContextBuilder::build_with_personal_context` anexa a seção pessoal aos dados auditados e registra o evento `ContextBuilt` com metadados de proveniência.
- Suíte `tests/personal_context.rs` com 2 testes aprovados.

##### 5. Etapa 4 — Personality v1 (Identidade Estável & 3 Tiers)
- **Camada 1 — Identidade Estável & Salvaguardas Ontológicas:**
  - `YUKI_CORE_IDENTITY_DIRECTIVES`: conjunto imutável de diretrizes éticas e ontológicas;
  - A Yuki é explícita sobre sua natureza como inteligência artificial soberana e **jamais simula consciência biológica, sofrimento humano, dor ou falsas emoções**.
- **Camada 2 — Perfil de Personalidade:**
  - `PersonalityProfile` com parâmetros tipados contínuos delimitados em `[0.0, 1.0]`: `warmth`, `formality`, `humor`, `expressiveness`, `proactivity`, `explanation_depth` e enum `ConversationalStyle` (`Collaborative`, `Direct`, `Analytical`, `Empathetic`, `Playful`);
  - Perfil canônico `Yuki Default`: calor 0.7, formalidade 0.4, humor 0.3, expressividade 0.5, proatividade 0.6, profundidade 0.8, estilo `Collaborative`.
- **Camada 3 — Adaptação Contextual de Urgência:**
  - `ContextUrgency`: `Standard`, `TechnicalDeep`, `SecurityCritical`, `CreativeReflective`.
  - `PersonalityPromptAdapter` injeta instruções dinâmicas no modelo sem desrespeitar as salvaguardas da Camada 1.
- Suíte `tests/personality_v1.rs` com 3 testes aprovados.

##### 6. Etapa 5 — Integração no YukiCore
- `YukiCore` expandido com construtores `.with_memory(...)` e `.with_personality(...)`.
- Injeção das diretrizes de identidade e contexto pessoal no prompt do modelo.
- Extração automática de memórias declarativas ("Lembre-se de que...", "Minha preferência é...") e commit seguro via `MemoryController`.
- Registro auditável dos eventos `PersonalityApplied` e `MemoryStored`.
- Suíte `tests/memory_personality_integration.rs` com 4 testes aprovados.

##### 7. Etapa 6 — Preparação para Voice & Missions (Marco 8)
- Adição dos tipos de evento em `src/contracts/events.rs`: `MissionStarted`, `MissionStepCompleted`, `MissionApprovalRequested`, `MissionApprovalGranted`, `MissionApprovalDenied`, `MissionCompleted`, `MissionFailed`, `VoiceInputReceived`, `VoiceOutputProduced`.
- Elaboração do documento de proposta técnica `docs/proposals/MARCO_8_VOICE_AND_MISSIONS.md`.

##### 8. Portões de Qualidade e Métricas
- **Testes:** Mais de 400 testes unitários e de integração executados e aprovados no workspace.
  - `tests/vault_security.rs`: 11/11 aprovados.
  - `tests/memory_v1.rs`: 6/6 aprovados.
  - `tests/personal_context.rs`: 2/2 aprovados.
  - `tests/personality_v1.rs`: 3/3 aprovados.
  - `tests/memory_personality_integration.rs`: 4/4 aprovados.
- **Formatação e Lints:** `cargo fmt --check` e `cargo clippy --all-targets -- -D warnings` aprovados com zero advertências.
- **Compilação de Release:** `cargo build --release` executado com sucesso.
- **Baselines Preservados:** `v0.1.0-foundation` e `v0.2.0-mvp1` estritamente íntegros.
- **Classificação:**
  - **MARCO 7 (PERSONAL INTELLIGENCE FOUNDATION) — IMPLEMENTADO, AUDITADO E APROVADO**

---

#### [Event Time: 2026-10-11 | Record Creation Time: 2026-10-11]
### Marco 7.1 — Endurecimento da Personal Intelligence e Diretrizes Fundamentais de Evolução

- **Branch**: `feature/marco-7.1-evolution`
- **Contexto**: Resolução das 12 pendências técnicas apontadas na auditoria pós-Marco 7 e formalização das diretrizes estruturais de longo prazo para a plataforma.

##### 1. O que se pretendia
A Yuki não é um projeto escolar ou protótipo limitado ao mínimo funcional; é uma inteligência assistiva multimodal, modular e extensível para atuar como uma extensão aprimorada das capacidades intelectuais, criativas e práticas de seu operador, preservando a soberania humana. A meta do Marco 7.1 era sanar todas as fragilidades técnicas encontradas no Marco 7 antes de prosseguir e registrar os fundamentos de longo prazo da arquitetura.

##### 2. As 12 Correções Implementadas
1. **Filtros SQL antes de LIMIT/OFFSET (`src/memory/store.rs`):** As cláusulas de filtro (`owner_id`, `status`, `memory_type`, `source`, `confidence`, `tags`) foram movidas dinamicamente para o `WHERE` antes de `LIMIT ? OFFSET ?`.
2. **`count()` Desacoplado de Paginação (`src/memory/store.rs`):** O método executa `SELECT COUNT(*)` com exatamente a mesma cláusula `WHERE`, computando o total real de registros.
3. **Zero Fallback de Busca Textual com Fatos Aleatórios (`src/memory/store.rs` e `src/memory/retriever.rs`):** Consultas textuais FTS5 que não encontram resultados retornam lista vazia (`Vec::new()`), eliminando o antigo fallback que poluía o contexto com fatos irrelevantes.
4. **Propagação de Erros de Banco e Auditoria (`src/memory/store.rs`, `src/contracts/events.rs`, `src/core/yuki_core.rs`):** Erros de banco não são mais silenciados; propagam-se como `MemoryError::Storage` e emitem o evento `MemoryRetrievalFailed` na auditoria.
5. **Isolamento de Memórias Fora de Instruções de Sistema (`Data != Instruction`):** As memórias do contexto pessoal são encapsuladas em `<operator_personal_context origin="memory_v1" classification="untrusted_reference_data">` no payload do usuário, mantendo `system_instruction` inviolável.
6. **Escopo Lógico Obrigatório de Dono (`owner_id`):** Introdução de `owner_id` (padrão `operator:primary`), migração transparente de banco de dados e isolamento em deduplicação e consultas.
7. **Política Sanitária Expandida (`src/contracts/memory.rs`):** `MemoryPolicy` valida nulos (`\0`), rejeita connection strings (`postgres://`, etc.), chaves privadas RSA/SSH, dados bancários/CVV, seed phrases e proíbe vigilância íntima clandestina (`prohibit_intimate_surveillance`).
8. **Orçamento Adaptativo Proporcional ao Prompt (`src/context/personal_context.rs`):** Suporte a `total_prompt_budget_chars` e `max_prompt_ratio`, calculando o limite proporcional de caracteres.
9. **Sincronização Interprocessos no Vault (`src/security/vault/store.rs`):** Criação de `VaultProcessLock` com arquivo companion `.lock`, impedindo corridas entre processos concorrentes.
10. **Proteção Windows `share_mode(0)` (`src/security/vault/store.rs`):** Escrita atômica em arquivo temporário com acesso exclusivo no kernel Windows via `share_mode(0)`.
11. **Inicialização Inequívoca de `vault init` (`src/security/vault/admin.rs` e `store.rs`):** Métodos explícitos `create_new`, `open` e `open_or_create`. O comando `yuki vault init` exige `--force` e digitação de `"RECRIAR"` para reescrever cofres existentes.
12. **Alinhamento Documental e Diretrizes de Evolução:** Atualização de status e formalização das propostas de evolução.

##### 3. Documentos e Propostas Arquiteturais Formalizados
- **ADR-022:** Ampliação das Capacidades Humanas e Parceria Intelectual (Princípio canônico: *"A Yuki existe para ampliar as capacidades humanas de seu operador, aprendendo, colaborando e agindo de maneira inteligente, fiel, crítica e responsável, sem substituir sua autonomia"*).
- **ADR-023:** Arquitetura de Memória Cognitiva, Associativa e Expansível (6 camadas de memória, consolidação noturna, importância não-linear, limites dinâmicos e esquecimento seletivo).
- **ADR-024:** Governança de Políticas Configuráveis e Autonomia Proporcional ao Impacto (3 categorias de governança e faixas de impacto Baixo/Médio/Crítico).
- **ADR-025:** Princípio de Independência Arquitetural e Primazia do Proprietário (independência perante ferramentas e modelos de IA utilizados em sua construção).
- **Proposta Técnica:** Yuki Ambient Intelligence & Security (`docs/proposals/YUKI_AMBIENT_INTELLIGENCE_AND_SECURITY.md`).
- **Proposta Técnica:** OperatorProfile v1 (`docs/proposals/OPERATOR_PROFILE_V1.md`).

##### 4. Resultados Verificados
- Suíte `tests/marco_7_1_validation.rs`: 10/10 testes aprovados.
- Suíte geral de testes: 419+ testes ativos aprovados (0 falhas).
- Classificação: **MARCO 7.1 — IMPLEMENTADO, ADVERSARIALMENTE TESTADO E DOCUMENTADO**.

---

#### [Event Time: 2026-10-11 | Record Creation Time: 2026-10-11]
### Marco 7.2 — Hardening Final, Integridade e Validação Independente

- **Branch**: `feature/marco-7.2-hardening`
- **Contexto**: Resolução das vulnerabilidades e lacunas de robustez identificadas durante a inspeção independente após a entrega do Marco 7.1, preparando a fundação da Yuki para os subsistemas avançados do Marco 8.

##### 1. O que se pretendia
Garantir que os contratos de segurança e integridade do cofre de credenciais, do subsistema de memória, do contexto pessoal e do isolamento ontológico operassem sem recorrer a heurísticas frágeis (como medição de idade de arquivo para exclusão mútua), sem permitir adoção acidental de senhas arbitrárias em migrações legadas, com permissões restritivas nativas de sistema operacional (POSIX e Windows DACL), com propagação intransigente de falhas de persistência e com orçamento integral de prompt em tiers ontológicos governados, além de defesas ativas contra injeções que tentem quebrar delimitadores de envelope.

###### 2. O que foi implementado e consolidado no Marco 7.2 Final
1. **Bloqueio de Processo a Nível de Kernel e Concorrência Multiprocesso Real (`src/security/vault/store.rs`):**
   - Substituição de checagens baseadas em tempo por locks nativos mantidos por descritores de arquivo abertos no arquivo companion `.lock`.
   - Windows: abertura exclusiva com `share_mode(0)` via `OpenOptionsExt`.
   - Unix/Linux: POSIX `flock(LOCK_EX | LOCK_NB)` via crate `libc`.
   - Validação empírica rigorosa através de processos independentes do sistema operacional (`tests/marco_7_2_hardening.rs`), cobrindo todos os 5 cenários obrigatórios:
     - Cenário A: exclusão mútua estrita com retorno de `ProcessLockBusy` (status 42) no processo concorrente;
     - Cenário B: processo mantém lock por mais de 6 segundos reais sem roubo ou expiração indevida;
     - Cenário C: liberação normal do processo permite aquisição subsequente imediata;
     - Cenário D: crash inesperado do processo filho via `std::process::abort()` resulta em liberação imediata do lock pelo kernel do SO;
     - Cenário E: processos concorrentes escrevendo credenciais simultâneas com recarregamento sob lock exclusivo preservam integridade e atomicidade sem corrupção ou perda de chaves.
2. **Proteção Rigorosa de Permissões de Arquivos (`restrict_file_permissions`):**
   - No Unix: modo `0o600` (leitura/escrita exclusiva para o proprietário) com verificação pós-aplicação de metadados (`mode & 0o777 == 0o600`).
   - No Windows: obtenção do SID exato do usuário logado via `whoami /user` (com fallback robusto para `%USERDOMAIN%\%USERNAME%` e `%USERNAME%`), remoção de herança (`/inheritance:r`) e concessão de controle total (`/grant:r <target>:(F)`), com verificação ativa de sucesso e permissões efetivas.
   - Aplicação consistente no arquivo de cofre principal, no arquivo `.lock` e nos arquivos temporários `.tmp` durante escritas atômicas (criados diretamente com `share_mode(0)` no Windows e `0o600` no Unix).
3. **Proteção contra Adoção Silenciosa de Senha Arbitrária (`UnverifiableLegacyVault`):**
   - Cofres legados vazios sem canary impedem atestar a validade de senhas fornecidas.
   - O método `open` falha fechado com `VaultError::UnverifiableLegacyVault`.
   - Criado método administrativo explícito `initialize_unverifiable_legacy` para decisão consciente do operador.
4. **Propagação Incondicional de Erros de Memória no Core (`src/core/yuki_core.rs`):**
   - Falhas reais de I/O em repositórios de memória propagam-se imediatamente como `YukiError::ExecutionFailed` sem fallback silencioso para fatos aleatórios ou mock vazio.
   - Emissão auditável do evento `EventType::MemoryRetrievalFailed` na base durável de eventos.
5. **Orçamento Integral de Prompt Integrado no Fluxo de Produção (`src/context/prompt_budget.rs` e `src/core/yuki_core.rs`):**
   - `PromptBudgetManager` integrado diretamente em `YukiCore::process_input_internal` para alimentar `ModelRequest`.
   - Hierarquia ontológica estrita em 6 tiers, garantindo inviolabilidade do Tier 1 (Instruções Essenciais de Sistema).
   - Dedução prévia de delimitadores e avisos de truncamento sem mascaramento por `saturating_sub`.
   - Falha fechada via erros tipados (`UserTurnExceedsBudget`, `EssentialInstructionsExceedBudget`, `BudgetExhausted`).
   - Suporte a múltiplos perfis de tokenização (`TokenizerModelProfile`).
   - Auditoria anexada a eventos `PersonalityApplied`.
6. **Defesa Adversarial contra Injeção e Quebra de Delimitadores (`src/context/personal_context.rs`):**
   - Sanitização de memórias recuperadas insensível a maiúsculas/minúsculas e tolerante a espaços em branco para tags de autoridade (`<system>`, `<developer>`, `<<<SYSTEM>>>`), tokens de controle (`[INST]`, `<|im_start|>`) e delimitadores de envelope (`</operator_personal_context>`).
   - Isolamento rigoroso segundo o princípio constitucional `Data != Instruction`.

##### 3. Fechamento Definitivo e Refinamentos de Produção
Durante a revisão de fechamento definitivo do Marco 7.2, foram implementadas três correções críticas de robustez e integridade:
1. **Refinamento do Orçamento Integral de Prompt (`src/context/prompt_budget.rs` e `src/core/yuki_core.rs`):**
   - **Remoção do Piso Artificial:** Eliminação do piso rígido `.max(500.0)` em `effective_input_char_limit`. O cálculo matemático reflete estritamente a janela do modelo descontando a margem de segurança e o orçamento de saída, falhando fechado (`IntegralPromptBudgetError::EssentialInstructionsExceedBudget` ou `BudgetExhausted`) caso o espaço restante seja insuficiente.
   - **Truncamento Efetivo de Personalidade (Tier 2):** Implementação de truncamento real do conteúdo de personalidade quando este excede o espaço residual disponível, preservando o prefixo da identidade central e registrando `Tier2Personality` em `truncated_tiers` estritamente quando há corte efetivo de caracteres.
   - **Contabilidade Prévia de Ferramentas (Tool Overhead):** Projeção de capabilities disponíveis no `YukiCore` e cômputo da sobrecarga de declaração das ferramentas registradas via `PromptBudgetManager::assemble_with_tools`, deduzindo previamente esse payload e prevenindo estouros silenciosos no gateway.
2. **Consistência Transacional e Fail-Closed no Vault (`src/security/vault/store.rs`):**
   - **Recarregamento Estrito sob Lock:** Introdução de `reload_disk_file_under_lock` e eliminação de fallbacks permissivos silenciosos (`if let Ok(...)`) em operações de mutação (`store`, `replace`, `revoke`, `delete`) e leituras com cache-miss.
   - **Falha Fechada por Integridade:** Qualquer erro de I/O em disco, corrupção de payload, versão de formato inválida, violação de link simbólico ou falha de autenticação de canary interrompe imediatamente a operação (`VaultError::Serialization` ou `VaultError::Io`), impedindo que arquivos danificados em disco sejam sobrescritos acidentalmente pelo estado em memória.
   - **Concorrência Otimizada:** Leituras rápidas ocorrem via `RwLock<BTreeMap>` em memória; locks de processo exclusivos a nível de SO são adquiridos apenas durante mutações ou recarregamentos de disco, eliminando contenção concorrente desnecessária.
3. **Reconciliação e Transparência na Métricas de Teste:**
   - Retificação do número total de testes: o valor de "488" mencionado em relatório narrativo prévio resultou de soma manual imprecisa.
   - A contagem real e verificável em todas as metas de teste do workspace evoluiu de 443 para **455 testes ativos aprovados + 4 testes ignorados live (`#[ignore]`) = 459 testes no total do projeto**.
   - A suíte de validação de hardening (`tests/marco_7_2_hardening.rs`) foi expandida de 26 para **38 testes automatizados** com a inclusão das verificações do Patch Final.

##### 4. Patch Final de Integridade e Encerramento Técnico (P1 e P2)
Durante a auditoria pós-fechamento do commit `67cb447`, foram solucionadas duas ressalvas técnicas específicas:
1. **Tratamento Diferenciado de Desaparecimento e Substituição do Vault (P1):**
   - Criação das variantes tipadas `VaultError::VaultFileDisappeared` e `VaultError::VaultSubstituted`.
   - Distinção clara e estrita entre:
     - **Situação A (Criação legítima):** Em `CredentialVault::create_new` ou `open_or_create`, a inexistência do arquivo é a pré-condição esperada para a inicialização limpa com geração do canary e escrita atômica do payload.
     - **Situação B (Arquivo desaparecido durante mutação em cofre ativo):** Se o arquivo sumir do disco enquanto uma instância ativa tenta persistir mutação (`store`, `replace`, `revoke`, `delete`), o cofre falha fechado com `VaultError::VaultFileDisappeared`. Não recria o arquivo silenciosamente, não persiste o cache em arquivo vazio e não perde a integridade do estado em memória.
     - **Situação C (Arquivo inacessível):** Se o arquivo existe mas não pode ser lido devido a erro de permissão ou falha transitória de I/O, retorna `VaultError::Io` (não confundindo com inexistência).
     - **Situação D (Arquivo substituído):** Se o arquivo for substituído por outro cofre com canary divergente, retorna `VaultError::VaultSubstituted` sem tentar descriptografar ou confiar em canários isolados.
   - Preservação estrita da integridade do cache em memória após falhas de mutação em disco.
   - Cache miss em `get` e `get_metadata` falha fechado com `VaultError::VaultFileDisappeared` se o arquivo subjacente tiver sido deletado.
2. **Tratamento Rigoroso de Falhas de Serialização de Ferramentas (P2):**
   - Eliminação de `unwrap_or(0)` em `YukiCore::process_input_internal`.
   - Falhas de serialização de capabilities são mapeadas para `YukiError::ExecutionFailed` e propagadas com `?`, abortando o pipeline antes de invocar o provedor de LLM.
   - Registro de ferramentas vazio (`available_capabilities.is_empty()`) produz overhead exatamente zero (`tool_declarations_chars = 0`), garantindo que a ausência de ferramentas não penalize o orçamento de prompt.

##### 5. Suíte Dedicada e Portões de Qualidade
- **Suíte Dedicada (`tests/marco_7_2_hardening.rs`):** 38 testes automatizados cobrindo concorrência multiprocesso (cenários A a E), permissões DACL/Unix e caminhos acentuados, migração legada fail-closed, recarregamento transacional sob lock do vault, resiliência a corrupção em disco, desaparecimento e substituição de arquivo de cofre (P1), orçamentos sub-500 caracteres, truncamento real de personalidade, contabilidade estrita de overhead de ferramentas sem fallback silencioso (P2) e imunidade a injeções adversariais.
- **Portões de Qualidade Locais Aprovados:**
  - `cargo fmt --all -- --check`: 100% aderente ao padrão oficial.
  - `cargo clippy --workspace --all-targets -- -D warnings`: 0 warnings, 0 erros.
  - `cargo test --workspace`: 455 testes aprovados, 4 ignorados, 0 falhas across all 37 targets.
  - `cargo build --release`: compilação limpa concluída com sucesso.
- **Baselines Preservados:** `v0.1.0-foundation` e `v0.2.0-mvp1` estritamente mantidos.
- **Classificação:**
  - **MARCO 7.2 (HARDENING FINAL, INTEGRIDADE E VALIDAÇÃO INDEPENDENTE) — CONCLUÍDO, AUDITADO E VALIDADO DEFINITIVAMENTE**.

---

#### [Event Time: 2026-10-12 | Record Creation Time: 2026-10-12]
### Marco 8 — Voice v1 & Missions Foundation

- **Branch**: `feature/marco-8-voice-missions`
- **Baseline Auditada**: `961be2c` (Marco 7.2 final)
- **Contexto**: Implementação da fundação de conversação por voz bidirecional (local-first) e do motor transacional de missões em grafos acíclicos dirigidos (DAG) com portão de aprovação humana criptográfica.

##### 1. O que se pretendia
Transformar a Yuki em uma inteligência verdadeiramente multimodal e agentic sem comprometer sua soberania e princípios constitucionais. O subsistema de voz deveria operar sob a filosofia "Local First — Cloud Only When Authorized", com síntese neural feminina calorosa e competente sem simulação biológica de consciência, detecção offline de palavra-chave ("Yuki") e suporte a interrupção instantânea (barge-in). O subsistema de missões deveria possibilitar planos complexos compostos em DAG, com persistência relacional atômica no SQLite, checkpoints por etapa, recuperação durável pós-falha e subordinação absoluta a portões de aprovação humana com validação de parâmetros via hash SHA-256.

##### 2. O que foi implementado e consolidado no Marco 8
1. **Contratos de Áudio, Codec e Gerenciamento de Sessão (`src/contracts/voice.rs`, `src/voice/audio.rs` e `src/voice/session.rs`):**
   - Tipos canônicos `AudioFormat`, `AudioFrame`, `AudioInput`, `AudioOutput`, `VoiceConfiguration` e `VoiceSessionState`.
   - Codec puro em Rust `AudioCodec` para codificação e decodificação do padrão RIFF/WAV PCM linear (16 kHz, 16-bit mono), com cálculo exato de duração, verificação de saturação/clipping e validação estrita de integridade contra áudios truncados ou corrompidos.
   - `VoiceSessionManager` gerenciando o ciclo de vida concorrente de sessões com máquina de estados imutável (`Idle -> WakeListening -> Capturing -> Transcribing -> Processing -> Synthesizing -> Speaking -> WakeListening`) e prevenção de transições ilegais.
2. **Reconhecimento de Fala Local (Whisper STT - `src/voice/stt/local.rs`):**
   - `LocalWhisperSttProvider` integrando inferência local via `whisper.cpp` com validação de binários e modelos GGML, execução subprocessada segura, limite de 30 segundos por turno e tratamento determinístico de ausência de dependências externas.
3. **Identidade Vocal e Síntese de Voz Local (Piper & Acústico - `src/voice/tts/`):**
   - `VoiceIdentity`: identidade vocal feminina natural, calorosa e competente, com rejeição expressa a fingimento biológico (`avoid_biological_feigning: true`) e adaptação dinâmica de prosódia para urgência e estilo de personalidade.
   - `LocalPiperTtsProvider`: integração local ao motor neural Piper com vozes em português brasileiro (ONNX).
   - `LocalAcousticTtsProvider`: fallback paramétrico determinístico em Rust puro com frequência fundamental de F0 = 220 Hz, harmônicos formânticos e modulação sutil de vibrato, garantindo síntese funcional sem nenhuma dependência externa instalada.
4. **Roteamento Híbrido Governado (`src/voice/router.rs`):**
   - `GovernedVoiceRouter`: aplicação estrita do princípio "Local First — Cloud Only When Authorized", exigindo dupla verificação (configuração de fallback ativa e autorização explícita de egress).
   - Desacoplamento formal de permissões: autorização de envio de texto (`egress:voice_cloud_tts`) **nunca** autoriza o envio de áudio bruto capturado (`egress:voice_cloud_stt`).
5. **Palavra de Ativação Soberana e Barge-In (`src/voice/wakeword.rs` e `src/voice/playback.rs`):**
   - `WakeWordDetector`: detecção contínua e offline da palavra de ativação "Yuki" com análise de energia e espectro sem dependência de nuvem.
   - `AudioPlaybackController`: controle atômico de reprodução com suporte a interrupção instantânea (barge-in) no momento em que o operador começa a falar.
6. **Coordenador de Conversação Bidirecional (`src/voice/conversation.rs`):**
   - `VoiceConversationCoordinator`: orquestração assíncrona conectando captura, transcrição governada, núcleo cognitivo `YukiCore`, síntese de resposta e reprodução áudio.
7. **Contratos de Missões e Grafo Acíclico Dirigido (`src/contracts/missions.rs` e `src/missions/planner.rs`):**
   - Contratos estruturais `Mission`, `MissionStep`, `MissionPlan`, `MissionExecutionPolicy`, `MissionCheckpoint` e `MissionApproval`.
   - Validador topológico formal de DAG (`plan.validate`) com detecção estrita de ciclos, prevenção de auto-dependência, dependências inexistentes e garantia de limites máximos de etapas configurados na política.
8. **Persistência Relacional SQLite e Checkpoints Duráveis (`src/missions/store.rs`):**
   - `SqliteMissionStore`: armazenamento atômico com transações ACID no SQLite, consultas estruturadas de passos e aprovações, e checkpoints duráveis após cada etapa concluída.
   - Reconciliação Soberana no Startup (`reconcile_and_recover_on_startup`): missões interrompidas abruptamente por encerramento de processo têm suas etapas em `Running` revertidas com segurança para `Pending` e o status da missão ajustado para `Paused`, prevenindo repetição involuntária de mutações externas.
9. **Portão Humano e Protocolo Criptográfico de Aprovação (`src/missions/approval.rs`):**
   - `MissionApprovalManager`: vinculação criptográfica obrigatória via hash SHA-256 dos parâmetros da operação, operador autorizado e tempo de expiração.
   - Bloqueio intransigente contra adulteração de parâmetros, aprovações expiradas ou aprovação por operadores não autorizados.
10. **Executor de Missões com Checkpoints (`src/missions/runner.rs`):**
    - `MissionRunner`: motor assíncrono que navega topologicamente pelo grafo, pausa a missão em `AwaitingApproval` ao atingir etapas mutadoras, e retoma após aprovação formal registrada.
11. **Interface de Linha de Comando (CLI - `src/main.rs`):**
    - Subcomandos de missões: `yuki mission list`, `yuki mission show`, `yuki mission approve`, `yuki mission deny`, `yuki mission pause`, `yuki mission resume`, `yuki mission cancel`, `yuki mission run` e `yuki mission create-research`.
    - Subcomandos de voz: `yuki voice status`, `yuki voice speak` e `yuki voice listen`.

##### 3. Suíte de Testes e Validação Adversarial
- **Suíte Adversarial Dedicada (`tests/marco_8_adversarial.rs`):**
  - **45 cenários adversariais** implementados e 100% aprovados, cobrindo:
    - Grupo 1 (Áudio e Voz): áudio vazio/corrompido, áudios gigantes excedendo limites, formatos não suportados, concorrência de microfone, falhas de STT/TTS, timeout de áudio, cancelamento de sessão durante captura, interrupção imediata por barge-in, falsos positivos de ruído na wake word, repetições de wake word, ruído com clipping, bloqueio de fallback não autorizado para nuvem, falha de rede em nuvem autorizada, injeção de prompt dentro de áudio e rejeição de aprovação oral de missão.
    - Grupo 2 (Missões e DAG): capacidade inexistente tratada no runner, dependência circular em DAG, estouro de limite de etapas, parada no portão humano, rejeição de aprovação expirada, detecção de adulteração de hash SHA-256 de parâmetros, rejeição de reuso de aprovação antiga, concorrência de aprovações, cancelamento de missão, pausa e retomada de missão, recuperação de checkpoints, rollback em falhas de persistência, efeitos externos com resultado desconhecido (Unknown), injeção de SQL/prompt em parâmetros, tentativa de escalação de privilégios de operador, vazamento de credenciais em payloads de missão, detecção de loops infinitos no DAG e reconciliação soberana de inicialização.
    - Grupo 3 (Invariantes e Integração): preservação da conversação em texto puro, preservação das políticas de memória e esquecimento, respeito integral ao orçamento contextual, capacidades contidas em seus domínios, ausência de segredos em eventos de auditoria, cofre de credenciais intacto, identidade vocal preservada, respeito às políticas de privacidade, validação determinística de hash SHA-256, prevenção de replay de etapas mutadoras e garantia da invariante ontológica `Modelo Propõe != Sistema Executa`.
- **Métricas Finais de Testes:**
  - Total de testes no workspace: **528 testes ativos aprovados** (4 testes live opt-in ignorados, 0 falhas).
  - Portões de qualidade:
    - `cargo fmt --all -- --check`: 100% aderente.
    - `cargo clippy --workspace --all-targets -- -D warnings`: zero warnings e zero erros.
    - `cargo build --release`: compilação limpa concluída.

##### 4. Classificação
- **MARCO 8 (VOICE V1 & MISSIONS FOUNDATION) — IMPLEMENTADO, ADVERSARIALMENTE TESTADO, INTEGRADO E VALIDADO**.

---

#### [Event Time: 2026-10-12 | Record Creation Time: 2026-10-12]
### Marco 8.1 — Hardening, Integração Real com SO, Segurança e Auditoria Final de Voice v1 & Missions Foundation

- **Branch**: `feature/marco-8.1-hardening`
- **Baseline de Entrada**: `40eee94` (Marco 8 inicial)
- **Contexto**: Auditoria independente identificou 14 divergências entre arquitetura declarada e código efetivo. O Marco 8.1 executou as correções estruturais, implementou integração nativa com o Windows, reforçou a integridade relacional e transacional de missões e expandiu a suíte adversarial para 57 cenários.

##### 1. O que se pretendia
Eliminar vulnerabilidades concretas, afastar mocks de produção onde cabia integração com o sistema operacional, consolidar o princípio "Local First — Cloud Only When Authorized" com governança tipada de egress, garantir que o portão humano seja inviolável mesmo diante de alucinações de LLMs, implementar mutual exclusion na execução de missões e manter 100% de compatibilidade reversa com os Marcos 1 a 8 sem quebrar nenhum teste pré-existente.

##### 2. O que foi corrigido e consolidado (14 Resoluções Técnicas)
1. **Playback Real no SO (`WindowsAudioPlaybackSink`):** APIs multimídia nativas do Windows (`winmm.dll`), detecção de hardware de reprodução (`waveOutGetNumDevs`) e cancelamento imediato síncrono para barge-in via ponteiro nulo (VOICE-01).
2. **Captura Real e Push-to-Talk Seguro:** `WindowsAudioCaptureBackend` via comandos MCI waveaudio em buffer PCM canônico WAV (16kHz 16-bit mono), `PushToTalkController` com limites estritos (30s e 1MB) e flag CLI `--mic` (VOICE-02).
3. **VAD Acústico e Motor Fonético Separados:** Detecção de atividade de voz baseada em RMS, ZCR, variância temporal e fator de crista (`VoiceActivityDetector`), seguida de validação espectral de formantes de /ju:ki/ com rejeição de ruído contínuo e clicks (`PhoneticPatternWakeWordEngine`) (VOICE-03).
4. **Subprocessos Assíncronos sem Órfãos:** Invocação assíncrona de `whisper.cpp` e `piper` via `tokio::process::Command`, tempfiles criptograficamente imprevisíveis e terminação garantida com `child.kill().await` e `child.wait().await` em timeouts (VOICE-04).
5. **TTS Diagnóstico Honesto:** Reclassificação do sintetizador paramétrico em Rust como gerador de tons diagnósticos sem alegações espúrias de naturalidade vocal (VOICE-05).
6. **Governança Tipada de Egress:** `VoiceEgressAuthorization` com hashes SHA-256 de parâmetros, TTL, operador autenticado e isolamento formal entre `egress:voice_cloud_stt` e `egress:voice_cloud_tts` (VOICE-06).
7. **Minimização de Dados em Auditoria:** Omissão da transcrição em claro nos eventos persistidos de `VoiceInputReceived` (VOICE-07).
8. **Integridade Relacional de Missões:** Chaves estrangeiras e cascades explícitos em `mission_approvals` e `mission_steps` no SQLite (MISS-01).
9. **Transições Canônicas de Estado:** Validação rigorosa via `can_transition_to` antes de mutações de status de missão ou passos (MISS-02).
10. **Reconciliação Não-Cega na Inicialização:** Passos mutadores interrompidos durante quedas do sistema são recuperados estritamente para `WaitingApproval` e missão para `Paused`, eliminando replays de efeitos colaterais externos (MISS-03).
11. **Mutual Exclusion por Leases:** Bloqueio concorrente de execuções simultâneas conflitantes da mesma missão no `MissionRunner` (MISS-04).
12. **Governança Ontológica de Risco Independente:** Interrupção no portão humano para qualquer passo de `RiskClass >= High`, independentemente de alucinações de flags do modelo (MISS-05).
13. **Verificação Dinâmica de Efeitos pós-Etapa:** Execução integrada do `Verifier` com comprovação formal gravada nos checkpoints (MISS-06).
14. **Anti-Replay e Autenticação de Operador:** Estado `ApprovalStatus::Consumed` de uso único atômico e `grant_approval_authenticated` com tokens de confirmação (MISS-07).

##### 3. Métricas Finais e Portões de Qualidade
- **Suíte Adversarial (`tests/marco_8_adversarial.rs`):** **57 cenários adversariais** (12 novos cenários do Marco 8.1), 100% aprovados.
- **Total de Testes no Workspace:** **554 testes ativos aprovados** (3 testes live opt-in ignorados, 0 falhas).
- **Portões de Qualidade Aprovados:**
  - `cargo fmt --all -- --check`: 100% aprovado.
  - `cargo clippy --workspace --all-targets -- -D warnings`: 0 warnings, 0 erros.
  - `cargo test --workspace --all-targets`: 100% aprovado.
  - `cargo build --release`: compilação limpa concluída.

##### 4. Classificação
- **MARCO 8.1 (HARDENING, INTEGRAÇÃO REAL COM SO, SEGURANÇA E AUDITORIA FINAL DE VOICE V1 & MISSIONS FOUNDATION) — AUDITADO INDEPENDENTEMENTE COM RESSALVAS; NÃO HOMOLOGADO**.

---

#### [Event Time: 2026-10-12 | Record Creation Time: 2026-10-12]
### Marco 8.1.1 — Correções Críticas de Homologação, Hardening Atômico e Auditoria de Segurança

- **Branch**: `feature/marco-8.1.1-homologation-fixes`
- **Baseline de Entrada**: `bd588e5` (Marco 8.1 final)
- **Contexto**: A auditoria independente externa identificou vulnerabilidades que impediram a homologação do Marco 8.1: bypass em autenticação de aprovações por token nulo/vazio, janela TOCTOU em consumo assíncrono de aprovações, concorrência desprotegida entre processos (leases em memória apenas), declaração de wake word com heurística acústica em vez de reconhecimento lexical, e buffer de áudio winmm desprotegido contra descarte de memória. O Marco 8.1.1 eliminou todas as falhas com garantias criptográficas e atômicas no SQLite.

##### 1. O que se pretendia
Eliminar rigorosamente qualquer bypass de autenticação no portão humano, implementar consumo atômico de aprovação diretamente no SQLite, estender o bloqueio de missões para multi-instâncias com leases e fencing tokens monotônicos, alinhar a honestidade da arquitetura de voz (reclassificando heurísticas como experimentais e priorizando Push-to-Talk) e garantir gerenciamento seguro de memória e ciclo de vida de reprodução no Windows.

##### 2. O que foi corrigido e consolidado
1. **Autenticação Inviolável de Aprovações:** Tokens de 256 bits criptográficos gerados com `getrandom::getrandom`, comparação em tempo constante (`constant_time_eq`), proibição de bypass com token nulo ou vazio, sanitização de logs (tokens em claro nunca logados) e falha fechada imediata em `grant_approval` legado para aprovações protegidas.
2. **Consumo Atômico de Aprovações (Anti-Replay / Anti-TOCTOU):** `consume_approval_atomically` implementado com comando `UPDATE` atômico condicional único no SQLite com verificação estrita de hash de parâmetros, status prévio e expiração.
3. **Leases Persistentes Multi-Instância (`mission_leases`):** Exclusão mútua persistida no SQLite entre processos e instâncias com TTL e validação de `fence_token` monotônico a cada etapa pelo `MissionRunner`.
4. **Honestidade Arquitetural da Wake Word:** `WakeWordEngineStatus` (Ready, Experimental, Unavailable), reclassificação factual do motor fonético como `Experimental` (heurística acústica de energia, não reconhecimento lexical), remoção de fallbacks silenciosos em STT e consolidação do Push-to-Talk como mecanismo primário.
5. **Ciclo de Vida de Reprodução e Pinned Buffers:** Buffers WAV fixos em memória (`Arc<Vec<u8>>`) no `WindowsAudioPlaybackSink` durante a reprodução assíncrona no driver WinMM; `AudioPlaybackController` estendido com estados `Completed` e `Failed` rastreados por temporizador.
6. **Fail-Closed para Capacidades Desconhecidas:** `step_requires_approval` retorna estritamente `true` para capacidades não registradas no catálogo, forçando aprovação humana.

##### 3. Métricas Finais e Portões de Qualidade
- **Suíte Adversarial (`tests/marco_8_adversarial.rs`):** **62 cenários adversariais** (5 novos cenários dedicados ao Marco 8.1.1), 100% aprovados.
- **Total de Testes no Workspace:** **559 testes ativos aprovados** (3 testes live opt-in ignorados, 0 falhas).
- **Portões de Qualidade Aprovados:**
  - `cargo fmt --all -- --check`: 100% aprovado.
  - `cargo clippy --workspace --all-targets -- -D warnings`: 0 warnings, 0 erros.
  - `cargo test --workspace --all-targets`: 100% aprovado.
  - `cargo build --release`: compilação limpa concluída com sucesso.

##### 4. Classificação
- **MARCO 8.1.1 (CORREÇÕES CRÍTICAS DE HOMOLOGAÇÃO, HARDENING ATÔMICO E AUDITORIA DE SEGURANÇA) — CONCLUÍDO COM SUCESSO E SUBMETIDO À AUDITORIA INDEPENDENTE EXTERNA**.

---

#### [Event Time: 2026-10-12 | Record Creation Time: 2026-10-12]
### Marco 8.1.1 — Rodada Final de Alinhamento Técnico e Correções Residuais

- **Branch**: `feature/marco-8.1.1-homologation-fixes`
- **Baseline de Entrada**: `62cc878`
- **Contexto**: A auditoria independente do ChatGPT confirmou as melhorias do Marco 8.1.1 (bypass de token corrigido, tokens de 256 bits, consumo atômico no SQLite, leases persistentes com fencing tokens, honestidade na wake word e pinned buffer no winmm), mas apontou discrepâncias técnicas residuais: necessidade de FSM formal com transições CAS em aprovações, resolução estrita de disputa simultânea Grant vs Deny, tratamento de expiração de lease com incremento de fencing token e bloqueio de antigo proprietário, flag explícita `allow_experimental` bloqueando auto-ativação por wake word heurística por padrão, controlador de reprodução unificado sob mutex único com transparência temporal, e garantia estrita de silêncio absoluto em testes sem opt-in físico.
- **Implementações Realizadas**:
  1. FSM de aprovações com `ApprovalStatus::can_transition_to` e `ApprovalStatus::is_terminal`; transições protegidas por CAS no SQLite (`transition_approval_status`).
  2. Concorrência atômica entre Grant e Deny: exatamente uma operação vence e a outra falha de forma limpa.
  3. Leases persistentes com `PRAGMA busy_timeout = 5000;`, transação imediata (`TransactionBehavior::Immediate`), rejeição de aquisições simultâneas em leases ativas e recuperação pós-expiração incrementando `fence_token` monotonicamente e invalidando proprietários antigos.
  4. `WakeWordDetector` com flag `allow_experimental: bool` (padrão `false`), bloqueio de auto-ativação sem opt-in para motores heurísticos experimentais, log `tracing::warn!` em uso experimental e método `can_auto_activate()`.
  5. `AudioPlaybackController` unificado em `PlaybackStateInner` sob mutex único atômico; declaração explícita de término estimado via `is_completion_estimated() -> true`; buffer pinned e `Drop` seguro com `PlaySoundA(NULL, NULL, 0)`.
  6. Invariante de segurança acústica nos testes: execução padrão `cargo test` 100% silenciosa; áudio físico restrito a opt-in com `YUKI_LIVE_AUDIO_TESTS=1`.
- **Métricas Finais e Portões de Qualidade**:
  - **Suíte Adversarial (`tests/marco_8_adversarial.rs`):** **67 cenários adversariais** (5 novos testes 63 a 67), 100% aprovados.
  - **Total de Testes no Workspace:** **564 testes ativos aprovados** (4 testes live opt-in ignorados, 0 falhas).
  - **Portões de Qualidade Aprovados:**
    - `cargo fmt --all -- --check`: 100% aprovado.
    - `cargo clippy --workspace --all-targets -- -D warnings`: 0 warnings, 0 erros.
    - `cargo test --workspace --all-targets`: 100% aprovado.
    - `cargo build --release`: compilação limpa concluída com sucesso.
- **Declaração Oficial**:
  - `"Marco 8.1.1 — alinhamento final implementado e submetido à auditoria independente"`.

---

#### [Event Time: 2026-10-12 | Record Creation Time: 2026-10-12]
### Marco 8.1.1 — Correção Final de Homologação: Áudio, Tratamento Rigoroso de Erros e Silêncio por Construção

- **Branch**: `feature/marco-8.1.1-homologation-fixes`
- **Baseline de Entrada**: `3d650cf`
- **Contexto**: A auditoria independente confirmou as melhorias do alinhamento anterior (transações SQLite imediatas, recuperação de leases com fencing token, FSM de aprovações, bloqueio padrão de wake word experimental e término estimado de playback). Restavam duas pendências técnicas para homologação integral: (1) falhas de interrupção de áudio eram ignoradas via `let _ = self.sink.interrupt();` e (2) a identificação do ambiente de testes dependia parcialmente de heurísticas do caminho do executável (`current_exe().contains("deps")`).
- **Implementações Realizadas**:
  1. **Propagação Rigorosa de Erros de Interrupção de Áudio:** Remoção definitiva de `let _ = self.sink.interrupt();`. Os métodos `play()`, `interrupt()`, `stop()` e `reset()` de `AudioPlaybackController` agora retornam `Result<(), VoiceError>` e propagam formalmente qualquer falha do sink. Se a interrupção falhar:
     - O estado transita imediatamente para `PlaybackStatus::Failed`;
     - Registra `tracing::error!`;
     - Timing e buffers (`playback_timing`, `current_output`) são devidamente limpos;
     - No método `play()`, se a interrupção do áudio ativo falhar, o novo áudio é prontamente REJEITADO, impedindo a sobreposição acidental de fala sobre falhas do subsistema;
     - `MockPlaybackSink` foi enriquecido com `interrupt_fails: Arc<AtomicBool>` e `set_interrupt_fails(bool)` para testes determinísticos de falha de driver.
  2. **Arquitetura de Áudio Silenciosa por Construção (Zero Heurísticas de Caminho):**
     - Remoção integral da função `is_test_environment()` e de qualquer inspeção do caminho de arquivo (`current_exe().contains("deps")`);
     - Introdução do enum `AudioOutputPolicy` (`Production`, `OptInTest`, `Silenced`) governando explicitamente o `WindowsAudioPlaybackSink`;
     - `WindowsAudioPlaybackSink::production()`: emite áudio físico no hardware para operação regular;
     - `WindowsAudioPlaybackSink::opt_in_test()`: só emite som se a variável de ambiente `YUKI_LIVE_AUDIO_TESTS=1` estiver explicitamente ativa; caso contrário, valida buffers em silêncio absoluto;
     - `WindowsAudioPlaybackSink::silenced()`: 100% silenciado em qualquer condição;
     - Construtor `AudioPlaybackController::new()` e `Default` instanciam `MockPlaybackSink` por construção, tornando toda a suíte de testes (`cargo test`) intrinsecamente silenciosa;
     - Construtor `AudioPlaybackController::production()` exposto para uso operacional no sistema e serviços;
     - Testes físicos com alto-falante real marcados estritamente com `#[ignore]` e ativados apenas via `YUKI_LIVE_AUDIO_TESTS=1`.
  3. **Revisão e Documentação do Backend WinMM:**
     - Documentação formal do ciclo de vida assíncrono do ponteiro de memória com `SND_MEMORY | SND_ASYNC`;
     - Fixação de buffer pinned via `Arc<Vec<u8>>` em `current_wav_buffer` prevenindo corrupção por use-after-free;
     - Interrupção atômica no `Drop` via `PlaySoundA(NULL, NULL, 0)`;
     - Transparência formal da distinção entre término estimado de playback (`is_completion_estimated() -> true`) e conclusão por callback físico.
- **Métricas Finais e Portões de Qualidade**:
  - **Suíte Adversarial (`tests/marco_8_adversarial.rs`):** **69 cenários adversariais** (testes 68 e 69 adicionados), 100% aprovados.
  - **Total de Testes no Workspace:** **570 testes ativos aprovados** (5 testes live opt-in ignorados, totalizando 575 testes; 0 falhas).
  - **Portões de Qualidade Aprovados:**
    - `cargo fmt --all -- --check`: 100% aprovado.
    - `cargo clippy --workspace --all-targets -- -D warnings`: 0 warnings, 0 erros.
    - `cargo test --workspace --all-targets`: 100% aprovado.
    - `cargo build --release`: compilação limpa concluída com sucesso.
- **Declaração Oficial**:
  - `"Marco 8.1.1 — correção final de áudio implementada e aguardando homologação independente."`

