# YUKI — ENVIRONMENT BASELINE

**Documento:** `docs/deployment/00_ENVIRONMENT_BASELINE.md`  
**Status:** ATIVO  
**Fase:** MVP-1 (Marco 1)  
**Última Atualização:** 2026-10-03  
**Verificado Por:** Yuki Architecture Review  

---

## 1. Visão Geral

Este documento estabelece o inventário formal e soberano de tecnologias, ferramentas de compilação, plataformas-alvo e requisitos de sistema necessários para construir, testar, empacotar e operar a Yuki a partir de um estado limpo.

Em conformidade com a **Regra Operacional de Inventário**, nenhuma tecnologia ou ferramenta pode ser utilizada sem registro explícito neste baseline.

---

## 2. Toolchain e Compilador

| Componente | Versão Especificada | Origem / Gerenciador | Propósito | Criticidade |
|---|---|---|---|---|
| **Rust Toolchain** | `1.98.1` (Edition 2021) | `rustup` | Compilação estrita e verificação de tipos do núcleo Yuki | Primária / Obrigatória |
| **Cargo** | `1.98.1` | Distribuído com Rust | Gerenciamento de dependências, builds e suítes de testes | Primária / Obrigatória |
| **Clippy** | `0.1.98` | Componente Rustup | Análise estática de código com política `-D warnings` | Primária / Obrigatória |
| **Rustfmt** | `1.8.0` | Componente Rustup | Formatação determinística de código | Primária / Obrigatória |
| **Target de Desenvolvimento** | `x86_64-pc-windows-gnu` | MinGW-w64 / rustup | Target local na máquina de desenvolvimento | Transitória (Janela ~3m) |
| **Linker de Desenvolvimento** | `rust-lld` | Rust bundled | Linkagem rápida de binários no Windows | Otimização |
| **Target de Produção / CI** | `x86_64-unknown-linux-gnu` | Linux libc / GCC | Target padrão para CI e execução remota / container | Primária / Durável |

---

## 3. Requisitos Mínimos de Sistema

### Desenvolvimento Local (Windows GNU)
- **SO:** Windows 10/11 x86_64
- **CPU:** 2 núcleos ou superior
- **Memória RAM:** 4 GB mínimo (8 GB recomendado para compilação rápida)
- **Disco:** 2 GB livres para cache de `target/` e crates
- **Git:** Git for Windows (executável no PATH ou via GitHub Desktop)

### Produção / Container / CI (Linux)
- **SO:** Linux Kernel 5.4+ (Debian 12 Bookworm, Ubuntu 22.04/24.04 ou Alpine)
- **CPU:** 1 vCPU (mínimo)
- **Memória RAM:** 512 MB (mínimo para execução do runtime)
- **Disco:** 500 MB livres para binário, logs e banco SQLite local

---

## 4. Variáveis de Ambiente Suportadas

Nenhum segredo real é registrado neste arquivo. Consulte `.env.example` para referências.

| Variável | Tipo | Valor Padrão | Propósito |
|---|---|---|---|
| `YUKI_ENV` | `string` | `development` | Identificador do ambiente (`development`, `test`, `production`) |
| `YUKI_CONFIG_PATH` | `path` | `config/yuki.toml` | Caminho do arquivo de configuração do runtime |
| `YUKI_LOG_LEVEL` | `string` | `info` | Nível de emissão de logs (`trace`, `debug`, `info`, `warn`, `error`) |
| `YUKI_LOG_FORMAT` | `string` | `text` | Formato dos logs (`text` para dev, `json` para produção/container) |
| `YUKI_DATABASE_PATH` | `path` | `data/yuki.db` | Caminho do arquivo de persistência SQLite do EventStore |
| `YUKI_GEMINI_API_KEY` | `secret` | *(vazio)* | Chave de API externa para o adapter Google Gemini (Marco 2) |

---

## 5. Serviços Externos e Conectividade de Rede

No Marco 1, a Yuki opera com **zero conexões de rede externas**.

| Serviço | Protocolo | Porta | Endpoint | Necessário no Marco 1? |
|---|---|---|---|---|
| **Google Gemini API** | HTTPS / TLS 1.3 | 443 | `generativelanguage.googleapis.com` | **NÃO** (Introduzido no Marco 2) |
| **GitHub Actions** | HTTPS / Git | 443 | `github.com/JoseLeandroAC/YUKI` | **SIM** (Para validação contínua remota) |

---

## 6. Procedimento de Verificação de Baseline em Máquina Limpa

Para validar se uma máquina atende a este baseline:

```bash
# 1. Verificar versões do toolchain
rustc --version
cargo --version

# 2. Verificar integridade do workspace
cargo check --all-targets
cargo test --all-targets
```
