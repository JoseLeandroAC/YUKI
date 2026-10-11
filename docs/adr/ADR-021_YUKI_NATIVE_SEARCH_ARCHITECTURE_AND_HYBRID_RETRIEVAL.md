# ADR-021 — Yuki Native Search Architecture, Indexing Engine & Hybrid Retrieval

**Versão:** 1.0  
**Status:** PROPOSED / ARCHITECTURAL BLUEPRINT (Marco 5 Concluído)  
**Domínio:** 15 — Integrations / Cognitive Capabilities / Information Retrieval & Local Indexing  
**Data:** 2026-10-11  
**Decisão:** Proposed (Arquitetura formal e roadmap definidos para busca e recuperação híbrida soberana)  

---

## 1. Objetivo & Visão Geral

Definir a arquitetura técnica, os subsistemas de processamento de documentos, as estratégias de armazenamento em índices locais e o modelo de fusão híbrida da **Yuki Native Search** (busca nativa soberana da Yuki), complementando os provedores externos comerciais implementados na Research v1 (Brave Search e LangSearch).

A Native Search tem como meta conferir à Yuki independência e soberania informativa perante serviços proprietários de indexação, permitindo a recuperação de evidências em três modos operacionais:

1. **Local Search:** Busca determinística e offline sobre o workspace de trabalho, documentações do projeto, arquivos de código-fonte, notas pessoais e histórico auditável persistido.
2. **External Search:** Busca sob demanda contra a internet pública governada via provedores configuráveis (Brave Search, LangSearch ou outros adaptadores conformes ao ADR-020).
3. **Hybrid Retrieval:** Fusão ponderada e re-ranking de evidências provenientes tanto do índice local quanto de snippets/páginas externas, combinando Full-Text Search (BM25) e recuperação vetorial (Embeddings semânticos) via *Reciprocal Rank Fusion (RRF)*.

Esta decisão preserva incondicionalmente os princípios fundamentais da Yuki:

```text
«Data != Instruction»
«Index != Truth»
«Offline First, Network On-Demand»
«Local Sovereignty & Privacy by Design»
«Capability != Authorization»
```

---

## 2. Contexto & Motivação

### 2.1 Limitações de Provedores Comerciais de Busca Web

Durante os Marcos 1 a 4 e na integração do LangSearch (Marco 5), a Yuki estabeleceu um fluxo governado de consulta a motores de busca comerciais (`research.search`). Contudo, o uso exclusivo de APIs externas acarreta limitações estruturais:

1. **Dependência e Soberania:** Interrupções de serviço, mudanças de termos de uso, descontinuação de planos gratuitos ou bloqueios geográficos de provedores externos podem paralisar as capacidades cognitivas da Yuki.
2. **Latência de Rede e Custo:** Cada pesquisa consome tempo de ida-e-volta (RTT) de rede (frequentemente 300 ms a 1500 ms) e consome cotas financeiras ou limites de taxa (rate limits HTTP 429).
3. **Privacidade e Minimização de Dados (ADR-015):** Consultas enviadas a motores comerciais revelam os interesses técnicos, pesquisas de código e intenções do operador a servidores de terceiros.
4. **Cegueira Informacional sobre o Domínio Local:** Motores web globais não indexam (e não devem indexar) repositórios privados, arquivos de configuração locais, especificações de projeto, notas de desenvolvimento e logs de auditoria da Yuki.

### 2.2 Limitações de Busca Exclusivamente Local

Por outro lado, depender unicamente de arquivos em disco ou de grep simples não atende à necessidade de:
- Descobrir documentações e APIs recém-lançadas na web;
- Investigar vulnerabilidades e patches em tempo real;
- Rastrear e indexar documentações técnicas oficiais completas (ex.: Rust docs, especificações W3C, documentações de cloud) para consulta instantânea offline.

### 2.3 A Necessidade de uma Busca Nativa Híbrida

