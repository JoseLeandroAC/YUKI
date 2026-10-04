# YUKI — CONFIGURATION SPECIFICATION

**Documento:** `docs/deployment/CONFIGURATION.md`  
**Status:** ATIVO  
**Fase:** MVP-1 (Marco 1)  
**Última Atualização:** 2026-10-04  

---

## 1. Princípio da Configuração Hierárquica

A Yuki adota uma hierarquia estrita de resolução de configurações em três camadas, inspirada no padrão *12-Factor App*:

```text
Valores Padrão no Código (Hardcoded Defaults)
                      ↓
Arquivo de Configuração Local (`config/yuki.toml`)
                      ↓
Variáveis de Ambiente do Sistema Operacional (`YUKI_*`)
```

- Se um parâmetro não for explicitado no arquivo ou no ambiente, o valor padrão seguro do código é assumido.
- Uma variável de ambiente sempre sobrescreve a chave correspondente presente no arquivo `config/yuki.toml`.

---

## 2. Invariante de Segurança: Zero Secrets em Arquivos

> **Regra Absoluta:** O arquivo `config/yuki.toml` destina-se unicamente a configurações operacionais não-sensíveis (portas, caminhos locais, timeouts, níveis de log e nomes de modelo).
>
> Chaves de API, tokens e senhas **NUNCA** devem ser inseridos em arquivos `.toml`. Credenciais são entregues exclusivamente via variáveis de ambiente ou referências formais `SecretRef` resolvidas pelo `CredentialBroker`.

---

## 3. Parâmetros de Configuração

### Seção `[runtime]`
- `environment`: Nome do ambiente (`"development"`, `"test"`, `"production"`). Padrão: `"development"`.
- `request_timeout_ms`: Tempo limite máximo em milissegundos para um turno de processamento completo da Yuki. Padrão: `30000` (30s).
- `graceful_shutdown_timeout_ms`: Tempo de carência para finalização de tarefas ao receber `SIGINT`/`SIGTERM`. Padrão: `5000` (5s).

### Seção `[logging]`
- `level`: Nível mínimo de log (`"trace"`, `"debug"`, `"info"`, `"warn"`, `"error"`). Padrão: `"info"`.
- `format`: Formato de saída dos traces (`"text"` para desenvolvimento interativo, `"json"` para containers/produção). Padrão: `"text"`.

### Seção `[storage]` (Previsto para Marco 3)
- `database_path`: Caminho do arquivo SQLite do EventStore. Padrão: `"data/yuki.db"`.
- `sqlite_synchronous`: Modo de sincronização do SQLite (`"NORMAL"` ou `"FULL"`). Padrão: `"NORMAL"`.

### Seção `[model]`
- `provider`: Provedor ativo (`"mock"` ou `"gemini"`). Padrão: `"mock"`. Controlado via `YUKI_MODEL_PROVIDER`.
- `model_id`: Identificador do modelo upstream (ex: `"gemini-3.8-flash"`). Padrão compilado: `"gemini-3.8-flash"`. Controlado via `YUKI_MODEL_ID`. Sanitiza automaticamente prefixos redundantes (`models/`) e aspas.
- `api_version`: Versão da API REST upstream (ex: `"v1beta"`, `"v1"`). Padrão compilado: `"v1beta"`. Controlado via `YUKI_API_VERSION`.
- `max_tool_iterations`: Número máximo de iterações do loop de continuação governada de ferramentas por turno conversacional. Previne ciclos infinitos e consumo descontrolado de tokens. Padrão compilado: `5`. Controlado via `YUKI_MAX_TOOL_ITERATIONS`.
- `temperature`: Parâmetro de aleatoriedade de amostragem. Padrão: `0.2`.
- `timeout_ms`: Timeout específico para a chamada HTTP do provedor. Padrão: `30000` (30s). Controlado via `YUKI_REQUEST_TIMEOUT_MS`.

---

## 4. Mapeamento de Variáveis de Ambiente

| Chave TOML | Variável de Ambiente Correspondente | Exemplo de Valor |
|---|---|---|
| `runtime.environment` | `YUKI_ENV` | `production` |
| `runtime.request_timeout_ms` | `YUKI_REQUEST_TIMEOUT_MS` | `45000` |
| `logging.level` | `YUKI_LOG_LEVEL` | `debug` |
| `logging.format` | `YUKI_LOG_FORMAT` | `json` |
| `storage.database_path` | `YUKI_DATABASE_PATH` | `/var/lib/yuki/data/yuki.db` |
| `storage.sqlite_synchronous` | `YUKI_SQLITE_SYNCHRONOUS` | `NORMAL` |
| `model.provider` | `YUKI_MODEL_PROVIDER` | `gemini` |
| `model.model_id` | `YUKI_MODEL_ID` | `gemini-3.8-flash` |
| `model.api_version` | `YUKI_API_VERSION` | `v1beta` |
| `model.max_tool_iterations` | `YUKI_MAX_TOOL_ITERATIONS` | `5` |
| *(Segredo Externo)* | `YUKI_GEMINI_API_KEY` | *(Material bruto do segredo)* |
