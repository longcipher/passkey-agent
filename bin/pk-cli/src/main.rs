//! CLI frontend for passkey-agent.
//!
//! Handles user interaction, FIDO2 authentication, and
//! process replacement for sops execution.

#![allow(clippy::print_stdout, clippy::print_stderr)]

mod commands;
mod error;
mod key;

use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "pk", version, about = "Passkey-based credential agent for SOPS")]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, clap::Subcommand)]
enum Commands {
    /// Initialize the agent with a new age key and passkey
    Init,
    /// Show the current daemon status
    Status,
    /// Lock the daemon immediately
    Lock,
    /// Run a command with the age key injected
    Run {
        /// Command and arguments to run
        #[arg(last = true)]
        args: Vec<String>,
    },
    /// Edit an encrypted file (like sops edit)
    Edit {
        /// File to edit
        file: String,
    },
    /// Encrypt a file (like sops encrypt)
    Encrypt {
        /// File to encrypt
        file: String,
        /// Output file (defaults to stdout)
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Decrypt a file (like sops decrypt)
    Decrypt {
        /// File to decrypt
        file: String,
        /// Output file (defaults to stdout)
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Display the project's .sops.yaml configuration
    ListConfig,
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let args = Args::parse();

    match args.command {
        Some(Commands::Init) => commands::init::init_command().await,
        Some(Commands::Status) => commands::status::status_command().await,
        Some(Commands::Lock) => commands::lock::lock_command().await,
        Some(Commands::Run { args: cmd_args }) => commands::run::run_command(&cmd_args).await,
        Some(Commands::Edit { file }) => commands::edit::edit_command(&file).await,
        Some(Commands::Encrypt { file, output }) => {
            commands::encrypt::encrypt_command(&file, output.as_deref()).await
        }
        Some(Commands::Decrypt { file, output }) => {
            commands::decrypt::decrypt_command(&file, output.as_deref()).await
        }
        Some(Commands::ListConfig) => commands::list_config::list_config_command(),
        None => {
            let _ = Args::parse_from(["pk", "--help"]);
            Ok(())
        }
    }
}
