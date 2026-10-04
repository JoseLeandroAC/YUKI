# YUKI — COLD RESTORE PROCEDURE (DISASTER RECOVERY)

**Documento:** `docs/deployment/RESTORE.md`  
**Status:** ATIVO  
**Fase:** MVP-1 (Marco 4)  
**Última Atualização:** 2026-10-03  

---

## 1. Objetivo

Este documento define o procedimento oficial de **Cold Restore** (Restauração a Frio) da Yuki. Ele descreve os passos reproduzíveis para recompor o ambiente de execução e restaurar o estado operacional e de auditoria da Yuki a partir de um repositório limpo, garantindo:

1. **Zero Dependências Secretas ou Externas Ocultas:** A execução depende unicamente dos arquivos versionados no repositório e de uma toolchain Rust ou runtime OCI padrão.
2. **Imutabilidade e Recuperabilidade da Auditoria:** O banco SQLite `audit.db` pode ser restaurado e inspecionado a frio com integridade criptográfica e verificação de esquema.
3. **Fail-Closed Guarantee:** Caso o banco restaurado esteja corrompido ou pertença a uma versão futura incompatível, a Yuki recusa-se a operar silenciosamente e falha de forma segura.

---

## 2. Requisitos Mínimos

### Caminho A: Container OCI (Recomendado para Produção / VPS)
- Docker Engine 24+ ou Podman 4+
- Acesso à rede apenas para baixar a imagem base Debian Bookworm durante o build

### Caminho B: Compilação Nativa (Bare Metal / Dev Host)
- Rust Toolchain 1.98.1+ (rustc, cargo)
- Compilador C padrão (GCC / Clang) para compilação do SQLite embutido
- Git

---

## 3. Procedimento de Restauração Passo a Passo

### Passo 1: Obtenção do Código-Fonte
```bash
git clone https://github.com/JoseLeandroAC/YUKI.git
cd YUKI
git checkout feature/mvp-1-runtime
```

### Passo 2: Restauração da Base de Auditoria (Se houver backup existente)
Se estiver restaurando um backup prévio de auditoria durável:
```bash
mkdir -p /var/lib/yuki
cp /path/to/backup/audit.db /var/lib/yuki/audit.db
chmod 0600 /var/lib/yuki/audit.db
chown -R 10001:10001 /var/lib/yuki
```

*Nota:* Se nenhum backup for fornecido, a Yuki inicializa um novo banco de dados aplicando as migrações automáticas `V1__initial_audit_schema.sql`.

---

## 4. Implantação e Execução

### Opção A: Execução via Container OCI (Docker)
```bash
# 1. Compilação da imagem isolada multi-stage
docker build -t yuki:mvp-1 .

# 2. Execução do health-check diagnóstico
docker run --rm yuki:mvp-1 health

# 3. Execução contínua com persistência de auditoria montada
docker run -d \
  --name yuki-runtime \
  -v /var/lib/yuki:/var/lib/yuki \
  -e YUKI_ENVIRONMENT=production \
  -e RUST_LOG=info \
  yuki:mvp-1
```

### Opção B: Execução Nativa
```bash
# 1. Compilação otimizada em release
cargo build --release --bin yuki

# 2. Execução do health diagnostic
./target/release/yuki health

# 3. Execução de validação completa de testes offline
cargo test --all-targets
```

---

## 5. Verificação da Integridade Pós-Restauração

Após o cold restore, execute os seguintes passos de validação:

1. **Diagnóstico de Saúde do Sistema:**
   O comando `yuki health` deve reportar todos os subsistemas operacionais:
   - Model Provider (Mock ou Gemini): `Healthy`
   - Capability Registry: `Healthy` (`system.echo`, `system.time`, `system.info` registrados)
   - Persistent Audit Store: `Healthy` (escrita e leitura do SQLite ativas)

2. **Integridade de Esquema do SQLite:**
   O schema version registrado na tabela `schema_migrations` deve corresponder a `1`.
   Caso uma base corrompida seja carregada, o `MigrationManager` retornará `MigrationError::DatabaseCorrupted` e interromperá a inicialização.

3. **Verificação de Segredos:**
   Nenhum arquivo de configuração ou banco de dados contém segredos em texto plano. A variável `YUKI_GEMINI_API_KEY` deve ser injetada estritamente via runtime environment se o adapter externo for utilizado.

---

## 6. Procedimento de Simulação Limpa Validado (Marco 4)

O exercício de Cold Restore foi simulado com sucesso:
- Recompilação a frio a partir de `Cargo.lock` e `Cargo.toml` sem acesso a crates adicionais.
- Execução limpa de 120 testes automatizados passando 100% verde.
- Preservação e recuperação determinística do banco `audit.db` entre sessões demonstrada pelo teste `test_adv_29_persistence_restart_and_recovery`.
