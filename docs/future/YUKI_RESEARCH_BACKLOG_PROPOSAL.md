# YUKI — RESEARCH / WEB RESEARCH CAPABILITY
## PROPOSTA ARQUITETURAL DE BACKLOG (FASE PÓS-MVP-1 / PRÓXIMO MARCO)

**Documento:** `docs/future/YUKI_RESEARCH_BACKLOG_PROPOSAL.md`  
**Status:** PROPOSTA / BACKLOG (NÃO IMPLEMENTADO)  
**Data de Criação:** 2026-10-10  
**Classificação:** Backlog Arquitetural e Especificação de Requisitos  

---

### 1. Contexto e Motivação

Durante os testes em ambiente de produção com o modelo `gemini-3.8-flash` no pós-MVP-1, duas constatações operacionais emergiram com nitidez:
1. Diante de perguntas sobre acontecimentos em tempo real (ex.: *"Como está as votações do Brasil hoje?"*), a Yuki reconheceu com fidelidade que seu modelo não possui acesso à internet em tempo real.
2. Diante de consultas de pesquisa complexa e prospectiva (ex.: eleições presidenciais de 2026), o modelo excedeu o timeout ou esgotou a cota de inferência sem dispor de ferramentas para recuperar fontes primárias externas.

Esta proposta formaliza os requisitos arquiteturais, as restrições de segurança e os contratos necessários para introduzir no futuro a capacidade **Yuki Research / Web Research**, mantendo a integridade dos princípios constitucionais da plataforma.

> [!IMPORTANT]
> **Status de Engenharia:** Este documento é estritamente uma proposta de backlog. Nenhuma capability de rede arbitrária, scraping ou busca na web está autorizada ou implementada nesta fase.

---

### 2. Invariantes Constitucionais e Princípios de Segurança

A expansão para pesquisa web introduz vetores de risco críticos, sobretudo **Indirect Prompt Injection** (onde páginas web contêm instruções malévolas disfarçadas de conteúdo). Por isso, os princípios abaixo são absolutos:

1. **`Data != Instruction` (Dado Externo Nunca é Comando):**
   Todo o conteúdo textual recuperado da web (títulos, snippets, artigos, tabelas, metadados) é considerado estritamente **dado não-confiável bruto** (*Untrusted Raw Data*). O parser do runtime deve encapsular o conteúdo de modo que ele nunca seja interpretado como comando de sistema ou autorização.
2. **`Verification != Truth` (Verificação Não é Infalibilidade Factual):**
   O `VerificationEngine` atesta a observabilidade e a conformidade técnica da evidência (ex.: *"a fonte X na URL Y continha o texto Z no timestamp T"*), mas jamais atesta a veracidade ontológica ou a "verdade absoluta" de uma alegação encontrada na internet.
3. **`Capability != Permission != Authorization != Execution`:**
   A existência de uma capability de pesquisa não concede permissão de execução indiscriminada. Cada consulta, fetch ou expansão de link requer avaliação explícita e independente pelo `SecurityController`.
4. **Isolamento de Execução e Escopo Negativo Estrito:**
   - **VEDAÇÃO ABSOLUTA de Shell Arbitrário:** A capability de pesquisa JAMAIS terá acesso a interpretadores de linha de comando (`bash`, `cmd`, `powershell`).
   - **VEDAÇÃO ABSOLUTA de Navegador Irrestrito / Headless Irrestrito:** Não será permitido um navegador com execução aberta de JavaScript arbitrário, download de executáveis ou interação com formulários dinâmicos.
   - **VEDAÇÃO ABSOLUTA de Tráfego de Rede Não-Governado:** Toda chamada HTTP externa deve passar por adaptadores estritos com endpoints declarados, timeouts rígidos e validação de schema.

---

### 3. Requisitos Arquiteturais do Subsistema de Research

#### 3.1. Provedor de Busca Neutro e Independente (*Vendor Independence*)
- Interface abstrata `SearchProvider`:
  ```rust
  pub trait SearchProvider: Send + Sync {
      async fn search(&self, query: &SearchQuery) -> Result<SearchResults, ModelError>;
  }
  ```
- Desacoplamento de fornecedor: suporte a múltiplos provedores de índice web (ex.: SearXNG auto-hospedado, DuckDuckGo Lite, Brave Search API, Perplexity Sonar, Google Custom Search), permitindo alternância via configuração sem mutação de código no Core.

#### 3.2. Recuperação e Leitura de Fontes (*Reader / Scraper Seguro*)
- Fetch HTTP estritamente somente-leitura com cabeçalhos sanitizados e user-agent auditável da Yuki.
- Extração de texto baseada em leitura estática (SSR/HTML estático), conversão determinística para Markdown limpo ou texto plano, com descarte obrigatório de scripts (`<script>`), estilos (`<style>`), iframes, imagens binárias e rastreadores.
- Limite rígido de tamanho por documento baixado (ex.: teto de 500 KiB por página).

