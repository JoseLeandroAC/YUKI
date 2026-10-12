# YUKI — Guia de Configuração e Dependências de Voz (Marco 8: Voice v1)

Este guia orienta a configuração dos provedores locais de fala para texto (STT) e texto para fala (TTS) da plataforma Yuki.

---

## 1. Princípio Arquitetural de Voz

> **Local First — Cloud Only When Authorized**
> Toda a captura, processamento de sinal, palavra de ativação (*Wake Word*) e síntese acústica básica operam **100% localmente e offline** por padrão. O envio de dados para provedores de nuvem requer dupla autorização explícita do operador: configuração ativa e permissão formal de egress específica.

---

## 2. Reconhecimento de Fala Local (Whisper STT)

A Yuki integra-se ao motor local de inferência neural Whisper (via `whisper.cpp`), garantindo transcrição veloz, privada e sem dependência de internet.

### 2.1. Instalação do Binário `whisper.cpp`
1. Baixe o release pré-compilado ou compile a partir do código-fonte oficial:
   ```bash
   git clone https://github.com/ggerganov/whisper.cpp.git
   cd whisper.cpp
   cmake -B build
   cmake --build build --config Release
   ```
2. No Windows, o executável gerado será `whisper-cli.exe` (ou `main.exe`).

### 2.2. Download dos Modelos GGML
Baixe os modelos recomendados no formato GGML:
- **Base (Recomendado para CPU / Equilíbrio)**: `ggml-base.bin` (~140 MB)
- **Small (Alta Precisão)**: `ggml-small.bin` (~460 MB)

### 2.3. Configuração de Variáveis de Ambiente
Defina as seguintes variáveis de ambiente no sistema ou no arquivo `.env`:
```powershell
# Windows PowerShell
$env:YUKI_WHISPER_BIN = "C:\caminho\para\whisper.cpp\build\bin\Release\whisper-cli.exe"
$env:YUKI_WHISPER_MODEL = "C:\caminho\para\modelos\ggml-base.bin"
```
```bash
# Linux / macOS
export YUKI_WHISPER_BIN="/usr/local/bin/whisper-cli"
export YUKI_WHISPER_MODEL="/opt/yuki/models/ggml-base.bin"
```

### 2.4. Salvaguardas e Limites
- Formato esperado: **WAV PCM linear, 16 kHz, 16-bit mono**.
- Duração máxima por turno: **30 segundos**. Áudios que excederem esse limite são rejeitados de forma segura (*fail-closed*).
- Áudios corrompidos, vazios ou sem cabeçalho RIFF/WAV válido são rejeitados antes de qualquer envio ao motor.

---

## 3. Síntese de Voz Feminina Local (Piper Neural TTS)

Para reproduzir a identidade vocal da Yuki de forma calorosa, feminina e natural, utilizamos o motor neural ultraleve Piper.

### 3.1. Instalação do Binário Piper
1. Baixe os binários estáveis do [Piper TTS Releases](https://github.com/rhasspy/piper/releases).
2. Extraia o binário `piper` (ou `piper.exe`) para um diretório local.

### 3.2. Download da Voz Feminina pt-BR
A identidade de voz v1 da Yuki requer uma voz feminina natural em português do Brasil:
- Modelo: `pt_BR-faber-medium.onnx`
- Configuração: `pt_BR-faber-medium.onnx.json`

### 3.3. Configuração de Variáveis de Ambiente
```powershell
# Windows PowerShell
$env:YUKI_PIPER_BIN = "C:\caminho\para\piper\piper.exe"
$env:YUKI_PIPER_MODEL = "C:\caminho\para\vozes\pt_BR-faber-medium.onnx"
$env:YUKI_PIPER_CONFIG = "C:\caminho\para\vozes\pt_BR-faber-medium.onnx.json"
```
```bash
# Linux / macOS
export YUKI_PIPER_BIN="/usr/local/bin/piper"
export YUKI_PIPER_MODEL="/opt/yuki/voices/pt_BR-faber-medium.onnx"
export YUKI_PIPER_CONFIG="/opt/yuki/voices/pt_BR-faber-medium.onnx.json"
```

---

## 4. Provedor Acústico Local Fallback (Rust Puro)

Se os binários do Piper não estiverem configurados ou instalados no ambiente:
- O subsistema de voz da Yuki aciona automaticamente o **`LocalAcousticTtsProvider`**.
- **Zero dependências externas:** Implementado integralmente em Rust puro.
- Gera síntese melódica paramétrica com frequência fundamental feminina de **F0 = 220 Hz**, sobretons harmônicos e modulação suave de vibrato, assegurando que o sistema **nunca fique em silêncio** mesmo em ambientes totalmente desprovidos de ferramentas externas.

---

## 5. Salvaguardas de Privacidade e Governança

1. **Desacoplamento de Egress (Isolamento Formal):**
   - Permitir `egress:voice_cloud_tts` autoriza apenas o envio de texto para sintetização externa.
   - **NÃO autoriza** e nunca autorizará o envio de áudio bruto capturado pelo microfone (`egress:voice_cloud_stt`).
2. **Palavra de Ativação ("Yuki"):**
   - 100% offline. O detector processa os frames localmente e descarta buffers sem salvar gravações contínuas em disco.
3. **Interrupção de Reprodução (Barge-In):**
   - No momento em que o operador começa a falar durante uma resposta da Yuki, o `AudioPlaybackController` interrompe imediatamente a reprodução de áudio, restaurando a prontidão para ouvir o usuário.
