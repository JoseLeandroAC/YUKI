# YUKI — MARCO 8: PLANO TÉCNICO DE ENGENHARIA
## Voice v1 & Missions Foundation

**Data:** 2026-10-11  
**Branch:** `feature/marco-8-voice-missions`  
**Baseline auditada:** Commit `961be2c` (Marco 7.2 final)  
**Status:** Fase Zero Concluída — Pronto para Implementação Incremental  

---

## 1. INVENTÁRIO DOS COMPONENTES EXISTENTES

A Yuki possui uma fundação arquitetural madura desenvolvida e verificada nos Marcos 1 a 7.2:

1. **Core & Pipeline Governado (`src/core/yuki_core.rs`):**
   - Fluxo auditado: `UserInput → PersonalContext → PromptBudget → ModelProvider → ModelProposal → SecurityController → ExecutionEngine → VerificationEngine → AuditStore`.
   - Limites ontológicos e separação: `Data != Instruction != Context != Evidence != Verification`.
2. **Sistema de Auditoria e Persistência (`src/persistence/`, `src/audit/`):**
   - `SqliteAuditStore` com migrations versionadas, integridade relacional e tabela de eventos imutáveis.
   - Eventos de auditoria em `src/contracts/events.rs` já incluem variantes reservadas: `MissionStarted`, `MissionStepCompleted`, `MissionApprovalRequested`, `MissionApprovalGranted`, `MissionApprovalDenied`, `MissionCompleted`, `MissionFailed`, `VoiceInputReceived`, `VoiceOutputProduced`.
3. **Cofre de Credenciais e Segurança (`src/security/vault/`):**
   - `CredentialVault` com criptografia AES-256-GCM, KDF PBKDF2 (100k iterações), canary upfront, exclusão mútua por lock de SO (`share_mode(0)` no Windows / `flock` no Unix) e controle de permissões nativas (`icacls` / `0o600`).
   - Broker de credenciais desacoplado para resolução de chaves governadas (`egress:voice_cloud`).
4. **Memória e Contexto Pessoal (`src/memory/`, `src/context/`):**
   - SQLite FTS5 BM25 para recuperação de memórias sem fallback cego.
   - `PromptBudgetManager` governando 6 tiers de prompt, deduzindo declarações de ferramentas e truncando com limites matemáticos estritos sem pisos arbitrários.
5. **Personalidade (`src/personality/`):**
   - Perfil estável de 3 camadas (`Yuki Default`), salvaguardas ontológicas (não simular estados biológicos) e adaptação contextual via `ContextUrgency`.
6. **Capabilities e Verificação (`src/capabilities/`, `src/verification/`):**
   - Registro de ferramentas (`CapabilityRegistry`) e verificação dinâmica pós-execução (`VerificationEngine`), diferenciando estritamente execução de efeito verificado.

---

## 2. MAPA DE DEPENDÊNCIAS

```
                                  ┌───────────────────────────┐
                                  │      Interface do         │
                                  │   Operador (CLI / Voz)    │
                                  └─────────────┬─────────────┘
                                                │
                 ┌──────────────────────────────┴──────────────────────────────┐
                 ▼                                                             ▼
   ┌───────────────────────────┐                                 ┌───────────────────────────┐
   │       Voice Subsystem     │                                 │     Missions Subsystem    │
   │  (Audio / STT / TTS / VAD)│                                 │   (Planner / DAG / Policy)│
   └─────────────┬─────────────┘                                 └─────────────┬─────────────┘
                 │ (Áudio decodificado e sanitizado)                           │ (Passos governados)
                 ▼                                                             ▼
   ┌─────────────────────────────────────────────────────────────────────────────────────────┐
   │                                   YukiCore Pipeline                                     │
   │  - PromptBudgetManager          - ContextBuilder            - PersonalityAdapter        │
   │  - SecurityController           - CapabilityRegistry        - ExecutionEngine           │
   └────────────────────────────────────────────┬────────────────────────────────────────────┘
                                                │
                                                ▼
   ┌─────────────────────────────────────────────────────────────────────────────────────────┐
   │                                Camada de Persistência                                   │
   │  - SqliteAuditStore (Eventos)                               - SqliteMissionStore (DAG)  │
   │  - SqliteMemoryStore (FTS5)                                 - CredentialVault (Segredos)│
   └─────────────────────────────────────────────────────────────────────────────────────────┘
```

