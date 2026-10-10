# ADR-020 — Research v1: Governed Web Retrieval, Vendor Abstraction & Cloud-Ready Boundaries

**Versão:** 1.0  
**Status:** PROPOSED  
**Domínio:** 15 — Integrations / Cognitive Capabilities / Information Retrieval  
**Data:** 2026-10-10  
**Decisão:** Proposed (Aguardando Aprovação do Owner)  

---

## 1. Objetivo

Definir a arquitetura técnica, os contratos formais, o modelo de ameaças, o perímetro de segurança e as fronteiras operacionais da primeira capability de pesquisa web governada da Yuki (**Research v1**), assegurando que o acesso a dados externos da internet respeite integralmente os princípios constitucionais:

```text
«Data != Instruction»
«Verification != Truth»
«Capability != Authorization»
«External Web Content = Untrusted Raw Data»
```

Este documento estabelece o desacoplamento de fornecedores de busca, a blindagem estrita contra ataques de *Server-Side Request Forgery* (SSRF) e *Indirect Prompt Injection*, o sistema de proveniência rastreável por citações e os contratos mínimos para que o subsistema seja agnóstico quanto ao ambiente de execução (computador de desenvolvimento, container isolado, servidor remoto ou nuvem híbrida).

---

## 2. Contexto

A conclusão do MVP-1 e a subsequente validação live do *Governed Bounded Tool Continuation Loop* (ADR-018) conectando a Yuki ao Google Gemini 3.8 demonstraram a maturidade das capacidades internas (`system.echo`, `system.time`, `system.info`).

No entanto, os testes em produção evidenciaram duas lacunas fundamentais quando o operador solicita informações além do conhecimento estático do modelo:
1. **Informação em tempo real:** Diante de perguntas sobre fatos contemporâneos, a Yuki reconheceu honestamente a ausência de acesso à internet.
2. **Consultas complexas e prospectivas:** Diante de tópicos de alta densidade informativa (ex.: eleições de 2026), o modelo excedeu o tempo limite individual de 30s ou esgotou a cota gratuita da API upstream (`RESOURCE_EXHAUSTED` em 5 RPM).

Para evoluir a Yuki para um assistente capaz de pesquisar, confrontar fontes e responder com base em dados atualizados sem perder o controle de segurança, faz-se necessário conceber a capability de **Research**.

Sem este ADR:
1. O Core correria o risco de acoplar-se a APIs proprietárias de busca (violando o ADR-017).
2. O sistema ficaria vulnerável a injeção indireta de prompt por meio de páginas maliciosas indexadas na web.
3. Requisições HTTP automáticas poderiam ser exploradas para varredura de rede local ou vazamento de metadados de nuvem (SSRF).
4. O consumo de APIs externas poderia gerar custos imprevisíveis ou saturação de cotas operacionais.

---

## 3. Decisão Arquitetural

A Yuki adota uma arquitetura soberana, modular e em camadas para a capability de Research, decompondo a busca em duas capacidades complementares e altamente tipadas:

```text
┌─────────────────────────────────────────────────────────────┐
│                          Yuki Core                          │
│               Governed Tool Continuation Loop               │
└──────────────┬───────────────────────────────┬──────────────┘
               │                               │
       Proposta│de Intent              Proposta│de Intent
               ▼                               ▼
     research.search                     research.fetch
  (Pesquisa Textual Canônica)         (Leitura Segura de Página)
               │                               │
               ▼                               ▼
       SecurityController              SecurityController
       (Política de Saída)             (Guarda SSRF e Egress)
               │                               │
               ▼                               ▼
      ExecutionEngine                 ExecutionEngine
               │                               │
       ┌───────┴───────┐               ┌───────┴───────┐
       ▼               ▼               ▼               ▼
  BraveSearch      SearXNG         HttpReader      JinaReader
    Adapter        Adapter          (Nativo)        Adapter
```

### 3.1. Separação de Responsabilidades: Search vs. Fetch
- **`research.search`**: Executa consultas textuais sobre um índice de pesquisa, retornando uma lista estruturada de candidatos (título, URL canônica, snippet resumido e metadados de publicação). Não baixa páginas completas.
- **`research.fetch`**: Recupera o conteúdo textual limpo e sanitizado de uma URL específica previamente validada, extraindo o texto relevante e descartando scripts, formulários, imagens e código executável.

