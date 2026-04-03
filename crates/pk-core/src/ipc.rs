//! IPC protocol for client-daemon communication over Unix domain sockets.
//!
//! Uses bincode serialization with length-prefixed framing.

use std::{path::PathBuf, time::Duration};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
};

/// Errors that can occur during IPC operations.
#[derive(Debug, Error)]
pub enum IpcError {
    /// Failed to connect to the daemon socket.
    #[error("connection failed: {0}")]
    ConnectionFailed(String),
    /// Failed to encode a message.
    #[error("encoding error: {0}")]
    EncodingError(String),
    /// Failed to decode a message.
    #[error("decoding error: {0}")]
    DecodingError(String),
    /// An I/O error occurred during communication.
    #[error("io error: {0}")]
    IoError(String),
    /// The daemon returned an error response.
    #[error("daemon returned error: {0}")]
    DaemonError(String),
    /// The operation timed out.
    #[error("operation timed out")]
    Timeout,
}

/// Agent state as reported by the daemon.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum AgentState {
    /// Agent is locked; no key in memory.
    Locked,
    /// Agent is unlocked; key available in memory.
    Unlocked,
}

/// Request sent from CLI to daemon.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Request {
    /// Query the current daemon state.
    Status,
    /// Unlock the daemon with a decrypted age key.
    Unlock {
        /// The decrypted age key to store in the daemon.
        key: String,
    },
    /// Force the daemon to lock immediately.
    Lock,
    /// Request the age key for command execution.
    GetKey,
}

/// Response sent from daemon to CLI.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Response {
    /// Current state with optional TTL in seconds.
    Status {
        /// The current agent state.
        state: AgentState,
        /// Remaining TTL in seconds, if unlocked.
        ttl_seconds: Option<u64>,
    },
    /// Acknowledgment of a successful operation.
    Ack,
    /// Confirmation that the agent is now locked.
    Locked,
    /// The age private key (only sent if unlocked).
    Key {
        /// The decrypted age private key.
        key: String,
    },
    /// Error response with message.
    Error {
        /// The error message.
        message: String,
    },
}

/// Client for communicating with the passkey daemon.
#[derive(Debug)]
pub struct AgentClient {
    socket_path: PathBuf,
    timeout: Duration,
}

impl AgentClient {
    /// Create a new client connecting to the default socket path.
    #[must_use]
    pub fn new() -> Self {
        Self { socket_path: socket_path(), timeout: Duration::from_secs(5) }
    }

    /// Create a client with a custom timeout.
    #[must_use]
    pub const fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Query the daemon's current state.
    pub async fn status(&self) -> Result<(AgentState, Option<u64>), IpcError> {
        let response = self.send_request(Request::Status).await?;
        match response {
            Response::Status { state, ttl_seconds } => Ok((state, ttl_seconds)),
            Response::Error { message } => Err(IpcError::DaemonError(message)),
            _ => Err(IpcError::DecodingError("unexpected response type".to_string())),
        }
    }

    /// Send an unlock request to the daemon.
    pub async fn unlock(&self, key: String) -> Result<(), IpcError> {
        let response = self.send_request(Request::Unlock { key }).await?;
        match response {
            Response::Ack => Ok(()),
            Response::Error { message } => Err(IpcError::DaemonError(message)),
            _ => Err(IpcError::DecodingError("unexpected response type".to_string())),
        }
    }

    /// Send a lock request to the daemon.
    pub async fn lock(&self) -> Result<(), IpcError> {
        let response = self.send_request(Request::Lock).await?;
        match response {
            Response::Locked | Response::Ack => Ok(()),
            Response::Error { message } => Err(IpcError::DaemonError(message)),
            _ => Err(IpcError::DecodingError("unexpected response type".to_string())),
        }
    }

    /// Request the age key from the daemon.
    pub async fn get_key(&self) -> Result<String, IpcError> {
        let response = self.send_request(Request::GetKey).await?;
        match response {
            Response::Key { key } => Ok(key),
            Response::Error { message } => Err(IpcError::DaemonError(message)),
            _ => Err(IpcError::DecodingError("unexpected response type".to_string())),
        }
    }

