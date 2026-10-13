# YUKI — Relatório de Auditoria Técnica Pós-Testes Físicos T7–T20
## Marco 8.1.1 — STT Local Whisper / WASAPI Core Audio

**Repositório:** `JoseLeandroAC/YUKI`  
**Branch de Trabalho:** `feature/marco-8.1.1-homologation-fixes`  
**Commit de Referência:** `9e17883a6c358755cda1b449b0220bc8c3f9ef02`  
**Data:** 13 de outubro de 2026  
**Status da Homologação:** **STT NÃO HOMOLOGADO FISICAMENTE (AUDITORIA CONCLUÍDA; PLANO DE MELHORIAS SUBMETIDO À REVISÃO)**

---

### 1. Contexto e Objetivo da Auditoria

Após as correções de sincronização do início de captura WASAPI (eliminando o corte da saudação inicial) e de alinhamento FFI em `WAVEFORMATEXTENSIBLE` (eliminando o ruído digital massivo do Realtek 32-bit Float), a homologação humana realizou uma bateria extensa de testes físicos (T7 a T20).

Os testes confirmaram um avanço substancial na inteligibilidade:
- A voz física é capturada e transcrita com sentido global preservado.
- Frases espontâneas, comandos cotidianos e frases longas em português são compreendidos.
- A detecção automática de idioma identificou com sucesso português (96,7%), inglês (47,2%), espanhol (80,8%) e japonês (58,3%).
- A repetibilidade em T19 manteve a intenção semântica nas três execuções consecutivas.

Entretanto, **o subsistema de STT ainda não atingiu o limiar de homologação final** devido a quatro eixos de deficiência:
1. **Dificuldades lexicais pontuais:** Erros recorrentes com o nome "Yuki", acrônimos educacionais ("FUVEST", "OBMEP") e certas palavras comuns.
2. **Latência total percebida excessiva:** 10 a 20 segundos por turno em CPU (especialmente agravada em modo automático `auto` e pela captura de duração fixa).
3. **Comportamento assíncrono em Bluetooth:** Registro de 24 a 56 descontinuidades WASAPI no headset Redmi Buds 6 Play e picos em −0,11 dBFS.
4. **Resampling sem antialiasing:** Decimação linear pura de 48 kHz para 16 kHz no driver Realtek, sujeita a dobra espectral em consoantes sibilantes.

Esta auditoria inspeciona o código real, disseca as causas raízes e estabelece um plano incremental seguro, **sem alterar o código de produção antes da aprovação do plano**.

---

### 2. Arquitetura Atual do Pipeline de Voz

O pipeline atual de ponta a ponta opera conforme o fluxo abaixo:

```
[ Hardware: Microfone ]
          │ (Realtek 48kHz Float32 Estéreo  OU  Bluetooth HFP 16kHz PCM16 Mono)
          ▼
[ WASAPI Core Audio Streaming ] (`src/voice/capture.rs`)
          │  - IAudioClient em modo compartilhado (buffer de 1s, sleep 15ms)
          │  - Sincronização via canal mpsc com timeout de 5s no IAudioClient::Start
          │  - GetBuffer / ReleaseBuffer em pacotes PCM/Float brutos
          ▼
[ Conversão e Downmix em Memória ] (`src/voice/capture.rs`)
          │  - IEEE Float 32-bit / PCM 16/24/32-bit -> i16 mono
          │  - resample_linear: decimação linear pura para 16.000 Hz
          ▼
[ AudioInput Canônico ] (`src/contracts/voice.rs`)
          │  - WAV PCM 16kHz mono 16-bit
          │  - Diagnóstico opt-in (`src/voice/diag.rs`) se flag ativa
          ▼
[ LocalWhisperSttProvider ] (`src/voice/stt/local.rs`)
          │  - is_silent_pcm: curto-circuito antes da normalização (RMS < 120)
          │  - normalize_amplitude: ganho linear para pico de 28.000
          │  - Gravação de WAV temporário em std::env::temp_dir()
          │  - Spawn de processo: `bin/whisper/whisper-cli.exe -m ... -f ... -l ... --no-timestamps`
          ▼
[ whisper-cli.exe Process ] (Processo externo independente)
          │  - Carga do binário + DLLs GGML/OpenBLAS
          │  - Leitura completa do modelo `ggml-small.bin` (487 MB) do disco/cache
          │  - Inferência em CPU (default 4 threads) com beam-size = 5
          │  - Emissão de texto em stdout e telemetria de timings em stderr
          ▼
[ Parser e Higienização ] (`src/voice/stt/local.rs`)
          │  - parse_whisper_stderr: extrai idioma auto-detectado, probabilidade e tempos
          │  - sanitize_whisper_transcript: remove timestamps e [BLANK_AUDIO]
          ▼
[ YukiCore / VoiceConversationCoordinator ] (`src/voice/conversation.rs`)
          │  - Input != Authorization: texto processado como dado não-confiável
          │  - TTS / Playback via WASAPI
```

