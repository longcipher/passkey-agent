use pk_core::ipc::{AgentClient, IpcError};

pub(crate) async fn lock_command() -> eyre::Result<()> {
    let client = AgentClient::new();
    match client.lock().await {
        Ok(()) => {
            println!("Agent locked. Memory zeroized.");
            Ok(())
        }
        Err(IpcError::ConnectionFailed(_)) => {
            println!("Daemon not running.");
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}