A arquitetura Native Search propõe uma solução unificada: a Yuki deve ser capaz de criar, manter e consultar seus próprios índices estruturados em armazenamento local (SQLite FTS5 + vetores), com a capacidade opcional de alimentar esses índices via *Governed Crawler* em domínios autorizados pelo operador, unindo o melhor do mundo offline e online sob governança rigorosa.

---

## 3. Princípios Fundamentais & Invariantes de Governança

### Invariante 1 — Zero Rede para Consultas ao Índice Local
A execução de pesquisas locais no índice nativo (`native_search.local`) possui:
- `network_required: false`
- `secrets_required: false`
- Risco operacional: `RiskClass::Low`
Nenhum socket ou conexão externa pode ser instanciado durante uma busca local.

### Invariante 2 — Permissão Específica para Rastreador Local (`egress:native_crawl`)
O rastreamento de páginas pelo crawler nativo requer permissão explícita diferenciada:
- `egress:native_crawl`, distinta de `egress:web_fetch` e `egress:web_search`.
O operador deve delimitar explicitamente a whitelist de domínios autorizados para rastreamento.

### Invariante 3 — Tratamento de Dados como Conteúdo Passivo (`Data != Instruction`)
Qualquer documento indexado — seja código-fonte local, markdown ou página web rastreada — é classificado estritamente como dado não confiável. O índice nunca indexa código com privilégios de execução, e o texto indexado é sanitizado antes da exibição ao modelo e ao operador.

### Invariante 4 — Conformidade Rigorosa com Cortesia e robots.txt
O rastreador nativo respeita incondicionalmente:
- Diretivas `robots.txt` do domínio de destino;
- Limite de taxa de cortesia (*politeness delay* de no mínimo 1000 ms por host);
- Profundidade máxima de exploração (*max depth* padrão = 2);
- Teto volumétrico por domínio (padrão 50 páginas ou 10 MiB por sessão de ingestão).

### Invariante 5 — Rastreabilidade Criptográfica e Proveniência de Citações
Cada chunk indexado retém:
- Hash criptográfico SHA-256 do conteúdo bruto original;
- Hash SHA-256 do texto extraído;
- Data/hora de indexação e TTL de expiração;
- Classificação formal de proveniência (`SourceKind::LocalWorkspace`, `SourceKind::LocalCache`, `SourceKind::DirectSource`, `SourceKind::AggregatedSnippet`).

---

## 4. Arquitetura de Componentes da Yuki Native Search

A figura abaixo ilustra a topologia do ecossistema de busca nativa e recuperação híbrida:

```text
┌────────────────────────────────────────────────────────────────────────┐
│                               Yuki Core                                │
│                   Model Continuation & Synthesis Loop                  │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │
                                    ▼
┌────────────────────────────────────────────────────────────────────────┐
│                       Native Search Orchestrator                       │
│    ┌──────────────────┐  ┌───────────────────┐  ┌─────────────────┐    │
│    │  Local Dispatch  │  │ External Dispatch │  │  Hybrid Fusion  │    │
│    └────────┬─────────┘  └─────────┬─────────┘  └────────┬────────┘    │
└─────────────┼──────────────────────┼─────────────────────┼─────────────┘
              │                      │                     │
              ▼                      │                     ▼
┌───────────────────────────┐        │        ┌──────────────────────────┐
│    Search Index Storage   │        │        │ Reciprocal Rank Fusion   │
│ ┌───────────────────────┐ │        │        │        (RRF Engine)      │
│ │   SQLite FTS5 (BM25)  │ │        │        └──────────────────────────┘
│ └───────────────────────┘ │        │                     ▲
│ ┌───────────────────────┐ │        │                     │
│ │ Vector Embeddings     │ │        │                     │
│ │ (Cosine Similarity)   │ │        │                     │
│ └───────────────────────┘ │        │                     │
└─────────────▲─────────────┘        │                     │
              │                      ▼                     │
              │         ┌──────────────────────────┐       │
              │         │  External Search Engine  │───────┘
              │         │ (Brave / LangSearch API) │
              │         └──────────────────────────┘
              │
┌─────────────┴──────────────────────────────────────────────────────────┐
│                   Ingestion & Indexing Pipeline                        │
│ ┌────────────────────────┐  ┌─────────────────┐  ┌──────────────────┐ │
│ │  Document Processor    │  │ Semantic Chunker│  │ Embedding Model  │ │
│ │ (HTML / MD / Code/PDF) │  │ (Token-bounded) │  │  (Local ONNX)    │ │
│ └───────────▲────────────┘  └─────────────────┘  └──────────────────┘ │
└─────────────┼──────────────────────────────────────────────────────────┘
              │
    ┌─────────┴─────────┐
    │                   │
┌───┴──────────┐ ┌──────┴────────────────────────────────────────────────┐
│  Workspace   │ │ Governed Crawler & Fetch Engine                       │
│  Local Files │ │  - Robots.txt parser   - SSRF Guard & Connection Pin  │
│  (Markdown,  │ │  - Politeness limiter  - Domain Whitelist Enforcer    │
│   Rust, Docs)│ │  - Depth controller    - Max payload 2 MiB limiter    │
└──────────────┘ └───────────────────────────────────────────────────────┘
```

