//! Unix domain socket server for the passkey daemon.

use std::{path::Path, sync::Arc};

use pk_core::ipc::{Request, Response, decode_request, encode_response};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixListener,
    sync::Mutex,
};
use tracing::{debug, error, info};

/// Start the Unix domain socket server.
pub async fn start_server(
    socket_path: &Path,
    state: Arc<Mutex<crate::state::AgentState>>,
) -> eyre::Result<()> {
    let _ = std::fs::remove_file(socket_path);

    let listener = UnixListener::bind(socket_path)
        .map_err(|e| eyre::eyre!("failed to bind to {}: {}", socket_path.display(), e))?;

    info!("listening on {}", socket_path.display());

    loop {
        match listener.accept().await {
            Ok((stream, _addr)) => {
                let state = Arc::clone(&state);
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(stream, state).await {
                        error!("connection error: {e}");
                    }
                });
            }
            Err(e) => {
                error!("accept error: {e}");
            }
        }
    }
}

async fn handle_connection(
    mut stream: tokio::net::UnixStream,
    state: Arc<Mutex<crate::state::AgentState>>,
) -> eyre::Result<()> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await?;
    let msg_len = u32::from_le_bytes(len_buf) as usize;

    let mut buf = vec![0u8; msg_len];
    stream.read_exact(&mut buf).await?;

    let request = decode_request(&buf).map_err(|e| eyre::eyre!("decode error: {e}"))?;

    debug!("received request: {request:?}");

    let response = dispatch(request, &state).await;

    debug!("sending response: {response:?}");

    let encoded = encode_response(&response).map_err(|e| eyre::eyre!("encode error: {e}"))?;

    let resp_len =
        u32::try_from(encoded.len()).map_err(|e| eyre::eyre!("response too large: {e}"))?;

    stream.write_all(&resp_len.to_le_bytes()).await?;
    stream.write_all(&encoded).await?;
    stream.flush().await?;

    Ok(())
}

async fn dispatch(request: Request, state: &Mutex<crate::state::AgentState>) -> Response {
    match request {
        Request::Status => {
            let s = state.lock().await;
            if s.is_unlocked() {
                let ttl_secs = s.ttl_remaining().map(|d| d.as_secs());
                Response::Status {
                    state: pk_core::ipc::AgentState::Unlocked,
                    ttl_seconds: ttl_secs,
                }
            } else {
                Response::Status { state: pk_core::ipc::AgentState::Locked, ttl_seconds: None }
            }
        }
        Request::Unlock { key } => {
            let mut s = state.lock().await;
            s.unlock(key);
            info!("agent unlocked");
            Response::Ack
        }
        Request::Lock => {
            let mut s = state.lock().await;
            s.lock();
            info!("agent locked");
            Response::Locked
        }
        Request::GetKey => {
            let s = state.lock().await;
            match s.get_key() {
                Some(key) => Response::Key { key },
                None => Response::Error { message: "agent is locked".to_string() },
            }
        }
    }
}
