# YUKI — DEPENDENCY INVENTORY

**Documento:** `docs/deployment/DEPENDENCIES.md`  
**Status:** ATIVO  
**Fase:** MVP-1 (Marco 3)
**Última Atualização:** 2026-10-03  
**Governança:** Engineering Rule — Regra Operacional de Inventário  

---

## 1. Diretriz de Governança

Nenhuma dependência externa pode ser introduzida no arquivo `Cargo.toml` sem estar catalogada neste documento com justificativa técnica estrita, análise de criticidade e rota de substituição / saída (*Exit Path*), em conformidade com o ADR-017 (External Platform Reuse & Vendor Independence).

---

## 2. Inventário de Crates do Rust (`Cargo.toml`)

### A. Dependências Adicionadas no MVP-1 (Marco 3)

#### 1. `rusqlite`
- **Requisito no `Cargo.toml`:** `0.40` (SemVer: `>= 0.40.0, < 0.41.0`)
- **Versão Resolvida no `Cargo.lock`:** `0.40.2` (com `libsqlite3-sys 0.38.2`)
- **Features Habilitadas:** `bundled`
- **Propósito:** Adaptador de persistência local SQLite embarcado (`SqliteAuditStore`) para registro durável de auditoria (`audit_events`) e resultados de verificação (`verification_records`).
- **Por que é necessária:** Garante persistência append-only em disco com modo WAL (`PRAGMA journal_mode = WAL`), durabilidade proporcional (`PRAGMA synchronous = NORMAL`), checagem preliminar de integridade (`PRAGMA quick_check(1)`), isolamento de concorrência com timeout (`PRAGMA busy_timeout = 5000`), paginação estável e triggers SQL defensivos contra mutação e deleção. A feature `bundled` compila o código-fonte C do SQLite 3 diretamente embutido no binário, eliminando dependência de bibliotecas dinâmicas do sistema operacional (`sqlite3.so` ou `sqlite3.dll`).
- **Requisitos de Compilação (Toolchain):** Requer compilador C compatível (`gcc` ou `clang`) invocado pelo build script de `libsqlite3-sys` via `cc-rs`. No ambiente de desenvolvimento local Windows GNU, foi provisionada a toolchain portátil `MartinStorsjo.LLVM-MinGW.MSVCRT` (versão `22.1.8-20260616`). No CI/Linux (`ubuntu-latest`), utiliza o `gcc` padrão já presente na imagem base.
- **Isolamento Arquitetural:** O `rusqlite` e `libsqlite3-sys` estão estritamente contidos no módulo `src/persistence/` (`src/persistence/sqlite.rs`, `src/persistence/migrations.rs`, `src/persistence/schema.rs`). Nenhuma estrutura ou tipo do `rusqlite` vaza para o núcleo da Yuki (`YukiCore`) ou para os contratos de domínio público (`contracts/`).
- **Exit Path / Substituição:** O port abstrato `AuditQueryStore` e a trait herdada da Foundation `EventStore` permitem substituição transparente por PostgreSQL, DuckDB, LMDB ou arquivo de log estruturado append-only (JSONL/parquet).

---

### B. Dependências Adicionadas no MVP-1 (Marco 2)

#### 2. `reqwest`
- **Requisito no `Cargo.toml`:** `^0.12` (SemVer: `>= 0.12.0, < 0.13.0`)
- **Versão Resolvida no `Cargo.lock`:** `0.12.4`
- **Features Habilitadas:** `default-tls`, `json`
- **Propósito:** Cliente HTTP assíncrono para o adaptador de provedor de modelo (`GeminiProviderAdapter`).
- **Por que é necessária:** Comunicação REST HTTPS com a API do Google Gemini (`generateContent`) com suporte a timeout de conexão, timeout total de requisição e parsing de JSON.
- **Justificativa do Recurso de TLS:** A feature `default-tls` utiliza a pilha de TLS nativa do sistema operacional através de `native-tls`: no Windows de desenvolvimento, utiliza SChannel (acessado via FFI direto para bibliotecas do SO sem necessidade de compilador C `gcc.exe` no PATH); no Linux (CI, containers OCI e servidores de produção), utiliza OpenSSL (requer `libssl-dev` para compilação e `libssl3` + `ca-certificates` em runtime). Isso viabilizou compilação local limpa no host Windows GNU sem toolchain C externa, mantendo compatibilidade padrão com o ambiente Linux de execução remota.
- **Isolamento Arquitetural:** O `reqwest` está restrito a `src/models/gemini.rs`. Tipos do `reqwest` (como `Client`, `Response`, `StatusCode`) não vazam para os contratos de domínio ou para o núcleo da Yuki.
- **Exit Path / Substituição:** Qualquer outro cliente HTTP assíncrono (ex: `hyper`, `surf`, `ureq` ou implementação baseada em sockets TLS brutos).

