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