### 4.1 Native Search Orchestrator

O componente orquestrador central atua como fachada unificada para as consultas:
- **Estratégia de Roteamento:** Seleciona entre `Local`, `External` ou `Hybrid` conforme configuração do operador ou intenção validada pela política de segurança.
- **Deduplicação de Evidências:** Unifica itens idênticos indexados localmente e recuperados de APIs externas via canonical URL e SHA-256 de conteúdo.
- **Controle Orçamentário:** Aplica limites de tempo e contadores por turno (`ResearchBudgetTracker`).

### 4.2 Governed Crawler & Fetch Engine

O módulo de ingestão externa automatizada estende as garantias do Marco 3 (`research.fetch`):
- **Respeito a robots.txt:** Parser de conformidade RFC 9309 que valida permissão de caminho antes de qualquer requisição.
- **Perímetro SSRF:** Reutiliza integralmente o `SsrfGuard`, bloqueando loopback, IPv6 link-local, CGNAT, endereços não roteáveis e DNS rebinding.
- **Connection Pinning:** Validação do endereço IP resolvido diretamente na amarração do socket TLS/TCP.
- **Politeness Delay:** Fila de trabalho com rate limiter estrito por domínio (mínimo de 1000 ms entre requisições ao mesmo host).
- **Controle de Profundidade:** Algoritmo BFS limitado a profundidade 2 para evitar armadilhas de rastreamento (*crawler traps*).

### 4.3 Document Processor & Chunking Pipeline

Transforma conteúdo heterogêneo em registros padronizados para o índice:
- **Higienização de HTML:** Reutiliza `HtmlExtractFilter` para remover scripts, styles, iframes e tags perigosas, extraindo títulos, cabeçalhos estruturados e texto contínuo.
- **Processamento de Código e Markdown:** Reconhecimento de títulos Markdown (`#`, `##`), blocos de código e tabelas.
- **Chunking Semântico com Janela Deslizante:** Divisão de documentos longos em blocos de 300 a 800 tokens, com sobreposição (*overlap*) de 50 tokens, preservando limites de frases e parágrafos.
- **Hashing Determinístico:** Geração de `chunk_hash = SHA256(chunk_text)` e `doc_hash = SHA256(raw_bytes)`.

### 4.4 Search Index Storage (SQLite FTS5 + Vetores)

O armazenamento do índice adota o SQLite como mecanismo soberano embarcado, alinhado ao ADR-019:
- **Full-Text Search (FTS5):** Tabela virtual FTS5 configurada com tokenizador `unicode61` (suporte completo a caracteres acentuados, pontuação e termos técnicos de programação).
- **Ranking BM25:** Utilização da função nativa `bm25(documents_fts)` do SQLite FTS5 com pesos configuráveis por campo (título peso 2.0, texto peso 1.0).
- **Armazenamento Vetorial Embutido:** Representação vetorial armazenada como `BLOB` compacto (vetores normalizados float32) ou via extensão `sqlite-vec`. Similaridade por cosseno calculada via produto escalar vetorial.
- **Modelo de Embedding Local:** Execução de modelo leve de embeddings (ex.: BGE-small-en-v1.5 ou MiniLM-L6-v2 quantizado via `candle` ou ONNX Runtime local), sem qualquer tráfego para a internet.

