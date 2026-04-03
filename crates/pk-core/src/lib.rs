//! Core library for passkey-agent.
//!
//! Provides cryptographic primitives, FIDO2 passkey interaction,
//! IPC protocol definitions, and configuration management.

pub mod config;
pub mod crypto;
pub mod fido;
pub mod ipc;
pub mod sops_config;

/// Re-exports of all public items for convenient importing.
/// Re-exports of all public items for convenient importing.
pub mod prelude {
    pub use crate::{
        config::{AgentConfig, ConfigError, PasskeyRoute, PasswordRoute},
        crypto::{
            CryptoError, decrypt_dek, decrypt_with_dek, derive_kek_password, encrypt_dek,
            encrypt_with_dek, generate_dek,
        },
        fido::{
            CredentialId, FidoError, authenticate_passkey, generate_passkey_credential,
            generate_salt,
        },
        ipc::{AgentClient, AgentState, IpcError, Request, Response},
        sops_config::{CreationRule, SopsConfig, SopsConfigError},
    };
}
