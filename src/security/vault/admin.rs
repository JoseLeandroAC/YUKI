use crate::security::credentials::SecretMaterial;
use crate::security::vault::store::{CredentialVault, VaultConfig, VaultError};
use clap::Subcommand;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Subcomandos administrativos para gerenciamento seguro do Credential Vault.
#[derive(Subcommand, Debug, Clone)]
pub enum VaultCommand {
    /// Inicializa um novo cofre de credenciais
    Init {
        /// Forçar recriação do cofre caso já exista (requer confirmação explícita 'RECRIAR')
        #[arg(long)]
        force: bool,

        /// Caminho opcional do arquivo do cofre
        #[arg(long)]
        path: Option<PathBuf>,
    },

    /// Armazena uma nova credencial cifrada no cofre
    Store {
        /// Alias canônico da credencial (ex: brave_api_key)
        alias: String,

        /// Provedor associado (ex: brave, langsearch, gemini)
        #[arg(long, default_value = "custom")]
        provider: String,

        /// Descrição descritiva opcional
        #[arg(long)]
        description: Option<String>,

        /// Caminho opcional do arquivo do cofre
        #[arg(long)]
        path: Option<PathBuf>,
    },

    /// Substitui o segredo de uma credencial existente
    Replace {
        /// Alias canônico da credencial
        alias: String,

        /// Caminho opcional do arquivo do cofre
        #[arg(long)]
        path: Option<PathBuf>,
    },

    /// Lista os metadados das credenciais cadastradas sem exibir segredos
    List {
        /// Caminho opcional do arquivo do cofre
        #[arg(long)]
        path: Option<PathBuf>,
    },

    /// Exibe os detalhes e status de uma credencial específica
    Status {
        /// Alias canônico da credencial
        alias: String,

        /// Caminho opcional do arquivo do cofre
        #[arg(long)]
        path: Option<PathBuf>,
    },

    /// Marca uma credencial como revogada
    Revoke {
        /// Alias canônico da credencial
        alias: String,

        /// Caminho opcional do arquivo do cofre
        #[arg(long)]
        path: Option<PathBuf>,
    },

    /// Exclui permanentemente uma credencial do cofre
    Delete {
        /// Alias canônico da credencial
        alias: String,

        /// Caminho opcional do arquivo do cofre
        #[arg(long)]
        path: Option<PathBuf>,
    },
}

