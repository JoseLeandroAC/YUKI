use clap::{Parser, Subcommand};
use std::sync::Arc;
use yuki::contracts::input::UserInput;
use yuki::contracts::output::ResultStatus;
use yuki::core::yuki_core::YukiCore;
use yuki::models::{resolve_model_provider_from_env, HealthStatus};
use yuki::persistence::sqlite::SqliteAuditStore;

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
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

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
            println!(
                "Selected Model Provider: {} (model: {}, version: {})",
                model_meta.provider_name, model_meta.model_name, model_meta.version
            );
            println!(
                "Model Provider Health: {}",
                match model_health {
                    HealthStatus::Healthy => "OK".to_string(),
                    HealthStatus::Degraded(reason) => format!("DEGRADED ({})", reason),
                    HealthStatus::Unhealthy(reason) => format!("FAIL ({})", reason),
                }
            );
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
    }
}