---

## 3. DECISÕES TECNOLÓGICAS JUSTIFICADAS

### 3.1 Voice v1: Áudio, STT e TTS

1. **Formato de Áudio Canônico:**
   - PCM Linear 16-bit, 16.000 Hz, Mono (1 canal). Padrão global para modelos acústicos de STT (Whisper, Vosk) e algoritmos de detecção de atividade vocal (VAD).
   - Manipulação de contêiner WAV com leitor/escritor determinístico baseado na especificação RIFF PCM sem acoplamento a bibliotecas pesadas de C++.
2. **Speech-to-Text (STT) Local First:**
   - **Abstração:** Trait `SpeechToTextProvider` com metadados de offline/online, idiomas, latência e diagnóstico de saúde.
   - **Provedor Local Primário:** Adaptador local modular com normalização de áudio, rejeição de clipping/ruído estático e suporte configurável a executável local Whisper (`whisper.cpp` / CLI local) ou modelo ONNX acoplado por caminho.
   - **Provedor Fallback em Nuvem:** Adaptador governado `CloudSttProvider` que exige explicitamente a permissão `egress:voice_cloud` e credencial válida no `CredentialVault`.
3. **Text-to-Speech (TTS) Local First & Identidade Vocal:**
   - **Identidade Vocal da Yuki:** Feminina, calorosa, clara, natural, sem dramatização ou simulação de respirações biológicas. Coerente com o perfil `Yuki Default`.
   - **Abstração:** Trait `TextToSpeechProvider` com parâmetros de prosódia (velocidade, pitch, estilo) informados pelo `PersonalityProfile`.
   - **Provedor Local Primário:** Adaptador local modular `PiperTtsProvider` e sintetizador acústico local gerador de áudio WAV estruturado.
   - **Provedor Fallback em Nuvem:** Adaptador `CloudTtsProvider` com governança estrita de envio de texto.
4. **Palavra de Ativação ("Yuki") Local:**
   - `WakeWordDetector` 100% local e soberano.
   - Análise espectral e temporal das sílabas fonéticas "Yu-ki" (/ju:ki/) via energia ponderada em bandas críticas e taxa de cruzamento por zero, com limiar de sensibilidade configurável.
   - **Invariante:** Detecção de "Yuki" apenas abre a sessão de escuta (`WakeListening → Capturing`); **nunca** autoriza comandos ou concede privilégios.
5. **Prevenção de Realimentação e Sessão Half-Duplex:**
   - Modelo inicial focado em half-duplex robusto com máquina de estados de sessão explícita (`Idle`, `WakeListening`, `Capturing`, `Transcribing`, `Processing`, `Synthesizing`, `Speaking`, `Interrupted`, `Failed`), impedindo que o microfone capture o próprio áudio sintetizado da Yuki.

### 3.2 Missions Foundation: Persistência, Governança e DAG

1. **Armazenamento Transacional no SQLite:**
   - Reutilização da infraestrutura do `rusqlite` já incorporada ao projeto.
   - Tabelas dedicadas:
     - `missions`: id, title, objective, status, created_at, updated_at, completed_at, failure_reason, correlation_id;
     - `mission_steps`: id, mission_id, step_number, description, capability_id, requires_approval, status, input_payload, output_payload, approval_id, created_at, updated_at;
     - `mission_approvals`: id, mission_id, step_id, capability_id, operator_id, parameters_hash, status, created_at, expires_at;
     - `mission_checkpoints`: id, mission_id, step_number, state_snapshot, created_at.
2. **Ciclo de Vida e Máquina de Estados:**
   - `MissionStatus`: `Planned`, `InProgress`, `AwaitingApproval`, `Paused`, `Completed`, `Failed`, `Cancelled`.
   - `MissionStepStatus`: `Pending`, `Running`, `WaitingApproval`, `Completed`, `Failed`, `Skipped`.
