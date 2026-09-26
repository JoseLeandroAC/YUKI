use clap::{Parser, Subcommand};
use yuki::contracts::input::UserInput;
use yuki::contracts::output::ResultStatus;
use yuki::core::yuki_core::YukiCore;

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

fn main() {
    let cli = Cli::parse();
    let core = YukiCore::new();

    match cli.command {
        Some(Commands::Health) => {
            let health = core.health();
            println!("Yuki");
            if health.is_all_ok() {
                println!("Status: OK\n");
            } else {
                println!("Status: DEGRADED\n");
            }

            println!("Core: {}", if health.core_ok { "OK" } else { "FAIL" });
            println!("Context: {}", if health.context_ok { "OK" } else { "FAIL" });
            println!("Model: {}", if health.model_ok { "OK" } else { "FAIL" });
            println!(
                "Capability Registry: {}",
                if health.registry_ok { "OK" } else { "FAIL" }
            );
            println!(
                "Security: {}",
                if health.security_ok { "OK" } else { "FAIL" }
            );
            println!(
                "Execution: {}",
                if health.execution_ok { "OK" } else { "FAIL" }
            );
            println!(
                "Verification: {}",
                if health.verification_ok { "OK" } else { "FAIL" }
            );
            println!("Audit: {}", if health.audit_ok { "OK" } else { "FAIL" });
        }
        None => {
            if cli.prompt.is_empty() {
                println!("Yuki Foundation v0.1");
                println!("Uso: yuki \"<sua mensagem>\" ou yuki health");
                return;
            }

            let input_text = cli.prompt.join(" ");
            let user_input = UserInput::new(input_text);

            match core.process_input(user_input) {
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
    }
}
