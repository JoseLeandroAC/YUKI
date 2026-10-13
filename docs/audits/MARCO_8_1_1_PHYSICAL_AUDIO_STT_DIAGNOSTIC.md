# YUKI — Relatório de Auditoria Diagnóstica
## Investigação da Causa Raiz do Áudio Físico e da Fidelidade do STT (Marco 8.1.1)

**Repositório:** `JoseLeandroAC/YUKI`  
**Branch de Trabalho:** `feature/marco-8.1.1-homologation-fixes`  
**Commit Base:** `2c5cdfc`  
**Data da Auditoria:** 13 de outubro de 2026  
**Status da Homologação:** **STT NÃO HOMOLOGADO FISICAMENTE (AUDITORIA DIAGNÓSTICA CONCLUÍDA; AGUARDANDO VALIDAÇÃO HUMANA CONTROLADA)**  

---

### 1. Declaração Inicial e Princípio de Rigor

Em estrita consonância com os princípios de integridade técnica da Yuki, **o subsistema de reconhecimento local de fala (STT Whisper) NÃO é considerado homologado fisicamente** até que testes práticos com o operador humano comprovem a inteligibilidade e exatidão da fala transcrita nos dispositivos reais de hardware.

Este documento consolida a investigação técnica aprofundada, as descobertas matemáticas e empíricas, a instrumentação diagnóstica opt-in implementada e o roteiro para os testes humanos finais.

---

### 2. Investigação Técnica e Validação de Hipóteses

Foram investigadas quatro hipóteses sobre as causas da baixa fidelidade de reconhecimento observada na homologação anterior com o microfone Realtek e com o headset Bluetooth Redmi Buds 6 Play:

```
+---------------------------------------------------------------------------------------------------------+
|                                    QUADRO DE VALIDAÇÃO DE HIPÓTESES                                     |
+----+--------------------------------------------+-----------------------+-------------------------------+
| ID | Descrição da Hipótese                      | Classificação         | Evidência Empírica            |
+----+--------------------------------------------+-----------------------+-------------------------------+
| H1 | Discrepância de duração e corte inicial    | COMPROVADA (100%)     | Setup WASAPI de ~680ms a 1,3s |
| H2 | Corrupção de formato FFI no Realtek        | COMPROVADA (100%)     | Struct packing bug (Float/Int)|
| H3 | Aliasing espectral na reamostragem         | TEÓRICA / EM ANÁLISE  | Resampler linear sem anti-alias|
| H4 | Degradação por perfil Bluetooth SCO/HFP    | TEÓRICA / EM ANÁLISE  | 16kHz nativo com AGC agressivo|
+----+--------------------------------------------+-----------------------+-------------------------------+
```

#### 2.1 Hipótese 1 (Comprovada): Discrepância de Duração e Perda de Sílabas Iniciais

* **Sintoma Observado:** Ao solicitar 10 segundos de gravação, o áudio resultante no microfone Realtek possuía 9,17 segundos e, no headset Bluetooth, ~8,7 segundos. Além disso, a primeira sílaba ("Olá...") era frequentemente deglutida ou omitida.
* **Causa Raiz Empírica:** A thread WASAPI (`WasapiAudioCaptureBackend::start_capture`) demorava entre 680ms e 835ms no Realtek (e até 1300ms no Bluetooth) para enumerar dispositivos COM, inicializar `IAudioClient`, negociar mix format, instanciar `IAudioCaptureClient` e disparar o hardware com `IAudioClient::Start()`. No entanto, a thread principal da aplicação iniciava o temporizador de contagem regressiva (`std::thread::sleep(Duration::from_secs(seconds))`) **imediatamente após disparar a thread trabalhadora**, antes que o hardware começasse a capturar amostras de fato. Dessa forma, `10.0s - 0.835s = 9.165s` (~9,17 segundos de áudio gravado). O operador falava "Olá..." enquanto o hardware ainda estava no ciclo de setup COM!
* **Correção Implementada:** Foi introduzido um canal síncrono `std::sync::mpsc::channel` com timeout determinístico de 5 segundos. O método `start_capture()` agora aguarda a confirmação de que `IAudioClient::Start()` retornou `S_OK` antes de devolver o controle à thread principal.
* **Evidência Empírica Pós-Fix:** Em teste real de 2 segundos com o Realtek:
  - `Setup COM + WASAPI: 0.68s (676 ms)`
  - `Gravação Efetiva: 2.01s (2013 ms)`
  - `Amostras Geradas: 2.00s (2000 ms — 96.000 frames)`
  - A fala inicial agora é capturada integralmente desde a primeira sílaba.

#### 2.2 Hipótese 2 (Comprovada): Corrupção Estrutural de FFI e Ruído Digital no Realtek