#### 3. `zeroize`
- **Requisito no `Cargo.toml`:** `^1.8` (SemVer: `>= 1.8.0, < 2.0.0`)
- **Versão Resolvida no `Cargo.lock`:** `1.8.1`
- **Features Habilitadas:** `derive`
- **Propósito:** Sobrescrita defensiva de memória para segredos transitórios (`SecretMaterial`) no momento do `Drop`.
- **Por que é necessária:** Executa a sobrescrita defensiva de buffers de memória diretamente gerenciados pela aplicação (`SecretMaterial`) no momento do descarte (`Drop`), reduzindo a janela de permanência de resíduos de segredos em memória sob controle direto da Yuki.
- **Isolamento Arquitetural:** O `zeroize` está restrito a `src/security/credentials.rs`.
- **Exit Path / Substituição:** Limpeza manual de memória via ponteiros voláteis (`std::ptr::write_volatile`).

---

### C. Dependências Adicionadas no MVP-1 (Marco 1)

#### 4. `tokio`
- **Requisito no `Cargo.toml`:** `^1.43` (SemVer: `>= 1.43.0, < 2.0.0`)
- **Versão Resolvida no `Cargo.lock`:** `1.43.0`
- **Features Habilitadas:** `rt-multi-thread`, `macros`, `signal`, `time`
- **Propósito:** Prover o runtime assíncrono multithread do processo, suporte a timers/timeouts não-bloqueantes e captura de sinais de interrupção do sistema operacional (`SIGINT`/`Ctrl+C`).
- **Por que é necessária:** A comunicação futura com modelos externos via HTTPS e a captura graciosa de sinais de encerramento do container requerem espera não-bloqueante de I/O.
- **Isolamento Arquitetural:** O Tokio está restrito a `src/main.rs`, às fronteiras de rede e timers. Tipos específicos do Tokio não vazam para as interfaces públicas do domínio Yuki Core.
- **Exit Path / Substituição:** Qualquer outro executor assíncrono Rust compatível com `std::future::Future` (ex: `async-std` ou `smol`).

---

### D. Dependências Transitivas e Restrição de Toolchain Local

#### `LOCAL TOOLCHAIN COMPATIBILITY CONSTRAINT` (Apenas no `Cargo.lock`)
- **Crates Envolvidos:** `mio` (resolvido em `1.0.3`), `hyper-util` (`0.1.3`), `native-tls` (`0.2.12`), `schannel` (`0.1.23`), e `windows-sys` (`0.52.0` / `0.48.0` com `windows-targets`).
- **Contexto Técnico:** Na compilação local sob o target `x86_64-pc-windows-gnu` em caminhos de diretório contendo caracteres acentuados (ex: `Miriã`, `JOSÉ`), versões mais recentes do `windows-sys` (`>= 0.59`) geram bibliotecas de importação dinamicamente invocando `dlltool.exe`, o que provoca falha interna de `CreateProcess` no MinGW-w64.
- **Resolução Adotada:** O `Cargo.lock` trava transitivamente as versões do ecossistema de rede em lançamentos que utilizam `windows-sys 0.52.0/0.48.0`. Essas versões utilizam bibliotecas estáticas `.a` pré-compiladas em `windows-targets`, eliminando a necessidade de chamar `dlltool.exe`.
- **Natureza:** É estritamente uma restrição de compatibilidade transitiva do ambiente de compilação local Windows GNU no lockfile. Esses crates **NÃO** são dependências diretas do `Cargo.toml`.
- **Condição de Saída:** Compilação em ambiente Linux / container OCI (onde `windows-sys` não é compilado), migração para toolchain MSVC, ou atualização futura com correção no MinGW upstream.

---

### E. Dependências Herdadas da Foundation v0.1 (MVP-0)

#### 5. `serde`
- **Requisito no `Cargo.toml`:** `^1.0`
- **Versão Resolvida no `Cargo.lock`:** `1.0.219`
- **Features Habilitadas:** `derive`
- **Propósito:** Framework canônico para serialização e desserialização de estruturas de dados em Rust.
- **Por que é necessária:** Serialização e deserialização de contratos, identificadores, requisições e eventos.
- **Exit Path / Substituição:** Framework padrão da indústria no ecossistema Rust; substituição teórica exigiria implementação manual de codecs.