### 4.5 Reciprocal Rank Fusion (RRF Engine)

Para consultas no modo híbrido, a Native Search combina a lista ranqueada do índice FTS5, a lista de similaridade vetorial e os snippets de provedores externos via algoritmo RRF:

$$RRF\_Score(d) = \sum_{m \in M} \frac{w_m}{k + Rank_m(d)}$$

Onde:
- $M$ é o conjunto de métodos de busca consultados ($M \subseteq \{FTS5, Vector, ExternalSearch\}$);
- $Rank_m(d)$ é a posição ordinal do documento $d$ no resultado do método $m$ (1-indexed);
- $k$ é a constante de suavização (padronizada em $k = 60$);
- $w_m$ é o peso relativo conferido à modalidade (padrão: $w_{FTS} = 1.0$, $w_{Vector} = 1.0$, $w_{External} = 0.8$).

Documentos que aparecem tanto no topo da busca vetorial quanto da busca por palavras-chave são priorizados no topo do resultado final unificado.

---

## 5. Esquema de Dados do Índice Nativo (SQLite)

O esquema relacional e FTS do índice local da Yuki reside em `$YUKI_HOME/data/search_index.db`:

```sql
-- Metadados de documentos indexados
CREATE TABLE IF NOT EXISTS indexed_documents (
    doc_id TEXT PRIMARY KEY,               -- UUIDv4 canônico
    source_uri TEXT NOT NULL UNIQUE,       -- file:// ou https://
    domain TEXT NOT NULL,                  -- "workspace" ou domínio web
    source_kind TEXT NOT NULL,             -- LocalWorkspace, DirectSource, LocalCache
    title TEXT NOT NULL,
    raw_hash_sha256 TEXT NOT NULL,
    extracted_hash_sha256 TEXT NOT NULL,
    byte_size INTEGER NOT NULL,
    char_count INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    indexed_at TEXT NOT NULL,
    expires_at TEXT,                       -- NULL para workspace permanente
    etag TEXT,
    last_modified_header TEXT
);

-- Chunks semânticos para granularidade de citação e busca
CREATE TABLE IF NOT EXISTS document_chunks (
    chunk_id TEXT PRIMARY KEY,             -- doc_id:chunk_index
    doc_id TEXT NOT NULL REFERENCES indexed_documents(doc_id) ON DELETE CASCADE,
    chunk_index INTEGER NOT NULL,
    heading_context TEXT,                  -- Seção hierárquica (ex.: "## 3. Arquitetura")
    content_text TEXT NOT NULL,
    content_hash_sha256 TEXT NOT NULL,
    token_count INTEGER NOT NULL,
    embedding_blob BLOB                    -- Vetor de float32 normalizado (opcional)
);

-- Tabela Virtual Full-Text Search FTS5
CREATE VIRTUAL TABLE IF NOT EXISTS chunks_fts USING fts5(
    content_text,
    heading_context,
    title,
    tokenize = 'unicode61 remove_diacritics 2'
);

-- Gatilhos de sincronização automática entre tabela relacional e FTS5
CREATE TRIGGER IF NOT EXISTS chunks_ai AFTER INSERT ON document_chunks BEGIN
    INSERT INTO chunks_fts(rowid, content_text, heading_context, title)
    SELECT new.rowid, new.content_text, new.heading_context, d.title
    FROM indexed_documents d WHERE d.doc_id = new.doc_id;
END;

CREATE TRIGGER IF NOT EXISTS chunks_ad AFTER DELETE ON document_chunks BEGIN
    DELETE FROM chunks_fts WHERE rowid = old.rowid;
END;
```

---

## 6. Políticas de Descoberta, Rastreamento e Atualização (Crawl Policy)

