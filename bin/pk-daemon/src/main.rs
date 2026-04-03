//! Background daemon for passkey-agent.
//!
//! Listens on a Unix domain socket and manages the in-memory
//! age private key with TTL-based auto-locking.

#![allow(clippy::print_stdout, clippy::print_stderr, unreachable_pub)]

mod server;
mod state;

use std::sync::Arc;

use clap::Parser;
use tokio::sync::Mutex;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(name = "pk-daemon", version, about = "Background daemon for passkey-agent")]
struct Args {
    /// Run in foreground mode (do not daemonize)
    #[arg(long)]
    foreground: bool,
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let args = Args::parse();

    tracing_subscriber::fmt().with_env_filter(EnvFilter::from_default_env()).init();

    if args.foreground {
        tracing::info!("Running in foreground mode");
    }

    let state = Arc::new(Mutex::new(state::AgentState::new()));

    pk_core::ipc::ensure_socket_dir()
        .map_err(|e| eyre::eyre!("failed to ensure socket dir: {e}"))?;
    let socket_path = pk_core::ipc::socket_path();

    let _ = std::fs::remove_file(&socket_path);

    let state_clone = Arc::clone(&state);
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            let mut s = state_clone.lock().await;
            if s.is_unlocked() && s.ttl_remaining().is_none() {
                s.lock();
                tracing::info!("Agent auto-locked due to TTL expiry");
            }
        }
    });

    let socket_path_clone = socket_path.clone();
    tokio::spawn(async move {
        let mut sigterm =
            match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!("Failed to create SIGTERM handler: {e}");
                    return;
                }
            };
        let mut sigint =
            match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt()) {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!("Failed to create SIGINT handler: {e}");
                    return;
                }
            };
        tokio::select! {
            _ = sigterm.recv() => {},
            _ = sigint.recv() => {},
        }
        tracing::info!("Shutting down daemon");
        let _ = std::fs::remove_file(&socket_path_clone);
        std::process::exit(0);
    });

    server::start_server(&socket_path, state).await
}