#### 6. `serde_json`
- **Requisito no `Cargo.toml`:** `^1.0`
- **Versão Resolvida no `Cargo.lock`:** `1.0.140`
- **Features Habilitadas:** Padrão
- **Propósito:** Manipulação estruturada de objetos e valores JSON.
- **Por que é necessária:** Representação interoperável de payloads de contexto, argumentos de capacidades e metadados de eventos.
- **Exit Path / Substituição:** `simd-json` para maior performance ou outro formato binário (MessagePack, CBOR).

#### 7. `thiserror`
- **Requisito no `Cargo.toml`:** `^2.0`
- **Versão Resolvida no `Cargo.lock`:** `2.0.12`
- **Features Habilitadas:** Padrão
- **Propósito:** Geração ergonômica e tipada da trait `std::error::Error` para enums de domínio.
- **Por que é necessária:** Garante tipagem estrita de erros sem converter falhas em strings livres genéricas.
- **Exit Path / Substituição:** Implementação manual de `std::fmt::Display` e `std::error::Error`.

#### 8. `chrono`
- **Requisito no `Cargo.toml`:** `^0.4`
- **Versão Resolvida no `Cargo.lock`:** `0.4.40`
- **Features Habilitadas:** `default-features = false`, `features = ["std", "serde"]`
- **Propósito:** Manipulação de datas, fusos horários e timestamps em formato padrão ISO 8601 / RFC 3339.
- **Por que é necessária:** Registro temporal preciso e imutável para eventos de auditoria e tolerância temporal.
- **Exit Path / Substituição:** `time` crate ou `std::time::SystemTime`.

#### 9. `clap`
- **Requisito no `Cargo.toml`:** `^4.5`
- **Versão Resolvida no `Cargo.lock`:** `4.5.31`
- **Features Habilitadas:** `default-features = false`, `features = ["derive", "std", "help", "usage", "error-context"]`
- **Propósito:** Parsing de linha de comando para a interface CLI da Yuki.
- **Por que é necessária:** Permite a interação com comandos como `yuki health` e passagem de prompts na linha de comando.
- **Exit Path / Substituição:** `lexopt` ou parsing manual via `std::env::args`.

#### 10. `tracing`
- **Requisito no `Cargo.toml`:** `^0.1`
- **Versão Resolvida no `Cargo.lock`:** `0.1.41`
- **Features Habilitadas:** Padrão
- **Propósito:** Instrumentação estruturada e contextualizada de eventos de observabilidade em tempo de execução.
- **Por que é necessária:** Diagnóstico e telemetria operacional sem acoplamento a arquivos ou destinos de log específicos.
- **Exit Path / Substituição:** `log` crate padrão ou sistema de telemetria OpenTelemetry.

#### 11. `tracing-subscriber`
- **Requisito no `Cargo.toml`:** `^0.3`
- **Versão Resolvida no `Cargo.lock`:** `0.3.19`
- **Features Habilitadas:** `default-features = false`, `features = ["fmt"]`
- **Propósito:** Formatação e despacho de spans e logs de `tracing` para `stdout`/`stderr`.
- **Por que é necessária:** Visualização legível de traces operacionais em desenvolvimento e formato estruturado em produção.
- **Exit Path / Substituição:** Qualquer subscriber customizado ou `env_logger`.

---

## 3. Ações Externas do CI (`.github/workflows/ci.yml`)

| Action | Versão / Tag | Mantenedor | Propósito | Permissões Requeridas |
|---|---|---|---|---|
| `actions/checkout` | `v4` | GitHub Oficial | Clonar o repositório para o runner limpo | `contents: read` |
| `dtolnay/rust-toolchain` | `stable` | David Tolnay (Rust Libs) | Instalação determinística do compilador Rust, Clippy e Rustfmt | Nenhuma permissão extra |

*Nota de Endurecimento:* Para o MVP-1, as tags oficiais documentadas são utilizadas. O pin estrito por commit SHA imutável poderá ser adicionado em fases posteriores de endurecimento de infraestrutura.

---

## 4. Dependências Previstas para os Próximos Marcos (Ainda NÃO Adicionadas)

As seguintes dependências estão catalogadas para avaliação mas **NÃO** foram adicionadas no Marco 3:

- `toml` (Marco 4 — Parser de arquivo de configuração estruturado em disco `config/yuki.toml`)
- `sha2` (Opcional / Futuro — Hashing criptográfico adicional se requerido por novos adapters)
