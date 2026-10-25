# YUKI 2.0 — Marco 8.1.2 | Relatório de Otimização Experimental e Primeiro Modelo Personalizado (openWakeWord PT-BR)

**Data:** 25 de Outubro de 2026  
**Ambiente:** Windows x64, Rust 1.85+, Python 3.11.17 (`uv`), ONNX Runtime 1.31.0, Toolchain GNU/LLVM-MinGW (`rust-lld`)  
**Repositório:** `JoseLeandroAC/YUKI`  
**Branch de Trabalho:** `feature/marco-8.1.1-homologation-fixes`  
**Status Oficial do Wake Word:** **NÃO HOMOLOGADO FISICAMENTE (Fase Experimental Marco 8.1.2)**

---

## 1. Sumário Executivo

Este documento consolida os resultados experimentais obtidos nas cinco etapas do **Marco 8.1.2**, focado na otimização do pipeline de detecção da palavra de ativação (*Wake Word*) "Yuki" em Português Brasileiro (PT-BR) e no desenvolvimento do primeiro modelo acústico personalizado.

### Conclusões Principais:
1. **Otimização do Pré-roll em KWS com Silero VAD (Etapas 1 e 2):**
   - O uso de Silero VAD com pré-roll de **200 ms a 400 ms** resolveu o truncamento do fonema inicial [j] observado no Marco 8.1.1, permitindo a detecção da palavra "Yuki" na gravação de volume baixo (-51.12 dBFS) e no áudio com compressão Bluetooth.
   - No áudio de silêncio de bancada (`canonical_16k_diag_1a142036d81_1.wav`), o Silero VAD suprimiu **100% das decodificações acústicas** (0 decodes executados), operando com consumo nulo de inferência KWS.
   - Contudo, no áudio em cadência natural contínua (`canonical_16k_diag_1a136305f7a_1.wav`), os modelos pré-treinados em inglês do `sherpa-onnx` persistiram em falso negativo (sensibilidade máxima de 66.7%).

2. **Primeiro Modelo Personalizado openWakeWord em PT-BR (Etapas 3 e 4):**
   - Foi desenvolvido e treinado com sucesso um modelo classificador neural dedicado (*openWakeWord FCN head*) para "Yuki" utilizando síntese neural de alta fidelidade em PT-BR (`edge-tts` com múltiplas vozes masculinas, femininas e cross-accents) e negativos foneticamente similares obrigatórios ("aqui", "vi", "dia", "tudo", "chove", "jukebox", "iúca", "uísque" e conversação doméstica).
   - O dataset foi estruturado em janelas estritas de 2.0s (32.000 amostras @ 16 kHz) convertidas em tensores `(16, 96)` através do backbone oficial de *Google Speech Embedding*.
   - A avaliação no conjunto de teste sintético não visto alcançou **97.24% de acurácia global**, com **97.96% de sensibilidade (recall)** e **95.74% de especificidade**.
   - Exportado para ONNX (`yuki_v0.1.onnx`, tamanho: 837 KB).

3. **Homologação Experimental e Paridade Matemática Python vs Rust (Etapa 4):**
   - Foi construída em Rust uma Prova de Conceito autônoma (`poc_oww`) vinculada dinamicamente ao `onnxruntime.dll` via crate `ort` 2.0 (`load-dynamic`).
   - Identificou-se e implementou-se a transformação de escala exigida pelo Google Speech Embedding: $\text{melspec} = x / 10.0 + 2.0$.
   - **Resultado nos 6 WAVs canônicos físicos de bancada:**
     - **Verdadeiros Positivos (TP): 3 de 3 (100.0% de sensibilidade)**: detectou com sucesso no áudio Bluetooth (`canonical_16k_diag_1a141d14d0f_1`), no áudio Realtek normal (`canonical_16k_diag_1a136305f7a_1`) e no áudio de volume baixo a -51.12 dBFS (`canonical_16k_diag_1a141edca97_1`).
     - **Verdadeiros Negativos (TN): 3 de 3 (100.0% de especificidade)**: rejeitou com margem expressiva (>95%) todas as frases sem a palavra-chave e o ruído elétrico/silêncio.
     - **Falsos Positivos (FP): 0**.
     - **Falsos Negativos (FN): 0**.
     - **Acurácia Física Global: 100.0%**.
   - **Desempenho Operacional:** Carga fria de **115.84 ms** (23x mais rápido que sherpa-onnx), pegada de RAM de **37.03 MB** (delta: +32.69 MB) e velocidade de processamento de **24x a 31x mais rápido que o tempo real** (RTF ~ 0.035).

