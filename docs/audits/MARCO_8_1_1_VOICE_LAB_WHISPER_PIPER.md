# YUKI — Relatório de Auditoria e Homologação Técnica
## Preparação do Laboratório Vocal e Integração Whisper + Piper (Marco 8.1.1)

**Repositório:** `JoseLeandroAC/YUKI`  
**Branch:** `feature/marco-8.1.1-homologation-fixes`  
**Data:** 12 de outubro de 2026  
**Responsável:** Agente de Desenvolvimento e Auditoria Soberana  
**Status:** **HOMOLOGADO INTERNAMENTE — PRONTO PARA AUDITORIA INDEPENDENTE E TESTES COM MODELOS REAIS**

---

### 1. Resumo Executivo

Este ciclo de trabalho concluiu a estruturação do subsistema vocal soberano da Yuki, resolvendo a inconsistência diagnóstica de reprodução de áudio, conectando os adaptadores locais de Whisper.cpp (STT) e Piper TTS, e estabelecendo o **Laboratório Yuki Voice** para avaliação, comparação e calibração estruturada de qualidade fonêmica e identidade vocal.

Principais entregas realizadas:
1. **Unificação Operacional WASAPI:** Correção no `AudioPlaybackController::production()` e `AudioPlaybackController::production_with_device()`, eliminando o fallback indevido para WinMM e garantindo que o diagnóstico `voice status` reflita com precisão o driver `WindowsAudioPlaybackSink (WASAPI Core Audio)`.
2. **Reconhecimento de Fala Local (Whisper.cpp STT):** Descoberta robusta de binários e modelos quantizados (`.bin`), detecção inteligente de silêncio acústico (curto-circuito econômico que evita acionar subprocessos para ruído/silêncio), medição de latência de inferência e sanitização estrita de transcrições (limpeza de timestamps e tags `[BLANK_AUDIO]`).
3. **Síntese Neural de Voz Local (Piper TTS):** Suporte completo aos parâmetros de inferência (`--length_scale`, `--noise_scale`, `--noise_w`), catálogo oficial pt-BR (`pt_BR-faber-medium`, `pt_BR-cadu-medium`, `pt_BR-edresson-low`, `pt_BR-jeff-medium`), detecção automática de modelos ONNX customizados em `models/piper/` e instruções transparentes para download sem qualquer execução silenciosa em segundo plano.
4. **Laboratório Yuki Voice (`src/voice/lab.rs`):**
   - 5 frases de teste padronizadas (`saudacao`, `explicacao_tecnica`, `conversa_natural`, `aviso`, `pronuncia`).
   - Geração de amostras com nomenclatura canônica `{voice_id}_{phrase_id}_{timestamp}.wav` em `data/voice_samples/` com metadados estruturados `.meta.json`.
   - Sistema de avaliação com critérios de 1 a 100 (Naturalidade, Tonalidade, Suavidade, Expressividade, Ritmo), persistência atômica em `data/voice_evaluations.json` e confronto com o Perfil Vocal Yuki v0.1.
5. **Comandos de Linha de Comando (CLI):**
   - `yuki voice voices`: catálogo de modelos com status e URLs oficiais.
   - `yuki voice sample`: geração e audição de amostras específicas.
   - `yuki voice compare`: comparação sequencial A/B/C entre modelos disponíveis.
   - `yuki voice evaluate`: registro estruturado de notas do operador.
   - `yuki voice profile`: exibição da especificação técnica v0.1 e agregação histórica de avaliações.

---

### 2. Diagnóstico e Resolução da Inconsistência de Playback no `voice status`

- **Sintoma:** Apesar de o playback WASAPI de 32 bits ter sido homologado no Redmi Buds 6 Play, o comando `cargo run -- voice status` exibia:
  `Driver de Reprodução: DISPONÍVEL (WindowsAudioPlaybackSink (winmm))`
- **Causa Raiz:** Em `src/voice/playback.rs`, o método construtor `AudioPlaybackController::production()` retornava por padrão a instância do sink `WindowsAudioPlaybackSink::production()` (que utilizava a API legada WinMM `PlaySoundA`). O método `production_with_device(None)` delegava para `Self::production()`.
- **Correção Conclusiva:**
  - `AudioPlaybackController::production()` e `AudioPlaybackController::production_with_device(device_id)` no Windows agora instanciam diretamente `WasapiAudioPlaybackSink::production(device_id)`.
  - Foi criado o método explícito `AudioPlaybackController::winmm()` para os testes que deliberadamente avaliam o backend legado.
  - A saída de `voice status` agora reporta estritamente:
    `Driver de Reprodução: DISPONÍVEL (WindowsAudioPlaybackSink (WASAPI Core Audio))`.

---

### 3. Integração Whisper.cpp Local (STT)

- **Descoberta de Recursos:**
  - Variáveis de ambiente: `YUKI_WHISPER_BIN`, `YUKI_WHISPER_MODEL`.
  - Diretórios locais: `./bin/whisper/`, `./bin/`, `./models/whisper/`, `./models/`.
  - Executáveis verificados: `whisper-cli.exe`, `whisper.exe`, `main.exe` (e equivalentes Unix).
  - Modelos quantizados: `ggml-small.bin`, `ggml-base.bin`, `ggml-medium.bin`, `ggml-tiny.bin` e qualquer arquivo `.bin` na pasta.
