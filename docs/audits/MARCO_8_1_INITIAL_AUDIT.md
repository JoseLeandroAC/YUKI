# YUKI — Relatório de Auditoria Inicial (Marco 8.1)
## Auditoria Técnica e Diagnóstico de Segurança de Voice v1 & Missions Foundation

**Data da Auditoria:** 2026-10-12  
**Baseline Auditada:** Commit `40eee94d6b0d1f68c5e566199a738811fb895ab2`  
**Branch de Trabalho:** `feature/marco-8.1-hardening`  
**Auditores:** Engenharia de Sistemas Yuki & Auditoria Independente

---

## 1. Resumo da Auditoria

A inspeção detalhada do código-fonte e dos testes entregues no Marco 8 confirmou avanços fundamentais nos contratos de voz e no arcabouço de missões. Entretanto, identificou **diferenças cruciais entre a arquitetura declarada e o comportamento efetivo da implementação**.

A baseline `40eee94` conta com 528 testes unitários e de integração aprovados (e 4 testes opt-in live ignorados). Contudo, muitos desses testes validavam apenas transições de variáveis em memória ou componentes mock, mascarando a ausência de integrações reais de hardware e lacunas severas de governança de autorização.

---

## 2. Inventário de Achados por Subsistema

### 2.1. Subsistema de Voz (Voice v1)

| ID | Achado | Severidade | Impacto | Exploração Plausível | Correção Necessária |
|---|---|---|---|---|---|
| **VOICE-01** | **Reprodução apenas simulada** (`src/voice/playback.rs`) | **ALTA** | Não há emissão de som físico no sistema operacional; o controlador apenas altera status para `Playing`. | Falsa confirmação de que o assistente falou com o operador. | Implementar backend real de saída (Windows WinMM/waveOut) e backend mock observável para CI, com interrupção física. |
| **VOICE-02** | **Ausência de captura real de microfone** (`src/voice/`) | **ALTA** | CLI aceita apenas `--file`; não há escuta de microfone em tempo real nem push-to-talk. | Impossibilidade de uso natural da Yuki sem gravação manual externa. | Implementar módulo de captura (`src/voice/capture/`) com push-to-talk, limites de buffer e conversão PCM 16kHz mono. |
| **VOICE-03** | **Wake word baseada apenas em heurística de energia** (`src/voice/wakeword.rs`) | **ALTA** | `WakeWordDetector` usa RMS e taxa de cruzamento por zero; qualquer som de 2 sílabas ativa o gatilho. | Falsos positivos frequentes disparados por ruídos ambientes, tosse ou palmas. | Separar VAD de reconhecimento lexical; introduzir validação acústica/fonética ou verificação via STT local antes de ativar o turno. |
| **VOICE-04** | **Fallback acústico produz tons paramétricos, não fala** (`src/voice/tts/local.rs`) | **MÉDIA** | `LocalAcousticTtsProvider` emite onda senoidal de 220Hz sem fonemas; classificado indevidamente como fala feminina. | Expectativa frustrada de inteligibilidade em ambientes sem Piper instalado. | Reclassificar como sinal sonoro de diagnóstico de fallback; clarificar nos diagnósticos que fala neural requer Piper. |
| **VOICE-05** | **Subprocessos Whisper e Piper zumbis pós-timeout** (`src/voice/stt/local.rs`, `tts/local.rs`) | **ALTA** | `spawn_blocking` com `Command::output` abandona o processo filho em caso de timeout; processo fica órfão. | DoS por exaustão de CPU/memória com processos zumbis acumulados. | Migrar para `tokio::process::Command` assíncrono com `child.kill().await` obrigatório no timeout. Nomes de tempfiles criptograficamente seguros. |
| **VOICE-06** | **Bypass de governança cloud por parâmetro booleano** (`src/voice/router.rs`) | **ALTA** | Roteador recebe `operator_authorized_stt_cloud: bool`; qualquer código interno pode passar `true`. | Envio desautorizado de áudio ou texto para APIs em nuvem. | Exigir autorização tipada e verificável pelo `SecurityController` com checagem de permissão específica (`egress:voice_cloud_*`). |
| **VOICE-07** | **Vazamento de transcrição privada em logs de auditoria** (`src/voice/conversation.rs`) | **ALTA** | `VoiceInputReceived` salva o texto completo falado (`transcribed_text`) no banco de auditoria SQLite. | Violação da intimidade do operador; armazenamento indiscriminado de falas em logs persistentes. | Aplicar minimização de dados: gravar apenas metadados (duração, contagem de caracteres, taxa de amostragem) sem transcrição em claro. |

---

### 2.2. Subsistema de Missões (Missions Foundation)

