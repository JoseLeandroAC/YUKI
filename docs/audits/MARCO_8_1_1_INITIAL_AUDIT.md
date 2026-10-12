# YUKI — RELATÓRIO DE AUDITORIA INICIAL DO MARCO 8.1.1
## Falhas Comprovadas, Auditoria de Segurança e Plano de Correção Estrutural

**Projeto:** YUKI — Personal AI Platform  
**Repositório:** `JoseLeandroAC/YUKI`  
**Baseline de Entrada:** `bd588e5` (Marco 8.1 final)  
**Branch de Trabalho:** `feature/marco-8.1.1-homologation-fixes`  
**Data:** 12 de Outubro de 2026  
**Auditor/Engenheiro:** Antigravity (Google DeepMind Agentic Assistant)  
**Status do Marco 8.1:** **NÃO HOMOLOGADO** (Auditoria Independente Externa Identificou Falhas Críticas de Segurança)

---

## 1. Contexto e Motivação

O Marco 8.1 implementou drivers Win32, isolamento de subprocessos e 57 testes adversariais. Contudo, a auditoria externa independente comprovou vulnerabilidades graves que impedem a homologação:

1. **Bypass Crítico na Autenticação de Aprovações:** A verificação de token em `grant_approval_authenticated` utiliza `if let (Some(expected), Some(provided))`, permitindo que passar `auth_token = None` simplesmente ignore a verificação de token!
2. **Consumo Não-Atômico de Aprovações:** O `MissionRunner` lê a aprovação e, após etapas assíncronas, atualiza o status para `Consumed` em chamada separada, criando uma janela de corrida (TOCTOU) onde a mesma aprovação pode ser reutilizada por execuções concorrentes.
3. **Lease de Missão Apenas em Memória:** `active_leases: Arc<Mutex<HashSet<String>>>` protege apenas tarefas concorrentes dentro do mesmo processo. Dois processos distintos ou instâncias separadas conectadas ao mesmo SQLite executam a mesma missão simultaneamente sem bloqueio, e não há fencing tokens para impedir que executores atrasados continuem mutando o estado.
4. **Falsa Alegação de Wake Word Lexical:** O `PhoneticPatternWakeWordEngine` baseia-se em RMS, ZCR e heurísticas de energia. Não realiza reconhecimento fonético ou lexical confiável da palavra "Yuki", gerando falsos positivos e falsos negativos inaceitáveis. A wake word precisa ser reclassificada honestamente e o VAD completamente dissociado de modelos de transcrição local.
5. **Ciclo de Vida de Reprodução WinMM Incompleto:** `PlaySoundA` com `SND_ASYNC` apenas aceita a chamada, não rastreia conclusão de reprodução nem gerencia seguramente o buffer durante a reprodução no driver.
6. **Lacunas na Política de Capacidades e Verificação:** Capacidades desconhecidas podem não falhar fechado com segurança; recuperação de passos em `WaitingApproval` não garante ausência de efeitos mutadores executados antes da falha.

---

## 2. Inventário de Falhas e Plano de Ação Estrutural

### 2.1. Falha Crítica 1 — Autenticação de Aprovações (`src/missions/approval.rs`)
- **Problema:**
  ```rust
  if let (Some(expected_token), Some(provided)) =
      (&approval.confirmation_token, auth_token)
  {
      if provided != expected_token {
          return Err(MissionError::ApprovalMismatch("Token de confirmação inválido".to_string()));
      }
  }
  ```
  Se o atacante passar `auth_token = None`, a condição é falsa e o código aprova a missão sem conferir nada!
  Além disso, a API legada `grant_approval` permite conceder aprovações sem token algum.
- **Plano de Correção:**
  1. Se a aprovação possui `confirmation_token`, a passagem de um token válido é **obrigatória**.
  2. Tokens ausentes (`None`), vazios (`""`), com tamanho incorreto ou expirados devem retornar erro `MissionError::ApprovalMismatch` imediatamente.
  3. Utilizar comparação de tempo constante para comparação do segredo de confirmação (evitando timing attacks).
  4. Restringir a API legada `grant_approval`: torná-la obsoleta ou exigir autenticação obrigatória internamente.
  5. Vincular a identidade do operador: operador não pode ser vazio nem "anonymous".
  6. Não expor tokens em claro em logs ou exceções.

