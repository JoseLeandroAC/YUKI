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
        /// Idioma falado para o reconhecimento ('pt', 'en', 'es', 'ja', ou 'auto')
        #[arg(short, long)]
        language: Option<String>,
        /// Prompt lexical opcional para enviesar o vocabulário (ex: "Yuki, FUVEST, OBMEP")
        #[arg(long)]
        prompt: Option<String>,
        /// Número de threads de decodificação do Whisper (padrão: 4)
        #[arg(short, long)]
        threads: Option<u32>,
        /// Beam size para busca em feixe (padrão: 5, 1 para greedy)
        #[arg(long)]
        beam_size: Option<u32>,
        /// Quantidade de candidatos best-of (padrão: 5, 1 para greedy)
        #[arg(long)]
        best_of: Option<u32>,
        /// Desativa fallbacks de temperatura em caso de incerteza (evita re-tentativas lentas)
        #[arg(long, default_value_t = false)]
        no_fallback: bool,
        /// Modo experimental rápido (greedy search: beam_size=1, best_of=1, sem fallback)
        #[arg(long, visible_alias = "fast", default_value_t = false)]
        fast_stt: bool,
        /// Ativa modo de diagnóstico detalhado de áudio (grava WAVs e métricas acústicas em data/voice_diag/)
        #[arg(long, visible_alias = "diag", default_value_t = false)]
        diagnostics: bool,
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
        /// Idioma falado para o reconhecimento ('pt', 'en', 'es', 'ja', ou 'auto')
        #[arg(short, long)]
        language: Option<String>,
        /// Prompt lexical opcional para enviesar o vocabulário (ex: "Yuki, FUVEST, OBMEP")
        #[arg(long)]
        prompt: Option<String>,
        /// Número de threads de decodificação do Whisper (padrão: 4)
        #[arg(short, long)]
        threads: Option<u32>,
        /// Beam size para busca em feixe (padrão: 5, 1 para greedy)
        #[arg(long)]
        beam_size: Option<u32>,
        /// Quantidade de candidatos best-of (padrão: 5, 1 para greedy)
        #[arg(long)]
        best_of: Option<u32>,
        /// Desativa fallbacks de temperatura em caso de incerteza (evita re-tentativas lentas)
        #[arg(long, default_value_t = false)]
        no_fallback: bool,
        /// Modo experimental rápido (greedy search: beam_size=1, best_of=1, sem fallback)
        #[arg(long, visible_alias = "fast", default_value_t = false)]
        fast_stt: bool,
        /// Ativa modo de diagnóstico detalhado de áudio (grava WAVs e métricas acústicas em data/voice_diag/)
        #[arg(long, visible_alias = "diag", default_value_t = false)]
        diagnostics: bool,
    },
    /// Executa comparação A/B de decodificação STT sobre um mesmo arquivo WAV (Baseline vs Otimizado)
    CompareStt {
        /// Caminho do arquivo WAV para teste
        file: String,
        /// Idioma falado para o reconhecimento ('pt', 'en', 'es', 'ja', ou 'auto')
        #[arg(short, long)]
        language: Option<String>,
        /// Prompt lexical para a Configuração B (se omitido, usa vocabulário padrão da Yuki para o idioma)
        #[arg(long)]
        prompt: Option<String>,
        /// Threads para a Configuração B (opcional)
        #[arg(short, long)]
        threads: Option<u32>,
        /// Beam size para a Configuração B (opcional)
        #[arg(long)]
        beam_size: Option<u32>,
        /// Ativa modo de decodificação rápida na Configuração B (beam=1, best_of=1, no-fallback)
        #[arg(long, visible_alias = "fast", default_value_t = false)]
        fast: bool,
        /// Ativa modo de diagnóstico detalhado de áudio
        #[arg(long, visible_alias = "diag", default_value_t = false)]
        diagnostics: bool,
    },
    /// Avalia objetivamente a qualidade do STT (WER, CER, Termos Críticos e Latência) no Laboratório de Voz
    EvalStt {
        /// Caminho de um arquivo WAV individual ou omitir para avaliar o corpus
        #[arg(short, long)]
        audio: Option<String>,
        /// Transcrição de referência esperada (ground truth) ao avaliar um arquivo individual
        #[arg(short, long)]
        reference: Option<String>,
        /// Termos importantes separados por vírgula (ex: "Yuki,FUVEST,OBMEP")
        #[arg(long)]
        terms: Option<String>,
        /// Idioma alvo do reconhecimento ('pt', 'en', 'es', 'ja', 'auto')
        #[arg(short, long)]
        language: Option<String>,
        /// Caminho do arquivo de manifesto do corpus JSON
        #[arg(long, default_value = "data/voice_corpus/corpus_manifest.json")]
        manifest: String,
        /// Filtrar casos de teste por categoria (command, short_phrase, long_phrase, spontaneous, multilingual, phonetic_challenge)
        #[arg(long)]
        category: Option<String>,
        /// ID específico do caso de teste no manifesto (ex: "T14_curto_calculadora")
        #[arg(long)]
        case_id: Option<String>,
        /// Número de repetições por teste para estabilidade estatística de latência (padrão: 1, recomendado: 3)
        #[arg(long, default_value_t = 1)]
        runs: usize,
        /// Exportar relatório completo de avaliação em formato JSON
        #[arg(long)]
        export_json: Option<String>,
        /// Prompt lexical inicial opcional para condicionamento fonético do Whisper
        #[arg(long)]
        prompt: Option<String>,
        /// Número de threads do Whisper (-t)
        #[arg(short, long)]
        threads: Option<u32>,
        /// Beam size para o Whisper (-bs)
        #[arg(long)]
        beam_size: Option<u32>,
        /// Ativa modo de decodificação rápida (beam=1, best_of=1, no-fallback)
        #[arg(long, visible_alias = "fast", default_value_t = false)]
        fast: bool,
    },
    /// Executa comparação A/B entre os algoritmos de reamostragem (Linear Baseline vs FIR Antialiasing)
    CompareResamplers {
        /// Caminho de arquivo WAV bruto nativo (ex: data/voice_diag/raw_captured_diag_1a13645e57e_1.wav)
        #[arg(long)]
        raw_audio: Option<String>,
        /// ID do caso de teste no manifesto (ex: "T14_curto_calculadora")
        #[arg(long)]
        case_id: Option<String>,
        /// Caminho do arquivo de manifesto do corpus JSON
        #[arg(long, default_value = "data/voice_corpus/corpus_manifest.json")]
        manifest: String,
        /// Idioma alvo do Whisper ('pt', 'en', 'es', 'ja', 'auto')
        #[arg(short, long)]
        language: Option<String>,
        /// Transcrição de referência esperada
        #[arg(long)]
        reference: Option<String>,
    },
    /// Lista os modelos de voz suportados (catálogo oficial pt-BR e modelos locais detectados)
    Voices,
    /// Gera uma amostra de teste no Laboratório Vocal para calibração e análise auditiva
    Sample {
        /// ID do modelo de voz (ex: "pt_BR-faber-medium", "acoustic")
        #[arg(short, long, default_value = "acoustic")]
        voice: String,
        /// ID da frase padronizada (saudacao, explicacao_tecnica, conversa_natural, aviso, pronuncia)
        #[arg(long, default_value = "saudacao")]
        phrase: String,
        /// Reproduz fisicamente a amostra no dispositivo de áudio
        #[arg(long, default_value_t = false)]
        play: bool,
        /// Identificador ou nome do dispositivo de saída para reprodução (opcional)
        #[arg(long)]
        output_device: Option<String>,
        /// Fator de escala de duração/cadência (ex: 1.14)
        #[arg(long)]
        length_scale: Option<f32>,
        /// Escala de variabilidade fonêmica (ex: 0.55)
        #[arg(long)]
        noise_scale: Option<f32>,
        /// Peso de estabilidade fonêmica (ex: 0.70)
        #[arg(long)]
        noise_w: Option<f32>,
    },
    /// Gera amostras comparativas da mesma frase padronizada entre os modelos de voz disponíveis
    Compare {
        /// ID da frase padronizada (saudacao, explicacao_tecnica, conversa_natural, aviso, pronuncia)
        #[arg(long, default_value = "saudacao")]
        phrase: String,
        /// Reproduz fisicamente cada amostra sequencialmente no dispositivo de áudio
        #[arg(long, default_value_t = false)]
        play: bool,
        /// Identificador ou nome do dispositivo de saída para reprodução (opcional)
        #[arg(long)]
        output_device: Option<String>,
    },
    /// Registra uma avaliação estruturada de qualidade e naturalidade vocal no histórico
    Evaluate {
        /// ID da voz avaliada (ex: pt_BR-faber-medium, acoustic)
        #[arg(short, long)]
        voice: String,
        /// ID da frase avaliada (saudacao, explicacao_tecnica, etc.)
        #[arg(long)]
        phrase: Option<String>,
        /// Pontuação de Naturalidade fonêmica (1 a 100)
        #[arg(long)]
        naturalness: u8,
        /// Pontuação de Tonalidade/Timbre (1 a 100)
        #[arg(long)]
        tonality: u8,
        /// Pontuação de Suavidade melódica (1 a 100)
        #[arg(long)]
        smoothness: u8,
        /// Pontuação de Expressividade/Empatia (1 a 100)
        #[arg(long)]
        expressiveness: u8,
        /// Pontuação de Ritmo e Cadência (1 a 100)
        #[arg(long)]
        rhythm: u8,
        /// Marca a voz como aprovada para operação padrão da Yuki
        #[arg(long, default_value_t = false)]
        approved: bool,
        /// Comentários e observações qualitativas do operador
        #[arg(long, default_value = "")]
        notes: String,
    },
    /// Exibe o Perfil Vocal da Yuki (especificação v0.1, parâmetros recomendados e estatísticas de avaliações)
    Profile,
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
    use yuki::contracts::voice::{AudioInput, TextToSpeechProvider};
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
            language,
            prompt,
            threads,
            beam_size,
            best_of,
            no_fallback,
            fast_stt,
            diagnostics,
        } => {
            use yuki::voice::stt::{
                normalize_language_code, LocalWhisperConfig, LocalWhisperSttProvider,
            };

            // 1. Resolução de idioma soberana: CLI flag (--language) > env (YUKI_WHISPER_LANGUAGE) > "pt"
            let env_lang = std::env::var("YUKI_WHISPER_LANGUAGE").ok();
            let raw_lang = language.as_deref().or(env_lang.as_deref()).unwrap_or("pt");
            let effective_lang = normalize_language_code(raw_lang);
            let is_diag_active = diagnostics || yuki::voice::diag::is_diagnostics_enabled();

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

                if is_diag_active {
                    let acoustic =
                        yuki::voice::diag::compute_acoustic_metrics(&samples, format.sample_rate);
                    println!(
                        "\n=== Métricas Acústicas do Arquivo de Áudio ('{}') ===",
                        path
                    );
                    println!(
                        "• Amostras:    {} | Formato: {}Hz, {} canal(is), {}-bit PCM",
                        samples.len(),
                        format.sample_rate,
                        format.channels,
                        format.bits_per_sample
                    );
                    println!(
                        "• RMS:         {:.1} ({:.2} dBFS)",
                        acoustic.rms, acoustic.rms_dbfs
                    );
                    println!(
                        "• Pico:        {} ({:.2} dBFS)",
                        acoustic.peak, acoustic.peak_dbfs
                    );
                    println!(
                        "• Clipping:    {} amostras ({:.2}%)",
                        acoustic.clipping_samples, acoustic.clipping_ratio_pct
                    );
                    println!("• Silêncio:    {:.1}%", acoustic.silence_ratio_pct);
                }

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
                if is_diag_active {
                    println!("Modo de diagnóstico ativo: capturando áudio bruto, canônico e métricas em data/voice_diag/");
                }

                let duration_sec = seconds.clamp(1, 30);
                println!(
                    "Iniciando captura de áudio do microfone por {} segundos...",
                    duration_sec
                );

                let backend: Arc<dyn AudioCaptureBackend> = {
                    #[cfg(windows)]
                    {
                        let win = WindowsAudioCaptureBackend::with_diagnostics(
                            target_device,
                            is_diag_active,
                        );
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

            let mut config = LocalWhisperConfig::default().with_language(effective_lang);
            if fast_stt {
                config = config.with_fast_decoding();
            }
            if let Some(t) = threads {
                if let Err(e) = config.set_threads(t) {
                    eprintln!("Aviso: {}", e);
                }
            }
            if let Some(bs) = beam_size {
                if let Err(e) = config.set_beam_size(bs) {
                    eprintln!("Aviso: {}", e);
                }
            }
            if let Some(bo) = best_of {
                if let Err(e) = config.set_best_of(bo) {
                    eprintln!("Aviso: {}", e);
                }
            }
            if no_fallback {
                config = config.with_no_fallback(true);
            }
            if let Some(ref p) = prompt {
                config = config.with_initial_prompt(p.clone());
            }

            let stt = LocalWhisperSttProvider::new(config);
            println!(
                "Transcrevendo áudio via STT local Whisper (fonte: {}, idioma solicitado: '{}')...",
                audio_input.source,
                stt.config().language
            );

            match stt.transcribe_with_metadata(&audio_input).await {
                Ok(result) => {
                    println!("\n=== Resultado da Transcrição Local Whisper ===");
                    println!("Transcrição obtida: \"{}\"", result.text);
                    println!("Modelo utilizado:   {}", result.model_path.display());
                    println!("Idioma solicitado:  {}", result.language_requested);
                    if let Some(detected) = result.detected_language {
                        let prob_str = result
                            .probability
                            .map(|p| format!(" (confiança: {:.1}%)", p * 100.0))
                            .unwrap_or_default();
                        println!("Idioma detectado:   {}{}", detected, prob_str);
                    } else if result.language_requested == "auto" {
                        println!("Idioma detectado:   não identificado pelo log do Whisper");
                    }
                    println!(
                        "Duração do áudio:   {:.2}s ({} ms)",
                        audio_input.duration_ms as f64 / 1000.0,
                        audio_input.duration_ms
                    );
                    println!(
                        "Latência total:     {:.2}s (Subprocesso: {:.2}s)",
                        result.elapsed.as_secs_f64(),
                        result.timings.subprocess_wall_ms / 1000.0
                    );
                    if let Some(inference_ms) = result.whisper_total_time_ms {
                        println!("Tempo inferência:   {:.2}s", inference_ms / 1000.0);
                    }
                    if let Some(fb) = result.timings.whisper_fallbacks {
                        if fb > 0 {
                            println!("Fallbacks acionados:{}", fb);
                        }
                    }

                    if let Some(ref diag) = result.diagnostics {
                        yuki::voice::diag::print_diagnostic_summary(diag);
                    } else if let Some(ref diag) = audio_input.diagnostics {
                        yuki::voice::diag::print_diagnostic_summary(diag);
                    }
                }
                Err(e) => eprintln!("Falha na transcrição: {}", e),
            }
        }
        VoiceCommands::Turn {
            seconds,
            input_device,
            output_device,
            play,
            language,
            prompt,
            threads,
            beam_size,
            best_of,
            no_fallback,
            fast_stt,
            diagnostics,
        } => {
            use std::sync::Arc;
            use std::time::Duration;
            use yuki::voice::capture::{
                AudioCaptureBackend, PushToTalkController, WindowsAudioCaptureBackend,
            };
            use yuki::voice::conversation::VoiceConversationCoordinator;
            use yuki::voice::playback::AudioPlaybackController;

            let is_diag_active = diagnostics || yuki::voice::diag::is_diagnostics_enabled();

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
            if is_diag_active {
                println!("Modo de diagnóstico ativo: capturando áudio bruto, canônico e métricas em data/voice_diag/");
            }

            let duration_sec = seconds.clamp(1, 30);
            println!(
                "Iniciando turno conversacional de voz Push-to-Talk ({}s de captura)...",
                duration_sec
            );

            let capture_backend: Arc<dyn AudioCaptureBackend> = {
                #[cfg(windows)]
                {
                    let win = WindowsAudioCaptureBackend::with_diagnostics(
                        target_input_id,
                        is_diag_active,
                    );
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
                    if let Some(ref diag) = input.diagnostics {
                        yuki::voice::diag::print_diagnostic_summary(diag);
                    }
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

            // Resolução do idioma de STT para o roteador governado de conversação
            let env_lang = std::env::var("YUKI_WHISPER_LANGUAGE").ok();
            let raw_lang = language.as_deref().or(env_lang.as_deref()).unwrap_or("pt");
            let effective_lang = yuki::voice::stt::normalize_language_code(raw_lang);
            let mut whisper_cfg =
                yuki::voice::stt::LocalWhisperConfig::default().with_language(effective_lang);
            if fast_stt {
                whisper_cfg = whisper_cfg.with_fast_decoding();
            }
            if let Some(t) = threads {
                if let Err(e) = whisper_cfg.set_threads(t) {
                    eprintln!("Aviso: {}", e);
                }
            }
            if let Some(bs) = beam_size {
                if let Err(e) = whisper_cfg.set_beam_size(bs) {
                    eprintln!("Aviso: {}", e);
                }
            }
            if let Some(bo) = best_of {
                if let Err(e) = whisper_cfg.set_best_of(bo) {
                    eprintln!("Aviso: {}", e);
                }
            }
            if no_fallback {
                whisper_cfg = whisper_cfg.with_no_fallback(true);
            }
            if let Some(ref p) = prompt {
                whisper_cfg = whisper_cfg.with_initial_prompt(p.clone());
            }
            let local_stt = Arc::new(yuki::voice::stt::LocalWhisperSttProvider::new(whisper_cfg));
            let local_tts: Arc<dyn yuki::contracts::voice::TextToSpeechProvider> =
                if yuki::voice::tts::LocalPiperTtsProvider::default().is_environment_ready() {
                    Arc::new(yuki::voice::tts::LocalPiperTtsProvider::default())
                } else {
                    Arc::new(yuki::voice::tts::LocalAcousticTtsProvider::default())
                };

            let router = Arc::new(yuki::voice::router::GovernedVoiceRouter::new(
                local_stt,
                None,
                local_tts,
                None,
                yuki::contracts::voice::VoiceConfiguration::default(),
            ));

            let coordinator = VoiceConversationCoordinator::new(
                core,
                router,
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
        VoiceCommands::CompareStt {
            file,
            language,
            prompt,
            threads,
            beam_size,
            fast,
            diagnostics,
        } => {
            use yuki::voice::stt::{
                normalize_language_code, LocalWhisperConfig, LocalWhisperSttProvider,
            };

            let data = match std::fs::read(&file) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("Erro ao ler arquivo '{}': {}", file, e);
                    return;
                }
            };
            let (format, samples) = match AudioCodec::decode_wav(&data) {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("Erro ao decodificar arquivo WAV '{}': {}", file, e);
                    return;
                }
            };
            let duration_ms = (samples.len() as f64 / format.sample_rate as f64 * 1000.0) as u64;

            let env_lang = std::env::var("YUKI_WHISPER_LANGUAGE").ok();
            let raw_lang = language.as_deref().or(env_lang.as_deref()).unwrap_or("pt");
            let effective_lang = normalize_language_code(raw_lang);
            let is_diag_active = diagnostics || yuki::voice::diag::is_diagnostics_enabled();

            let audio_input = AudioInput::new(data, format.clone(), duration_ms, file.clone());
            if is_diag_active {
                let acoustic =
                    yuki::voice::diag::compute_acoustic_metrics(&samples, format.sample_rate);
                println!("\n=== Análise Acústica Prévia do Arquivo ('{}') ===", file);
                println!(
                    "• Duração: {:.2}s ({} ms) | RMS: {:.1} ({:.2} dBFS) | Pico: {} ({:.2} dBFS)",
                    duration_ms as f64 / 1000.0,
                    duration_ms,
                    acoustic.rms,
                    acoustic.rms_dbfs,
                    acoustic.peak,
                    acoustic.peak_dbfs
                );
            }

            println!("\n╔══════════════════════════════════════════════════════════════════════════════╗");
            println!(
                "║              TESTE COMPARATIVO A/B — STT LOCAL WHISPER (MARCO 8.1.1)         ║"
            );
            println!(
                "╚══════════════════════════════════════════════════════════════════════════════╝"
            );
            println!("• Arquivo testado:     {}", file);
            println!(
                "• Duração do áudio:    {:.2}s ({} ms)",
                duration_ms as f64 / 1000.0,
                duration_ms
            );
            println!("• Idioma base:         {}", effective_lang);

            // 1. Execução A: Baseline neutro (parâmetros padrão, sem prompt)
            let config_a = LocalWhisperConfig::default().with_language(&effective_lang);
            let stt_a = LocalWhisperSttProvider::new(config_a);
            println!("\n[1/2] Executando Configuração A (Baseline neutro sem prompt)...");
            let res_a = match stt_a.transcribe_with_metadata(&audio_input).await {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("Falha na execução A (Baseline): {}", e);
                    return;
                }
            };

            // 2. Execução B: Experimental (Prompt configurado ou default da Yuki + flags de decodificação)
            let mut config_b = LocalWhisperConfig::default().with_language(&effective_lang);
            if fast {
                config_b = config_b.with_fast_decoding();
            }
            if let Some(t) = threads {
                if let Err(e) = config_b.set_threads(t) {
                    eprintln!("Aviso: {}", e);
                }
            }
            if let Some(bs) = beam_size {
                if let Err(e) = config_b.set_beam_size(bs) {
                    eprintln!("Aviso: {}", e);
                }
            }
            if let Some(ref p) = prompt {
                config_b = config_b.with_initial_prompt(p.clone());
            } else {
                config_b = config_b.with_default_prompt_for_language();
            }

            let stt_b = LocalWhisperSttProvider::new(config_b);
            println!("[2/2] Executando Configuração B (Experimental com contexto lexical/decodificação)...");
            let res_b = match stt_b.transcribe_with_metadata(&audio_input).await {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("Falha na execução B (Experimental): {}", e);
                    return;
                }
            };

            // 3. Relatório Comparativo Estruturado
            println!("\n════════════════════════════════════════════════════════════════════════════════");
            println!(
                "                           QUADRO COMPARATIVO A/B                               "
            );
            println!(
                "════════════════════════════════════════════════════════════════════════════════"
            );
            println!(
                "{:<24} | {:<25} | {:<25}",
                "Métrica / Parâmetro", "Config A (Baseline)", "Config B (Experimental)"
            );
            println!("{:-<24}-+-{:-<25}-+-{:-<25}", "", "", "");

            let p_a_desc = "Desativado";
            let p_b_desc = res_b
                .decoding_config
                .prompt_preview
                .as_deref()
                .unwrap_or("Ativo");
            println!(
                "{:<24} | {:<25} | {:<25}",
                "Prompt Lexical", p_a_desc, p_b_desc
            );

            let th_a = res_a
                .decoding_config
                .threads
                .map(|v| v.to_string())
                .unwrap_or_else(|| "4 (default)".into());
            let th_b = res_b
                .decoding_config
                .threads
                .map(|v| v.to_string())
                .unwrap_or_else(|| "4 (default)".into());
            println!("{:<24} | {:<25} | {:<25}", "Threads (-t)", th_a, th_b);

            let bs_a = res_a
                .decoding_config
                .beam_size
                .map(|v| v.to_string())
                .unwrap_or_else(|| "5 (default)".into());
            let bs_b = res_b
                .decoding_config
                .beam_size
                .map(|v| v.to_string())
                .unwrap_or_else(|| "5 (default)".into());
            println!("{:<24} | {:<25} | {:<25}", "Beam Size (-bs)", bs_a, bs_b);

            let bo_a = res_a
                .decoding_config
                .best_of
                .map(|v| v.to_string())
                .unwrap_or_else(|| "5 (default)".into());
            let bo_b = res_b
                .decoding_config
                .best_of
                .map(|v| v.to_string())
                .unwrap_or_else(|| "5 (default)".into());
            println!("{:<24} | {:<25} | {:<25}", "Best-of (-bo)", bo_a, bo_b);

            let nf_a = if res_a.decoding_config.no_fallback {
                "Sim"
            } else {
                "Não (0.2, 0.4, 0.6)"
            };
            let nf_b = if res_b.decoding_config.no_fallback {
                "Sim (--no-fallback)"
            } else {
                "Não (0.2, 0.4, 0.6)"
            };
            println!("{:<24} | {:<25} | {:<25}", "Sem Fallback", nf_a, nf_b);

            println!("{:-<24}-+-{:-<25}-+-{:-<25}", "", "", "");
            println!(
                "{:<24} | {:<25} | {:<25}",
                "Subprocesso (Wall)",
                format!("{:.1} ms", res_a.timings.subprocess_wall_ms),
                format!("{:.1} ms", res_b.timings.subprocess_wall_ms)
            );

            let w_tot_a = res_a
                .timings
                .whisper_total_ms
                .map(|v| format!("{:.1} ms", v))
                .unwrap_or_else(|| "N/D".into());
            let w_tot_b = res_b
                .timings
                .whisper_total_ms
                .map(|v| format!("{:.1} ms", v))
                .unwrap_or_else(|| "N/D".into());
            println!(
                "{:<24} | {:<25} | {:<25}",
                "Whisper Total", w_tot_a, w_tot_b
            );

            let enc_a = res_a
                .timings
                .whisper_encode_ms
                .map(|v| format!("{:.1} ms", v))
                .unwrap_or_else(|| "N/D".into());
            let enc_b = res_b
                .timings
                .whisper_encode_ms
                .map(|v| format!("{:.1} ms", v))
                .unwrap_or_else(|| "N/D".into());
            println!("{:<24} | {:<25} | {:<25}", "Encoder Whisper", enc_a, enc_b);

            let dec_a = res_a
                .timings
                .whisper_decode_ms
                .map(|v| format!("{:.1} ms", v))
                .unwrap_or_else(|| "N/D".into());
            let dec_b = res_b
                .timings
                .whisper_decode_ms
                .map(|v| format!("{:.1} ms", v))
                .unwrap_or_else(|| "N/D".into());
            println!("{:<24} | {:<25} | {:<25}", "Decoder Whisper", dec_a, dec_b);

            let fb_a = res_a
                .timings
                .whisper_fallbacks
                .map(|v| v.to_string())
                .unwrap_or_else(|| "0".into());
            let fb_b = res_b
                .timings
                .whisper_fallbacks
                .map(|v| v.to_string())
                .unwrap_or_else(|| "0".into());
            println!("{:<24} | {:<25} | {:<25}", "Fallbacks Usados", fb_a, fb_b);

            println!(
                "{:<24} | {:<25} | {:<25}",
                "Latência Total STT",
                format!("{:.2} s", res_a.elapsed.as_secs_f64()),
                format!("{:.2} s", res_b.elapsed.as_secs_f64())
            );

            println!(
                "════════════════════════════════════════════════════════════════════════════════"
            );

            // Cálculo do delta de desempenho
            let wall_a = res_a.timings.subprocess_wall_ms;
            let wall_b = res_b.timings.subprocess_wall_ms;
            if wall_a > 0.0 && wall_b > 0.0 {
                let speedup = wall_a / wall_b;
                let reduction_pct = (1.0 - wall_b / wall_a) * 100.0;
                if speedup >= 1.0 {
                    println!(
                        "• Ganho de Velocidade: {:.2}x mais rápido ({:+.1}% de latência economizada)",
                        speedup, reduction_pct
                    );
                } else {
                    println!(
                        "• Variação de Tempo:   {:.2}x ({:+.1}% de tempo adicional)",
                        speedup, -reduction_pct
                    );
                }
            }

            println!("\n--- Transcrições Obtidas ---");
            println!("• Config A (Baseline):");
            println!("  \"{}\"", res_a.text);
            println!("• Config B (Experimental):");
            println!("  \"{}\"", res_b.text);

            if res_a.text == res_b.text {
                println!(
                    "\n• Análise Textual:      Transcrição IDÊNTICA entre as duas configurações."
                );
            } else {
                println!(
                    "\n• Análise Textual:      Diferença identificada no reconhecimento do texto."
                );
            }
            println!("════════════════════════════════════════════════════════════════════════════════\n");
        }
        VoiceCommands::EvalStt {
            audio,
            reference,
            terms,
            language,
            manifest,
            category,
            case_id,
            runs,
            export_json,
            prompt,
            threads,
            beam_size,
            fast,
        } => {
            use std::str::FromStr;
            use yuki::voice::stt::eval::{
                evaluate_single_test_case, CorpusCategory, CorpusEvaluationReport, CorpusTestCase,
                VoiceCorpusManifest,
            };
            use yuki::voice::stt::{normalize_language_code, LocalWhisperConfig};

            let runs_count = runs.max(1);
            let env_lang = std::env::var("YUKI_WHISPER_LANGUAGE").ok();
            let raw_lang = language.as_deref().or(env_lang.as_deref()).unwrap_or("pt");
            let effective_lang = normalize_language_code(raw_lang);

            let mut config = LocalWhisperConfig::default().with_language(&effective_lang);
            if let Some(ref p) = prompt {
                config = config.with_initial_prompt(p);
            }
            if let Some(t) = threads {
                if let Err(e) = config.set_threads(t) {
                    eprintln!("Aviso: {}", e);
                }
            }
            if let Some(b) = beam_size {
                if let Err(e) = config.set_beam_size(b) {
                    eprintln!("Aviso: {}", e);
                }
            }
            if fast {
                let _ = config.set_beam_size(1);
                let _ = config.set_best_of(1);
                config.no_fallback = true;
            }

            println!("\n╔════════════════════════════════════════════════════════════════════════════════════════════════════════╗");
            println!("║                  LABORATÓRIO DE HOMOLOGAÇÃO E QUALIDADE DE VOZ — STT LOCAL (MARCO 8.1.1)               ║");
            println!("╚════════════════════════════════════════════════════════════════════════════════════════════════════════╝");
            println!("• Idioma Base:         {}", effective_lang);
            println!(
                "• Repetições p/ Áudio: {} (estatística robusta: mediana, min/max e dispersão)",
                runs_count
            );
            if let Some(ref p) = config.initial_prompt {
                println!("• Prompt Lexical:      \"{}\"", p);
            } else {
                println!("• Prompt Lexical:      [Desativado / Neutro]");
            }
            let th_str = config
                .threads
                .map(|t| t.to_string())
                .unwrap_or_else(|| "4 (default)".into());
            let bm_str = config
                .beam_size
                .map(|b| b.to_string())
                .unwrap_or_else(|| "5 (default)".into());
            let nf_str = if config.no_fallback {
                "Ativo (modo rápido)"
            } else {
                "Desativado (0.2, 0.4, 0.6)"
            };
            println!(
                "• Parâmetros Whisper:  Threads: {} | Beam: {} | No-Fallback: {}",
                th_str, bm_str, nf_str
            );

            let important_terms_list: Vec<String> = terms
                .map(|t| {
                    t.split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect()
                })
                .unwrap_or_default();

            // Ramo 1: Avaliação de arquivo individual avulso
            if let Some(ref audio_path) = audio {
                println!(
                    "• Modo de Execução:    Arquivo individual ('{}')",
                    audio_path
                );
                println!("────────────────────────────────────────────────────────────────────────────────────────────────────────");

                let manual_case = CorpusTestCase {
                    id: "audio_individual".to_string(),
                    audio_path: audio_path.clone(),
                    ground_truth: reference.clone(),
                    review_pending: reference.is_none(),
                    language: effective_lang.clone(),
                    device_name: None,
                    category: CorpusCategory::ShortPhrase,
                    important_terms: important_terms_list,
                    duration_ms: None,
                    notes: Some("Avaliação manual de arquivo individual via CLI".into()),
                };

                match evaluate_single_test_case(&manual_case, &config, runs_count).await {
                    Ok(rep) => {
                        println!("\n[Resultado da Transcrição]");
                        if let Some(ref gt) = rep.ground_truth {
                            println!("• Referência (GT):   \"{}\"", gt);
                        } else {
                            println!("• Referência (GT):   [Não fornecida — Review Pending]");
                        }
                        println!("• Hipótese (STT):    \"{}\"", rep.hypothesis);

                        if let Some(ref wer) = rep.wer {
                            println!("\n[Métricas de Precisão]");
                            println!("• WER (Word Error Rate):       {:.2}% (Subs: {}, Del: {}, Ins: {}, Hits: {} / Total: {})",
                                wer.wer_rate * 100.0, wer.substitutions, wer.deletions, wer.insertions, wer.hits, wer.reference_words
                            );
                        }
                        if let Some(ref cer) = rep.cer {
                            println!("• CER (Character Error Rate):  {:.2}% (Subs: {}, Del: {}, Ins: {}, Hits: {} / Total: {})",
                                cer.cer_rate * 100.0, cer.substitutions, cer.deletions, cer.insertions, cer.hits, cer.reference_chars
                            );
                        }
                        if let Some(ref kt) = rep.key_terms {
                            println!(
                                "• Termos Críticos (Acurácia):  {:.1}% (TP: {}, FN: {}, FP: {})",
                                kt.accuracy * 100.0,
                                kt.true_positives,
                                kt.false_negatives,
                                kt.false_positives
                            );
                            for d in &kt.details {
                                println!("  - \"{}\": {}", d.term, d.status);
                            }
                        }

                        println!("\n[Estatística de Latência]");
                        println!("• Mediana:            {:.1} ms", rep.latency.median_ms);
                        println!(
                            "• Média:              {:.1} ms (± {:.1} ms)",
                            rep.latency.mean_ms, rep.latency.std_dev_ms
                        );
                        println!(
                            "• Mínimo / Máximo:    {:.1} ms / {:.1} ms",
                            rep.latency.min_ms, rep.latency.max_ms
                        );
                        if rep.latency.runs_count > 1 {
                            println!("• Leituras brutas:    {:?}", rep.latency.raw_runs_ms);
                        }

                        if let Some(ref export_path) = export_json {
                            let report = CorpusEvaluationReport {
                                generated_at: yuki::contracts::identifiers::now_utc().to_rfc3339(),
                                manifest_path: None,
                                total_cases: 1,
                                evaluated_cases: if rep.ground_truth.is_some() { 1 } else { 0 },
                                review_pending_cases: if rep.review_pending { 1 } else { 0 },
                                average_wer: rep.wer.as_ref().map(|w| w.wer_rate),
                                average_cer: rep.cer.as_ref().map(|c| c.cer_rate),
                                overall_key_terms_accuracy: rep
                                    .key_terms
                                    .as_ref()
                                    .map(|k| k.accuracy),
                                average_latency_ms: rep.latency.median_ms,
                                reports: vec![rep],
                            };
                            if let Err(e) = report.export_to_json(export_path) {
                                eprintln!("Erro ao exportar JSON: {}", e);
                            } else {
                                println!(
                                    "\nRelatório exportado com sucesso para '{}'",
                                    export_path
                                );
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("\nFalha na avaliação do áudio: {}", e);
                    }
                }
                println!("════════════════════════════════════════════════════════════════════════════════════════════════════════\n");
                return;
            }

            // Ramo 2: Avaliação baseada no Manifesto do Corpus
            println!("• Modo de Execução:    Corpus Manifest ('{}')", manifest);
            let corpus_manifest = match VoiceCorpusManifest::load_from_file(&manifest) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!(
                        "\nErro ao carregar manifesto do corpus '{}': {}",
                        manifest, e
                    );
                    return;
                }
            };

            let parsed_category = if let Some(ref cat_str) = category {
                match CorpusCategory::from_str(cat_str) {
                    Ok(c) => Some(c),
                    Err(e) => {
                        eprintln!("Aviso: {}. Avaliando todas as categorias.", e);
                        None
                    }
                }
            } else {
                None
            };

            let mut target_cases: Vec<&CorpusTestCase> =
                corpus_manifest.test_cases.iter().collect();
            if let Some(ref cid) = case_id {
                target_cases.retain(|c| c.id.eq_ignore_ascii_case(cid));
            }
            if let Some(cat) = parsed_category {
                target_cases.retain(|c| c.category == cat);
            }

            println!(
                "• Casos Selecionados:  {} de {}",
                target_cases.len(),
                corpus_manifest.test_cases.len()
            );
            println!("────────────────────────────────────────────────────────────────────────────────────────────────────────");

            if target_cases.is_empty() {
                println!("Nenhum caso de teste correspondeu aos filtros aplicados.");
                return;
            }

            let mut reports = Vec::new();

            for (idx, case) in target_cases.iter().enumerate() {
                print!(
                    "• [{}/{}] Avaliando '{}' ({})... ",
                    idx + 1,
                    target_cases.len(),
                    case.id,
                    case.category
                );
                std::io::Write::flush(&mut std::io::stdout()).ok();

                match evaluate_single_test_case(case, &config, runs_count).await {
                    Ok(rep) => {
                        let status_str = if rep.review_pending {
                            "PENDENTE (sem GT)".to_string()
                        } else if let Some(ref w) = rep.wer {
                            format!("WER: {:.1}%", w.wer_rate * 100.0)
                        } else {
                            "OK".to_string()
                        };
                        println!(
                            "Concluído ({}, Lat: {:.0}ms)",
                            status_str, rep.latency.median_ms
                        );
                        reports.push(rep);
                    }
                    Err(e) => {
                        println!("FALHOU ({})", e);
                    }
                }
            }

            println!("\n╔════════════════════════════════════════════════════════════════════════════════════════════════════════╗");
            println!("║                             TABELA CONSOLIDADA DE RESULTADOS DO CORPUS                                 ║");
            println!("╠══════════════════════════════╦══════════╦══════════╦════════════════════╦══════════════╦══════════════════╣");
            println!("║ Caso de Teste / ID           ║ WER (%)  ║ CER (%)  ║ Termos Críticos    ║ Latência Med ║ Status           ║");
            println!("╠══════════════════════════════╬══════════╬══════════╬════════════════════╬══════════════╬══════════════════╣");

            let mut evaluated_count = 0;
            let mut review_pending_count = 0;
            let mut sum_wer = 0.0;
            let mut count_wer = 0;
            let mut sum_cer = 0.0;
            let mut count_cer = 0;
            let mut sum_terms_acc = 0.0;
            let mut count_terms = 0;
            let mut sum_latency = 0.0;

            for r in &reports {
                sum_latency += r.latency.median_ms;
                let wer_str = r
                    .wer
                    .as_ref()
                    .map(|w| format!("{:.1}%", w.wer_rate * 100.0))
                    .unwrap_or_else(|| "N/D".into());
                let cer_str = r
                    .cer
                    .as_ref()
                    .map(|c| format!("{:.1}%", c.cer_rate * 100.0))
                    .unwrap_or_else(|| "N/D".into());
                let terms_str = r
                    .key_terms
                    .as_ref()
                    .map(|kt| format!("{:.0}% (TP:{})", kt.accuracy * 100.0, kt.true_positives))
                    .unwrap_or_else(|| "N/D".into());
                let status_str = if r.review_pending {
                    review_pending_count += 1;
                    "Review Pending"
                } else {
                    evaluated_count += 1;
                    if let Some(ref w) = r.wer {
                        sum_wer += w.wer_rate;
                        count_wer += 1;
                    }
                    if let Some(ref c) = r.cer {
                        sum_cer += c.cer_rate;
                        count_cer += 1;
                    }
                    if let Some(ref kt) = r.key_terms {
                        sum_terms_acc += kt.accuracy;
                        count_terms += 1;
                    }
                    "Avaliador OK"
                };

                println!(
                    "║ {:<28} ║ {:<8} ║ {:<8} ║ {:<18} ║ {:>9.1} ms ║ {:<16} ║",
                    if r.test_case_id.len() > 28 {
                        &r.test_case_id[..28]
                    } else {
                        &r.test_case_id
                    },
                    wer_str,
                    cer_str,
                    terms_str,
                    r.latency.median_ms,
                    status_str
                );
            }
            println!("╚══════════════════════════════╩══════════╩══════════╩════════════════════╩══════════════╩══════════════════╝");

            println!("\n--- Detalhes Textuais das Transcrições ---");
            for r in &reports {
                println!("\n• [{}]", r.test_case_id);
                if let Some(ref gt) = r.ground_truth {
                    println!("  Referência: \"{}\"", gt);
                } else {
                    println!("  Referência: [Pendente de Revisão / Documentação Oficial]");
                }
                println!("  Hipótese:   \"{}\"", r.hypothesis);
                if let Some(ref w) = r.wer {
                    println!(
                        "  WER:        {:.1}% (S: {}, D: {}, I: {}, C: {})",
                        w.wer_rate * 100.0,
                        w.substitutions,
                        w.deletions,
                        w.insertions,
                        w.hits
                    );
                }
                if let Some(ref c) = r.cer {
                    println!(
                        "  CER:        {:.1}% (S: {}, D: {}, I: {}, C: {})",
                        c.cer_rate * 100.0,
                        c.substitutions,
                        c.deletions,
                        c.insertions,
                        c.hits
                    );
                }
                if let Some(ref kt) = r.key_terms {
                    let details_str = kt
                        .details
                        .iter()
                        .map(|d| format!("{}: {:?}", d.term, d.status))
                        .collect::<Vec<_>>()
                        .join(", ");
                    println!(
                        "  Termos:     {:.0}% ({})",
                        kt.accuracy * 100.0,
                        details_str
                    );
                }
                println!("  Latência:   Mediana: {:.1} ms | Média: {:.1} ms (± {:.1} ms) | Min: {:.1} ms | Max: {:.1} ms",
                    r.latency.median_ms, r.latency.mean_ms, r.latency.std_dev_ms, r.latency.min_ms, r.latency.max_ms
                );
            }

            let avg_wer = if count_wer > 0 {
                Some(sum_wer / count_wer as f64)
            } else {
                None
            };
            let avg_cer = if count_cer > 0 {
                Some(sum_cer / count_cer as f64)
            } else {
                None
            };
            let avg_terms = if count_terms > 0 {
                Some(sum_terms_acc / count_terms as f64)
            } else {
                None
            };
            let avg_lat = if !reports.is_empty() {
                sum_latency / reports.len() as f64
            } else {
                0.0
            };

            println!("\n════════════════════════════════════════════════════════════════════════════════════════════════════════");
            println!("RESUMO CONSOLIDADO DA ETAPA 2A:");
            println!("• Total de Casos no Relatório:    {}", reports.len());
            println!("• Casos com Referência Avaliada:  {}", evaluated_count);
            println!(
                "• Casos com Revisão Pendente:     {} (preservados sem referências fictícias)",
                review_pending_count
            );
            if let Some(w) = avg_wer {
                println!("• WER Médio:                      {:.2}%", w * 100.0);
            } else {
                println!("• WER Médio:                      N/D (nenhuma referência validada)");
            }
            if let Some(c) = avg_cer {
                println!("• CER Médio:                      {:.2}%", c * 100.0);
            } else {
                println!("• CER Médio:                      N/D (nenhuma referência validada)");
            }
            if let Some(t) = avg_terms {
                println!("• Acurácia de Termos Críticos:   {:.2}%", t * 100.0);
            }
            println!("• Latência Mediana Média:         {:.1} ms", avg_lat);
            println!("════════════════════════════════════════════════════════════════════════════════════════════════════════");

            if let Some(ref export_path) = export_json {
                let report = CorpusEvaluationReport {
                    generated_at: yuki::contracts::identifiers::now_utc().to_rfc3339(),
                    manifest_path: Some(manifest.clone()),
                    total_cases: reports.len(),
                    evaluated_cases: evaluated_count,
                    review_pending_cases: review_pending_count,
                    average_wer: avg_wer,
                    average_cer: avg_cer,
                    overall_key_terms_accuracy: avg_terms,
                    average_latency_ms: avg_lat,
                    reports,
                };
                if let Err(e) = report.export_to_json(export_path) {
                    eprintln!("Erro ao exportar JSON: {}", e);
                } else {
                    println!(
                        "\nRelatório do corpus exportado com sucesso para '{}'",
                        export_path
                    );
                }
            }
            println!();
        }
        VoiceCommands::CompareResamplers {
            raw_audio,
            case_id,
            manifest,
            language,
            reference,
        } => {
            use yuki::voice::stt::eval::{run_resampler_ab_comparison, VoiceCorpusManifest};
            use yuki::voice::stt::{normalize_language_code, LocalWhisperConfig};

            let env_lang = std::env::var("YUKI_WHISPER_LANGUAGE").ok();
            let raw_lang = language.as_deref().or(env_lang.as_deref()).unwrap_or("pt");
            let effective_lang = normalize_language_code(raw_lang);
            let config = LocalWhisperConfig::default().with_language(&effective_lang);

            println!("\n╔════════════════════════════════════════════════════════════════════════════════════════════════════════╗");
            println!("║          COMPARAÇÃO A/B DE REAMOSTRAGEM — LINEAR (BASELINE) VS FIR ANTIALIASING (MARCO 8.1.1)          ║");
            println!("╚════════════════════════════════════════════════════════════════════════════════════════════════════════╝");

            if let Some(ref raw_path) = raw_audio {
                let id = case_id.as_deref().unwrap_or("raw_custom");
                println!("• Áudio Bruto:         {}", raw_path);
                println!("• Idioma Whisper:      {}", effective_lang);
                if let Some(ref r) = reference {
                    println!("• Referência:          \"{}\"", r);
                }

                match run_resampler_ab_comparison(id, raw_path, reference.as_deref(), &[], &config)
                    .await
                {
                    Ok(rep) => {
                        println!("\n════════════════════════════════════════════════════════════════════════════════");
                        println!("                    RESULTADOS COMPARATIVOS A/B (ÁUDIO BRUTO)");
                        println!("════════════════════════════════════════════════════════════════════════════════");
                        println!(
                            "{:<26} | {:<25} | {:<25}",
                            "Métrica / Parâmetro",
                            "Método A (Linear)",
                            "Método B (FIR Antialiasing)"
                        );
                        println!("{:-<26}-+-{:-<25}-+-{:-<25}", "", "", "");
                        println!(
                            "{:<26} | {:<25} | {:<25}",
                            "Energia RMS",
                            format!("{:.1}", rep.rms_linear),
                            format!("{:.1}", rep.rms_fir)
                        );
                        println!(
                            "{:<26} | {:<25} | {:<25}",
                            "Latência Inferência",
                            format!("{:.1} ms", rep.latency_linear_ms),
                            format!("{:.1} ms", rep.latency_fir_ms)
                        );
                        if let (Some(wl), Some(wf)) = (rep.wer_linear, rep.wer_fir) {
                            println!(
                                "{:<26} | {:<25} | {:<25}",
                                "WER",
                                format!("{:.1}%", wl * 100.0),
                                format!(
                                    "{:.1}% (Delta: {:+.1}%)",
                                    wf * 100.0,
                                    rep.wer_delta.unwrap() * 100.0
                                )
                            );
                        }
                        if let (Some(cl), Some(cf)) = (rep.cer_linear, rep.cer_fir) {
                            println!(
                                "{:<26} | {:<25} | {:<25}",
                                "CER",
                                format!("{:.1}%", cl * 100.0),
                                format!(
                                    "{:.1}% (Delta: {:+.1}%)",
                                    cf * 100.0,
                                    rep.cer_delta.unwrap() * 100.0
                                )
                            );
                        }
                        println!("{:-<26}-+-{:-<25}-+-{:-<25}", "", "", "");
                        println!("• Hipótese A (Linear):");
                        println!("  \"{}\"", rep.hypothesis_linear);
                        println!("• Hipótese B (FIR Antialiasing):");
                        println!("  \"{}\"", rep.hypothesis_fir);
                        if rep.hypothesis_linear == rep.hypothesis_fir {
                            println!("\n• Análise: Hipóteses IDÊNTICAS.");
                        } else {
                            println!("\n• Análise: DIFERENÇA DETECTADA entre as hipóteses.");
                        }
                    }
                    Err(e) => eprintln!("Erro ao executar comparação A/B: {}", e),
                }
                println!("════════════════════════════════════════════════════════════════════════════════\n");
                return;
            }

            // Modo Manifesto: avalia casos com gravações brutas existentes
            let corpus_manifest = match VoiceCorpusManifest::load_from_file(&manifest) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("Erro ao carregar manifesto '{}': {}", manifest, e);
                    return;
                }
            };

            let target_cases: Vec<&yuki::voice::stt::eval::CorpusTestCase> =
                if let Some(ref cid) = case_id {
                    corpus_manifest
                        .test_cases
                        .iter()
                        .filter(|c| &c.id == cid)
                        .collect()
                } else {
                    corpus_manifest.test_cases.iter().collect()
                };

            let mut compared_count = 0;
            println!("Procurando capturas brutas (raw_captured_*.wav) correspondentes...\n");

            for case in &target_cases {
                let canonical_path = std::path::Path::new(&case.audio_path);
                let file_name = canonical_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                let raw_file_name = file_name.replace("canonical_16k_", "raw_captured_");
                let raw_path = canonical_path.with_file_name(raw_file_name);

                if !raw_path.exists() {
                    continue;
                }

                compared_count += 1;
                let case_config = config.clone().with_language(&case.language);
                println!("▶ [{}] {}", case.id, raw_path.display());
                if let Some(ref gt) = case.ground_truth {
                    println!("  Ref:    \"{}\"", gt);
                }

                match run_resampler_ab_comparison(
                    &case.id,
                    &raw_path,
                    case.ground_truth.as_deref(),
                    &case.important_terms,
                    &case_config,
                )
                .await
                {
                    Ok(rep) => {
                        println!("  Linear: \"{}\"", rep.hypothesis_linear);
                        println!("  FIR:    \"{}\"", rep.hypothesis_fir);
                        if let (Some(wl), Some(wf)) = (rep.wer_linear, rep.wer_fir) {
                            println!(
                                "  WER:    Linear: {:.1}% | FIR: {:.1}% | Delta: {:+.1}%",
                                wl * 100.0,
                                wf * 100.0,
                                rep.wer_delta.unwrap() * 100.0
                            );
                        }
                        if let (Some(cl), Some(cf)) = (rep.cer_linear, rep.cer_fir) {
                            println!(
                                "  CER:    Linear: {:.1}% | FIR: {:.1}% | Delta: {:+.1}%",
                                cl * 100.0,
                                cf * 100.0,
                                rep.cer_delta.unwrap() * 100.0
                            );
                        }
                        println!(
                            "  Tempo:  Linear: {:.1} ms | FIR: {:.1} ms\n",
                            rep.latency_linear_ms, rep.latency_fir_ms
                        );
                    }
                    Err(e) => eprintln!("  Falha: {}\n", e),
                }
            }

            println!(
                "════════════════════════════════════════════════════════════════════════════════"
            );
            println!("Total de casos físicos comparados: {}", compared_count);
            println!("════════════════════════════════════════════════════════════════════════════════\n");
        }
        VoiceCommands::Voices => {
            use yuki::voice::stt::{list_local_whisper_models, LocalWhisperSttProvider};
            use yuki::voice::tts::{get_piper_voice_catalog, LocalPiperTtsProvider};

            println!("=== Catálogo de Modelos de Voz da Yuki (STT e TTS) ===");
            println!("\n--- Síntese Neural de Voz (Piper TTS) ---");
            let piper_catalog = get_piper_voice_catalog();
            for model in &piper_catalog {
                let status_str = if model.is_installed {
                    format!(
                        "[INSTALADO] ({})",
                        model
                            .local_path
                            .as_ref()
                            .map(|p| p.display().to_string())
                            .unwrap_or_default()
                    )
                } else {
                    "[NÃO BAIXADO]".to_string()
                };
                println!(
                    "• ID: {} | {} ({})",
                    model.id, model.name, model.speaker_gender
                );
                println!("  Status:       {}", status_str);
                println!(
                    "  Amostragem:   {}Hz | Qualidade: {}",
                    model.sample_rate, model.quality
                );
                println!("  Descrição:    {}", model.description);
                if !model.is_installed && !model.download_onnx_url.is_empty() {
                    println!("  Download ONNX: {}", model.download_onnx_url);
                    println!("  Download JSON: {}", model.download_json_url);
                }
                println!();
            }

            println!("--- Sintetizador Acústico Embarcado (Fallback de Diagnóstico) ---");
            println!("• ID: acoustic | Sintetizador Harmônico Determinístico");
            println!("  Status:       [DISPONÍVEL] (Embarcado no binário)");
            println!("  Amostragem:   16000Hz | Tom: Feminino afável (~220Hz)");
            println!("  Descrição:    Sintetizador melódico puro para diagnóstico de hardware e testes sem dependências.\n");

            println!("--- Reconhecimento de Fala (Whisper STT Local) ---");
            let whisper_models = list_local_whisper_models();
            if whisper_models.is_empty() {
                println!("  Nenhum modelo .bin do Whisper detectado em ./models/whisper/.");
            } else {
                for m in &whisper_models {
                    println!("• Modelo local: {}", m.display());
                }
            }

            println!("\n--- Instruções de Download e Instalação (Sem Downloads Silenciosos) ---");
            println!("{}", LocalPiperTtsProvider::download_instructions());
            println!("\n{}", LocalWhisperSttProvider::installation_instructions());
        }
        VoiceCommands::Sample {
            voice,
            phrase,
            play,
            output_device,
            length_scale,
            noise_scale,
            noise_w,
        } => {
            use yuki::voice::lab::{find_phrase, generate_voice_sample};

            println!("=== Laboratório Yuki Voice — Geração de Amostra ===");
            if let Some(p) = find_phrase(&phrase) {
                println!("Frase selecionada [{}]: \"{}\"", p.id, p.text);
            } else {
                println!("Frase customizada/ID: \"{}\"", phrase);
            }
            println!("Voz alvo: {}", voice);

            match generate_voice_sample(&voice, &phrase, length_scale, noise_scale, noise_w).await {
                Ok((audio_out, meta)) => {
                    println!("\n✓ Amostra gerada com sucesso!");
                    println!("ID da Amostra:     {}", meta.sample_id);
                    println!("Arquivo WAV:       {}", meta.file_path);
                    println!(
                        "Duração:           {:.2}s ({} ms)",
                        meta.duration_ms as f64 / 1000.0,
                        meta.duration_ms
                    );
                    println!(
                        "Formato:           {}Hz, {} canais, {} bits",
                        meta.sample_rate, meta.channels, audio_out.format.bits_per_sample
                    );
                    println!(
                        "Parâmetros:        length_scale={:.2}, noise_scale={:.2}, noise_w={:.2}",
                        meta.length_scale, meta.noise_scale, meta.noise_w
                    );
                    println!(
                        "Tipo de motor:     {}",
                        if meta.is_neural {
                            "Neural (Piper ONNX)"
                        } else {
                            "Acústico de Diagnóstico (Harmônico F0=220Hz)"
                        }
                    );

                    if play {
                        let target_device_id = if let Some(ref dev_query) = output_device {
                            match find_device(dev_query, AudioDeviceFlow::Output) {
                                Ok(dev) => Some(dev.id),
                                Err(e) => {
                                    eprintln!(
                                        "Falha ao resolver dispositivo de saída '{}': {}",
                                        dev_query, e
                                    );
                                    return;
                                }
                            }
                        } else {
                            None
                        };

                        let playback =
                            AudioPlaybackController::production_with_device(target_device_id);
                        if !playback.is_device_available() {
                            eprintln!(
                                "Aviso: Nenhum dispositivo de áudio físico disponível para reprodução."
                            );
                        } else {
                            println!(
                                "Reproduzindo amostra no dispositivo ({}) ...",
                                playback.sink().name()
                            );
                            if let Err(e) = playback.play(&audio_out) {
                                eprintln!("Erro na reprodução física: {}", e);
                            } else {
                                tokio::time::sleep(std::time::Duration::from_millis(
                                    audio_out.duration_ms.max(100),
                                ))
                                .await;
                                println!("Reprodução concluída.");
                            }
                        }
                    }
                }
                Err(e) => eprintln!("Erro ao gerar amostra: {}", e),
            }
        }
        VoiceCommands::Compare {
            phrase,
            play,
            output_device,
        } => {
            use yuki::voice::lab::{find_phrase, generate_voice_sample};
            use yuki::voice::tts::get_piper_voice_catalog;

            println!("=== Laboratório Yuki Voice — Comparação Multi-Voz ===");
            let phrase_text = find_phrase(&phrase).map(|p| p.text).unwrap_or(&phrase);
            println!("Frase de teste [{}]: \"{}\"\n", phrase, phrase_text);

            let mut candidates: Vec<String> = vec!["acoustic".to_string()];
            let catalog = get_piper_voice_catalog();
            for model in catalog {
                if model.is_installed {
                    candidates.push(model.id);
                }
            }

            println!(
                "Vozes disponíveis para comparação: {} candidato(s)",
                candidates.len()
            );

            let mut results = Vec::new();
            for cand in &candidates {
                println!("Gerando amostra para voz '{}'...", cand);
                match generate_voice_sample(cand, &phrase, None, None, None).await {
                    Ok((audio_out, meta)) => {
                        results.push((cand.clone(), audio_out, meta));
                    }
                    Err(e) => {
                        eprintln!("  Falha na voz '{}': {}", cand, e);
                    }
                }
            }

            println!("\n--- Tabela Comparativa de Amostras ---");
            println!(
                "{:<24} | {:<10} | {:<8} | {:<32}",
                "Voz", "Duração", "Tipo", "Caminho"
            );
            println!("{:-<24}-+-{:-<10}-+-{:-<8}-+-{:-<32}", "", "", "", "");
            for (voice_name, _, meta) in &results {
                println!(
                    "{:<24} | {:<9.2}s | {:<8} | {}",
                    voice_name,
                    meta.duration_ms as f64 / 1000.0,
                    if meta.is_neural {
                        "Neural"
                    } else {
                        "Acústico"
                    },
                    meta.file_path
                );
            }

            if play && !results.is_empty() {
                let target_device_id = if let Some(ref dev_query) = output_device {
                    match find_device(dev_query, AudioDeviceFlow::Output) {
                        Ok(dev) => Some(dev.id),
                        Err(e) => {
                            eprintln!("Falha ao resolver dispositivo de saída: {}", e);
                            return;
                        }
                    }
                } else {
                    None
                };

                let playback = AudioPlaybackController::production_with_device(target_device_id);
                if playback.is_device_available() {
                    println!("\nIniciando audição sequencial comparativa...");
                    for (name, audio_out, _) in &results {
                        println!("▶ Reproduzindo voz '{}'...", name);
                        let _ = playback.play(audio_out);
                        tokio::time::sleep(std::time::Duration::from_millis(
                            audio_out.duration_ms.max(100) + 600,
                        ))
                        .await;
                    }
                    println!("Audição comparativa finalizada.");
                }
            }
        }
        VoiceCommands::Evaluate {
            voice,
            phrase,
            naturalness,
            tonality,
            smoothness,
            expressiveness,
            rhythm,
            approved,
            notes,
        } => {
            use yuki::contracts::identifiers::generate_unique_id;
            use yuki::voice::lab::{save_evaluation, VoiceEvaluation, YukiVocalProfile};

            if naturalness > 100
                || tonality > 100
                || smoothness > 100
                || expressiveness > 100
                || rhythm > 100
            {
                eprintln!("Erro: Todas as pontuações devem estar entre 1 e 100.");
                return;
            }

            let avg = (naturalness as f32
                + tonality as f32
                + smoothness as f32
                + expressiveness as f32
                + rhythm as f32)
                / 5.0;
            let now = yuki::contracts::identifiers::now_utc().to_rfc3339();

            let eval = VoiceEvaluation {
                evaluation_id: generate_unique_id("eval"),
                voice_id: voice.clone(),
                phrase_id: phrase,
                naturalness,
                tonality,
                smoothness,
                expressiveness,
                rhythm,
                average_score: avg,
                approved,
                operator_notes: notes.clone(),
                evaluated_at: now,
            };

            match save_evaluation(eval.clone()) {
                Ok(_) => {
                    println!("=== Avaliação de Voz Registrada com Sucesso ===");
                    println!("ID da Avaliação:   {}", eval.evaluation_id);
                    println!("Voz Avaliada:      {}", eval.voice_id);
                    println!(
                        "Frase Avaliada:    {}",
                        eval.phrase_id.as_deref().unwrap_or("Todas / Geral")
                    );
                    println!("Naturalidade:      {}/100", eval.naturalness);
                    println!("Tonalidade:        {}/100", eval.tonality);
                    println!("Suavidade:         {}/100", eval.smoothness);
                    println!("Expressividade:    {}/100", eval.expressiveness);
                    println!("Ritmo/Cadência:    {}/100", eval.rhythm);
                    println!("Média Geral:       {:.1}/100", eval.average_score);
                    println!(
                        "Aprovada para Uso: {}",
                        if eval.approved {
                            "SIM (Recomendada)"
                        } else {
                            "NÃO (Reprovada / Requer Ajuste)"
                        }
                    );
                    if !notes.is_empty() {
                        println!("Observações:       \"{}\"", notes);
                    }

                    let profile = YukiVocalProfile::default();
                    println!("\n--- Confronto com Metas do Perfil Yuki v0.1 ---");
                    let check_nat = if eval.naturalness >= profile.min_target_naturalness {
                        "✓ OK"
                    } else {
                        "✗ Abaixo da meta (>= 80)"
                    };
                    let check_ton = if eval.tonality >= profile.min_target_tonality {
                        "✓ OK"
                    } else {
                        "✗ Abaixo da meta (>= 75)"
                    };
                    let check_smo = if eval.smoothness >= profile.min_target_smoothness {
                        "✓ OK"
                    } else {
                        "✗ Abaixo da meta (>= 80)"
                    };
                    let check_exp = if eval.expressiveness >= profile.min_target_expressiveness {
                        "✓ OK"
                    } else {
                        "✗ Abaixo da meta (>= 70)"
                    };
                    let check_rhy = if eval.rhythm >= profile.min_target_rhythm {
                        "✓ OK"
                    } else {
                        "✗ Abaixo da meta (>= 75)"
                    };
                    println!("• Naturalidade:    {}", check_nat);
                    println!("• Tonalidade:      {}", check_ton);
                    println!("• Suavidade:       {}", check_smo);
                    println!("• Expressividade:  {}", check_exp);
                    println!("• Ritmo/Cadência:  {}", check_rhy);
                }
                Err(e) => eprintln!("Erro ao persistir avaliação: {}", e),
            }
        }
        VoiceCommands::Profile => {
            use yuki::voice::lab::{compute_evaluations_summary, YukiVocalProfile};

            let profile = YukiVocalProfile::default();
            println!("=== Perfil Vocal da Yuki — Especificação Técnica v0.1 ===");
            println!("Versão da Identidade:   {}", profile.version);
            println!("Identidade Alvo:        {}", profile.target_identity);
            println!("F0 (Freq. Fundamental): {}", profile.target_f0_hz);
            println!("Cadência e Ritmo:       {}", profile.target_cadence);
            println!("\nParâmetros Recomendados de Inferência (Piper):");
            println!(
                "• length_scale:         {:.2} (velocidade serena ~0.88x)",
                profile.recommended_length_scale
            );
            println!(
                "• noise_scale:          {:.2} (redução de ruído e tom mais suave)",
                profile.recommended_noise_scale
            );
            println!(
                "• noise_w:              {:.2} (estabilidade fonêmica)",
                profile.recommended_noise_w
            );

            println!("\nMetas Mínimas de Qualidade (Escala 1-100):");
            println!(
                "• Naturalidade:         >= {}",
                profile.min_target_naturalness
            );
            println!("• Tonalidade:           >= {}", profile.min_target_tonality);
            println!(
                "• Suavidade:            >= {}",
                profile.min_target_smoothness
            );
            println!(
                "• Expressividade:       >= {}",
                profile.min_target_expressiveness
            );
            println!("• Ritmo/Cadência:       >= {}", profile.min_target_rhythm);

            println!("\nMapeamento de Modelos Candidatos:");
            println!("• Candidata A: pt_BR-faber-medium   (Oficial Rhasspy, locutor masculino, alta clareza)");
            println!("• Candidata B: pt_BR-cadu-medium    (Oficial Rhasspy, locutor masculino, coloquial)");
            println!("• Candidata C: pt_BR-edresson-low   (Oficial Rhasspy, locutor masculino, ultraleve)");
            println!("• Diagnóstico: acoustic             (Embarcado, harmônico determinístico F0=220Hz)");

            println!("\nNota de Transparência sobre Gênero e Voz Feminina:");
            println!("O catálogo upstream oficial da Rhasspy para pt-BR possui modelos exclusivamente masculinos.");
            println!("Para utilizar voz feminina neural nativa:");
            println!("1. Posicione modelo comunitário ONNX pt-BR feminino em ./models/piper/");
            println!("2. Ou aguarde treinamento / fine-tuning dedicado com dataset da Yuki (planejado para Marco 9+)");

            let summaries = compute_evaluations_summary();
            println!(
                "\n--- Histórico de Avaliações Registradas ({}) ---",
                summaries.len()
            );
            if summaries.is_empty() {
                println!("  Nenhuma avaliação registrada ainda. Use 'yuki voice evaluate' após testar as amostras.");
            } else {
                for (voice_id, stat) in summaries {
                    println!(
                        "• Voz: {} ({} avaliações, {} aprovações)",
                        voice_id, stat.total_evaluations, stat.approvals_count
                    );
                    println!(
                        "  Média Geral: {:.1}/100 | Nat: {:.1} | Ton: {:.1} | Suav: {:.1} | Exp: {:.1} | Rit: {:.1}",
                        stat.avg_overall, stat.avg_naturalness, stat.avg_tonality, stat.avg_smoothness, stat.avg_expressiveness, stat.avg_rhythm
                    );
                }
            }
        }
    }
}
