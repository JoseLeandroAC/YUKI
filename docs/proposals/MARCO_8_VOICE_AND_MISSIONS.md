# MARCO 8: VOICE V1 & MISSIONS FOUNDATION
## Proposta Técnica e Arquitetura de Referência

**Status:** PROPOSTA / EM PLANEJAMENTO  
**Referência:** Extensão natural do Marco 7 (Personal Intelligence Foundation)  
**Repositório:** `JoseLeandroAC/YUKI`  
**Autor:** Engenharia de Arquitetura da Plataforma Yuki  

---

## 1. VISÃO GERAL E MOTIVAÇÃO

O Marco 7 estabeleceu a **Personal Intelligence Foundation** da Yuki:
1. **Credential Vault v1 Hardened**: Gestão segura e transacional de credenciais.
2. **Memory v1**: Persistência durável SQLite FTS5 BM25 com políticas governadas e direito ao esquecimento.
3. **Personal Context v1**: Priorização em 5 camadas com orçamento estrito de tokens.
4. **Personality v1**: Identidade estável de 3 camadas com salvaguardas ontológicas e adaptação contextual.

O **Marco 8** avança na personificação e na proatividade governada da plataforma, introduzindo:
- **Voice v1**: Interface conversacional multimodal de voz bidirecional (Speech-to-Text governado e Text-to-Speech expressivo) com streaming e baixa latência.
- **Missions Foundation**: Capacidade de executar planos multi-etapas orientados a objetivos de longo prazo, com máquinas de estado persistentes, checkpoints transacionais e governança rigorosa *human-in-the-loop*.

---

## 2. INVARIANTES INEGOCIÁVEIS DE GOVERNANÇA

Todas as extensões do Marco 8 herdam integralmente os princípios canônicos da plataforma:

```
Audio Input != Command != Authorization != Execution
Model Proposal != Mission Step Execution != Permission
Voice Output != Unchecked Synthesis
Mission Objective != Unbounded Execution
Think != Authorize != Execute
Data != Instruction != Context != Evidence != Verification != Truth
```

### Salvaguardas Específicas para Voz e Missões:
1. **Transparência Ontológica na Fala**: A síntese de voz (TTS) da Yuki nunca deverá fingir estados emocionais biológicos, dores humanas ou respirações artificiais concebidas para simular senciência.
2. **Isolamento de Áudio e Credenciais**: Fluxos de áudio jamais trafegam segredos em logs, transcrições transitórias sem sanitização ou caches não encriptados.
3. **Aprovação Humana Inegociável para Ações Mutadoras**: Nenhuma etapa de missão com efeitos externos ou mutações no sistema do operador executará autonomamente sem emissão de `MissionApprovalRequested` e recebimento de `MissionApprovalGranted` explícito.
4. **Resiliência a Reinicializações**: Missões são armazenadas de forma determinística no SQLite, permitindo que uma missão interrompida seja retomada com integridade verificada a partir do último checkpoint aprovado.

---

## 3. ARQUITETURA DE VOZ V1

### 3.1 Pipeline de Processamento de Áudio

```
[Microfone / Stream de Entrada]
             │
             ▼
   [Voice Activity Detection - VAD]
             │
             ▼
   [Governed STT Provider] ─── (Whisper Edge / Local ou Cloud via Vault)
             │
             ▼ (Emissão de VoiceInputReceived)
      [UserInput (YukiCore)]
             │
             ▼
      [Pipeline YukiCore] ─── (Memória v1 + Personalidade v1 + Capability Registry)
             │
             ▼ (Emissão de ResponseProduced)
      [Governed TTS Provider] ─── (Piper Edge / Bark / Cloud via Vault)
             │
             ▼ (Emissão de VoiceOutputProduced)
[Auto-falante / Stream de Saída]
```

### 3.2 Estratégia de Provedores de Voz (Edge First)

A Yuki adota uma estratégia híbrida:
- **Nível 1 (Offline / Edge Local)**:
  - STT: `whisper.cpp` ou `onnxruntime` executando modelos quantizados locais (ex: Whisper Small/Base).
  - TTS: `piper-rs` para síntese ultra-rápida em CPU local (< 150ms).
- **Nível 2 (Provedores Externos Governados)**:
  - Provedores cloud de alta fidelidade (Gemini Live Audio, ElevenLabs, OpenAI Audio) habilitados via Credential Vault v1 e estritamente autorizados por capability (`egress:voice_cloud`).