### 3.2. Abstração de Fornecedor (*Vendor Independence*)
O runtime do Core interage exclusivamente com traits abstratos de domínio:
- `SearchProvider`: Trait assíncrono para provedores de busca (Brave Search API como provedor primário sugerido; SearXNG para auto-hospedagem soberana; Mock para testes de CI).
- `ContentFetchProvider`: Trait assíncrono para leitura de conteúdo web (leitor HTTP nativo em Rust com limpeza HTML estática como padrão seguro).

### 3.3. Perímetro de Defesa e Segurança em Profundidade
1. **SSRF Guard & Pinning de IP:** Toda URL alvo de `fetch` deve passar por resolução DNS prévia com bloqueio obrigatório de IPs privados (RFC 1918), loopback (`127.0.0.1`, `::1`), link-local e endpoints de metadados de provedores cloud (`169.254.169.254`). O socket de conexão é amarrado diretamente ao IP validado para prevenir *DNS Rebinding*.
2. **Defesa contra Injeção Indireta de Prompt:** O texto recuperado da web é delimitado por marcadores semânticos especiais (`<<<UNTRUSTED_WEB_CONTENT>>>`), e o prompt de sistema reforça que o conteúdo externo constitui dados literais para citação, sem autoridade para alterar instruções ou invocar capacidades.
3. **Orçamentos Rígidos (*Fail-Closed Budgets*):**
   - Limite de consultas por turno: máximo 3 a 5 buscas.
   - Limite de páginas por turno: máximo 3 a 5 leituras.
   - Limite de tamanho por página: 256 KiB comprimido / 1 MiB texto.
   - Timeout individual rígido: 10s para busca, 15s para fetch.
4. **Tratamento de Falhas e Degradação Graciosa:** Páginas com HTTP 404, 403, 5xx ou bloqueadas por segurança retornam registros estruturados de erro tipado (`FetchError::BlockedByPolicy`, `FetchError::NotFound`), permitindo ao modelo prosseguir com as demais fontes disponíveis sem quebrar o turno.

---

## 4. Contratos Formais de Capability

### 4.1. Manifesto Canônico: `research.search`
- **ID da Capability:** `research.search`
- **Classe de Risco:** `Low`
- **Parâmetros de Entrada (JSON Schema):**
  ```json
  {
    "type": "object",
    "required": ["query"],
    "properties": {
      "query": {
        "type": "string",
        "minLength": 2,
        "maxLength": 200,
        "description": "Termo de busca textual claro e objetivo"
      },
      "max_results": {
        "type": "integer",
        "minimum": 1,
        "maximum": 10,
        "default": 5,
        "description": "Número máximo de resultados desejados"
      },
      "freshness": {
        "type": "string",
        "enum": ["any", "day", "week", "month", "year"],
        "default": "any",
        "description": "Filtro temporal de recência dos resultados"
      }
    },
    "additionalProperties": false
  }
  ```

- **Estrutura de Retorno (Saída Verificada):**
  ```json
  {
    "query": "eleições presidenciais 2026 brasil",
    "provider": "BraveSearch",
    "searched_at": "2026-10-10T07:15:00Z",
    "results_count": 3,
    "results": [
      {
        "cite_id": "src:1",
        "url": "https://noticias.exemplo.com.br/politica/eleicoes-2026",
        "title": "Cenário e Calendário Eleitoral para 2026",
        "snippet": "Resumo fático do artigo...",
        "domain": "noticias.exemplo.com.br",
        "published_date": "2026-09-15T12:00:00Z",
        "confidence_state": "AggregatedSnippet"
      }
    ]
  }
  ```

### 4.2. Manifesto Canônico: `research.fetch`
- **ID da Capability:** `research.fetch`
- **Classe de Risco:** `Low` (para domínios públicos válidos) / `Medium` (domínios desconhecidos ou múltiplos fetches consecutivos)
- **Parâmetros de Entrada (JSON Schema):**
  ```json
  {
    "type": "object",
    "required": ["url"],
    "properties": {
      "url": {
        "type": "string",
        "format": "uri",
        "maxLength": 500,
        "description": "URL HTTP/HTTPS pública canônica a ser recuperada"
      },
      "max_length_chars": {
        "type": "integer",
        "minimum": 500,
        "maximum": 30000,
        "default": 10000,
        "description": "Limite máximo de caracteres de texto útil a extrair"
      }
    },
    "additionalProperties": false
  }
  ```

