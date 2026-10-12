# YUKI — Demonstrações Técnicas Reprodutíveis (Marco 8: Voice v1 & Missions Foundation)

Este documento registra a especificação, passos de reprodução e evidências factuais das seis demonstrações principais do Marco 8.

---

## Demonstração A: Conversação Bidirecional por Voz Ativada por Botão

### Objetivo
Validar o ciclo completo de conversação por voz sob demanda (Push-to-Talk / botão):
`Captura de Áudio -> Transcrição Local -> Raciocínio Governado -> Síntese de Voz -> Reprodução Segura`.

### Passos de Reprodução
1. Executar o comando CLI:
   ```bash
   yuki voice speak --text "Olá José! Estou pronta para conversar com você."
   ```
2. Verificar o status do subsistema:
   ```bash
   yuki voice status
   ```
3. Teste automatizado correspondente:
   ```bash
   cargo test --test voice_integration test_voice_conversation_button_turn_end_to_end
   ```

### Resultado Verificado
- Áudio capturado e decodificado via `AudioCodec` (16kHz 16-bit mono).
- Processamento assíncrono respeitando o orçamento contextual e identidade vocal.
- Reprodução iniciada com status `Playing` e transição segura para `WakeListening` ao término.

---

## Demonstração B: Conversação Contínua com Ativação por Palavra-Chave ("Yuki")

### Objetivo
Validar que a Yuki permanece em escuta contínua local de baixo consumo, ativando-se apenas ao detectar a palavra-chave soberana "Yuki".

### Passos de Reprodução
1. Executar o teste automatizado:
   ```bash
   cargo test --test voice_integration test_wake_word_detector_local_and_invariants
   ```
2. Cenários adversariais comprovados:
   - Rejeição de ruído de alta frequência e silêncio (`test_10_tentativa_de_wake_word_falso_positivo_ruido` em `tests/marco_8_adversarial.rs`).
   - Detecção precisa do padrão fonético "Yuki" com pontuação superior ao limiar configurado.
   - Respeito ao estado desabilitado (`set_enabled(false)`).

---

## Demonstração C: Interrupção por Voz / Barge-in

### Objetivo
Comprovar que, quando a Yuki está falando, o início de uma nova fala pelo usuário interrompe instantaneamente a saída sonora.

### Passos de Reprodução
1. Executar o teste automatizado:
   ```bash
   cargo test --test voice_integration test_voice_interruption_barge_in
   ```
2. Cenário adversarial correspondente:
   ```bash
   cargo test --test marco_8_adversarial test_09_interrupcao_por_barge_in_durante_fala
   ```

### Resultado Verificado
- O `AudioPlaybackController` transita atomicamente de `Playing` para `Interrupted`.
- Os buffers de áudio restantes são descartados sem travamento de threads.

---

## Demonstração D: Missão Complexa de Ponta a Ponta (DAG de Pesquisa)

### Objetivo
Executar um plano estruturado de missão em grafo acíclico dirigido (DAG) com múltiplas etapas dependentes.

### Passos de Reprodução
1. Criar e planejar a missão via CLI:
   ```bash
   yuki mission create-research --id "m-demo-dag" --query "Arquitetura Rust Soberana" --output "relatorio_soberano.md"
   ```
2. Inspecionar o grafo planejado:
   ```bash
   yuki mission show --id "m-demo-dag"
   ```
3. Executar o DAG:
   ```bash
   yuki mission run --id "m-demo-dag"
   ```
4. Teste de integração correspondente:
   ```bash
   cargo test --test missions_foundation test_mission_dag_execution_and_runner
   ```

### Resultado Verificado
- Todas as dependências topológicas são resolvidas na ordem correta (1 -> 2 -> 3).
- Checkpoints transacionais salvos no SQLite a cada etapa concluída.
- Status final da missão: `COMPLETED`.

---

## Demonstração E: Missão com Portão de Aprovação Humana Obrigatória

### Objetivo
Demonstrar a aplicação estrita do princípio constitucional:
`Think != Authorize != Execute` e `Capability != Permission != Authorization != Execution`.

### Passos de Reprodução
1. Executar o teste com etapa mutadora que requer aprovação humana:
   ```bash
   cargo test --test missions_foundation test_mission_human_approval_lifecycle
   ```
