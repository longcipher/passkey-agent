use thiserror::Error;

#[expect(dead_code)]
#[derive(Debug, Error)]
pub(crate) enum CliError {
    #[error("daemon error: {0}")]
    Daemon(#[from] pk_core::ipc::IpcError),

    #[error("config error: {0}")]
    Config(#[from] pk_core::config::ConfigError),

    #[error("crypto error: {0}")]
    Crypto(#[from] pk_core::crypto::CryptoError),

    #[error("FIDO2 error: {0}")]
    Fido(#[from] pk_core::fido::FidoError),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("operation cancelled")]
    Cancelled,

    #[error("{0}")]
    Other(String),
}