### 3.3 Eventos de Auditoria de Voz
Em `src/contracts/events.rs`:
- `VoiceInputReceived`: Registra áudio capturado, duração em milissegundos, taxa de amostragem, nível de ruído e texto transcrito sanitizado.
- `VoiceOutputProduced`: Registra texto sintetizado, voz utilizada, duração do áudio gerado e latência total.

---

## 4. ARQUITETURA DE MISSÕES (MISSIONS FOUNDATION)

### 4.1 Conceito e Ciclo de Vida da Missão

Uma **Missão** representa um objetivo de alto nível atribuído à Yuki pelo operador (ex: *"Auditar repositório e preparar relatório de conformidade"* ou *"Pesquisar vulnerabilidades recentes no crate X"*).

```
   ┌─────────┐
   │ Planned │
   └────┬────┘
        │
        ▼
  ┌───────────┐         Ação Crítica        ┌──────────────────┐
  │ InProgress ├────────────────────────────► AwaitingApproval  │
  └─────┬─────┘                             └────────┬─────────┘
        │                                            │
        │ Sucesso de todos                           ├───── Aprovado: Continua
        │ os passos                                  │
        ▼                                            ▼ Reprovado: Falha ou Replaneja
  ┌───────────┐                             ┌──────────────────┐
  │ Completed │                             │      Failed      │
  └───────────┘                             └──────────────────┘
```

### 4.2 Estados da Missão e de Passos
- **MissionStatus**: `Planned`, `InProgress`, `AwaitingApproval`, `Completed`, `Failed`, `Cancelled`.
- **MissionStepStatus**: `Pending`, `Running`, `WaitingApproval`, `Completed`, `Failed`, `Skipped`.

### 4.3 Esquema Relacional de Persistência (SQLite)

```sql
CREATE TABLE IF NOT EXISTS missions (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    objective TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    completed_at TEXT,
    failure_reason TEXT,
    correlation_id TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS mission_steps (
    id TEXT PRIMARY KEY,
    mission_id TEXT NOT NULL,
    step_number INTEGER NOT NULL,
    description TEXT NOT NULL,
    capability_id TEXT,
    requires_approval INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL,
    input_payload TEXT,
    output_payload TEXT,
    approval_id TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (mission_id) REFERENCES missions(id) ON DELETE CASCADE
);
```

### 4.4 Eventos de Auditoria de Missões
Em `src/contracts/events.rs`:
- `MissionStarted`: Disparado quando uma nova missão entra em execução.
- `MissionStepCompleted`: Disparado após a execução verificada de cada passo.
- `MissionApprovalRequested`: Disparado quando um passo mutador exige confirmação do operador.
- `MissionApprovalGranted`: Confirmação explícita registrada do operador.
- `MissionApprovalDenied`: Rejeição do operador que interrompe ou replaneja o passo.
- `MissionCompleted`: Disparado com o sumário final da missão concluída com sucesso.
- `MissionFailed`: Disparado em caso de falha irrecuperável ou violação de política.

---

## 5. INTEGRAÇÃO COM MARCO 7 (PERSONAL INTELLIGENCE)

1. **Contexto Pessoal em Missões**:
   - Cada missão consulta o `DefaultMemoryRetriever` para incorporar preferências de trabalho e diretrizes do operador no contexto de planejamento.
2. **Adaptação de Personalidade para Missões**:
   - Durante etapas autônomas críticas de uma missão, a Yuki adota automaticamente o modo `ContextUrgency::SecurityCritical`, minimizando informalidade e focando em precisão e clareza técnica.
3. **Síntese de Voz Parametrizada**:
   - O `PersonalityProfile` (warmth, expressiveness, style) informa parâmetros de prosódia e pitch para o provedor TTS governado.

---

## 6. CRITÉRIOS DE ACEITE PARA O MARCO 8

- [ ] Contratos canônicos `VoiceEngine`, `AudioStream`, `Mission`, `MissionStep` implementados em `src/contracts/`.
- [ ] Provedor STT local/edge e TTS implementados com testes unitários offline.
- [ ] Mecanismo de persistência transacional de missões em SQLite com suporte a checkpoints.
- [ ] Protocolo de aprovação humana interativo via CLI (`yuki mission approve <id>`).
- [ ] Integração total com o pipeline auditado do `YukiCore`.
- [ ] Testes de validação adversarial garantindo que missões não ultrapassem seus escopos autorizados.
- [ ] Suite completa de testes aprovada e documentação canônica atualizada.