2. Cenário adversarial com adulteração de parâmetros (SHA-256 mismatch):
   ```bash
   cargo test --test marco_8_adversarial test_22_adulteracao_de_parametros_hash_mismatch
   ```
3. Fluxo operacional via CLI:
   - A missão pausa automaticamente no status `AwaitingApproval`.
   - O operador lista as pendências: `yuki mission list`
   - O operador aprova explicitamente: `yuki mission approve --id "<approval_id>"`
   - O executor valida o hash criptográfico SHA-256 e prossegue com a execução.

---

## Demonstração F: Recuperação Após Falha ou Reinicialização (Reconciliação Soberana)

### Objetivo
Garantir que um crash ou encerramento abrupto do sistema não corrompa missões nem repita operações mutadoras perigosas.

### Passos de Reprodução
1. Executar o teste de persistência e recuperação:
   ```bash
   cargo test --test missions_foundation test_mission_sqlite_persistence_and_recovery
   ```
2. Executar o teste adversarial de reconciliação de inicialização:
   ```bash
   cargo test --test marco_8_adversarial test_34_recuperacao_na_inicializacao_reconciliacao
   ```

### Resultado Verificado
- O método `reconcile_and_recover_on_startup()` identifica passos que estavam em `Running` durante a queda.
- Reseta passos de forma segura: passos de leitura/idempotentes para `Pending`, e passos com mutação externa ou aprovação prévia para `WaitingApproval`, colocando a missão em `Paused`.
- Permite que o operador humano inspecione o estado e decida retomar (`yuki mission resume`) com segurança e previsibilidade.

---

## Demonstração G: Captura Real de Microfone e Push-to-Talk Seguro no Windows (Marco 8.1)

### Objetivo
Validar a captura real de áudio do microfone através de APIs nativas multimídia do Windows (MCI/WinMM) com controles rígidos de segurança de buffer (limite de 30s e 1MB).

### Passos de Reprodução
1. Iniciar escuta via microfone físico por 5 segundos:
   ```bash
   yuki voice listen --mic --seconds 5
   ```
2. Testes automatizados correspondentes:
   ```bash
   cargo test --test marco_8_adversarial test_47_push_to_talk_duracao_e_overflow
   cargo test --lib voice::capture::tests
   ```

### Resultado Verificado
- Dispositivo físico de gravação detectado via `waveInGetNumDevs()`.
- Buffer PCM gravado em contêiner canônico WAV 16kHz 16-bit mono.
- Rejeição atômica (`VoiceError::AudioTooLarge` / `VoiceError::AudioTooLong`) se os limites forem violados.

---

## Demonstração H: Reprodução Real com Cancelamento Instantâneo para Barge-in (Marco 8.1)

### Objetivo
Comprovar o envio de som real ao dispositivo de saída de áudio do Windows (`winmm.dll`) e o cancelamento síncrono em menos de 1ms ao detectar fala do operador.

### Passos de Reprodução
1. Executar teste automatizado do sink e interrupção:
   ```bash
   cargo test --test marco_8_adversarial test_46_audio_playback_sink_interrupcao_e_hardware
   cargo test --lib voice::playback::tests
   ```

### Resultado Verificado
- O sink `WindowsAudioPlaybackSink` gerencia `PlaySoundA` de forma assíncrona.
- A chamada `interrupt()` despacha ponteiro nulo para o driver de áudio, interrompendo o som instantaneamente a nível de SO sem travar threads.

---

## Demonstração I: Portão Humano Autenticado com Token Criptográfico e Anti-Replay (Marco 8.1)

### Objetivo
Garantir que a aprovação humana de missões seja irrepetível (`ApprovalStatus::Consumed`) e associada ao operador autorizado portando token de segurança válido.

### Passos de Reprodução
1. Executar testes automatizados correspondentes:
   ```bash
   cargo test --test marco_8_adversarial test_53_mission_runner_anti_replay_aprovacao_consumida
   cargo test --test marco_8_adversarial test_57_autenticacao_de_operador_com_token_de_seguranca
   ```

### Resultado Verificado
- Tentativas de reusar uma aprovação já concedida resultam em falha fechada.
- Operadores vazios ("") ou "anonymous" são sumariamente rejeitados com `MissionError::ApprovalMismatch`.