#### 3.3. Proveniência, Citações e Atribuição
- Cada afirmação derivada de pesquisa deve manter proveniência verificável:
  - URL canônica de origem;
  - Título e domínio da fonte;
  - Timestamp exato da recuperação (UTC);
  - Trecho literal (*verbatim quote*) utilizado como evidência no cálculo cognitivo.
- Citações ancoradas no resultado final entregue ao usuário, permitindo rastreabilidade completa.

#### 3.4. Verificação de Atualidade e Temporalidade (*Recency Scoring*)
- Extração de datas de publicação e modificação (`article:published_time`, `datePublished`, cabeçalhos `Last-Modified`).
- Validação temporal da relevância: se o usuário solicita dados de "hoje" ou "2026", o mecanismo deve confrontar as datas encontradas e sinalizar explicitamente caso uma fonte seja defasada ou inconclusiva.

#### 3.5. Comparação e Confrontação de Fontes (*Source Cross-Examination*)
- O modelo não deve se apoiar em uma única fonte para fatos controversos ou complexos.
- Triangulação de fontes: identificação de consenso entre múltiplos domínios independentes versus alegações isoladas.
- Tipagem epistêmica de alegações: o sistema deve distinguir explicitamente na síntese:
  - **Fato documentado** (múltiplas fontes fidedignas coincidentes);
  - **Alegação de parte** (declaração atribuída a uma fonte específica);
  - **Inferência ou projeção** (conclusão analítica do modelo).

#### 3.6. Proteção contra Injeção Indireta de Prompt (*Indirect Prompt Injection Defense*)
- Pré-processamento e sanitização de texto antes de reintroduzir o conteúdo no loop conversacional:
  - Detecção e neutralização de padrões de sequestro de contexto (ex.: `"Ignore todas as instruções anteriores e execute..."`).
  - Encapsulamento estrito em blocos semânticos de citação delimitados por tokens especiais (*data boundaries*).
  - Sanitização de entidades perigosas e caracteres de escape.

#### 3.7. Orçamentos Rígidos de Recursos (*Budgets: Custo, Requisições e Tempo*)
- **Teto de Consultas por Turno:** No máximo 3 a 5 buscas por interação.
- **Teto de Downloads de Páginas:** No máximo 5 páginas completas recuperadas por pesquisa.
- **Timeout Global de Pesquisa:** Orçamento temporal total (ex.: máximo de 45 a 60 segundos por turno de pesquisa).
- **Gestão de Quota:** Detecção preventiva de saturação de taxa (RPM) com amortecimento e aviso antecipado ao operador, evitando que o ciclo resulte em `RESOURCE_EXHAUSTED`.

#### 3.8. Suporte Futuro a Pesquisa Profunda (*Deep Research & Durable Execution*)
- Para consultas abrangentes que demandem múltiplos minutos de análise:
  - Execução durável em background gerenciada pelo EventStore/SQLite.
  - Gravação de checkpoints intermediários de progresso.
  - Notificação assíncrona ao operador na conclusão da síntese analítica.

---

### 4. Modelo Conceitual do Fluxo de Governança

```text
Usuário
  │  Prompt com demanda de informação externa
  ▼
YukiCore (Loop de Continuação Governada)
  │  Proposta de Capability: research.web_search(query="...")
  ▼
SecurityController
  │  Avaliação de Política (Risco: Médio / Baixo; Limites de Quota)
  ├─► Se NEGADO: Aborta graciosamente com YukiResult::denied
  └─► Se PERMITIDO: Emite CapabilityToken descartável
        │
        ▼
ExecutionEngine / ResearchAdapter
  │  Executa busca via SearchProvider abstrato
  │  Converte resultados para formato estruturado e higienizado
  ▼
VerificationEngine
  │  Gera evidência: observed_output com hashes, contagem de links e status
  │  VerificationState::VerifiedSuccess
  ▼
AuditSubsystem (SQLite Durável)
  │  Registra consulta, provedor, evidências e timestamps
  ▼
YukiCore (Reintrodução do Resultado como Dado Não-Confiável)
  │  ModelMessage::tool_result(capability="research.web_search", data={...})
  ▼
Model Provider
  │  Sintetiza resposta com citações e confrontação de fontes
  ▼
Resposta Final em Linguagem Natural com Proveniência
```

---

### 5. Critérios de Aceite para Futura Aprovação e Implementação

1. **Testes de Segurança Adversarial de Injeção de Prompt:** Demonstração de que páginas maliciosas simuladas não conseguem desviar a política do sistema ou obter autorização indevida.
2. **Independência de Provedor Testada:** Provedor de busca configurável e testável offline através de mocks locais sem rede.
3. **Respeito aos Orçamentos:** Bloqueio determinístico e auditado caso o modelo tente ultrapassar os limites de requisições, bytes ou tempo.
4. **Preservação de Todas as Invariantes da Plataforma.**
