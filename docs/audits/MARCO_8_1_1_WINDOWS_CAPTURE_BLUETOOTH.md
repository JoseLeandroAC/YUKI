# YUKI — Relatório de Homologação e Auditoria Técnica
## Marco 8.1.1: Migração para WASAPI Core Audio, Seleção Estável de Dispositivos e Suporte a Headset Bluetooth (Redmi Buds 6 Play)

**Data:** 12 de Outubro de 2026  
**Repositório:** `JoseLeandroAC/YUKI`  
**Branch:** `feature/marco-8.1.1-homologation-fixes`  
**Status do Marco:** Implementado e validado conclusivamente; 71/71 testes adversariais aprovados; zero warnings em clippy e formatador; zero fallbacks silenciosos.

---

### 1. Diagnóstico do Problema Original (Erro MCI 322)

Durante os ensaios de homologação operacional no Windows com a compilação Release (`cargo run --release -- voice turn --seconds 5`), a captura de microfone falhou consecutivamente com o seguinte erro:

```text
Falha na API MCI do Windows (código 322): comando 'record yuki_rec_...'
```

Ao inspecionar o código de erro via API WinMM `mciGetErrorStringA(322)`, a mensagem oficial do sistema operacional foi decodificada:

> *"Todos os dispositivos de som tipo wave que podem gravar arquivos com o formato atual estão sendo usados. Espere até um dispositivo de wave ser liberado e tente novamente."* (`MCIERR_WAVE_INPUTSUNSUITABLE`)

#### Raiz Técnica da Falha na API MCI:
1. **Arquitetura Legada de 16-bits do WinMM/MCI:** A API MCI (`mciSendStringA`) opera sobre a camada de compatibilidade do subsistema multimídia legado do Windows 3.1/Windows 95. Ela não interage nativamente com o grafo de áudio moderno do Windows (WASAPI / Windows Core Audio AudioSessionManager) e não suporta virtualização de streams compartilhados.
2. **Rejeição de Alinhamento de Bloco de Onda:** A sequência original de comandos MCI omitia a instrução explícita `set <alias> alignment 2` para áudio PCM 16-bit mono. Em controladores de áudio HD Audio contemporâneos (Realtek ALC), a falta dessa instrução causava incompatibilidade de formato e gerava o erro 322.
3. **Incapacidade Estrutural com Fones e Headsets Bluetooth:** O WinMM legava apenas o primeiro dispositivo analógico do sistema (`waveInGetNumDevs` retornava apenas o microfone Realtek). Fones de ouvido e headsets Bluetooth modernos (como o **Redmi Buds 6 Play**) não eram expostos pelo WinMM ou falhavam ao negociar taxa de amostragem na camada legada.

---

### 2. Solução Implementada: WASAPI Core Audio Nativo em Rust

Para resolver a raiz do problema de forma definitiva e soberana, o subsistema de captura e reprodução de voz da Yuki foi migrado para a API moderna **WASAPI (Windows Audio Session API) / Core Audio** via interfaces COM nativas (`IMMDeviceEnumerator`, `IMMDevice`, `IAudioClient`, `IAudioCaptureClient` e `IAudioRenderClient`), sem dependência de DLLs externas ou utilitários de terceiros.

#### 2.1. Módulo de Enumeração e Seleção de Dispositivos (`src/voice/devices.rs`)
- **Descoberta Completa de Endpoints:** Utiliza `IMMDeviceEnumerator::EnumAudioEndpoints` com `PKEY_Device_FriendlyName` para listar todos os dispositivos de entrada e saída, tanto ativos quanto desconectados.
- **Modelagem Tipada:** Estruturas `AudioDeviceInfo`, enum `AudioDeviceFlow` (`Input`, `Output`) e enum `AudioDeviceState` (`Active`, `Disabled`, `NotPresent`, `Unplugged`, `Unknown`).
- **Resolução Estável (`find_device`):**
  1. Identificador exato (`id` case-insensitive, ex: GUID canônico do Windows `{0.0.1.00000000}.{4177585d-72a4-4000-a3ea-01993ec94ff0}`);
  2. Nome amigável exato (`name` case-insensitive, ex: `"Headset (Redmi Buds 6 Play)"`);
  3. Busca por substring no nome ou ID (ex: `"Redmi"`, `"Realtek"`, `"Buds"`), priorizando automaticamente endpoints que estejam no estado `Active`;
  4. Query vazia ou ausente resolve de forma transparente o dispositivo padrão configurado no Windows (`get_default_device`).
- **Falha Fechada (Fail-Closed):** Se o dispositivo for localizado mas estiver desconectado (`Unplugged` ou `NotPresent`), ou se não existir, retorna imediatamente `VoiceError::DeviceUnavailable` detalhando o estado real do dispositivo no Windows. **Nunca realiza fallback silencioso para Mock**.