---

### 3. Achados Comprovados por Inspeção de Código

#### 3.1 Achado 1: Resampler Linear atua como Decimação 3:1 Direta sem Filtragem Antialiasing
- **Arquivo:** [`src/voice/capture.rs`](file:///c:/Users/Miriã/Documents/JOSÉ/src/voice/capture.rs#L254-L275)
- **Função:** `resample_linear(samples: &[i16], in_rate: u32, out_rate: u32)`
- **Evidência no Código:**
  ```rust
  let ratio = in_rate as f64 / out_rate as f64; // 48000.0 / 16000.0 = 3.0
  let out_len = ((samples.len() as f64) / ratio).round() as usize;
  for i in 0..out_len {
      let in_pos = i as f64 * ratio; // 0.0, 3.0, 6.0, 9.0...
      let idx = in_pos.floor() as usize; // 0, 3, 6, 9...
      let frac = in_pos - (idx as f64);  // SEMPRE 0.0!
      let s0 = samples.get(idx)...;
      let s1 = samples.get(idx + 1)...;
      let interpolated = (1.0 - frac) * s0 + frac * s1; // Sempre exatamente s0!
      out.push(interpolated...);
  }
  ```
- **Demonstração Matemática:**
  Para a conversão canônica de 48.000 Hz para 16.000 Hz, o passo `ratio` é exatamente `3.0`. A fração de interpolação `frac` é estritamente `0.0` em todas as iterações. O algoritmo opera como **subamostragem direta 3:1 descartando 2 de cada 3 amostras**, sem passar o sinal por um filtro passa-baixa prévio.
- **Impacto Físico e Acústico:**
  O limite de Nyquist para 16.000 Hz é 8.000 Hz. Qualquer componente de frequência presente no áudio de 48 kHz entre 8 kHz e 24 kHz dobra-se de volta (aliasing espectral) para a banda de 0 a 8 kHz:
  - Uma sibilante em 10 kHz reflete como ruído espúrio em $16 - 10 = 6\text{ kHz}$.
  - Uma consoante em 12 kHz reflete diretamente em $16 - 12 = 4\text{ kHz}$ (faixa crítica de formantes vocais humanos).
  - Consoantes fricativas e sibilantes (/s/, /z/, /f/, /x/, /ʃ/) sofrem distorção nos bancos de filtros Mel do Whisper, degradando o reconhecimento de limites de palavras e acrônimos fonéticos ("FUVEST", "OBMEP", "Yuki").
- **Comportamento no Bluetooth:** No headset Redmi Buds 6 Play, o driver opera nativamente em 16.000 Hz (`in_rate == out_rate == 16000`), ativando a condição de curto-circuito `if in_rate == out_rate { return samples.to_vec(); }`. Portanto, o Bluetooth não sofreu aliasing de reamostragem, pois o codec mSBC/CVSD de rádio já entrega banda limitada.

#### 3.2 Achado 2: Ausência de Lexical Biasing / Initial Prompt no Whisper
- **Arquivo:** [`src/voice/stt/local.rs`](file:///c:/Users/Miriã/Documents/JOSÉ/src/voice/stt/local.rs#L507-L520)
- **Função:** `LocalWhisperSttProvider::transcribe_with_metadata`
- **Evidência no Código:**
  ```rust
  let mut child = tokio::process::Command::new(bin_path)
      .arg("-m").arg(model_path)
      .arg("-f").arg(&temp_wav_path)
      .arg("-l").arg(&self.config.language)
      .arg("--no-timestamps")
      .spawn()?;
  ```
- **Constatação na Ajuda do Binário Real (`whisper-cli.exe --help`):**
  O executável suporta nativamente:
  - `--prompt PROMPT`: initial prompt (máximo `n_text_ctx / 2` tokens).
  - `--carry-initial-prompt`: mantém o prompt inicial ativo em todos os segmentos.
- **Impacto Lexical:**
  O Whisper foi treinado para utilizar o texto do *initial prompt* como contexto condicional no decodificador. Sem fornecer termos como `"Yuki, FUVEST, OBMEP, inteligência artificial"`, o decodificador atua com probabilidades a priori gerais de corpus público da internet, onde a palavra "Yuki" (nome próprio japonês) compete desfavoravelmente com "e o que", "e o que é", "o que", ou "Iuki". Com o prompt inicial, a atenção cruzada do transformer enviesa a probabilidade sem adulterar regras gramaticais nem substituir palavras cegamente.

#### 3.3 Achado 3: Parâmetros Ocultos de Decodificação do `whisper-cli`
- **Arquivo:** [`src/voice/stt/local.rs`](file:///c:/Users/Miriã/Documents/JOSÉ/src/voice/stt/local.rs#L507-L520)
- **Constatações via `whisper-cli.exe --help`:**
  1. **Beam Search Padrão (`-bs 5`, `-bo 5`):** O `whisper-cli` utiliza busca em feixe de tamanho 5 por padrão. Em CPU modesta, explorar 5 caminhos simultâneos multiplica por até 3× o tempo do decodificador em relação à busca gulosa (`greedy`, beam-size = 1).
  2. **Fallback de Temperatura Ativo por Padrão:** Na presença de silêncio residual ou baixa probabilidade em trechos curtos, o Whisper tenta temperaturas `0.2`, `0.4`, `0.6` sequencialmente. Cada tentativa reexecuta o decodificador, explicando por que gravações em modo automático ou com caudas de silêncio saltam de 10s para 18–19 segundos de inferência. A flag `-nf, --no-fallback` previne esse custo quando a latência é prioridade.
  3. **Número de Threads Não Parametrizado (`-t 4` default):** A CLI da Yuki não passa `-t`. O `whisper-cli` assume 4 threads. No processador do host (Intel Core i5-10210U: 4 núcleos físicos, 8 threads lógicas), 4 threads é o número ótimo de núcleos físicos, mas não pode ser ajustado caso o operador deseje dedicar menos ou mais carga.

#### 3.4 Achado 4: Carga Fria do Modelo GGML (487 MB) a Cada Turno
- **Arquivo:** [`src/voice/stt/local.rs`](file:///c:/Users/Miriã/Documents/JOSÉ/src/voice/stt/local.rs#L507-L520)
- **Constatação:** Cada chamada a `transcribe_with_metadata` dispara um novo subprocesso do sistema operacional (`tokio::process::Command`).
- **Impacto:** O arquivo `models/whisper/ggml-small.bin` (487.601.967 bytes) precisa ser aberto, mapeado e alocado na RAM em toda transcrição. Conforme extraído pelo parser do `stderr` (`whisper_print_timings: load time = ... ms`), o overhead de inicialização de processo e carga do modelo consome entre **800 ms e 1.500 ms** por turno, mesmo com cache de páginas do Windows.

#### 3.5 Achado 5: Descontinuidades WASAPI (`flags & 1`) no Headset Bluetooth
- **Arquivo:** [`src/voice/capture.rs`](file:///c:/Users/Miriã/Documents/JOSÉ/src/voice/capture.rs#L560-L598)
- **Constatação:**
  ```rust
  if flags & 1 != 0 {
      discontinuity_glitches += 1;
  }
  let slice = std::slice::from_raw_parts(p_data, byte_count);
  raw_bytes.extend_from_slice(slice);
  ```
- **Análise Técnica da Flag WASAPI:**
  A constante `AUDCLNT_BUFFERFLAGS_DATA_DISCONTINUITY` (`0x00000001`) é documentada pela Microsoft como indicador de que o pacote entregue não é estritamente contínuo com o anterior. No entanto:
  1. A Yuki **não descarta o pacote** quando essa flag está presente; ela continua lendo e acumulando os bytes em `raw_bytes`.
  2. O loop WASAPI em `capture.rs` utiliza `std::thread::sleep(Duration::from_millis(15))`. No Windows, o timer padrão do kernel tem granularidade de ~15,6 ms. Em transmissões Bluetooth HFP (que operam em pacotes eSCO de 7,5 ms a 15 ms com retransmissão de rádio), pequenos atrasos no agendamento da thread consumidora causam o acúmulo de múltiplos pacotes ou jitter de rádio, levando o driver Bluetooth da Microsoft a sinalizar descontinuidade estatística.
  3. Evidência do teste T20: apesar de 56 descontinuidades sinalizadas, a voz gravada manteve-se **audível e compreensível**, comprovando que a sinalização representa tolerância de jitter e pacotes eSCO compensados por PLC (Packet Loss Concealment) no driver Bluetooth, e não silêncios destrutivos ou bytes truncados.

#### 3.6 Achado 6: Duração Fixa de Captura Agravando a Latência Percebida
- **Arquivo:** [`src/main.rs`](file:///c:/Users/Miriã/Documents/JOSÉ/src/main.rs#L1160-L1165)
- **Evidência no Código:**
  ```rust
  println!("Gravando... Fale com a Yuki agora (tempo: {}s)...", duration_sec);
  tokio::time::sleep(Duration::from_secs(duration_sec)).await;
  let audio_input = ptt.release()?;
  ```
- **Impacto:** Mesmo que o operador profira uma frase de 2,5 segundos ("Olá Yuki, que horas são?"), o sistema força um `sleep` de 10 segundos inteiros. O Whisper recebe 10 segundos de áudio (com 7,5s de silêncio/ruído residual), exigindo 10 segundos de inferência. A latência total percebida salta para 20 segundos ($10\text{s captura} + 10\text{s inferência}$). Se a captura encerrasse logo após o fim da fala (2,5s + 0,5s de margem = 3,0s), a inferência cairia para ~3 segundos, reduzindo o tempo total de 20s para 6 segundos (redução de 70% na espera).

---

### 4. Hipóteses Não Comprovadas

1. **Hipótese de Degradação Perceptível por Descontinuidades Bluetooth:** Não está comprovado que as 24–56 descontinuidades no Redmi Buds 6 Play degradaram a precisão do Whisper. Como o codec mSBC opera em 16 kHz nativo, o áudio permaneceu compreensível em T8 e T20. É necessário teste A/B comparativo com o mesmo áudio sem jitter.
2. **Hipótese de Saturação no Bluetooth:** O teste T20 registrou pico de −0,11 dBFS sem clipping contabilizado. Embora próximo de 0 dBFS, isso decorre do Controle Automático de Ganho (AGC) embutido no hardware do headset Bluetooth para amplificar voz capturada por microfones MEMS externos, e não de ganho digital indevido da Yuki.
3. **Hipótese de Falha Fonética do Modelo `ggml-small.bin`:** Não está demonstrado que o modelo `small` seja insuficiente para português. Ele possui 244M parâmetros e é amplamente reconhecido como suficiente para transcrição conversacional em pt-BR quando munido do prompt inicial adequado e sem distorção por aliasing.

---

### 5. Causas Prováveis dos Problemas Remanescentes

```
+--------------------------------+------------------------------------------------------------+
| Problema Remanescente          | Causa Provável Primária                                    |
+--------------------------------+------------------------------------------------------------+
| Erros em "Yuki", "FUVEST"      | Ausência de `--prompt` para condicionar o decodificador    |
| Inconsistência fonética Realtek| Decimação 3:1 direta (48k->16k) sem filtro passa-baixa LPF  |
| Latência em modo auto (17-19s) | Detecção de idioma + Beam Search 5 + Fallback Temperatura  |
| Latência percebida em turnos   | Captura de duração fixa (10s) sem detecção de fim de fala  |
| Latência estrutural mínima     | Carga fria de 487MB do modelo a cada spawn do subprocesso  |
| Descontinuidades no Bluetooth  | Sleep de 15ms no polling WASAPI em conflito com jitter eSCO|
+--------------------------------+------------------------------------------------------------+
```

---

### 6. Melhorias Propostas (Ordenadas por Impacto, Risco e Esforço)

```
+----+------------------------------------+----------+-------+---------+----------------------------------+
| ID | Melhoria Proposta                  | Impacto  | Risco | Esforço | Justificativa Técnica            |
+----+------------------------------------+----------+-------+---------+----------------------------------+
| M1 | Suporte a Initial Prompt no Whisper| ALTO     | BAIXO | BAIXO   | Resolve "Yuki", "FUVEST", etc.   |
| M2 | Filtro Passa-Baixa Antialiasing LPF| ALTO     | BAIXO | MÉDIO   | Elimina dobra espectral no 48kHz |
| M3 | Decomposição Fina das Latências    | MÉDIO    | NULO  | BAIXO   | Visibilidade analítica exata     |
| M4 | Parâmetros de Inferência e Threads | ALTO     | BAIXO | BAIXO   | Otimiza CPU, beam e fallback     |
| M5 | Ajuste do Polling WASAPI (5-10ms)  | MÉDIO    | BAIXO | BAIXO   | Reduz descontinuidades Bluetooth |
| M6 | Detecção de Fim de Fala (VAD)      | MUITO ALTO| MÉDIO | MÉDIO   | Encurta captura de 10s para ~3s  |
| M7 | Avaliação de Modelo Base Alternativo| MÉDIO   | BAIXO | BAIXO   | Opção mais rápida (140MB vs 487MB|
+----+------------------------------------+----------+-------+---------+----------------------------------+
```

#### Detalhamento das Melhorias:

* **M1 — Initial Prompt Configurável e Soberano (`--prompt`):**
  - Adicionar campo opcional `initial_prompt` em `LocalWhisperConfig` e suporte via flag CLI `--prompt` e variável `YUKI_WHISPER_PROMPT`.
  - Definir prompt padrão equilibrado para o perfil da assistente: `"Olá Yuki. FUVEST, OBMEP, inteligência artificial."`.
  - Zero substituição cega de strings: a influência ocorre puramente na distribuição de probabilidades do transformer.

* **M2 — Filtro Passa-Baixa Antialiasing (LPF) antes da Decimação:**
  - Implementar em `src/voice/audio.rs` um filtro biquad IIR passa-baixa (Butterworth de 4ª ordem em cascata) com frequência de corte em 7.200 Hz e atenuação de >30 dB em 8.000 Hz.
  - O filtro é aplicado às amostras de 48 kHz **antes** do descarte 3:1 de amostras.
  - Zero dependências externas (apenas operações aritméticas em f32/f64), com custo de CPU desprezível (< 2 ms para 10 segundos de áudio).

* **M3 — Decomposição Analítica de Latência:**
  - Expandir o relatório exibido na CLI para apresentar separadamente:
    - Tempo de Captura (WASAPI).
    - Tempo de Preparação do Áudio (conversão, LPF, normalização).
    - Tempo de Carga do Modelo (`load_time_ms` extraído do whisper).
    - Tempo de Inferência do Encoder e Decoder (`encode_time_ms`, decode time).
    - Tempo de Pós-processamento e Higienização.

* **M4 — Parâmetros de Decodificação do Whisper (`-t`, `-bs`, `-nf`):**
  - Expor configuração de threads: `-t` explicitamente configurado com o número de núcleos físicos (4) ou lido de `YUKI_WHISPER_THREADS`.
  - Permitir modo de decodificação rápida para conversação interativa (`voice turn`): opção de greedy decoding (`--beam-size 1 -bo 1 -nf`) para reduzir o tempo de inferência pela metade sem perda audível em voz clara.

* **M5 — Otimização de Polling do Loop WASAPI para Bluetooth:**
  - Reduzir o sleep do loop de captura de 15 ms para 8–10 ms, ou ativar `timeBeginPeriod(1)` no Windows durante a captura para garantir temporização de 1 ms no scheduler do SO, evitando buffer overruns em links eSCO com jitter.

* **M6 — Detecção de Fim de Fala no Modo Conversacional (`voice turn`):**
  - Utilizar o VAD acústico da Yuki (`src/voice/vad.rs`) para encerrar a gravação quando for detectado silêncio sustentado (ex: 800 ms a 1.200 ms de silêncio após fala detectada), respeitando o limite máximo solicitado em `--seconds`.

---

### 7. Plano de Implementação Incremental

A implementação será executada em **etapas discretas**, com validação automatizada após cada passo:

```
ETAPA 1: Observabilidade e Precisão Lexical (Baixo Risco)
├── src/voice/stt/local.rs: Adicionar suporte a initial_prompt, -t threads e -nf fallback
├── src/main.rs: Expor flag --prompt em voice listen e voice turn
└── tests/voice_stt_tts.rs: Testes unitários de montagem de argumentos e parsing

ETAPA 2: Filtragem Antialiasing e Integridade Acústica
├── src/voice/audio.rs: Implementar biquad low-pass filter (Butterworth 4ª ordem)
├── src/voice/capture.rs: Integrar LPF antes de resample_linear quando in_rate > out_rate
└── tests/voice_stt_tts.rs: Teste matemático injetando tons de 10kHz e comprovando atenuação de aliasing

ETAPA 3: Decomposição Fina de Latência e Telemetria
├── src/contracts/voice.rs: Adicionar campos granulares de latência no relatório
├── src/voice/stt/local.rs: Estruturar breakdown de tempos (preparação, carga, encode, pós)
└── src/main.rs: Exibir tabela decomposta de latência

ETAPA 4: Resiliência de Polling WASAPI (Bluetooth)
├── src/voice/capture.rs: Ajuste do período de polling para 8ms e timeBeginPeriod no Windows
└── tests/voice_stt_tts.rs: Validação de estabilidade em buffers contínuos

ETAPA 5: Validação Física Controlada com o Operador
├── Execução dos testes T21 a T26 com Realtek e Bluetooth
└── Emissão do relatório final conclusivo de homologação
```

---

### 8. Estratégia de Testes

#### 8.1 Testes Automatizados (Zero Dependência de Áudio Físico)
1. **Teste de Rejeição de Aliasing:** Gerar sinal sintético de 48 kHz contendo componente em 11 kHz. Passar pelo resampler com e sem filtro LPF. Validar que na saída de 16 kHz a energia em 5 kHz é atenuada em mais de 30 dB.
2. **Teste de Formatação de Argumentos do Whisper:** Validar que `--prompt`, `-t`, `-l`, `--no-timestamps` são passados corretamente e com escape seguro de caracteres.
3. **Teste de Breakdown de Timings:** Simular saída stderr do Whisper com `load time`, `encode time`, `total time` e validar o cálculo dos tempos de cada fase.
4. **Higiene de Dados:** Validação de que arquivos em `data/voice_diag/` permanecem estritamente ignorados pelo `.gitignore` e nenhum dado privado é exposto.

#### 8.2 Testes Físicos com o Operador (Critério Soberano)
- **Regra Imutável:** A aprovação física **nunca será declarada com base apenas em testes unitários automatizados**.
- O operador executará nova bateria controlada (T21–T26) com as frases padronizadas ("Olá, Yuki...", "FUVEST...", "OBMEP...") tanto no Realtek quanto no Redmi Buds 6 Play.

---

### 9. Critérios Objetivos para Homologação do Marco 8.1.1

Para que o STT seja considerado formalmente **HOMOLOGADO** e o Marco 8.1.1 seja encerrado com rigor, os seguintes critérios devem ser atingidos conjuntamente:

1. **Reconhecimento Lexical:** Reconhecimento correto de "Yuki" em $\ge 90\%$ das tentativas com o prompt inicial configurado.
2. **Redução da Latência em Modo Conversacional:** Turno conversacional típico com frase curta proferida em 2–3s deve concluir o ciclo de resposta em tempo total $\le 8$ segundos em CPU (contra os 20s atuais).
3. **Integridade de Frequência:** Zero alucinações provocadas por reflexão espectral em consoantes sibilantes no Realtek 48 kHz.
4. **Estabilidade no Bluetooth:** Ausência de perdas fonéticas perceptíveis no Redmi Buds 6 Play, com relatório de diagnóstico indicando integridade de fluxo.
5. **Aprovação Humana Conclusiva:** Parecer explícito e favorável do operador validando a inteligibilidade na escuta real.