3. **Human-in-the-Loop para Ações com Efeito Externo:**
   - Distinção estrita entre passos de leitura (executáveis conforme política) e passos mutadores/efeito externo (que exigem emissão de `MissionApprovalRequested` e aprovação humana explícita).
   - Token de aprovação `MissionApproval` com expiração temporal, hash exato dos parâmetros e vinculação ao `operator_id`. Qualquer divergência de parâmetros invalida a aprovação.
4. **Resiliência a Reinicializações e Idempotência:**
   - Na inicialização, a Yuki recupera missões não finalizadas do banco de dados, inspeciona o último checkpoint verificado e não repete passos já executados com efeito externo observado.

---

## 4. CRITÉRIOS DE ACEITE VERIFICÁVEIS

1. **Contratos de Voz (Etapa 8.1):**
   - Traits `VoiceEngine`, `SpeechToTextProvider`, `TextToSpeechProvider`, structs de áudio (`AudioFrame`, `AudioInput`, `AudioOutput`) e máquina de estados `VoiceSession` compilam com zero advertências e possuem cobertura unitária completa.
2. **STT Local (Etapa 8.2):**
   - Reconhecimento local decodifica áudio PCM/WAV, valida taxas de amostragem, duração máxima e normaliza amplitude, reportando falhas de forma explícita.
3. **TTS & Identidade Vocal (Etapa 8.3):**
   - Síntese local produz WAV válido em conformidade com as diretrizes de personalidade da Yuki, com suporte a interrupção e cancelamento imediato.
4. **Governança Híbrida & Fallback (Etapa 8.4):**
   - Tentativa de fallback para STT/TTS em nuvem sem permissão concedida no Security Controller ou sem credencial falha fechado imediatamente, sem vazar áudio ou texto para a rede.
5. **Ativação por Botão/CLI (Etapa 8.5):**
   - Fluxo completo ponta a ponta: acionamento → captura → transcrição → `YukiCore` → resposta → síntese.
6. **Wake Word "Yuki" (Etapa 8.6):**
   - Detecção local da palavra "Yuki" sem streaming contínuo para terceiros, comprovando isolamento entre ativação de escuta e autorização de comando.
7. **Conversação Natural & Interrupção (Etapa 8.7):**
   - Interrupção durante a fala limpa o buffer de reprodução e devolve o estado para escuta ou pronto sem conflitos concorrentes.
8. **Contratos e Máquina de Estados de Missões (Etapa 8.8):**
   - Transições de estado válidas e rejeição de transições inválidas (ex: `Completed → InProgress`).
9. **Persistência SQLite e Recuperação (Etapa 8.9):**
   - Reinício do sistema recarrega missões e passos com integridade verificada a partir de checkpoints.
10. **Aprovação Humana e Execução Governada (Etapa 8.10):**
    - Passos mutadores não executam sem `MissionApprovalGranted` correspondente aos parâmetros exatos.
11. **Planejamento de Missão Real (Etapa 8.11):**
    - Validador de plano rejeita ciclos, passos acima do limite e capabilities inexistentes; missão demonstrável de pesquisa gera relatório verificado.
12. **Integração Voz + Missões (Etapa 8.12):**
    - Comandos de voz interagem com o sistema de missões preservando todas as políticas de segurança.
13. **Configuração e Extensibilidade (Etapa 8.13):**
    - Configurações de voz e missões versionadas e declarativas com padrão seguro (privacy-first).
14. **Bateria de Testes Adversariais (Etapa 8.14):**
    - 45 cenários adversariais obrigatórios testados e aprovados.
15. **Demonstrações Funcionais e Documentação (Etapas 8.15 a 8.17):**
    - Demonstrações documentadas e reproduzíveis; todos os portões de qualidade (`fmt`, `clippy`, `test`, `build --release`) aprovados.

---

## 5. RISCOS DE INTEGRAÇÃO E MITIGAÇÕES