- **Detecção de Silêncio Acústico:**
  - Função `is_silent_pcm(samples: &[i16]) -> bool`.
  - Calcula o RMS e amplitude de pico. Se RMS < 120 e pico < 250, o áudio é classificado como silêncio/ruído ambiente e retorna `Ok(String::new())` imediatamente, economizando CPU/GPU e tempo sem instanciar subprocesso.
- **Higienização e Latência:**
  - Medição de tempo via `std::time::Instant`.
  - Função `sanitize_whisper_transcript(raw)` remove tags `[BLANK_AUDIO]` e marcações de tempo do tipo `[00:00.000 --> 00:02.000]`.

---

### 4. Integração Piper TTS Local

- **Parâmetros de Inferência Ajustáveis:**
  - `--length_scale`: cadência e velocidade (padrão recomendado Yuki: `1.14` para fala serena e pausada).
  - `--noise_scale`: controle de ruído fonêmico (padrão recomendado Yuki: `0.55` para tom aveludado e suave).
  - `--noise_w`: estabilidade de variabilidade fonêmica (padrão recomendado Yuki: `0.70`).
- **Catálogo Oficial pt-BR e Transparência de Gênero:**
  - Modelos upstream documentados e mapeados para download manual:
    1. `pt_BR-faber-medium` (22050 Hz, qualidade medium, locutor masculino, alta clareza).
    2. `pt_BR-cadu-medium` (22050 Hz, qualidade medium, locutor masculino, entonação coloquial).
    3. `pt_BR-edresson-low` (16000 Hz, qualidade low, locutor masculino, ultraleve).
    4. `pt_BR-jeff-medium` (22050 Hz, qualidade medium, locutor masculino, comunitário).
  - **Fato Técnico Documentado:** O repositório upstream oficial da Rhasspy para pt-BR possui modelos exclusivamente masculinos. Para operar com voz feminina neural nativa, o operador pode alocar qualquer modelo `.onnx` comunitário em `models/piper/` (o sistema detecta e expõe automaticamente) ou utilizar o sintetizador acústico de diagnóstico padrão com F0=220Hz.
  - Zero downloads silenciosos: comandos fornecem URLs exatas e snippets PowerShell `Invoke-WebRequest`.

---

### 5. Laboratório Yuki Voice

- **Conjunto de Frases Padronizadas:**
  1. `saudacao`: "Olá! Eu sou a Yuki, sua assistente pessoal no Windows. O que faremos hoje?" (Saudação e Identidade)
  2. `explicacao_tecnica`: "O subsistema de voz agora utiliza WASAPI para captura e reprodução de áudio de alta precisão." (Explicação Técnica)
  3. `conversa_natural`: "Estou pronta para ajudar você com tarefas, pesquisas e automações no sistema." (Conversação Cotidiana)
  4. `aviso`: "Atenção: uma ação de alto impacto requer sua confirmação explícita antes de continuar." (Alerta e Governança)
  5. `pronuncia`: "Inicializando o subsistema de áudio na arquitetura x86_64 com latência mínima." (Desafio Fonético)
- **Especificação Técnica do Perfil Vocal da Yuki (v0.1):**
  - Identidade: Feminina, jovem-adulta (20-25 anos), calorosa, serena, profissional mas afável.
  - F0: ~200-240 Hz.
  - Cadência: Levemente pausada (~0.88x velocidade normal, `length_scale = 1.14`).
  - Metas de Qualidade (escala 1 a 100):
    - Naturalidade: >= 80
    - Tonalidade: >= 75
    - Suavidade: >= 80
    - Expressividade: >= 70
    - Ritmo: >= 75
- **Persistência Estruturada:**
  - Amostras em `data/voice_samples/`: `{voice_id}_{phrase_id}_{timestamp}.wav` e `{voice_id}_{phrase_id}_{timestamp}.meta.json`.
  - Avaliações em `data/voice_evaluations.json`: gravação atômica via arquivo temporário com rename seguro.

---

### 6. Matriz de Comandos CLI Validados

| Comando | Função | Exemplo de Saída |
|---|---|---|
| `yuki voice status` | Diagnóstico de drivers e engines | Driver de Reprodução: DISPONÍVEL (WindowsAudioPlaybackSink (WASAPI Core Audio)) |
| `yuki voice voices` | Catálogo de vozes pt-BR e Whisper | Lista 4 modelos oficiais, URLs de download e instruções transparentes |
| `yuki voice sample` | Geração de amostra vocal | Salva WAV em `data/voice_samples/` com duração, sample rate e metadados |
| `yuki voice compare` | Comparação multi-voz de uma frase | Gera tabela comparativa de durações e tipos (Neural vs Acústico) |
| `yuki voice evaluate` | Registro de notas de auditoria humana | Registra avaliação com médias e confronta com metas do perfil v0.1 |
| `yuki voice profile` | Especificação técnica e histórico | Exibe identidade v0.1, parâmetros de inferência e estatísticas agregadas |

---

### 7. Portões de Qualidade e Métricas de Homologação

- **`cargo fmt --all -- --check`:** 100% aprovado.
- **`cargo clippy --workspace --all-targets -- -D warnings`:** 0 warnings, 0 erros.
- **Suíte de Testes no Workspace:**
  - **`tests/marco_8_adversarial.rs`:** **71/71 testes aprovados** (100%).
  - **`tests/voice_integration.rs`:** **12/12 testes aprovados** (100%).
  - **`tests/voice_stt_tts.rs`:** **12/12 testes aprovados** (100%).
  - **Total no Workspace:** **589 testes aprovados** (5 testes live opt-in ignorados, 0 falhas).
- **Compilação de Release:** `cargo build --release` concluído com sucesso.