impl VaultCommand {
    /// Executa o comando administrativo do cofre no terminal.
    pub fn execute(self) -> Result<(), Box<dyn std::error::Error>> {
        match self {
            VaultCommand::Init { force, path } => {
                let vault_path = resolve_path(path);
                println!("=== Inicialização do Yuki Credential Vault ===");
                println!("Arquivo de destino: {}", vault_path.display());

                if vault_path.exists() {
                    if !force {
                        eprintln!(
                            "Erro: O arquivo do cofre já existe em '{}'. Para recriá-lo do zero apagando todas as credenciais existentes, utilize a flag '--force' e confirme a operação digitando 'RECRIAR'.",
                            vault_path.display()
                        );
                        return Err(
                            "Arquivo do cofre já existe. Operação cancelada por segurança.".into(),
                        );
                    }

                    println!("ATENÇÃO CRÍTICA: A flag --force foi informada.");
                    println!("Recriar o cofre DESTRUIRÁ permanentemente todas as credenciais e segredos já existentes!");
                    print!(
                        "Para confirmar a destruição e recriação, digite exatamente 'RECRIAR': "
                    );
                    io::stdout().flush()?;
                    let mut confirmation = String::new();
                    io::stdin().read_line(&mut confirmation)?;
                    if confirmation.trim() != "RECRIAR" {
                        println!("Confirmação inválida. Operação de reinicialização abortada.");
                        return Ok(());
                    }

                    let _ = std::fs::remove_file(&vault_path);
                    let lock_path = vault_path.with_file_name(format!(
                        "{}.lock",
                        vault_path
                            .file_name()
                            .and_then(|f| f.to_str())
                            .unwrap_or("vault")
                    ));
                    let _ = std::fs::remove_file(&lock_path);
                }

                let pass1 = prompt_masked("Defina a senha mestra do cofre: ")?;
                if pass1.trim().is_empty() {
                    eprintln!("Erro: A senha mestra não pode ser vazia.");
                    return Ok(());
                }

                let pass2 = prompt_masked("Confirme a senha mestra: ")?;
                if pass1 != pass2 {
                    eprintln!("Erro: As senhas digitadas não coincidem.");
                    return Ok(());
                }

                CredentialVault::create_new(&vault_path, SecretMaterial::new(&pass1))?;
                println!(
                    "Cofre inicializado com sucesso em: {}",
                    vault_path.display()
                );
            }

            VaultCommand::Store {
                alias,
                provider,
                description,
                path,
            } => {
                let vault_path = resolve_path(path);
                let vault = open_vault_interactive(&vault_path)?;

                if vault.get_metadata(&alias).is_ok() {
                    eprintln!(
                        "Erro: A credencial '{}' já existe no cofre. Utilize 'yuki vault replace {}' para atualizá-la.",
                        alias, alias
                    );
                    return Ok(());
                }

                let secret1 = prompt_masked(&format!("Digite o segredo para '{}': ", alias))?;
                if secret1.trim().is_empty() {
                    eprintln!("Erro: O segredo não pode ser vazio.");
                    return Ok(());
                }

                let secret2 = prompt_masked("Confirme o segredo: ")?;
                if secret1 != secret2 {
                    eprintln!("Erro: Os segredos digitados não coincidem.");
                    return Ok(());
                }

                let meta = vault.store(&alias, &secret1, &provider, description)?;
                println!(
                    "Credencial '{}' armazenada com sucesso (Provedor: {}, Status: {:?}).",
                    meta.alias, meta.provider, meta.status
                );
            }

            VaultCommand::Replace { alias, path } => {
                let vault_path = resolve_path(path);
                let vault = open_vault_interactive(&vault_path)?;

                if let Err(VaultError::NotFound(_)) = vault.get_metadata(&alias) {
                    eprintln!("Erro: Credencial '{}' não encontrada no cofre.", alias);
                    return Ok(());
                }

                if !confirm_action(&format!(
                    "Deseja realmente substituir o segredo da credencial '{}'? [s/N]: ",
                    alias
                ))? {
                    println!("Operação cancelada.");
                    return Ok(());
                }

                let secret1 = prompt_masked(&format!("Digite o novo segredo para '{}': ", alias))?;
                if secret1.trim().is_empty() {
                    eprintln!("Erro: O segredo não pode ser vazio.");
                    return Ok(());
                }

                let secret2 = prompt_masked("Confirme o novo segredo: ")?;
                if secret1 != secret2 {
                    eprintln!("Erro: Os segredos digitados não coincidem.");
                    return Ok(());
                }

                let meta = vault.replace(&alias, &secret1, None)?;
                println!(
                    "Credencial '{}' atualizada com sucesso (Provedor: {}, Status: {:?}).",
                    meta.alias, meta.provider, meta.status
                );
            }

            VaultCommand::List { path } => {
                let vault_path = resolve_path(path);
                let vault = open_vault_interactive(&vault_path)?;
                let list = vault.list_metadata()?;

                println!("=== Credenciais no Cofre ({}) ===", vault_path.display());
                if list.is_empty() {
                    println!("Nenhuma credencial cadastrada.");
                    return Ok(());
                }

                println!(
                    "{:<24} {:<15} {:<10} {:<24}",
                    "ALIAS", "PROVEDOR", "STATUS", "ATUALIZADO EM"
                );
                println!("{:-<75}", "");
                for item in &list {
                    println!(
                        "{:<24} {:<15} {:<10} {:<24}",
                        item.alias,
                        item.provider,
                        format!("{:?}", item.status),
                        item.updated_at.to_rfc3339()
                    );
                }
                println!("{:-<75}", "");
                println!("Total: {} credencial(is). Segredos protegidos.", list.len());
            }

            VaultCommand::Status { alias, path } => {
                let vault_path = resolve_path(path);
                let vault = open_vault_interactive(&vault_path)?;
                let meta = vault.get_metadata(&alias)?;

                println!("=== Detalhes da Credencial: {} ===", meta.alias);
                println!("Provedor:       {}", meta.provider);
                println!("Status:         {:?}", meta.status);
                println!("Criado em:      {}", meta.created_at.to_rfc3339());
                println!("Atualizado em:  {}", meta.updated_at.to_rfc3339());
                if let Some(desc) = meta.description {
                    println!("Descrição:      {}", desc);
                }
            }

            VaultCommand::Revoke { alias, path } => {
                let vault_path = resolve_path(path);
                let vault = open_vault_interactive(&vault_path)?;

                if !confirm_action(&format!(
                    "Deseja realmente revogar a credencial '{}'? [s/N]: ",
                    alias
                ))? {
                    println!("Operação cancelada.");
                    return Ok(());
                }

                let meta = vault.revoke(&alias)?;
                println!(
                    "Credencial '{}' revogada com sucesso (Novo status: {:?}).",
                    meta.alias, meta.status
                );
            }

            VaultCommand::Delete { alias, path } => {
                let vault_path = resolve_path(path);
                let vault = open_vault_interactive(&vault_path)?;

                if !confirm_action(&format!(
                    "ATENÇÃO: Deseja realmente EXCLUIR permanentemente '{}'? Esta ação é irreversível. [s/N]: ",
                    alias
                ))? {
                    println!("Operação cancelada.");
                    return Ok(());
                }

                vault.delete(&alias)?;
                println!("Credencial '{}' excluída permanentemente do cofre.", alias);
            }
        }

        Ok(())
    }
}

fn resolve_path(override_path: Option<PathBuf>) -> PathBuf {
    override_path.unwrap_or_else(|| VaultConfig::default().vault_path)
}

fn prompt_masked(prompt: &str) -> io::Result<String> {
    print!("{}", prompt);
    io::stdout().flush()?;
    rpassword::read_password()
}

fn confirm_action(prompt: &str) -> io::Result<bool> {
    print!("{}", prompt);
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let trimmed = input.trim();
    Ok(trimmed.eq_ignore_ascii_case("s") || trimmed.eq_ignore_ascii_case("y"))
}

fn open_vault_interactive(path: &Path) -> Result<CredentialVault, Box<dyn std::error::Error>> {
    if !path.exists() {
        return Err(format!(
            "Arquivo do cofre não encontrado em: {}. Inicialize com 'yuki vault init'.",
            path.display()
        )
        .into());
    }

    let pass = prompt_masked("Digite a senha mestra do cofre: ")?;
    if pass.trim().is_empty() {
        return Err("Senha mestra não pode ser vazia.".into());
    }

    match CredentialVault::open_or_create(path, SecretMaterial::new(&pass)) {
        Ok(vault) => Ok(vault),
        Err(VaultError::InvalidPassphrase) => {
            Err("Senha mestra incorreta. Acesso negado ao cofre.".into())
        }
        Err(e) => Err(format!("Falha ao abrir o cofre: {}", e).into()),
    }
}