| Risco Identificado | Severidade | Mitigação Arquitetural |
| :--- | :--- | :--- |
| **Injeção de Instrução via Áudio Transcrito** | Alta | Aplicação rigorosa de `Data != Instruction`. O texto transcrito entra no pipeline como dado não confiável (`UserInput`), sendo submetido à sanitização de delimitadores já validada no Marco 7. |
| **Autorização Espúria por Fala ("Yuki, confirme!")** | Crítica | A palavra falada não constitui token de autorização. O Security Controller exige confirmação interativa explícita no canal seguro (CLI / terminal) com token de aprovação assinado. |
| **Concorrência e Travamento em Gravação/Reprodução** | Média | Máquina de estados half-duplex com travas de estado (`RwLock<VoiceSessionState>`) e cancelamento via canais `tokio::sync::watch` / `CancellationToken`. |
| **Corrupção de Estado em Missões Interrompidas** | Alta | Transações atômicas no SQLite para gravação de passos e checkpoints. Reconciliação explícita na reinicialização (`status = Interrupted` até confirmação). |
| **Vazamento de Áudio ou Texto para Provedores Externos** | Crítica | Política "Local First — Cloud Only When Authorized". A camada de transporte de rede requer capacidade `egress:voice_cloud` checada pelo Security Controller antes de qualquer conexão HTTP. |

---

## 6. ESTRATÉGIA DE TESTES

1. **Testes Unitários:**
   - Contratos de voz e manipulação de PCM/WAV (cálculo de amplitude, decodificação, geração de cabeçalho).
   - Máquina de estados de `VoiceSession` e `MissionStatus`.
   - Sanitização de metadados e prevenção de log de áudio bruto.
2. **Testes de Integração:**
   - Ciclo completo de voz (entrada → transcrição → core → síntese → reprodução).
   - Ciclo de missões no SQLite (criação de DAG, persistência de passos, transições transacionais e recuperação após simulação de queda).
   - Protocolo de aprovação CLI de ponta a ponta.
3. **Testes Adversariais:**
   - Áudio vazio, distorcido, ruído puro, áudio longo (> 30s).
   - Injeção de instruções e tags de sistema dentro de transcrições de áudio.
   - Planos de missão com grafos cíclicos, capacidades não registradas e parâmetros mutados após aprovação.
   - Retomada após interrupção sem duplicação de efeitos de saída.

---

## 7. ORDEM DAS ETAPAS DE IMPLEMENTAÇÃO

- **Etapa 8.1:** Contratos de Voz e Sessão (`src/contracts/voice.rs`, `src/voice/session.rs`)
- **Etapa 8.2:** Reconhecimento de Voz Local e Normalização (`src/voice/stt/`, `src/voice/audio.rs`)
- **Etapa 8.3:** Síntese de Voz da Yuki e Identidade Vocal (`src/voice/tts/`, `src/voice/playback.rs`)
- **Etapa 8.4:** Arquitetura Híbrida e Seletor Governado de Provedores (`src/voice/router.rs`)
- **Etapa 8.5:** Ativação por Botão/Comando CLI (`src/voice/service.rs`, `src/main.rs`)
- **Etapa 8.6:** Palavra de Ativação "Yuki" Soberana (`src/voice/wakeword.rs`)
- **Etapa 8.7:** Conversação Natural e Interrupção (`src/voice/conversation.rs`)
- **Etapa 8.8:** Contratos de Missões e Máquina de Estados (`src/contracts/missions.rs`)
- **Etapa 8.9:** Persistência Transacional SQLite e Checkpoints (`src/persistence/missions.rs`)
- **Etapa 8.10:** Protocolo de Aprovação Humana e Execução Governada (`src/missions/approval.rs`, `src/missions/executor.rs`)
- **Etapa 8.11:** Planejamento, DAG e Primeira Missão Real de Pesquisa (`src/missions/planner.rs`, `src/missions/runner.rs`)
- **Etapa 8.12:** Integração Governamental entre Voz e Missões
- **Etapa 8.13:** Configuração Versionada e Extensibilidade
- **Etapa 8.14:** Suíte Completa de Testes Adversariais (`tests/marco_8_adversarial.rs`)
- **Etapa 8.15:** Demonstrações Funcionais Documentadas
- **Etapa 8.16:** Atualização da Documentação e Guias
- **Etapa 8.17:** Validação Final dos Portões de Qualidade e Relatório de Encerramento