* **Sintoma Observado:** O microfone Realtek funcionava normalmente em outros aplicativos do Windows, mas na Yuki produzia transcrições desconexas com graves alucinações fonéticas ("eu sou o que são, eu sou o microf").
* **Causa Raiz Empírica:** O driver nativo do Realtek negocia formato `WAVE_FORMAT_EXTENSIBLE` (0xFFFE) em IEEE Float de 32 bits, 48.000 Hz estéreo (384.000 bytes/segundo). Na definição da FFI em `src/voice/capture.rs`, as estruturas `WAVEFORMATEX` e `WAVEFORMATEXTENSIBLE` não declaravam `#[repr(C, packed(2))]`. O compilador Rust inseria 4 bytes de padding para alinhar o campo `dw_channel_mask` (u32), deslocando o campo `sub_format` (GUID) em 4 bytes na memória. Ao testar `sub_format == KSDATAFORMAT_SUBTYPE_IEEE_FLOAT`, a comparação falhava. Como consequência, o áudio era decodificado pelo branch de fallback como **PCM Inteiro de 32 bits** (`>> 16`). Cada valor de ponto flutuante no intervalo `[-1.0, 1.0]` era interpretado como inteiro, gerando ruído digital de intensidade colossal.
* **Medição Acústica do Silêncio Antes do Fix:**
  - RMS: 15.218 (-6.66 dBFS) — ruído ensurdecedor em sala silenciosa!
  - Pico: 32.767 (0.00 dBFS)
  - Clipping: 24.38% de amostras ceifadas!
* **Correção Implementada:** Foi aplicado `#[repr(C, packed(2))]` e leitura unaligned com `std::ptr::read_unaligned` nas estruturas de formato de captura WASAPI.
* **Medição Acústica do Silêncio Após o Fix:**
  - Formato detectado: `WAVE_FORMAT_EXTENSIBLE (IEEE Float)` | GUID: `00000003-0000-0010-8000-00aa00389b71`
  - RMS: 203.3 (-44.15 dBFS) — piso de ruído perfeitamente acústico e limpo.
  - Pico: 540 (-35.66 dBFS)
  - Clipping: 0 (0.00%)!
  - Resultado do Whisper: `""` (silêncio reconhecido como silêncio, sem nenhuma alucinação!).

#### 2.3 Hipótese 3 (Teórica / Próximo Passo): Aliasing Espectral na Reamostragem Linear

* **Fundamentação Acústica:** A taxa nativa do Realtek é 48.000 Hz, enquanto a taxa canônica do Whisper é 16.000 Hz (fator de decimação 3:1). A função `resample_linear` realiza interpolação linear simples sem aplicar filtro passa-baixa anti-aliasing antes da decimação.
* **Impacto Teórico:** Conforme o teorema de Nyquist-Shannon, qualquer componente de frequência acima de 8.000 Hz presente no sinal de 48.000 Hz sofre reflexão espectral (aliasing) de volta para a faixa de 0 a 8.000 Hz. Na voz humana, consoantes sibilantes e fricativas (/s/, /z/, /f/, /x/) concentram energia entre 8.000 Hz e 14.000 Hz. O aliasing deforma os bancos de filtros de energia Mel gerados pelo Whisper, reduzindo a precisão das consoantes.
* **Ação Recomendada:** Manter como hipótese teórica documentada. Os novos WAVs de diagnóstico permitirão auditar se a correção do FFI (Hipótese 2) já foi suficiente para homologar a fala ou se um filtro sinc/FIR passa-baixa será necessário.

#### 2.4 Hipótese 4 (Teórica / Microfone Bluetooth Redmi Buds 6 Play): Perfil HFP e AGC

* **Fundamentação Acústica:** Quando conectado em modo de microfone bidirecional, o Bluetooth Windows força o perfil Hands-Free Profile (HFP) utilizando codec mSBC (16.000 Hz mono) ou CVSD (8.000 Hz mono).
* **Vantagem:** Em 16.000 Hz nativo, o sinal não passa por downmix estéreo nem por reamostragem, contornando a Hipótese 3.
* **Desafio:** Fones True Wireless aplicam Controle Automático de Ganho (AGC) e supressão agressiva de ruído de hardware nos microfones externos, atenuando as primeiras sílabas e reduzindo o ganho médio da voz se o usuário falar baixo ou afastado.

---

### 3. Implementação do Sistema Diagnóstico Opt-in

Para permitir uma auditoria sem achismos e preservar a privacidade do usuário em produção, foi criado um subsistema completo de telemetria e diagnóstico acústico:

#### 3.1 Contrato Opt-in Rigoroso
- **Desativado por Padrão:** Zero gravação em disco e zero arquivos gerados durante a operação normal (`diagnostics_enabled: false`).
- **Ativação Explícita:** Habilitado exclusivamente via argumento CLI `--diagnostics` (ou `--diag`) nos comandos `voice listen` e `voice turn`, ou pela variável de ambiente `YUKI_VOICE_DIAGNOSTICS=1`.

#### 3.2 Estrutura e Localização dos Artefatos
Todos os arquivos são gravados no diretório `data/voice_diag/` (ignorado no `.gitignore`):
1. `raw_captured_{session_id}.wav`: Áudio bruto no formato nativo do driver (ex: Float 32-bit, 48kHz estéreo).
2. `canonical_16k_{session_id}.wav`: Áudio pós-conversão e downmix (16kHz PCM 16-bit mono).
3. `normalized_16k_{session_id}.wav`: Áudio após o ganho de normalização de pico aplicado pelo STT Whisper.
4. `diag_{session_id}.json`: Relatório estruturado em formato JSON com métricas completas de captura, temporização e acústica.