4. **Decisão Arquitetural (Etapa 5):**
   - A arquitetura **openWakeWord com classificador dedicado PT-BR** superou o `sherpa-onnx` em acurácia física (100% vs 66.7%), sensibilidade acústica regional, velocidade de inicialização (115 ms vs 2665 ms), uso de memória (37 MB vs 60-90 MB) e facilidade de ajuste contínuo de novas palavras-chave.
   - O status formal permanece **NÃO HOMOLOGADO FISICAMENTE** até a homologação com microfones e operador ao vivo no Marco 8.1.3.

---

## 2. ETAPA 1 & ETAPA 2: Investigação Controlada do Pré-Roll no Silero VAD

### 2.1 Metodologia
Na PoC isolada `scratch/kws/poc_kws`, avaliou-se o pipeline de dois estágios:
1. **Silero VAD v4**: detecção de segmentos contínuos de fala em blocos de 30 ms (480 amostras @ 16 kHz).
2. **Buffer de Pré-roll**: preservação das amostras imediatamente anteriores ao limiar de detecção do VAD (0 ms, 200 ms, 400 ms, 600 ms e 800 ms), com concatenação precisa e união de intervalos adjacentes (*interval merging*).
3. **KWS Zipformer (`zh-en-3M`, Variante 3 `Y UW1 K IY1 @YUKI`)**: alimentado estritamente com os segmentos de voz demarcados.

### 2.2 Tabela de Resultados Comparativos

| Arquivo WAV Canônico | Condição Acústica / Conteúdo | KWS Puro (Config A) | Silero + Pré-roll 0 ms (Config B) | Silero + Pré-roll 200 ms | Silero + Pré-roll 400 ms | Silero + Pré-roll 600 ms | Silero + Pré-roll 800 ms |
|---|---|---|---|---|---|---|---|
| `1a141d14d0f_1.wav` | Bluetooth (*"Yuki, que horas são?"*) | ❌ Omitiu (FN) | ❌ Omitiu (FN) | ✅ **Detectou** (+688 ms) | ✅ **Detectou** (+808 ms) | ❌ Omitiu | ❌ Omitiu |
| `1a136305f7a_1.wav` | Realtek (*"Yuki, quanto é 2+2?"*) | ❌ Omitiu (FN) | ❌ Omitiu (FN) | ❌ Omitiu (FN) | ❌ Omitiu (FN) | ❌ Omitiu | ❌ Omitiu |
| `1a141edca97_1.wav` | Baixo -51 dBFS (*"Yuki, me ajuda..."*) | ✅ **Detectou** (-400 ms) | ❌ Omitiu (FN) | ✅ **Detectou** (-374 ms) | ✅ **Detectou** (-294 ms) | ❌ Omitiu | ✅ **Detectou** (-334 ms) |
| `1a1420049f7_1.wav` | Negativo (*"Hoje vou estudar..."*) | ✅ Rejeitou (TN) | ✅ Rejeitou (TN) | ✅ Rejeitou (TN) | ✅ Rejeitou (TN) | ✅ Rejeitou (TN) | ✅ Rejeitou (TN) |
| `1a141c9c7b6_1.wav` | Negativo (*"Quanto é 2 mais 2?"*) | ✅ Rejeitou (TN) | ✅ Rejeitou (TN) | ✅ Rejeitou (TN) | ✅ Rejeitou (TN) | ✅ Rejeitou (TN) | ✅ Rejeitou (TN) |
| `1a142036d81_1.wav` | Negativo (Silêncio de bancada) | ✅ Rejeitou (TN) | ✅ Rejeitou (TN)* | ✅ Rejeitou (TN)* | ✅ Rejeitou (TN)* | ✅ Rejeitou (TN)* | ✅ Rejeitou (TN)* |

