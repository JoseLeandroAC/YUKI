use clap::{Parser, Subcommand};
use std::sync::Arc;
use yuki::capabilities::registry::CapabilityRegistry;
use yuki::contracts::input::UserInput;
use yuki::contracts::output::ResultStatus;
use yuki::core::yuki_core::YukiCore;
use yuki::models::{resolve_model_provider_from_env, HealthStatus};
use yuki::persistence::sqlite::SqliteAuditStore;
use yuki::verification::verifier::Verifier;

#[derive(Parser, Debug)]
#[command(
    name = "yuki",
    version = "0.1.0",
    about = "Yuki Personal AI Platform — Foundation v0.1"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Direct input prompt for Yuki
    #[arg(trailing_var_arg = true)]
    prompt: Vec<String>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Diagnóstico de saúde dos componentes fundamentais da Yuki
    Health,

    /// Gerenciamento seguro do cofre de credenciais da Yuki
    #[command(subcommand)]
    Vault(yuki::security::vault::VaultCommand),

    /// Gerenciamento e execução soberana de missões governadas da Yuki
    #[command(subcommand)]
    Mission(MissionCommands),

    /// Diagnóstico e operações do subsistema de voz da Yuki
    #[command(subcommand)]
    Voice(VoiceCommands),
}

#[derive(Subcommand, Debug)]
pub enum MissionCommands {
    /// Lista todas as missões cadastradas
    List,
    /// Exibe detalhes de uma missão específica e suas etapas
    Show {
        /// ID da missão
        id: String,
    },
    /// Concede aprovação humana para uma etapa de missão
    Approve {
        /// ID da solicitação de aprovação
        approval_id: String,
        /// Identificador do operador humano
        #[arg(short, long, default_value = "operator-jose")]
        operator: String,
        /// Token de confirmação de segurança (opcional/autenticado)
        #[arg(short, long)]
        token: Option<String>,
    },
    /// Rejeita aprovação humana para uma etapa de missão
    Deny {
        /// ID da solicitação de aprovação
        approval_id: String,
        /// Motivo da rejeição
        #[arg(short, long)]
        reason: String,
        /// Identificador do operador humano
        #[arg(short, long, default_value = "operator-jose")]
        operator: String,
    },
    /// Pausa uma missão em andamento
    Pause {
        /// ID da missão
        id: String,
    },
    /// Retoma uma missão pausada
    Resume {
        /// ID da missão
        id: String,
    },
    /// Cancela formalmente uma missão
    Cancel {
        /// ID da missão
        id: String,
        /// Motivo do cancelamento
        #[arg(short, long, default_value = "Cancelado pelo operador via CLI")]
        reason: String,
    },
    /// Executa ou avança uma missão elegível
    Run {
        /// ID da missão
        id: String,
    },
    /// Cria e registra uma missão canônica de pesquisa técnica
    CreateResearch {
        /// ID da missão
        #[arg(short, long, default_value = "mission_research_01")]
        id: String,
        /// Consulta técnica de pesquisa
        #[arg(short, long, default_value = "Reconhecimento de Voz Whisper local")]
        query: String,
        /// Arquivo de destino para o relatório
        #[arg(short, long, default_value = "relatorio_pesquisa.md")]
        output: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum VoiceCommands {
    /// Diagnóstico dos motores de STT, TTS, captura e reprodução local da Yuki
    Status,
    /// Lista os dispositivos de áudio disponíveis no sistema (microfones e alto-falantes/fones)
    Devices {
        /// Exibe todos os dispositivos conhecidos, incluindo desconectados e desabilitados
        #[arg(short, long, default_value_t = false)]
        all: bool,
    },
    /// Sintetiza uma mensagem usando a voz da Yuki e salva em WAV (opcionalmente reproduz no alto-falante)
    Speak {
        /// Texto a ser sintetizado pela Yuki
        text: String,
        /// Arquivo de saída WAV (padrão: yuki_speech.wav)
        #[arg(short, long, default_value = "yuki_speech.wav")]
        output: String,
        /// Reproduz o áudio sintetizado no alto-falante físico do sistema operacional
        #[arg(short, long, default_value_t = false)]
        play: bool,
        /// Identificador ou nome do dispositivo de saída para reprodução (opcional)
        #[arg(long)]
        output_device: Option<String>,
    },
    /// Transcreve áudio usando STT local (do microfone ou de arquivo WAV)
    Listen {
        /// Caminho do arquivo de áudio WAV para transcrição (se omitido, grava do microfone)
        #[arg(short, long)]
        file: Option<String>,
        /// Força gravação a partir do microfone
        #[arg(short, long, default_value_t = false)]
        mic: bool,
        /// Duração da gravação do microfone em segundos (máximo 30s, padrão: 5s)
        #[arg(short, long, default_value_t = 5)]
        seconds: u64,
        /// Identificador ou nome do dispositivo de microfone para captura (opcional)
        #[arg(long)]
        input_device: Option<String>,
    },
    /// Executa um turno conversacional completo via Push-to-Talk conectado ao VoiceConversationCoordinator
    Turn {
        /// Duração máxima da captura do microfone em segundos (máximo 30s, padrão: 5s)
        #[arg(short, long, default_value_t = 5)]
        seconds: u64,
        /// Identificador ou nome do dispositivo de microfone de entrada (opcional)
        #[arg(long)]
        input_device: Option<String>,
        /// Identificador ou nome do dispositivo de saída de áudio (opcional)
        #[arg(long)]
        output_device: Option<String>,
        /// Reproduz fisicamente a resposta de voz no alto-falante
        #[arg(short, long, default_value_t = false)]
        play: bool,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Comandos administrativos do Vault não exigem inicialização do modelo
    if let Some(Commands::Vault(vault_cmd)) = cli.command {
        if let Err(e) = vault_cmd.execute() {
            eprintln!("Erro no cofre: {}", e);
            std::process::exit(1);
        }
        return;
    }

    let missions_db_path = std::env::var("YUKI_MISSIONS_PATH")
        .or_else(|_| {
            std::env::var("YUKI_DATA_DIR").map(|d| {
                std::path::Path::new(&d)
                    .join("missions.db")
                    .to_string_lossy()
                    .to_string()
            })
        })
        .unwrap_or_else(|_| "data/missions.db".to_string());

    if let Some(Commands::Mission(mission_cmd)) = cli.command {
        handle_mission_command(mission_cmd, &missions_db_path).await;
        return;
    }

    if let Some(Commands::Voice(voice_cmd)) = cli.command {
        handle_voice_command(voice_cmd).await;
        return;
    }

    // Resolve model provider from environment (ADR-018 / CONFIGURATION.md)
    let model_provider = match resolve_model_provider_from_env() {
        Ok(provider) => provider,
        Err(err) => {
            eprintln!("Erro de configuração do provedor de modelo: {}", err);
            std::process::exit(1);
        }
    };

    // Resolve persistent database path (ADR-019 / CONFIGURATION.md / 00_ENVIRONMENT_BASELINE.md)
    let db_path = std::env::var("YUKI_DATABASE_PATH")
        .or_else(|_| std::env::var("YUKI_PERSISTENCE_PATH"))
        .or_else(|_| {
            std::env::var("YUKI_DATA_DIR").map(|d| {
                std::path::Path::new(&d)
                    .join("audit.db")
                    .to_string_lossy()
                    .to_string()
            })
        })
        .unwrap_or_else(|_| "data/yuki.db".to_string());
    let (core, sqlite_status) = match SqliteAuditStore::open(&db_path) {
        Ok(store) => {
            let store = Arc::new(store);
            (
                YukiCore::new()
                    .with_model_provider(model_provider)
                    .with_persistent_audit(store),
                "OK",
            )
        }
        Err(e) => {
            eprintln!(
                "Aviso: Falha ao inicializar banco de auditoria durável ('{}'): {}. Operando em modo degradado (in-memory).",
                db_path, e
            );
            (
                YukiCore::new().with_model_provider(model_provider),
                "DEGRADED (In-Memory Fallback)",
            )
        }
    };

    match cli.command {
        Some(Commands::Health) => {
            let health = core.health();
            let model_meta = core.model_provider.metadata();
            let model_health = core.model_provider.health();

            println!("Yuki");
            if health.is_all_ok() {
                println!("Status: OK\n");
            } else {
                println!("Status: DEGRADED\n");
            }

            println!("Core: {}", if health.core_ok { "OK" } else { "FAIL" });
            println!("Context: {}", if health.context_ok { "OK" } else { "FAIL" });
            let model_status_str = match &model_health {
                HealthStatus::Healthy => "OK".to_string(),
                HealthStatus::Degraded(reason) => format!("DEGRADED ({})", reason),
                HealthStatus::Unhealthy(reason) => format!("FAIL ({})", reason),
            };

            let subsystem_name = if model_meta.provider_name.contains("Mock") {
                "Model Subsystem (Mock)".to_string()
            } else {
                format!("Model Subsystem ({})", model_meta.provider_name)
            };
            println!("{}: {}", subsystem_name, model_status_str);
            println!(
                "Selected Model Provider: {} (model: {}, version: {})",
                model_meta.provider_name, model_meta.model_name, model_meta.version
            );
            println!("Model Provider Health: {}", model_status_str);
            println!(
                "Capability Registry: {}",
                if health.registry_ok { "OK" } else { "FAIL" }
            );
            println!(
                "Security Controller: {}",
                if health.security_ok { "OK" } else { "FAIL" }
            );
            println!(
                "Execution Engine: {}",
                if health.execution_ok { "OK" } else { "FAIL" }
            );
            println!(
                "Verification Engine: {}",
                if health.verification_ok { "OK" } else { "FAIL" }
            );
            println!(
                "Audit Subsystem (In-Memory): {}",
                if health.audit_ok { "OK" } else { "FAIL" }
            );
            println!("Persistent Audit (SQLite): {}", sqlite_status);

            let whisper_ready =
                yuki::voice::stt::LocalWhisperSttProvider::default().is_environment_ready();
            let piper_ready =
                yuki::voice::tts::LocalPiperTtsProvider::default().is_environment_ready();
            println!(
                "Voice STT (Local Whisper): {}",
                if whisper_ready {
                    "READY"
                } else {
                    "STANDBY (Install docs available)"
                }
            );
            println!(
                "Voice TTS (Piper Neural): {}",
                if piper_ready {
                    "READY"
                } else {
                    "STANDBY (Acoustic fallback active)"
                }
            );
            println!("Voice TTS (Acoustic Fallback): READY");
            println!("Missions Subsystem (SQLite Store): OK");
        }
        None => {
            if cli.prompt.is_empty() {
                println!("Yuki MVP-1");
                println!("Uso: yuki \"<sua mensagem>\" ou yuki health");
                return;
            }

            let input_text = cli.prompt.join(" ");
            let user_input = UserInput::new(input_text);

            tokio::select! {
                res = core.process_input_async(user_input) => {
                    match res {
                        Ok(result) => match result.status {
                            ResultStatus::Success => {
                                println!("{}", result.content);
                            }
                            ResultStatus::Denied => {
                                eprintln!("Acesso negado: {}", result.content);
                            }
                            ResultStatus::Failed => {
                                eprintln!("Erro: {}", result.content);
                            }
                            ResultStatus::Unknown => {
                                println!("Resultado indeterminado (UNKNOWN): {}", result.content);
                            }
                        },
                        Err(err) => {
                            eprintln!("Erro no processamento da Yuki: {}", err);
                            std::process::exit(1);
                        }
                    }
                }
                _ = tokio::signal::ctrl_c() => {
                    eprintln!("\nExecução cancelada pelo usuário (SIGINT). Encerrando graciosamente.");
                    std::process::exit(130);
                }
            }
        }
        Some(Commands::Vault(_)) | Some(Commands::Mission(_)) | Some(Commands::Voice(_)) => {
            unreachable!()
        }
    }
}

async fn handle_mission_command(cmd: MissionCommands, db_path: &str) {
    let store = match yuki::missions::SqliteMissionStore::open(db_path) {
        Ok(s) => Arc::new(s),
        Err(e) => {
            eprintln!("Erro ao abrir banco de missões ('{}'): {}", db_path, e);
            std::process::exit(1);
        }
    };
    let approval_mgr = Arc::new(yuki::missions::MissionApprovalManager::new(
        store.clone(),
        None,
    ));
    let capability_registry = Arc::new(CapabilityRegistry::new());
    let verifier = Arc::new(Verifier::new());
    let runner = yuki::missions::MissionRunner::new(
        store.clone(),
        approval_mgr.clone(),
        capability_registry,
        verifier,
        None,
    );

    match cmd {
        MissionCommands::List => match store.list_missions() {
            Ok(missions) => {
                if missions.is_empty() {
                    println!("Nenhuma missão cadastrada.");
                } else {
                    println!(
                        "{:<24} {:<18} {:<30} {:<6}",
                        "ID", "STATUS", "TÍTULO", "PASSOS"
                    );
                    println!("{}", "-".repeat(82));
                    for m in missions {
                        println!(
                            "{:<24} {:<18?} {:<30} {:<6}",
                            m.id,
                            m.status,
                            if m.title.len() > 28 {
                                format!("{}...", &m.title[..25])
                            } else {
                                m.title
                            },
                            m.steps.len()
                        );
                    }
                }
            }
            Err(e) => eprintln!("Erro ao listar missões: {}", e),
        },
        MissionCommands::Show { id } => {
            match store.get_mission(&id) {
                Ok(Some(m)) => {
                    println!("Missão: {} [{:?}]", m.title, m.status);
                    println!("ID: {}", m.id);
                    println!("Objetivo: {}", m.objective);
                    println!("Operador: {}", m.operator_id);
                    println!("Criada em: {}", m.created_at);
                    if let Some(c) = m.completed_at {
                        println!("Concluída em: {}", c);
                    }
                    if let Some(r) = m.failure_reason {
                        println!("Motivo da Falha/Cancelamento: {}", r);
                    }
                    println!("\nEtapas (DAG):");
                    match store.get_steps_for_mission(&id) {
                        Ok(steps) => {
                            for s in steps {
                                println!(
                                    "  [Passo {}] {:<16?} - {} (Capacidade: {:?}, Exige Aprovação: {})",
                                    s.step_number, s.status, s.description, s.capability_id.as_deref().unwrap_or("Nenhuma"), s.requires_approval
                                );
                                if let Some(app_id) = &s.approval_id {
                                    println!("    -> Solicitação de Aprovação: {}", app_id);
                                }
                            }
                        }
                        Err(e) => eprintln!("  Erro ao carregar etapas: {}", e),
                    }
                }
                Ok(None) => println!("Missão '{}' não encontrada.", id),
                Err(e) => eprintln!("Erro ao buscar missão: {}", e),
            }
        }
        MissionCommands::Approve {
            approval_id,
            operator,
            token,
        } => match approval_mgr.grant_approval_authenticated(
            &approval_id,
            &operator,
            token.as_deref(),
        ) {
            Ok(app) => {
                println!("Aprovação concedida com sucesso!");
                println!("ID: {}", app.id);
                println!("Missão: {}", app.mission_id);
                println!("Etapa: {}", app.step_id);
                println!("Operador: {}", app.operator_id);
                println!(
                    "Você pode continuar a execução com: yuki mission run {}",
                    app.mission_id
                );
            }
            Err(e) => eprintln!("Erro ao conceder aprovação: {}", e),
        },
        MissionCommands::Deny {
            approval_id,
            reason,
            operator,
        } => match approval_mgr.deny_approval(&approval_id, &operator, &reason) {
            Ok(app) => {
                println!("Aprovação rejeitada pelo operador.");
                println!("ID: {}", app.id);
                println!("Missão: {}", app.mission_id);
                println!("Etapa: {}", app.step_id);
                println!("Operador: {}", app.operator_id);
            }
            Err(e) => eprintln!("Erro ao rejeitar aprovação: {}", e),
        },
        MissionCommands::Pause { id } => match runner.pause_mission(&id) {
            Ok(_) => println!("Missão '{}' pausada com sucesso.", id),
            Err(e) => eprintln!("Erro ao pausar missão: {}", e),
        },
        MissionCommands::Resume { id } => match runner.resume_mission(&id).await {
            Ok(m) => println!("Missão '{}' retomada. Status atual: {:?}", id, m.status),
            Err(e) => eprintln!("Erro ao retomar missão: {}", e),
        },
        MissionCommands::Cancel { id, reason } => match runner.cancel_mission(&id, &reason) {
            Ok(_) => println!("Missão '{}' cancelada com sucesso. Motivo: {}", id, reason),
            Err(e) => eprintln!("Erro ao cancelar missão: {}", e),
        },
        MissionCommands::Run { id } => {
            println!("Iniciando execução da missão '{}'...", id);
            match runner.run_mission(&id).await {
                Ok(m) => {
                    println!("Execução concluída. Status da missão: {:?}", m.status);
                    if m.status == yuki::contracts::missions::MissionStatus::AwaitingApproval {
                        println!("A missão está pausada aguardando aprovação humana de uma etapa mutadora.");
                        println!("Verifique as pendências com: yuki mission show {}", id);
                    }
                }
                Err(e) => eprintln!("Falha na execução da missão: {}", e),
            }
        }
        MissionCommands::CreateResearch { id, query, output } => {
            let plan =
                yuki::missions::MissionPlanner::build_research_mission_plan(&id, &query, &output);
            let planner = yuki::missions::MissionPlanner::default();
            match planner.create_mission(
                &id,
                format!("Pesquisa Soberana: {}", query),
                format!(
                    "Pesquisar documentação técnica e gerar relatório em '{}'",
                    output
                ),
                "operator-jose",
                "cli-run",
                plan,
                None,
            ) {
                Ok(m) => {
                    if let Err(e) = store.create_mission(&m) {
                        eprintln!("Erro ao salvar missão no banco: {}", e);
                        return;
                    }
                    for step in &m.steps {
                        if let Err(e) = store.save_step(step) {
                            eprintln!("Erro ao salvar etapa {}: {}", step.step_number, e);
                        }
                    }
                    println!("Missão de pesquisa criada e persistida com sucesso!");
                    println!("ID: {}", m.id);
                    println!("Etapas cadastradas: {}", m.steps.len());
                    println!("Para executar a missão, execute: yuki mission run {}", m.id);
                }
                Err(e) => eprintln!("Erro ao criar plano de missão: {}", e),
            }
        }
    }
}

async fn handle_voice_command(cmd: VoiceCommands) {
    use yuki::contracts::voice::{AudioInput, SpeechToTextProvider, TextToSpeechProvider};
    use yuki::voice::audio::AudioCodec;
    use yuki::voice::devices::{
        find_device, get_default_device, list_audio_devices, AudioDeviceFlow,
    };
    use yuki::voice::playback::AudioPlaybackController;
    use yuki::voice::stt::LocalWhisperSttProvider;
    use yuki::voice::tts::{LocalAcousticTtsProvider, LocalPiperTtsProvider, VoiceIdentity};

    match cmd {
        VoiceCommands::Status => {
            println!("=== Diagnóstico do Subsistema de Voz da Yuki ===");
            let whisper_ready = LocalWhisperSttProvider::default().is_environment_ready();
            println!(
                "STT Local (Whisper.cpp / CLI): {}",
                if whisper_ready {
                    "DISPONÍVEL"
                } else {
                    "NÃO INSTALADO / NÃO CONFIGURADO (instruções em docs/)"
                }
            );

            let piper_ready = LocalPiperTtsProvider::default().is_environment_ready();
            println!(
                "TTS Local (Piper Neural):     {}",
                if piper_ready {
                    "DISPONÍVEL"
                } else {
                    "NÃO INSTALADO (instruções em docs/)"
                }
            );

            println!("TTS Local Acústico (Fallback): DISPONÍVEL (Embarcado, F0=220Hz, voz feminina afável)");
            let voice_id = VoiceIdentity::default();
            println!(
                "Identidade Vocal da Yuki:     {} ({}, idioma: {}, tom: {:?})",
                voice_id.name, voice_id.gender, voice_id.language, voice_id.tone
            );

            println!("\n--- Endpoints de Áudio do Sistema (WASAPI Core Audio) ---");
            match get_default_device(AudioDeviceFlow::Input) {
                Ok(Some(dev)) => println!(
                    "Microfone Padrão:              {} (ID: {})",
                    dev.name, dev.id
                ),
                Ok(None) => println!("Microfone Padrão:              NENHUM DETECTADO"),
                Err(e) => println!("Microfone Padrão (Erro):       {}", e),
            }
            match get_default_device(AudioDeviceFlow::Output) {
                Ok(Some(dev)) => println!(
                    "Saída de Áudio Padrão:         {} (ID: {})",
                    dev.name, dev.id
                ),
                Ok(None) => println!("Saída de Áudio Padrão:         NENHUMA DETECTADA"),
                Err(e) => println!("Saída de Áudio Padrão (Erro):  {}", e),
            }

            let playback_ctrl = AudioPlaybackController::production();
            let playback_avail = playback_ctrl.is_device_available();
            println!(
                "Driver de Reprodução:          {}",
                if playback_avail {
                    format!("DISPONÍVEL ({})", playback_ctrl.sink().name())
                } else {
                    "NENHUM DISPOSITIVO ATIVO".to_string()
                }
            );
        }
        VoiceCommands::Devices { all } => {
            println!("=== Dispositivos de Áudio do Sistema (WASAPI Core Audio) ===");
            match list_audio_devices(!all) {
                Ok(devices) => {
                    let (inputs, outputs): (Vec<_>, Vec<_>) = devices
                        .into_iter()
                        .partition(|d| d.flow == AudioDeviceFlow::Input);

                    println!(
                        "\n--- Dispositivos de Entrada (Microfones) [total: {}] ---",
                        inputs.len()
                    );
                    if inputs.is_empty() {
                        println!("  (Nenhum dispositivo encontrado)");
                    } else {
                        for d in &inputs {
                            let default_badge = if d.is_default { " [PADRÃO]" } else { "" };
                            println!(
                                "• {}{} [{}]\n    ID: {}",
                                d.name,
                                default_badge,
                                d.state.label(),
                                d.id
                            );
                        }
                    }

                    println!(
                        "\n--- Dispositivos de Saída (Alto-falantes / Fones) [total: {}] ---",
                        outputs.len()
                    );
                    if outputs.is_empty() {
                        println!("  (Nenhum dispositivo encontrado)");
                    } else {
                        for d in &outputs {
                            let default_badge = if d.is_default { " [PADRÃO]" } else { "" };
                            println!(
                                "• {}{} [{}]\n    ID: {}",
                                d.name,
                                default_badge,
                                d.state.label(),
                                d.id
                            );
                        }
                    }

                    if !all {
                        println!("\nNota: Apenas dispositivos ativos foram listados. Use '--all' para ver também dispositivos desconectados.");
                    }
                }
                Err(e) => eprintln!("Erro ao enumerar dispositivos de áudio: {}", e),
            }
        }
        VoiceCommands::Speak {
            text,
            output,
            play,
            output_device,
        } => {
            let acoustic = LocalAcousticTtsProvider::default();
            println!("Sintetizando voz da Yuki para o texto: \"{}\"...", text);
            match acoustic.synthesize(&text, None).await {
                Ok(audio_out) => {
                    if let Err(e) = std::fs::write(&output, &audio_out.payload) {
                        eprintln!("Erro ao salvar arquivo '{}': {}", output, e);
                    } else {
                        println!("Áudio sintetizado com sucesso salvo em: {}", output);
                        println!(
                            "Duração aproximada: {:.2}s ({} bytes)",
                            audio_out.duration_ms as f64 / 1000.0,
                            audio_out.payload.len()
                        );
                    }

                    if play {
                        let target_device_id = if let Some(ref dev_query) = output_device {
                            match find_device(dev_query, AudioDeviceFlow::Output) {
                                Ok(dev) => {
                                    println!(
                                        "Dispositivo de saída selecionado: {} (ID: {})",
                                        dev.name, dev.id
                                    );
                                    Some(dev.id)
                                }
                                Err(e) => {
                                    eprintln!(
                                        "Falha ao resolver dispositivo de saída '{}': {}",
                                        dev_query, e
                                    );
                                    return;
                                }
                            }
                        } else {
                            if let Ok(Some(def)) = get_default_device(AudioDeviceFlow::Output) {
                                println!(
                                    "Dispositivo de saída padrão: {} (ID: {})",
                                    def.name, def.id
                                );
                            }
                            None
                        };

                        let playback =
                            AudioPlaybackController::production_with_device(target_device_id);
                        if !playback.is_device_available() {
                            eprintln!("Aviso: Nenhum dispositivo de áudio físico disponível para reprodução.");
                        } else {
                            println!(
                                "Reproduzindo áudio no dispositivo de saída (sink: {})...",
                                playback.sink().name()
                            );
                            if let Err(e) = playback.play(&audio_out) {
                                eprintln!("Erro na reprodução física de áudio: {}", e);
                            } else {
                                let duration = std::time::Duration::from_millis(
                                    audio_out.duration_ms.max(100),
                                );
                                tokio::time::sleep(duration).await;
                                println!("Reprodução concluída.");
                            }
                        }
                    }
                }
                Err(e) => eprintln!("Erro ao sintetizar áudio: {}", e),
            }
        }
        VoiceCommands::Listen {
            file,
            mic,
            seconds,
            input_device,
        } => {
            let audio_input = if let (Some(path), false) = (file, mic) {
                let data = match std::fs::read(&path) {
                    Ok(d) => d,
                    Err(e) => {
                        eprintln!("Erro ao ler arquivo '{}': {}", path, e);
                        return;
                    }
                };
                let (format, samples) = match AudioCodec::decode_wav(&data) {
                    Ok(f) => f,
                    Err(e) => {
                        eprintln!("Erro ao decodificar arquivo WAV '{}': {}", path, e);
                        return;
                    }
                };
                let duration_ms =
                    (samples.len() as f64 / format.sample_rate as f64 * 1000.0) as u64;
                AudioInput::new(data, format, duration_ms, path)
            } else {
                use std::sync::Arc;
                use std::time::Duration;
                use yuki::voice::capture::{
                    AudioCaptureBackend, PushToTalkController, WindowsAudioCaptureBackend,
                };

                let target_device = if let Some(ref dev_query) = input_device {
                    match find_device(dev_query, AudioDeviceFlow::Input) {
                        Ok(dev) => {
                            println!(
                                "Dispositivo de microfone selecionado: {} (ID: {})",
                                dev.name, dev.id
                            );
                            Some(dev.id)
                        }
                        Err(e) => {
                            eprintln!(
                                "Falha ao resolver dispositivo de entrada '{}': {}",
                                dev_query, e
                            );
                            return;
                        }
                    }
                } else {
                    match get_default_device(AudioDeviceFlow::Input) {
                        Ok(Some(def)) => {
                            println!(
                                "Dispositivo de microfone padrão: {} (ID: {})",
                                def.name, def.id
                            );
                        }
                        Ok(None) => {
                            eprintln!("Erro: Nenhum dispositivo de microfone de entrada foi detectado ou está ativo.");
                            return;
                        }
                        Err(e) => {
                            eprintln!("Erro ao verificar dispositivo padrão de entrada: {}", e);
                            return;
                        }
                    }
                    None
                };

                println!("Formato de captura negociado: 16000Hz, 1 canal (mono), 16-bit PCM");

                let duration_sec = seconds.clamp(1, 30);
                println!(
                    "Iniciando captura de áudio do microfone por {} segundos...",
                    duration_sec
                );

                let backend: Arc<dyn AudioCaptureBackend> = {
                    #[cfg(windows)]
                    {
                        let win = WindowsAudioCaptureBackend::with_device(target_device);
                        if !win.is_device_available() {
                            eprintln!(
                                "Erro: O dispositivo de captura selecionado não está disponível."
                            );
                            return;
                        }
                        Arc::new(win)
                    }
                    #[cfg(not(windows))]
                    {
                        eprintln!("Erro: Captura física de áudio por microfone não suportada nesta plataforma.");
                        return;
                    }
                };

                let ptt = PushToTalkController::new(backend);
                if let Err(e) = ptt.press() {
                    eprintln!("Falha ao iniciar captura de microfone: {}", e);
                    return;
                }

                println!(
                    "Gravando... Fale com a Yuki agora (tempo máximo: {}s)...",
                    duration_sec
                );
                tokio::time::sleep(Duration::from_secs(duration_sec)).await;

                match ptt.release() {
                    Ok(input) => {
                        println!(
                            "Captura concluída com sucesso ({} bytes, {}ms).",
                            input.payload.len(),
                            input.duration_ms
                        );
                        input
                    }
                    Err(e) => {
                        eprintln!("Falha ao encerrar captura de áudio: {}", e);
                        return;
                    }
                }
            };

            let stt = LocalWhisperSttProvider::default();
            println!(
                "Transcrevendo áudio via STT local (fonte: {})...",
                audio_input.source
            );
            match stt.transcribe(&audio_input).await {
                Ok(transcript) => {
                    println!("Transcrição obtida:");
                    println!("\"{}\"", transcript);
                    println!("Duração: {:.2}s", audio_input.duration_ms as f64 / 1000.0);
                }
                Err(e) => eprintln!("Falha na transcrição: {}", e),
            }
        }
        VoiceCommands::Turn {
            seconds,
            input_device,
            output_device,
            play,
        } => {
            use std::sync::Arc;
            use std::time::Duration;
            use yuki::voice::capture::{
                AudioCaptureBackend, PushToTalkController, WindowsAudioCaptureBackend,
            };
            use yuki::voice::conversation::VoiceConversationCoordinator;
            use yuki::voice::playback::AudioPlaybackController;

            // 1. Resolução do microfone de entrada (fail-closed se indisponível)
            let target_input_id = if let Some(ref dev_query) = input_device {
                match find_device(dev_query, AudioDeviceFlow::Input) {
                    Ok(dev) => {
                        println!(
                            "Microfone de entrada selecionado: {} (ID: {})",
                            dev.name, dev.id
                        );
                        Some(dev.id)
                    }
                    Err(e) => {
                        eprintln!(
                            "Falha ao resolver microfone de entrada '{}': {}",
                            dev_query, e
                        );
                        return;
                    }
                }
            } else {
                match get_default_device(AudioDeviceFlow::Input) {
                    Ok(Some(def)) => {
                        println!("Microfone de entrada padrão: {} (ID: {})", def.name, def.id);
                    }
                    Ok(None) => {
                        eprintln!("Erro: Nenhum microfone de entrada está ativo ou disponível no sistema.");
                        return;
                    }
                    Err(e) => {
                        eprintln!("Erro ao verificar microfone padrão: {}", e);
                        return;
                    }
                }
                None
            };

            // 2. Resolução do endpoint de saída se reprodução requerida
            let target_output_id = if play {
                if let Some(ref dev_query) = output_device {
                    match find_device(dev_query, AudioDeviceFlow::Output) {
                        Ok(dev) => {
                            println!(
                                "Dispositivo de saída selecionado: {} (ID: {})",
                                dev.name, dev.id
                            );
                            Some(dev.id)
                        }
                        Err(e) => {
                            eprintln!(
                                "Falha ao resolver dispositivo de saída '{}': {}",
                                dev_query, e
                            );
                            return;
                        }
                    }
                } else {
                    if let Ok(Some(def)) = get_default_device(AudioDeviceFlow::Output) {
                        println!("Dispositivo de saída padrão: {} (ID: {})", def.name, def.id);
                    }
                    None
                }
            } else {
                None
            };

            println!("Formato de áudio negociado: 16000Hz mono 16-bit PCM (WASAPI streaming)");

            let duration_sec = seconds.clamp(1, 30);
            println!(
                "Iniciando turno conversacional de voz Push-to-Talk ({}s de captura)...",
                duration_sec
            );

            let capture_backend: Arc<dyn AudioCaptureBackend> = {
                #[cfg(windows)]
                {
                    let win = WindowsAudioCaptureBackend::with_device(target_input_id);
                    if !win.is_device_available() {
                        eprintln!(
                            "Erro: O microfone selecionado não está acessível no Windows WASAPI."
                        );
                        return;
                    }
                    Arc::new(win)
                }
                #[cfg(not(windows))]
                {
                    eprintln!("Erro: Captura física de áudio por microfone não suportada nesta plataforma.");
                    return;
                }
            };

            let ptt = PushToTalkController::new(capture_backend);
            if let Err(e) = ptt.press() {
                eprintln!("Falha ao iniciar captura de microfone: {}", e);
                return;
            }

            println!(
                "Gravando... Fale com a Yuki agora (tempo: {}s)...",
                duration_sec
            );
            tokio::time::sleep(Duration::from_secs(duration_sec)).await;

            let audio_input = match ptt.release() {
                Ok(input) => {
                    println!(
                        "Captura concluída ({} bytes, {}ms).",
                        input.payload.len(),
                        input.duration_ms
                    );
                    input
                }
                Err(e) => {
                    eprintln!("Falha ao encerrar captura de áudio: {}", e);
                    return;
                }
            };

            // Inicializa modelo para o Core (Mock fallback limpo se não configurado)
            let model_provider = resolve_model_provider_from_env()
                .unwrap_or_else(|_| Arc::new(yuki::models::MockModelProvider::new()));
            let core = Arc::new(YukiCore::new().with_model_provider(model_provider));

            let playback_controller = if play {
                Arc::new(AudioPlaybackController::production_with_device(
                    target_output_id,
                ))
            } else {
                Arc::new(AudioPlaybackController::for_environment(
                    yuki::voice::playback::PlaybackEnvironment::Disabled,
                ))
            };

            let coordinator = VoiceConversationCoordinator::new(
                core,
                Arc::new(yuki::voice::router::GovernedVoiceRouter::default_local(
                    yuki::contracts::voice::VoiceConfiguration::default(),
                )),
                Arc::new(yuki::voice::session::VoiceSessionManager::new()),
                playback_controller,
                Arc::new(tokio::sync::Mutex::new(
                    yuki::voice::wakeword::WakeWordDetector::default(),
                )),
                None,
            );

            println!("Processando turno de conversação governado...");
            match coordinator
                .process_button_turn("cli-turn", audio_input, false, false)
                .await
            {
                Ok((yuki_res, audio_out)) => {
                    println!("\nResposta da Yuki:");
                    println!("\"{}\"", yuki_res.content);
                    println!(
                        "Áudio de resposta gerado: {:.2}s ({} bytes).",
                        audio_out.duration_ms as f64 / 1000.0,
                        audio_out.payload.len()
                    );
                    if play {
                        let duration =
                            std::time::Duration::from_millis(audio_out.duration_ms.max(100));
                        tokio::time::sleep(duration).await;
                    }
                }
                Err(e) => eprintln!("Erro ao processar turno de voz: {}", e),
            }
        }
    }
}