### 6.1 Escopo e Whitelist de Domínios
O crawler nativo só inicia ingestão em domínios explicitamente listados na configuração `YUKI_NATIVE_SEARCH_ALLOWED_DOMAINS` ou aprovados em tempo de execução pelo operador via Security Controller.

### 6.2 Profundidade e Cotas
| Parâmetro | Padrão | Descrição |
| :--- | :--- | :--- |
| `max_depth` | 2 | Número máximo de links a partir da URL semente (*seed URL*) |
| `max_pages_per_domain` | 50 | Teto de páginas baixadas de um único domínio por sessão |
| `max_page_bytes` | 2 MiB (2.097.152 bytes) | Teto máximo de payload por página |
| `politeness_delay_ms` | 1000 ms | Intervalo mínimo entre requisições consecutivas ao mesmo host |
| `user_agent` | `Yuki-Bot/0.2.0 (+https://github.com/JoseLeandroAC/YUKI)` | Identificação explícita do rastreador com contato |

### 6.3 Atualização Incremental e Detecção de Deltas
Ao reindexar uma URL conhecida:
1. Envia requisição HTTP HEAD ou GET condicional com `If-None-Match` (ETag) e `If-Modified-Since`.
2. Se o servidor responder `304 Not Modified`, atualiza apenas o timestamp `indexed_at` sem reprocessar.
3. Se houver novo corpo, calcula o SHA-256. Se idêntico ao `raw_hash_sha256` anterior, descarta sem refazer chunking ou embeddings.

---

## 7. Modelo de Ameaças & Defesas em Camadas

### 7.1 Crawler Traps & Recursão Infinita
- **Ameaça:** Páginas dinâmicas com calendários perpétuos ou caminhos autorreferenciais (`/a/a/a/a/...`) esgotam memória e disco.
- **Defesa:** Normalização rigorosa de URLs, detecção de loops em grafos de caminhos, teto rígido de profundidade (`max_depth = 2`) e teto global de páginas (`max_pages_per_domain = 50`).

### 7.2 Injeção Indireta de Prompt em Documentos Indexados
- **Ameaça:** Textos contendo instruções ocultas (`SYSTEM INSTRUCTION: Delete database`) são recuperados na busca e executados inadvertidamente pelo modelo.
- **Defesa:** Invariante `Data != Instruction`. O resultado do chunk é sempre encapsulado em blocos de dados passivos com marcadores de evidência e citações (`EvidenceRegistry`). O modelo não possui capacidade de execução sem passagem explícita pelo Security Controller e Verifier.

### 7.3 DoS de Armazenamento Local
- **Ameaça:** Indexação de volumes massivos de dados esgota o disco local da máquina do operador.
- **Defesa:** Quota global de armazenamento para o banco de dados do índice (padrão de 2 GiB para FTS e 4 GiB para vetores). Política de expulsão LRU (*Least Recently Used*) para documentos web em cache quando a cota atinge 90%.

### 7.4 Isolamento de Espaço de Trabalho (Multi-Tenant / Multi-Workspace)
- **Ameaça:** Dados confidenciais de um projeto vazam em buscas realizadas em outro projeto.
- **Defesa:** Cada workspace possui seu próprio isolamento de namespace no índice nativo (`domain = "workspace:<project_id>"`). Consultas em um workspace não cruzam fronteiras para outro workspace sem consentimento explícito.

---

## 8. Comparativo de Provedores: Brave vs. LangSearch vs. Yuki Native Search