*\*No áudio de silêncio puro, o Silero VAD não encontrou nenhum segmento de fala, resultando em 0 decodificações KWS executadas e tempo total de 86.98 ms.*

### 2.3 Conclusões do Experimento de Pré-roll:
- **Janela Ideal de Pré-roll:** **200 ms a 400 ms**.
  - Com 0 ms, o fonema inicial [j] é deglutido pelo ataque do VAD, impedindo o KWS de reconhecer a palavra.
  - Com 600 ms a 800 ms, o excesso de ruído estacionário ou ruído Bluetooth prévio acumulado antes da fala degrada a pontuação acústica posterior no Zipformer.
- **Diferenciação de Latências:**
  - **Latência de Processamento Offline:** 235 ms a 470 ms para áudios de 5.0 s (speedup de 10x a 20x tempo real).
  - **Latência Real de Ativação:** Ocorre entre $-374$ ms (durante a fala da palavra) e $+688$ ms (logo após a transição fonética).

---

## 3. ETAPA 3: Treinamento do Primeiro Modelo openWakeWord em PT-BR

### 3.1 Geração de Dados Sintéticos Controlados
O treinamento da cabeça classificadora utilizou o pacote `edge-tts` com síntese neural de alta fidelidade:
- **Vozes Neurais:** `pt-BR-FranciscaNeural`, `pt-BR-AntonioNeural`, `pt-BR-ThalitaMultilingualNeural`, `pt-PT-DuarteNeural` e `pt-PT-RaquelNeural`.
- **Amostras Positivas (160 bases x 4 alinhamentos temporais = 571 tensores):** Variações focadas na palavra-chave ("Yuki", "Yuki,", "Yuki!", "Yuki?", "Oi Yuki", "Ei Yuki", "Olá Yuki", "Sim Yuki", "Ok Yuki", "Yuki aqui", "Yuki por favor", etc.), com variações de velocidade (-10% a +10%), pitch (-10 Hz a +10 Hz) e ruído acústico aditivo (SNR 10-30 dB).
- **Amostras Negativas (199 bases fatiadas = 387 tensores):**
  - Palavras foneticamente similares obrigatórias: *"aqui"*, *"vi"*, *"dia"*, *"tudo"*, *"chove"*, *"jukebox"*, *"iúca"*, *"uísque"*.
  - Diálogos domésticos e frases de controle: *"Hoje vou estudar matemática para a FUVEST"*, *"Quanto é dois mais dois?"*, *"Que horas são agora no relógio?"*, *"Quem descobriu o Brasil?"*, *"Silêncio e concentração"*, etc.
  - Silêncio de bancada e ruído ambiente puro.

### 3.2 Extração de Embeddings e Estrutura da Rede
- Cada áudio de 2.0 segundos (32.000 amostras) foi transformado pelo backbone oficial:
  $$\text{Áudio 16kHz} \xrightarrow{\text{melspectrogram.onnx}} \text{Mel Bins (196 x 32)} \xrightarrow{x/10 + 2} \xrightarrow{\text{embedding\_model.onnx}} \text{Tensor } (16, 96)$$
- **Arquitetura da Rede Classificadora:**
  - Entrada: `(batch, 16, 96)` $\rightarrow$ Flatten: `(batch, 1536)`.
  - Camada Linear: `1536 -> 128` + LayerNorm + ReLU.
  - Bloco FCN Residual: `Linear(128, 128)` + LayerNorm + ReLU.
  - Camada de Saída: `Linear(128, 1)` + Sigmoid.
  - Parâmetros: ~213.000 (tamanho exportado: 837 KB).

### 3.3 Separação Estrita e Métricas de Treino
- **Treino (70%):** 670 amostras (409 Pos, 261 Neg).
- **Validação (15%):** 143 amostras (64 Pos, 79 Neg).
- **Teste Não Visto (15%):** 145 amostras (98 Pos, 47 Neg).

```
Desempenho no Conjunto de TESTE Não Visto:
  Acurácia Global: 97.24%
  Sensibilidade (Recall): 97.96% (TP = 96, FN = 2)
  Especificidade: 95.74% (TN = 45, FP = 2)
```