#### 3.3 Métricas Acústicas e de Hardware Calculadas
- **Nativo:** Formato tag, canais, taxa de amostragem, bits por amostra, `BlockAlign`, bytes por segundo e `SubFormat GUID`.
- **Temporização:** Tempo de setup COM/WASAPI (ms), tempo de gravação efetiva (ms) e duração das amostras geradas (ms).
- **Pacotes:** Contagem de pacotes WASAPI lidos, frames totais, pacotes marcados com buffer silencioso e descontinuidades.
- **Acústica:** Energia RMS (linear e dBFS), pico máximo (linear e dBFS), amostras em clipping (e razão percentual) e proporção estimada de silêncio acústico.

---

### 4. Suíte de Testes Automatizados e Portões de Qualidade

Foram executados e validados todos os portões de qualidade do repositório:

1. **Testes Unitários de Diagnóstico (`tests/voice_stt_tts.rs`):**
   - `test_audio_codec_encode_raw_wav_formats`: Validação dos cabeçalhos WAV para PCM 16-bit, PCM 24-bit e IEEE Float 32-bit.
   - `test_acoustic_metrics_computation`: Validação matemática dos cálculos de RMS, dBFS, pico e silêncio acústico.
   - `test_voice_diagnostics_session_artifacts_and_cleanup`: Validação da criação e limpeza higiênica dos artefatos em `data/voice_diag/`.
   - **Resultado:** 19 testes executados e 100% aprovados.
2. **Suíte Completa do Projeto (`cargo test`):**
   - Mais de 104 testes de integração, adversariais, criptografia de vault e contratos de voz aprovados sem nenhuma falha.
3. **Qualidade Estática (`cargo fmt` & `cargo clippy`):**
   - `cargo fmt --check`: 100% aderente ao padrão.
   - `cargo clippy --all-targets --all-features -- -D warnings`: Zero advertências.
4. **Compilação de Produção (`cargo build --release`):**
   - Compilação do binário final `target/release/yuki.exe` concluída com sucesso.

---

### 5. Roteiro de Homologação Humana Controlada no Windows

Para comprovar fisicamente a inteligibilidade e eficácia do STT antes da homologação final, o operador humano deve executar os testes descritos abaixo no PowerShell.

#### 5.1 Protocolo Padronizado de Teste
* **Frase de Referência Obrigatória:**  
  `"Olá, Yuki. Meu nome é José. Hoje estou testando o seu microfone."`
* **Ambiente:** Sala silenciosa, falando em tom de voz natural e ritmo pausado.

#### 5.2 Teste A — Microfone Interno / Realtek (Validação do Fix Float 32-bit)
1. Certifique-se de que o Microfone Realtek é o dispositivo padrão de gravação no Windows.
2. No PowerShell, execute a captura diagnóstica por 10 segundos:
   ```powershell
   cargo run --release -- voice listen --mic --seconds 10 --diagnostics
   ```
3. Fale a frase padronizada com clareza.
4. Inspecione o relatório emitido no terminal:
   - Verifique se a duração das amostras geradas foi de ~10.0s.
   - Verifique se a tag é `WAVE_FORMAT_EXTENSIBLE (IEEE Float)`.
   - Verifique se a transcrição reconheceu corretamente a frase.

#### 5.3 Teste B — Headset Bluetooth Redmi Buds 6 Play
1. Conecte o fone Redmi Buds 6 Play ao Windows via Bluetooth.
2. Identifique o ID do dispositivo de captura via:
   ```powershell
   cargo run --release -- voice status
   ```
3. Execute a captura diagnóstica apontando para o headset:
   ```powershell
   cargo run --release -- voice listen --device "Redmi Buds 6 Play" --seconds 10 --diagnostics
   ```
4. Fale a frase padronizada diretamente no microfone do fone.
5. Inspecione a transcrição obtida.

#### 5.4 Teste C — Turno Conversacional Completo (Listen + Whisper + TTS)
Execute o ciclo interativo completo:
```powershell
cargo run --release -- voice turn --seconds 8 --diagnostics
```

---

### 6. Critérios Objetivos de Aprovação

Para que o STT seja formalmente homologado na próxima intervenção, os seguintes critérios devem ser atendidos nos testes físicos:

1. **Precisão da Transcrição:** Word Error Rate (WER) inferior a 10% na frase padronizada (palavras-chave "Olá", "Yuki", "José", "microfone" perfeitamente reconhecidas).
2. **Preservação Fonética Inicial:** Ausência total de corte da saudação inicial ("Olá...").
3. **Ausência de Alucinações:** Eliminação completa de alucinações de ruído digital ("eu sou o microf").
4. **Relatório Diagnóstico Conclusivo:** Confirmação de que os arquivos em `data/voice_diag/` reproduzem som limpo e audível sem ruídos de saturação.