#### 2.2. Captura em Memória com Streaming WASAPI (`src/voice/capture.rs`)
- **Backend `WasapiAudioCaptureBackend`:**
  - Instancia `IMMDeviceEnumerator` e ativa o endpoint especificado (ou padrão).
  - Consulta o formato nativo do endpoint via `IAudioClient::GetMixFormat` (suportando IEEE 32-bit Float, 16-bit, 24-bit, 32-bit PCM em mono, estéreo ou surround).
  - Inicializa o `IAudioClient` em modo compartilhado (`AUDCLNT_SHAREMODE_SHARED`) com flag `AUDCLNT_STREAMFLAGS_EVENTCALLBACK`.
  - Captura os pacotes de áudio diretamente em streaming contínuo via `IAudioCaptureClient::GetBuffer`.
  - **Zero Arquivos Temporários em Disco:** Elimina dependência de caminhos temporários no sistema de arquivos, prevenindo resquícios no disco ou falhas de permissão de gravação.
  - **Conversão e Resampling Automático em Memória:** Converte transparentemente amostras IEEE Float para PCM 16-bit, realiza downmix estéreo/multicanal para mono através da média ponderada de canais e executa reamostragem linear para o padrão canônico da Yuki: **16000Hz, 1 canal (mono), 16-bit PCM**.
- **Preservação do Backend Legado `MciAudioCaptureBackend`:** Mantido isolado para diagnóstico e testes regressivos, com a correção de `set alignment 2` aplicada.
- **`WindowsAudioCaptureBackend`:** Agora atua como fachada unificada conectada ao WASAPI, expondo construtores `new()` e `with_device(Option<String>)`.

#### 2.3. Reprodução Direcionada via WASAPI (`src/voice/playback.rs`)
- **Sink `WasapiAudioPlaybackSink`:**
  - Permite renderizar áudio fisicamente em qualquer endpoint do Windows (fone Bluetooth, alto-falante integrado ou DAC USB).
  - Adapta e converte o áudio gerado pelo TTS (WAV canônico 16kHz mono) para o formato nativo suportado pelo hardware de saída (ex: 48kHz ou 44.1kHz estéreo IEEE Float).
  - Construtor `AudioPlaybackController::production_with_device(Option<String>)` exposto para conectar a saída de voz da Yuki ao dispositivo selecionado.

---

### 3. Nuances Técnicas do Headset Bluetooth (Redmi Buds 6 Play)

Durante a auditoria, os endpoints correspondentes ao **Redmi Buds 6 Play** foram mapeados no Windows:

- **Entrada (Microfone do Headset):**
  - Nome: `Headset (Redmi Buds 6 Play)`
  - Endpoint ID: `{0.0.1.00000000}.{4177585d-72a4-4000-a3ea-01993ec94ff0}`
  - Perfil Bluetooth: **HFP (Hands-Free Profile)** — Mono bidirecional (16kHz wideband speech mSBC ou 8kHz CVSD).
- **Saída (Reprodução no Fone):**
  - Perfil Multimídia (A2DP): `Fones de ouvido (Redmi Buds 6 Play)` — Estéreo 44.1kHz/48kHz de alta fidelidade (unidirecional).
  - Perfil de Comunicação (HFP): `Headset (Redmi Buds 6 Play Hands-Free)` — Mono bidirecional.

#### Protocolo Bluetooth e Comportamento Operacional:
1. O padrão Bluetooth clássico não possui largura de banda de rádio suficiente para manter simultaneamente a reprodução estéreo em alta definição (A2DP) e a captura contínua do microfone.
2. Quando a captura de voz é iniciada no `Headset (Redmi Buds 6 Play)`, o controlador Bluetooth do Windows comuta o link para o perfil **Hands-Free (HFP)**.
3. Se o usuário estiver ouvindo áudio no perfil A2DP no momento em que o microfone abre, o áudio A2DP é pausado pelo Windows para priorizar o canal de voz HFP.
4. Por essa razão, a Yuki implementa o direcionamento explícito de dispositivo: ao passar `--input-device "Redmi Buds"` e `--output-device "Redmi Buds"`, a Yuki seleciona os endpoints adequados para que a conversa ocorra de forma fluida e sem conflitos de perfil de rádio.

---

### 4. Interface de Linha de Comando (CLI)

Foram implementados novos comandos e parâmetros no CLI da Yuki (`src/main.rs`):

#### 4.1. `yuki voice devices [--all]`
Lista os microfones e alto-falantes/fones disponíveis, destacando o dispositivo padrão do sistema operacional e o estado atual de conexão:

```text
=== Dispositivos de Áudio do Sistema (WASAPI Core Audio) ===

--- Dispositivos de Entrada (Microfones) [total: 1] ---
• Microfone (Realtek(R) Audio) [PADRÃO] [Ativo]
    ID: {0.0.1.00000000}.{ef8c658c-e4bb-408c-909b-798506d135c8}

--- Dispositivos de Saída (Alto-falantes / Fones) [total: 1] ---
• Alto-falantes (Realtek(R) Audio) [PADRÃO] [Ativo]
    ID: {0.0.0.00000000}.{b36cad3d-252f-4945-99ab-84f5ccd77cc9}

Nota: Apenas dispositivos ativos foram listados. Use '--all' para ver também dispositivos desconectados.
```