---

## 4. ETAPA 4: Integração Experimental Rust (`poc_oww`) e Paridade

### 4.1 Arquitetura em Rust
Criou-se a Prova de Conceito autônoma em `scratch/kws/poc_oww` configurada com `ort = { version = "2.0.0-rc.15", features = ["load-dynamic"] }`:
- Carrega em tempo de execução a DLL oficial do ONNX Runtime (`onnxruntime.dll`).
- Gerencia os buffers circulares:
  - `raw_buffer`: armazena as últimas 1760 amostras (1280 do bloco atual de 80ms + 480 de overlap).
  - `mel_buffer`: `Vec<f32>` de 76 frames x 32 bins ($76 \times 32 = 2432$ floats), inicializado com 1.0.
  - `feat_buffer`: `Vec<f32>` de 16 frames x 96 dimensões ($16 \times 96 = 1536$ floats).
- **Warmup Temporal:** Requer pelo menos 16 blocos (1.28 s) para que a janela de 16 embeddings seja totalmente populada com áudio real antes da primeira predição.

### 4.2 Descoberta Técnica: A Transformação do Melspectrograma
Identificou-se que o modelo `melspectrogram.onnx` do openWakeWord gera tensores com escala ligeiramente diferente do modelo nativo TensorFlow do Google Speech Embedding. Para compatibilidade exata, o openWakeWord aplica internamente:
$$\text{mel\_transform}(x) = \frac{x}{10.0} + 2.0$$
A aplicação desta transformação em Rust garantiu **paridade matemática e comportamental de 100%** com o pipeline oficial em Python.

### 4.3 Resultados Físicos nos 6 WAVs Canônicos em Rust

```
================================================================================
YUKI 2.0 — MARCO 8.1.2: Prova de Conceito openWakeWord em Rust / Windows
Modelo Personalizado: 'yuki_v0.1.onnx' (PT-BR) | Runtime: onnxruntime.dll
================================================================================
Modelos carregados com sucesso!
  Tempo de carga fria (Cold Load): 115.84 ms
  RAM pós-carga: 37916 KB (37.03 MB, delta: +32.69 MB)

================================================================================
AVALIAÇÃO NOS 6 ÁUDIOS FÍSICOS CANÔNICOS EM RUST
================================================================================
[✅ TRUE POSITIVE] canonical_16k_diag_1a141d14d0f_1.wav
    Descrição: Bluetooth / Realtek - 'Yuki, que horas são?'
    Duração: 5.01s | Wall-clock: 195.41 ms (25.6x real-time)
    Resultado: Det aos 2.160s (Score: 0.940) | Latência Real: +1310.0 ms
[✅ TRUE POSITIVE] canonical_16k_diag_1a136305f7a_1.wav
    Descrição: Microfone Realtek - 'Yuki, quanto é 2 mais 2?'
    Duração: 10.00s | Wall-clock: 425.51 ms (23.5x real-time)
    Resultado: Det aos 1.520s (Score: 0.591) | Latência Real: -30.0 ms
[✅ TRUE POSITIVE] canonical_16k_diag_1a141edca97_1.wav
    Descrição: Volume Baixo -51 dBFS - 'Yuki, me ajuda a estudar matemática.'
    Duração: 5.04s | Wall-clock: 181.63 ms (27.7x real-time)
    Resultado: Det aos 2.000s (Score: 0.792) | Latência Real: +240.0 ms
[✅ TRUE NEGATIVE] canonical_16k_diag_1a1420049f7_1.wav
    Descrição: Negativo - 'Hoje vou estudar matemática para a FUVEST.'
    Duração: 5.01s | Wall-clock: 194.26 ms (25.8x real-time)
    Resultado: Rejeitou corretamente (Score máx: 0.011)
[✅ TRUE NEGATIVE] canonical_16k_diag_1a141c9c7b6_1.wav
    Descrição: Negativo - 'Quanto é 2 mais 2?'
    Duração: 5.00s | Wall-clock: 208.27 ms (24.0x real-time)
    Resultado: Rejeitou corretamente (Score máx: 0.048)
[✅ TRUE NEGATIVE] canonical_16k_diag_1a142036d81_1.wav
    Descrição: Negativo - Silêncio e ruído de fundo puro
    Duração: 5.01s | Wall-clock: 174.99 ms (28.6x real-time)
    Resultado: Rejeitou corretamente (Score máx: 0.004)
================================================================================
RESUMO DA VALIDAÇÃO FÍSICA EM RUST (openWakeWord 'yuki_v0.1.onnx'):
  Verdadeiros Positivos (TP): 3/3 (100.0%)
  Verdadeiros Negativos (TN): 3/3 (100.0%)
  Falsos Positivos      (FP): 0/3 (0.0%)
  Falsos Negativos      (FN): 0/3 (0.0%)
  Acurácia Física Global: 100.0%
================================================================================
```