| ID | Achado | Severidade | Impacto | Exploração Plausível | Correção Necessária |
|---|---|---|---|---|---|
| **MISS-01** | **Identidade do operador sem autenticação real** (`src/main.rs`, `approval.rs`) | **CRÍTICA** | CLI aceita `--operator operator-jose` por texto puro; mera igualdade de strings não autentica o operador. | Qualquer processo ou usuário local pode aprovar ações mutadoras declarando o nome do operador. | Implementar confirmação autenticada via segredo/chave do Vault ou token de confirmação seguro na aprovação. |
| **MISS-02** | **Validação superficial de aprovação no executor** (`src/missions/runner.rs`) | **CRÍTICA** | `MissionRunner` checa apenas `status == Granted`; não valida expiração, hash SHA-256 nem consome a aprovação. | Replay de aprovações antigas ou execução de parâmetros adulterados pós-aprovação. | Validar integralmente `approval.is_valid_for(...)` no momento pré-execução e marcar a aprovação como consumida. |
| **MISS-03** | **Confiança exclusiva no campo `requires_approval` do plano** (`src/missions/runner.rs`) | **ALTA** | Se um plano gerado por modelo marcar etapa mutadora com `requires_approval = false`, o executor não pausa. | Quebra da invariante `Model Output != Command`: plano malicioso executa mutação sem portão humano. | `MissionRunner` deve consultar a classe de risco da capacidade no registro; capacidades mutadoras/alto risco exigem aprovação mandatória. |
| **MISS-04** | **Falta de concorrência e lease por missão** (`src/missions/runner.rs`) | **ALTA** | Múltiplas invocações simultâneas de `run_mission` executam etapas concorrentemente sem exclusão mútua. | Execução duplicada de ações no mundo real e condições de corrida no banco de dados. | Implementar mecanismo de lease/mutex de execução exclusivo por missão. |
| **MISS-05** | **Reconciliação no startup cega a efeitos externos** (`src/missions/store.rs`) | **ALTA** | Etapas em `Running` durante crash são revertidas para `Pending`, provocando repetição de ações já executadas. | Execução duplicada de operações não-idempotentes (ex: exclusão de arquivos, envio de dados). | Etapas mutadoras interrompidas devem ser reconciliadas para `WaitingApproval` ou flag de intervenção humana, nunca `Pending` automático. |
| **MISS-06** | **Integridade relacional e transições de estado no SQLite** (`src/missions/store.rs`) | **MÉDIA** | Schema sem FKs em `mission_approvals` e updates não validam máquina de estados antes da escrita. | Inconsistência de estados órfãos e transições inválidas forçadas em banco. | Adicionar constraints de chave estrangeira, unicidade e checar `can_transition_to` antes de persistir. |
| **MISS-07** | **Verificador de efeitos (`Verifier`) desconectado no executor** (`src/missions/runner.rs`) | **ALTA** | O `Verifier` é injetado no runner mas nunca invocado; etapa é dada como `Completed` apenas pelo retorno do handler. | Quebra da invariante `Execution != Verified Effect`: sucessos falsos sem verificação de evidências. | Integrar `Verifier::verify_operation` pós-execução e persistir o resultado formal de verificação. |

---

## 3. Plano de Correções para o Marco 8.1

1. **Etapa 8.1.1 — Áudio Real (Playback, Captura e Push-to-Talk):**
   - Criação de `src/voice/playback.rs` com suporte a Windows audio playback real via WinMM e backend observável para CI.
   - Criação de `src/voice/capture/` com abstração `AudioCaptureBackend`, captura Windows com push-to-talk explícito e backend mock determinístico.
2. **Etapa 8.1.2 — Wake Word e VAD Local com Evidência Positiva:**
   - Separação entre detecção de voz (VAD) e reconhecimento lexical do wake word. Validação acústica da palavra "Yuki" com rejeição de falsos positivos.
3. **Etapa 8.1.3 — Honestidade Vocal e Subprocessos Seguros (Whisper e Piper):**
   - Classificação honesta do fallback acústico (sinal diagnóstico, não fala).
   - Migração para `tokio::process::Command` assíncrono com eliminação garantida de processos filhos após timeout/cancelamento e tempfiles com exclusividade e alta entropia.
4. **Etapa 8.1.4 — Governança e Minimização de Dados (Router e Auditoria):**
   - Remoção de booleanos soltos em favor de tokens de autorização tipados e verificáveis.
   - Redação de falas privadas dos logs persistentes de auditoria (`VoiceInputReceived`).
5. **Etapa 8.1.5 — Hardening do Subsistema de Missões:**
   - Autenticação real de operador vinculada ao cofre/chave segura.
   - Validação obrigatória e estrita de aprovações (`is_valid_for`) imediatamente antes da execução com consumo transacional anti-replay.
   - Imposição mandatória de aprovação baseada na classe de risco da capacidade no registro, nunca confiando cegamente no plano.
   - Exclusão mútua / lease de execução por missão.
   - Reconciliação segura de inicialização que não reexecuta mutações cegas.
   - Integração do `Verifier` ao ciclo de vida da etapa.
6. **Etapa 8.1.6 — Suíte de Testes Adversariais Expandida e Quality Gates:**
   - Manutenção dos 45 testes anteriores + criação dos novos cenários de segurança e integração real.
   - Validação dos quality gates completos (`fmt`, `check`, `clippy -D warnings`, `test`, `build --release`).
