# CRÔNICAS DA YUKI
## Registro Histórico, Narrativo e Factual da Criação e Evolução da Yuki

---

### Protocolo Histórico e Princípios

Este documento é o registro perene da criação e maturação da Yuki. Ele não é um simples changelog técnico ou sumário de commits; seu objetivo é documentar a jornada real de engenharia: as hipóteses, os avanços, as hesitações, as decisões de design, os erros cometidos, as correções arquiteturais e as lições aprendidas.

#### Diretrizes Fundamentais do Registro:
1. **Veracidade e Transparência**: Nunca reescrever retrospectivamente uma falha para fazer o desenvolvimento parecer mais simples ou linear do que foi. Erros de projeto e execução fazem parte da identidade da Yuki e fornecem o contexto necessário para compreender sua robustez.
2. **Separação de Evidências**:
   - *Fatos tecnicamente verificados*: Registros ancorados em hashes de Git, branches, tags imutáveis e saídas de compilador.
   - *Horários fornecidos pelo Owner*: Timestamps humanos indicados explicitamente como aproximados.
   - *Interpretações narrativas*: Contexto e motivações documentados com clareza.
3. **Perspectiva Cronológica Progressiva**: As entradas documentais são registradas no momento de sua identificação formal. Marcos cronologicamente anteriores (como a concepção original da Yuki, a Foundation v0.1 e os Marcos 1–4) serão reconstruídos progressivamente com base em registros históricos preservados.

---

### Entradas Históricas

---

#### [2026-10-04] Primeira Execução Manual do MVP-1 e a Descoberta da Desconexão do Runtime

- **Data**: 04 de outubro de 2026
- **Horário**: Aproximadamente 12:09 (America/Sao_Paulo) — *Horário aproximado fornecido pelo Owner*
- **Milestone**: Pós-congelamento do MVP-1 (`v0.2.0-mvp1`, commit `6a9d07010719b18bb10a60a082606be54b813f66`)
- **Contexto Operacional**: Teste manual real executado no notebook de desenvolvimento do Owner.

##### O que se pretendia
Após a conclusão bem-sucedida dos Marcos 1 a 4 e o congelamento formal da tag `v0.2.0-mvp1`, o Owner realizou o primeiro teste prático fora de scripts automatizados de CI: compilar o binário em release, inicializar a persistência durável SQLite localmente, validar a execução das capabilities fundamentais (`system.echo`, `system.time`) e conectar a chave da API do Google Gemini para uma interação real.

##### O que aconteceu
1. O runtime iniciou com sucesso. O SQLite inicializou e reportou integridade durável (`Persistent Audit (SQLite): OK`).
2. A capability `system.echo` executou com sucesso.
3. A capability `system.time` executou com sucesso, reportando o horário e confirmando a tolerância dinâmica de verificação.
4. O Owner configurou temporariamente a variável `YUKI_GEMINI_API_KEY`.
5. O comando `yuki health` foi executado e reportou:
   ```text
   External Model Provider: CONFIGURED (Gemini available)
   ```
6. O Owner submeteu um prompt conversacional esperando a resposta do modelo Google Gemini. A resposta, porém, exibiu o texto determinístico característico do `MockModelProvider`.

##### O Erro e a Investigação Arquitetural
A investigação do código-fonte revelou uma desconexão crítica no bootstrap da aplicação:
- No Marco 2, o `GeminiProviderAdapter`, o `ModelRouter`, as estruturas de credenciais (`SecretRef`, `SecretMaterial`, `EnvSecretStore`) e os parsers haviam sido completamente implementados e testados em testes unitários e de segurança.
- No entanto, no ponto de entrada real da CLI (`src/main.rs`), o `YukiCore` continuava sendo instanciado exclusivamente via `YukiCore::new()`, que hardcodava o `MockModelProvider`.
- O método de injeção `core.with_model_provider(...)` existia no domínio do Core, mas a CLI nunca o invocava.
- A linha em `yuki health` apenas verificava `std::env::var("YUKI_GEMINI_API_KEY").is_ok()`, emitindo uma mensagem otimista e tecnicamente falsa: a credencial estava no ambiente, mas o provedor sequer havia sido instanciado pelo Core.
- As suítes de teste automatizadas possuíam 120 testes verdes porque os testes unitários utilizavam o Mock intencionalmente (para garantir determinismo e independência de rede em CI), enquanto o teste live ignorado (`model_gateway_live.rs`) invocava o `GeminiProviderAdapter` diretamente, sem passar pelo `YukiCore` ou pela CLI. Não existia nenhum teste de integração cobrindo o bootstrap de produção.

##### A Decisão e a Solução
O Owner e o Architecture Review deliberaram que o erro não seria acobertado ou contornado com atalhos:
1. O baseline `v0.2.0-mvp1` foi mantido 100% congelado e imutável.
2. Foi criada a branch pós-release `feature/post-mvp1-model-runtime-wiring`.
3. Desenvolveu-se uma função de resolução de provedor de modelo (`resolve_model_provider_from_env`), obedecendo ao princípio de que o bootstrap não deve tocar em `SecretMaterial`, transferindo apenas o `SecretRef` e o broker.
4. Removeu-se a mensagem enganosa do `health`, substituindo-a pela leitura dos metadados e do estado de saúde local real do provedor injetado.
5. Criou-se a suíte `tests/model_provider_wiring.rs` para proteger o bootstrap contra regressões futuras.

##### Lição Aprendida
> *«Um componente implementado com perfeição em sua unidade e coberto por testes isolados não tem valor operacional se não estiver conectado ao runtime real. O teste de integração do bootstrap é um portão de segurança indispensável.»*
