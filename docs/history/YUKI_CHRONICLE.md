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