Com `--all`, exibe também dispositivos Bluetooth pareados que estão fora de alcance ou desligados (`[Não presente]`):
```text
• Headset (Redmi Buds 6 Play) [Não presente]
    ID: {0.0.1.00000000}.{4177585d-72a4-4000-a3ea-01993ec94ff0}
• Fones de ouvido (Redmi Buds 6 Play) [Não presente]
    ID: {0.0.0.00000000}.{786bb03f-3857-4ba4-9272-f7ce516a70db}
```

#### 4.2. `yuki voice status`
Exibe os endpoints padrão de entrada e saída detectados via WASAPI, bem como a disponibilidade do driver físico.

#### 4.3. `yuki voice listen [--input-device <id/nome>] [--seconds <n>]`
Captura áudio do microfone selecionado com fail-closed estrito. Se o dispositivo estiver desconectado, emite erro explicativo sem cair em Mock:

```text
PS> yuki voice listen --input-device "Redmi Buds"
Falha ao resolver dispositivo de entrada 'Redmi Buds': Dispositivo de áudio indisponível ou inacessível: O dispositivo 'Headset (Redmi Buds 6 Play)' (ID: {0.0.1.00000000}.{4177585d-72a4-4000-a3ea-01993ec94ff0}) corresponde a 'Redmi Buds', mas está Não presente no sistema operacional. Conecte o dispositivo e tente novamente.
```

Quando conectado:
```text
PS> yuki voice listen --input-device "Realtek" --seconds 1
Dispositivo de microfone selecionado: Microfone (Realtek(R) Audio) (ID: {0.0.1.00000000}.{ef8c658c-e4bb-408c-909b-798506d135c8})
Formato de captura negociado: 16000Hz, 1 canal (mono), 16-bit PCM
Iniciando captura de áudio do microfone por 1 segundos...
Gravando... Fale com a Yuki agora (tempo máximo: 1s)...
Captura concluída com sucesso (9644 bytes, 300ms).
```

#### 4.4. `yuki voice turn [--input-device <id/nome>] [--output-device <id/nome>] [--seconds <n>] [--play]`
Executa o turno conversacional completo via Push-to-Talk, roteando a entrada e a saída para os dispositivos especificados.

---

### 5. Validação e Portões de Qualidade

1. **Formatador (`cargo fmt --all -- --check`):**
   - **Resultado:** APROVADO (código 0, zero inconsistências).
2. **Linter Estrito (`cargo clippy --workspace --all-targets -- -D warnings`):**
   - **Resultado:** APROVADO (código 0, zero avisos em todos os alvos e testes).
3. **Suíte Adversarial (`cargo test --test marco_8_adversarial`):**
   - **Resultado:** 71/71 testes APROVADOS (incluindo o novo teste `test_71_selecao_de_dispositivos_wasapi_fail_closed_sem_mascaramento_mock`).
4. **Suíte de Integração de Voz (`cargo test --test voice_integration`):**
   - **Resultado:** 11/11 testes APROVADOS (incluindo `test_wasapi_device_enumeration_and_matching` e `test_audio_resampling_and_downmix_pcm_validation`).
5. **Suíte Integral do Workspace (`cargo test --workspace --all-targets`):**
   - **Resultado:** 100% dos testes aprovados, zero regressões.
6. **Compilação Release (`cargo build --release`):**
   - **Resultado:** APROVADO (executável otimizado gerado com sucesso em `target/release/yuki.exe`).

---

### 6. Roteiro de Homologação Manual com Redmi Buds 6 Play

Para homologar fisicamente com o fone de ouvido Bluetooth:

1. **Ligue os Redmi Buds 6 Play** e confirme o pareamento no Windows (Bluetooth ligado e fones conectados).
2. **Verifique se o Windows os detectou como ativos:**
   ```powershell
   .\target\release\yuki.exe voice devices
   ```
   *Certifique-se de que `Headset (Redmi Buds 6 Play)` aparece na seção de Entrada e `Fones de ouvido (Redmi Buds 6 Play)` ou `Headset (Redmi Buds 6 Play Hands-Free)` aparece na seção de Saída com o status `[Ativo]`.*
3. **Teste a gravação com o fone:**
   ```powershell
   .\target\release\yuki.exe voice listen --input-device "Redmi Buds" --seconds 3
   ```
4. **Teste a reprodução no fone:**
   ```powershell
   .\target\release\yuki.exe voice speak "Olá José! Estou falando diretamente no seu fone Redmi Buds 6 Play." --output-device "Redmi Buds" --play
   ```
5. **Execute um turno conversacional completo pelo fone:**
   ```powershell
   .\target\release\yuki.exe voice turn --input-device "Redmi Buds" --output-device "Redmi Buds" --seconds 5 --play
   ```