    /// Send a request and receive a response over the Unix socket.
    async fn send_request(&self, request: Request) -> Result<Response, IpcError> {
        let stream = UnixStream::connect(&self.socket_path)
            .await
            .map_err(|e| IpcError::ConnectionFailed(e.to_string()))?;

        let (mut reader, mut writer) = stream.into_split();

        let encoded = encode_request(&request)?;
        let len =
            u32::try_from(encoded.len()).map_err(|e| IpcError::EncodingError(e.to_string()))?;
        writer.write_all(&len.to_le_bytes()).await.map_err(|e| IpcError::IoError(e.to_string()))?;
        writer.write_all(&encoded).await.map_err(|e| IpcError::IoError(e.to_string()))?;
        writer.flush().await.map_err(|e| IpcError::IoError(e.to_string()))?;

        let mut len_buf = [0u8; 4];
        reader.read_exact(&mut len_buf).await.map_err(|e| IpcError::IoError(e.to_string()))?;
        let response_len = u32::from_le_bytes(len_buf) as usize;

        let mut response_buf = vec![0u8; response_len];
        reader.read_exact(&mut response_buf).await.map_err(|e| IpcError::IoError(e.to_string()))?;

        decode_response(&response_buf)
    }
}

impl Default for AgentClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Encode a request to bytes using bincode.
pub fn encode_request(request: &Request) -> Result<Vec<u8>, IpcError> {
    bincode::serialize(request).map_err(|e| IpcError::EncodingError(e.to_string()))
}

/// Decode bytes to a request using bincode.
pub fn decode_request(bytes: &[u8]) -> Result<Request, IpcError> {
    bincode::deserialize(bytes).map_err(|e| IpcError::DecodingError(e.to_string()))
}

/// Encode a response to bytes using bincode.
pub fn encode_response(response: &Response) -> Result<Vec<u8>, IpcError> {
    bincode::serialize(response).map_err(|e| IpcError::EncodingError(e.to_string()))
}

/// Decode bytes to a response using bincode.
pub fn decode_response(bytes: &[u8]) -> Result<Response, IpcError> {
    bincode::deserialize(bytes).map_err(|e| IpcError::DecodingError(e.to_string()))
}

/// Get the default socket path for the daemon.
#[must_use]
pub fn socket_path() -> PathBuf {
    let mut path = dirs::cache_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("passkey-agent");
    path.push("agent.sock");
    path
}

/// Ensure the socket directory exists, creating it if necessary.
pub fn ensure_socket_dir() -> Result<(), IpcError> {
    let path = socket_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| IpcError::IoError(e.to_string()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    #[test]
    fn request_status_serializes_correctly() {
        let request = Request::Status;
        let encoded = encode_request(&request).unwrap();
        let decoded = decode_request(&encoded).unwrap();
        assert!(matches!(decoded, Request::Status));
    }

    #[test]
    fn request_unlock_serializes_correctly() {
        let request = Request::Unlock { key: "test-key".to_string() };
        let encoded = encode_request(&request).unwrap();
        let decoded = decode_request(&encoded).unwrap();
        assert!(matches!(decoded, Request::Unlock { key } if key == "test-key"));
    }

    #[test]
    fn response_status_serializes_correctly() {
        let response = Response::Status { state: AgentState::Unlocked, ttl_seconds: Some(900) };
        let encoded = encode_response(&response).unwrap();
        let decoded = decode_response(&encoded).unwrap();
        assert!(matches!(
            decoded,
            Response::Status { state: AgentState::Unlocked, ttl_seconds: Some(900) }
        ));
    }

    #[test]
    fn response_key_serializes_correctly() {
        let response = Response::Key { key: "secret-age-key".to_string() };
        let encoded = encode_response(&response).unwrap();
        let decoded = decode_response(&encoded).unwrap();
        assert!(matches!(decoded, Response::Key { key } if key == "secret-age-key"));
    }

    #[test]
    fn response_error_serializes_correctly() {
        let response = Response::Error { message: "not unlocked".to_string() };
        let encoded = encode_response(&response).unwrap();
        let decoded = decode_response(&encoded).unwrap();
        assert!(matches!(decoded, Response::Error { message } if message == "not unlocked"));
    }

    proptest! {
        #[test]
        fn request_roundtrip_arbitrary_string(
            key in "[a-zA-Z0-9_\\-]{1,64}",
        ) {
            let request = Request::Unlock { key: key.clone() };
            let encoded = encode_request(&request).unwrap();
            let decoded = decode_request(&encoded).unwrap();
            if let Request::Unlock { key: decoded_key } = decoded {
                prop_assert_eq!(key, decoded_key);
            } else {
                prop_assert!(false, "wrong variant");
            }
        }
    }
}