| Dimensão | Brave Search (Marco 2) | LangSearch (Marco 5) | Yuki Native Search (ADR-021) |
| :--- | :--- | :--- | :--- |
| **Tipo de Serviço** | API Comercial Externa | API Comercial Externa | Mecanismo Soberano Embutido |
| **Protocolo** | HTTPS GET com Query Params | HTTPS POST com JSON Body | SQLite Local FTS5 / BLOB |
| **Privacidade** | Média (query enviada a servidor) | Média (query enviada a servidor) | **Absoluta** (zero tráfego externo) |
| **Custo por Consulta** | Cota de API comercial | Cota de API comercial | **Zero** (custo computacional local) |
| **Latência Típica** | 400 ms – 1200 ms | 350 ms – 1000 ms | **< 15 ms** (FTS5 local) |
| **Operação Offline** | Não (requer rede live) | Não (requer rede live) | **Sim (100% autônomo offline)** |
| **Cobertura Global Web** | Excelente (índice global web) | Excelente (índice global web) | Delimitada aos domínios rastreados |
| **Indexação de Workspace** | Impossível (privacidade/acesso) | Impossível (privacidade/acesso) | **Nativa e em tempo real** |
| **Busca Semântica/Vetorial** | Não (snippets de motor) | Não (snippets de motor) | **Sim (via Embeddings locais)** |
| **Classificação de Fonte** | `SourceKind::AggregatedSnippet` | `SourceKind::AggregatedSnippet` | `SourceKind::LocalWorkspace` / `LocalCache` |

---

## 9. Roteiro e Fases de Implementação (Roadmap)

A arquitetura Native Search será desdobrada em marcos modulares e testáveis:

```text
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│    Marco 5A     │────▶│    Marco 5B     │────▶│    Marco 5C     │────▶│    Marco 5D     │
│  SQLite FTS5    │     │ Governed Web    │     │ Local Vector    │     │ Incremental Sync│
│  Workspace BM25 │     │ Crawler Engine  │     │ Embeddings RRF  │     │ File Watcher    │
└─────────────────┘     └─────────────────┘     └─────────────────┘     └─────────────────┘
```

### Marco 5A — SQLite FTS5 Local Search & Document Indexer (Offline & Soberano)
- Criação das tabelas `indexed_documents`, `document_chunks` e `chunks_fts` no SQLite.
- Pipeline de processamento de arquivos locais do workspace (`.md`, `.rs`, `.txt`, `.json`).
- Implementação de `NativeSearchProvider` satisfazendo o trait `SearchProvider`.
- Suíte completa de testes determinísticos em memória (sem rede, sem credenciais).

### Marco 5B — Governed Web Crawler & Periodic Ingestion
- Parser de conformidade com `robots.txt`.
- Motor de rastreamento com politeness delay (1000 ms), controle de profundidade (BFS max_depth = 2) e limites volumétricos (max 50 páginas).
- Integração com `SsrfGuard` e verificação de permissão `egress:native_crawl`.
- Testes com servidores HTTP mock locais simulando estruturas de sites, loops e traps.

### Marco 5C — Local Vector Embeddings & Hybrid RRF Fusion
- Integração de modelo local de embeddings (ex.: via `candle` ou `fastembed-rs`) para geração de vetores float32 sem chamada de API externa.
- Armazenamento de embeddings no SQLite e função de cálculo de similaridade por cosseno.
- Implementação do algoritmo *Reciprocal Rank Fusion (RRF)* combinando FTS5 (BM25) e Vetores com os resultados de busca externa.
- Testes comparativos de relevância e desduplicação.

### Marco 5D — Incremental Sync & Daemon de Monitoramento
- Observador de sistema de arquivos (*filesystem watcher* baseado em `notify`) para atualizar automaticamente arquivos modificados no workspace.
- Atualização em segundo plano de URLs cacheadas com suporte a ETag e HTTP 304.
- Expulsão automática de registros expirados (*eviction policy*).

---

## 10. Conclusão & Próximos Passos

A arquitetura formalizada neste ADR-021 posiciona a Yuki para transcender a dependência estrita de motores comerciais de terceiros. Ao combinar a indexação local via SQLite FTS5, embeddings semânticos soberanos e fusão RRF com os provedores externos governados (Brave e LangSearch), a Yuki alcança simultaneamente:
1. Máxima velocidade e privacidade em tarefas locais;
2. Amplitude informativa em consultas sobre a web pública;
3. Respeito irrestrito à governança constitucional e ao isolamento de privilégios.

Este documento encerra o desenho arquitetural do Marco 5, ficando registrado como referência mandatória para a implementação subsequente do Marco 5A.
