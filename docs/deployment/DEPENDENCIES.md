# YUKI — DEPENDENCY INVENTORY

**Documento:** `docs/deployment/DEPENDENCIES.md`  
**Status:** ATIVO  
**Fase:** MVP-1 (Marco 1)  
**Última Atualização:** 2026-10-03  
**Governança:** Engineering Rule — Regra Operacional de Inventário  

---

## 1. Diretriz de Governança

Nenhuma dependência externa pode ser introduzida no arquivo `Cargo.toml` sem estar catalogada neste documento com justificativa técnica estrita, análise de criticidade e rota de substituição / saída (*Exit Path*), em conformidade com o ADR-017 (External Platform Reuse & Vendor Independence).

---

## 2. Inventário de Crates do Rust (`Cargo.toml`)

### A. Dependências Adicionadas no MVP-1 (Marco 1)

#### 1. `tokio`
- **Versão:** `1.43`
- **Features Habilitadas:** `rt-multi-thread`, `macros`, `signal`, `time`
- **Propósito:** Prover o runtime assíncrono multithread do processo, suporte a timers/timeouts não-bloqueantes e captura de sinais de interrupção do sistema operacional (`SIGINT`/`Ctrl+C`).
- **Por que é necessária:** A comunicação futura com modelos externos via HTTPS e a captura graciosa de sinais de encerramento do container requerem espera não-bloqueante de I/O.
- **Isolamento Arquitetural:** O Tokio está restrito a `src/main.rs`, às fronteiras de rede e timers. Tipos específicos do Tokio não vazam para as interfaces públicas do domínio Yuki Core.
- **Exit Path / Substituição:** Qualquer outro executor assíncrono Rust compatível com `std::future::Future` (ex: `async-std` ou `smol`).

---

### B. Dependências Herdadas da Foundation v0.1 (MVP-0)

#### 2. `serde`
- **Versão:** `1.0`
- **Features Habilitadas:** `derive`
- **Propósito:** Framework canônico para serialização e desserialização de estruturas de dados em Rust.
- **Por que é necessária:** Serialização e deserialização de contratos, identificadores, requisições e eventos.
- **Exit Path / Substituição:** Framework padrão da indústria no ecossistema Rust; substituição teórica exigiria implementação manual de codecs.

#### 3. `serde_json`
- **Versão:** `1.0`
- **Features Habilitadas:** Padrão
- **Propósito:** Manipulação estruturada de objetos e valores JSON.
- **Por que é necessária:** Representação interoperável de payloads de contexto, argumentos de capacidades e metadados de eventos.
- **Exit Path / Substituição:** `simd-json` para maior performance ou outro formato binário (MessagePack, CBOR).

#### 4. `thiserror`
- **Versão:** `2.0`
- **Features Habilitadas:** Padrão
- **Propósito:** Geração ergonômica e tipada da trait `std::error::Error` para enums de domínio.
- **Por que é necessária:** Garante tipagem estrita de erros sem converter falhas em strings livres genéricas.
- **Exit Path / Substituição:** Implementação manual de `std::fmt::Display` e `std::error::Error`.

#### 5. `chrono`
- **Versão:** `0.4`
- **Features Habilitadas:** `default-features = false`, `features = ["std", "serde"]`
- **Propósito:** Manipulação de datas, fusos horários e timestamps em formato padrão ISO 8601 / RFC 3339.
- **Por que é necessária:** Registro temporal preciso e imutável para eventos de auditoria e tolerância temporal.
- **Exit Path / Substituição:** `time` crate ou `std::time::SystemTime`.

#### 6. `clap`
- **Versão:** `4.5`
- **Features Habilitadas:** `default-features = false`, `features = ["derive", "std", "help", "usage", "error-context"]`
- **Propósito:** Parsing de linha de comando para a interface CLI da Yuki.
- **Por que é necessária:** Permite a interação com comandos como `yuki health` e passagem de prompts na linha de comando.
- **Exit Path / Substituição:** `lexopt` ou parsing manual via `std::env::args`.

#### 7. `tracing`
- **Versão:** `0.1`
- **Features Habilitadas:** Padrão
- **Propósito:** Instrumentação estruturada e contextualizada de eventos de observabilidade em tempo de execução.
- **Por que é necessária:** Diagnóstico e telemetria operacional sem acoplamento a arquivos ou destinos de log específicos.
- **Exit Path / Substituição:** `log` crate padrão ou sistema de telemetria OpenTelemetry.

#### 8. `tracing-subscriber`
- **Versão:** `0.3`
- **Features Habilitadas:** `default-features = false`, `features = ["fmt"]`
- **Propósito:** Formatação e despacho de spans e logs de `tracing` para `stdout`/`stderr`.
- **Por que é necessária:** Visualização legível de traces operacionais em desenvolvimento e formato estruturado em produção.
- **Exit Path / Substituição:** Qualquer subscriber customizado ou `env_logger`.

---

## 3. Dependências Previstas para os Próximos Marcos (Ainda NÃO Adicionadas)

As seguintes dependências estão aprovadas na especificação mas **NÃO** foram adicionadas no Marco 1:

- `reqwest` (Marco 2 — Cliente HTTP para o Model Gateway)
- `rusqlite` (Marco 3 — Adaptador de persistência local SQLite)
- `toml` (Marco 2/3 — Parser de arquivo de configuração)
- `uuid` (Marco 2/3 — Geração de identificadores únicos universais v4)
- `sha2` (Marco 2/3 — Hashing determinístico independente de argumentos)
