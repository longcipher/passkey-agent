pub(crate) async fn status_command() -> eyre::Result<()> {
    let client = pk_core::ipc::AgentClient::new();
    match client.status().await {
        Ok((state, ttl)) => match state {
            pk_core::ipc::AgentState::Unlocked => {
                if let Some(secs) = ttl {
                    let mins = secs / 60;
                    let secs_remain = secs % 60;
                    println!("Unlocked (expires in {mins}m {secs_remain}s)");
                } else {
                    println!("Unlocked");
                }
            }
            pk_core::ipc::AgentState::Locked => {
                println!("Locked");
            }
        },
        Err(pk_core::ipc::IpcError::ConnectionFailed(_)) => {
            println!("Daemon not running. Run 'pk run' to start it.");
        }
        Err(e) => return Err(e.into()),
    }
    Ok(())
}