- **Estrutura de Retorno (Saída Verificada):**
  ```json
  {
    "url": "https://noticias.exemplo.com.br/politica/eleicoes-2026",
    "final_url": "https://noticias.exemplo.com.br/politica/eleicoes-2026",
    "fetched_at": "2026-10-10T07:15:10Z",
    "http_status": 200,
    "content_type": "text/html; charset=utf-8",
    "title": "Cenário e Calendário Eleitoral para 2026",
    "published_date": "2026-09-15T12:00:00Z",
    "extracted_text": "Texto limpo em formato Markdown legível...",
    "content_hash_sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "truncated": false,
    "confidence_state": "DirectSource"
  }
  ```

---

## 5. Rastreabilidade de Citações e Proveniência

Para assegurar que o operador possa verificar a proveniência fática de qualquer afirmação gerada pela Yuki:

1. **Atribuição por Âncora:** Cada resultado de busca ou fetch recebe um identificador imutável dentro do turno (`src:1`, `src:2`).
2. **Síntese Acompanhada de Citação:** O modelo de linguagem é instruído no prompt de sistema a ancorar suas afirmações nos identificadores de fonte correspondentes (ex.: *“De acordo com a apuração oficial [src:1], o pleito está previsto para outubro de 2026.”*).
3. **Tabela de Fontes no Fechamento:** Ao concluir uma resposta baseada em pesquisa, a Yuki inclui uma seção formal de referências com título, domínio, URL completa e data de coleta.

---

## 6. Evolução para Nuvem e Múltiplos Dispositivos (*Cloud-Ready Evolution*)

A capability de Research é projetada desde o dia zero para não possuir qualquer acoplamento com o laptop de desenvolvimento:

1. **Acesso a Segredos:** Chaves de API de busca (ex.: `YUKI_BRAVE_SEARCH_API_KEY`) são referenciadas via `SecretRef` e resolvidas via `CredentialBroker` (ADR-008), permitindo que em produção rodem via variáveis de ambiente, Docker Secrets, Kubernetes Secrets ou AWS/GCP Secrets Manager sem alteração de código.
2. **Desacoplamento de Rede e DNS:** O validador SSRF opera com resolução de sockets abstratos, permitindo rodar em containers OCI com proxy corporativo ou rede restrita.
3. **Worker Isolado para Fetch (Fase Futura):** O contrato de `research.fetch` é desacoplado do Core através do padrão de porta/adaptador, permitindo que no futuro o scraping de páginas execute dentro de um container isolado (sandbox gVisor/Wasm) via gRPC ou Unix Domain Socket, sem risco de corrupção do processo principal da Yuki.

---

## 7. Consequências e Trade-Offs

### Positivas:
- **Informação Fresca e Verificável:** A Yuki supera a limitação de data de corte de conhecimento dos modelos LLM.
- **Transparência e Auditabilidade Total:** Cada busca, página visitada, código HTTP e hash de conteúdo é registrado no SQLite durável (ADR-019).
- **Soberania e Independência de Fornecedor:** O operador pode alternar entre Brave Search, SearXNG auto-hospedado ou provedores futuros sem mutação do Core.
- **Proteção Robusta:** A Yuki não pode ser usada como vetor de SSRF para atacar redes internas ou roubar metadados de nuvem.

### Negativas / Custos Operacionais:
- **Latência Adicional:** Um turno conversacional com pesquisa web e continuação de ferramenta adiciona de 1 a 3 segundos de latência de rede.
- **Dependência de Quota Externa:** Provedores de busca em planos gratuitos possuem cotas mensais (ex.: Brave: 2.000 requisições/mês) que exigem amortecimento e monitoramento.
- **Risco de Páginas Bloqueadas:** Determinados portais utilizam proteção anti-bot / Cloudflare CAPTCHA que retornam HTTP 403 para clientes sem navegador completo. A Yuki aceita essa restrição como trade-off de segurança (não introduzirá navegadores arbitrários pesados para burlar CAPTCHAs).