### 2.2. Falha Crítica 2 — Consumo Atômico de Aprovações (`src/missions/runner.rs`, `src/missions/store.rs`)
- **Problema:** O runner faz `store.get_approval(id)` -> validação -> executa -> `store.save_approval(Consumed)`. Entre a validação e a gravação, outra thread ou processo pode validar e executar a mesma aprovação.
- **Plano de Correção:**
  1. Implementar método transacional atômico no `SqliteMissionStore`: `consume_approval_atomic(approval_id, mission_id, step_id, operator_id, params_hash) -> Result<MissionApproval, MissionError>`.
  2. Executar via transação SQLite com `UPDATE mission_approvals SET status = 'consumed', consumed_at = ? WHERE id = ? AND status = 'granted' AND mission_id = ? AND expires_at > ? AND (confirmation_token IS NOT NULL)`.
  3. Verificar `rows_affected == 1`. Se for 0, rejeitar como já consumida, expirada ou inválida.
  4. Consumir a aprovação **antes** de disparar o efeito colateral, ou amarrar o consumo a uma transação com fencing.

### 2.3. Falha Alta 3 — Exclusão Entre Instâncias e Processos via Lease Persistente e Fencing Token (`src/missions/store.rs`, `src/missions/runner.rs`)
- **Problema:** `Arc<Mutex<HashSet<String>>>` não existe entre processos.
- **Plano de Correção:**
  1. Criar tabela SQLite `mission_leases (mission_id TEXT PRIMARY KEY, owner_id TEXT NOT NULL, fence_token INTEGER NOT NULL, acquired_at TEXT NOT NULL, expires_at TEXT NOT NULL)`.
  2. Métodos no store: `acquire_mission_lease(mission_id, owner_id, ttl_secs) -> Result<u64, MissionError>` (retorna o fencing token).
  3. `renew_mission_lease(mission_id, owner_id, fence_token, ttl_secs)` e `release_mission_lease(mission_id, owner_id, fence_token)`.
  4. Cada checkpoint e gravação de passo deve validar o `fence_token` para evitar que um executor lento continue operando após ter perdido o lease.
  5. Suporte a recuperação de leases abandonados (após expiração do TTL).

### 2.4. Falha Alta 4 — Wake Word Verdadeira vs Honestidade Arquitetural (`src/voice/wakeword.rs`)
- **Problema:** O motor chamava RMS/ZCR de "reconhecimento fonético".
- **Plano de Correção:**
  1. Separar o VAD (atividade de voz) do motor lexical.
  2. Implementar `SttWakeWordEngine` baseado no `SpeechToTextProvider` local (Whisper) para validação lexical real da palavra "Yuki".
  3. Se nenhum provedor de STT local estiver configurado ou disponível, classificar a Wake Word explicitamente como `WakeWordEngineStatus::Unavailable` ou `Experimental`, desativando a ativação automática cega e priorizando o Push-to-Talk.
  4. Nunca afirmar que a heurística de energia é reconhecimento lexical.
  5. Adicionar testes com amostras de fala e ruído reais e documentadas.

### 2.5. Falha Média 5 — Ciclo de Vida da Reprodução (`src/voice/playback.rs`)
- **Problema:** `PlaySoundA(SND_ASYNC)` retorna imediatamente; o controller não sabe quando a fala terminou e pode desalocar o buffer de memória enquanto o driver toca.
- **Plano de Correção:**
  1. Definir máquina de estados clara: `Idle`, `Playing`, `Completed`, `Interrupted`, `Failed`.
  2. Para `WindowsAudioPlaybackSink`, manter o buffer de áudio fixo em memória (`Arc<Vec<u8>>`) garantindo que ele não seja desalocado durante a reprodução assíncrona.
  3. Monitorar a duração estimada do áudio e atualizar atomicamente para `Completed` quando o tempo expirar sem interrupção.
  4. Medição real e documentação honesta da latência de cancelamento.

### 2.6. Auditoria Complementar
- **A. Capacidades Desconhecidas:** Garantir que `step_requires_approval()` retorne `true` (fail-closed) para qualquer capacidade não registrada no catálogo.
- **B. Recuperação de Missões:** Etapas interrompidas com efeitos desconhecidos devem requerer reconciliação antes de reexecução.
- **C. Integridade SQLite:** Verificação de integridade referencial PRAGMA foreign_keys = ON em todas as conexões abertas.

---

## 3. Estratégia de Execução Incremental

- **Etapa 1:** Correção da autenticação de aprovações com tokens obrigatórios e timing-safe comparison.
- **Etapa 2:** Consumo atômico condicional de aprovações no SQLite.
- **Etapa 3:** Leases persistentes no SQLite com Fencing Tokens e mutual exclusion entre processos.
- **Etapa 4:** Reclassificação honesta da Wake Word, separação de VAD e integração a STT local.
- **Etapa 5:** Máquina de estados robusta e gerenciamento de buffers no playback.
- **Etapa 6:** Resolução da auditoria complementar (capacidades desconhecidas, PRAGMA foreign_keys).
- **Etapa 7:** Suíte de testes adversariais ampliada demonstrando falhas prévias e correções.
- **Etapa 8:** Quality gates e relatório final para submissão à auditoria independente.