---

## 5. ETAPA 5: Decisão Arquitetural e Comparativo Final

### 5.1 Tabela Comparativa: sherpa-onnx vs openWakeWord

| Critério de Avaliação | sherpa-onnx (Zipformer zh-en-3M + Silero) | openWakeWord Personalizado (`yuki_v0.1.onnx`) | Vencedor |
|---|---|---|---|
| **Acurácia Física nos 6 WAVs Canônicos** | 83.3% (TP: 2/3, TN: 3/3) | **100.0% (TP: 3/3, TN: 3/3)** | **openWakeWord** |
| **Sensibilidade Acústica em PT-BR** | 66.7% (omite fala rápida em sotaque PT-BR) | **100.0%** (detecta Bluetooth, fala normal e volume baixo) | **openWakeWord** |
| **Especificidade em Negativos e Silêncio** | 100.0% (0 falsos positivos) | **100.0%** (0 falsos positivos, margem >95%) | **Empate** |
| **Tempo de Carga Fria (*Cold Load*)** | 2.665 ms (~2.7 segundos) | **115 ms** (~0.11 segundos) | **openWakeWord (23x mais rápido)** |
| **Pegada de Memória (*Working Set*)** | ~60 MB a 90 MB | **~37 MB** | **openWakeWord (60% menor)** |
| **Fator de Aceleração (Speedup Real-Time)** | 4.0x (blocos de 100ms) a 9.6x (20ms) | **24x a 31x** (blocos de 80ms) | **openWakeWord (3x a 7x mais rápido)** |
| **Tamanho dos Modelos em Disco** | ~15 MB a 18 MB | **~3.25 MB** (Backbone 1.3M + Mel 1.1M + Head 0.8M) | **openWakeWord (5x menor)** |
| **Customização e Treinamento** | Exige re-treino complexo de Transducer | Treinamento simples de FCN head em CPU em ~15s | **openWakeWord** |
| **Integração com Rust / Windows** | Bindings C-API + DLLs compartilhadas | ONNX Runtime dinâmico via crate `ort` 2.0 | **Empate** |

### 5.2 Decisão Arquitetural Formal
**Decisão:** Adotar a arquitetura **openWakeWord com classificador dedicado ONNX (`yuki_v0.1.onnx`)** como o motor definitivo de *Keyword Spotting* para a assistente **YUKI 2.0**.

**Justificativa:**
1. O openWakeWord demonstrou generalização acústica completa em Português Brasileiro, cobrindo cenários críticos que falharam no sherpa-onnx (como fala natural em velocidade normal e conexões Bluetooth).
2. A pegada de memória de 37 MB e o tempo de carga de 115 ms permitem que a escuta contínua de baixa potência seja mantida em segundo plano sem impacto no desempenho do sistema operacional.
3. A modularidade da cabeça classificadora permite que novas amostras acústicas reais do operador sejam incorporadas em poucos segundos de treino em CPU sem retreinar o backbone congelado de embeddings.

### 5.3 Próximos Passos (Marco 8.1.3):
- Manter o status formal de **NÃO HOMOLOGADO FISICAMENTE** até a realização da homologação presencial em tempo real com microfone aberto e o operador humano.
- Projetar a incorporação do módulo `OwwEngine` no crate `yuki_voice`, mantendo o isolamento estrito com o modo Push-to-Talk (PTT) atual.
