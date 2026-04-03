//! Configuration management for passkey-agent.
//!
//! Handles loading and saving of agent configuration to
//! `~/.config/passkey-agent/config.json`.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::fido::CredentialId;

/// Errors that can occur during configuration operations.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// Failed to read config file from disk.
    #[error("failed to read config file: {0}")]
    ReadFailed(String),
    /// Failed to write config file to disk.
    #[error("failed to write config file: {0}")]
    WriteFailed(String),
    /// Failed to parse config content.
    #[error("failed to parse config: {0}")]
    ParseFailed(String),
    /// Config directory not found.
    #[error("config directory not found")]
    DirectoryNotFound,
}

/// Route configuration for Passkey-based unlock.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PasskeyRoute {
    /// The FIDO2 credential identifier.
    pub credential_id: CredentialIdWrapper,
    /// Salt used for hmac-secret extension.
    #[serde(with = "hex_bytes")]
    pub salt: [u8; 32],
    /// Encrypted DEK (XChaCha20-Poly1305 ciphertext).
    pub encrypted_dek: Vec<u8>,
}

/// Route configuration for password-based unlock.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PasswordRoute {
    /// Salt used for Argon2id key derivation.
    #[serde(with = "hex_bytes")]
    pub salt: [u8; 32],
    /// Encrypted DEK (XChaCha20-Poly1305 ciphertext).
    pub encrypted_dek: Vec<u8>,
}

/// Wrapper for CredentialId to enable serde derives.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CredentialIdWrapper(pub Vec<u8>);

impl Serialize for CredentialIdWrapper {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(&self.0)
    }
}

impl<'de> Deserialize<'de> for CredentialIdWrapper {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let bytes = Vec::<u8>::deserialize(deserializer)?;
        Ok(Self(bytes))
    }
}

impl From<CredentialId> for CredentialIdWrapper {
    fn from(id: CredentialId) -> Self {
        Self(id.0)
    }
}

impl From<CredentialIdWrapper> for CredentialId {
    fn from(wrapper: CredentialIdWrapper) -> Self {
        Self(wrapper.0)
    }
}

/// Full agent configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Passkey-based unlock route (primary).
    pub passkey_route: Option<PasskeyRoute>,
    /// Password-based unlock route (fallback).
    pub password_route: Option<PasswordRoute>,
    /// Encrypted age public key (for reference).
    pub encrypted_age_public_key: String,
    /// Encrypted age private key (XChaCha20-Poly1305 ciphertext).
    pub encrypted_age_private_key: Vec<u8>,
}

impl AgentConfig {
    /// Load configuration from the default path.
    pub fn load() -> Result<Self, ConfigError> {
        let path = config_path();
        Self::load_from_path(&path)
    }

    /// Load configuration from a specific path.
    pub fn load_from_path(path: &Path) -> Result<Self, ConfigError> {
        let content =
            std::fs::read_to_string(path).map_err(|e| ConfigError::ReadFailed(e.to_string()))?;
        serde_json::from_str(&content).map_err(|e| ConfigError::ParseFailed(e.to_string()))
    }

    /// Save configuration to the default path.
    pub fn save(&self) -> Result<(), ConfigError> {
        let path = config_path();
        self.save_to_path(&path)
    }

    /// Save configuration to a specific path.
    pub fn save_to_path(&self, path: &Path) -> Result<(), ConfigError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| ConfigError::WriteFailed(e.to_string()))?;
        }
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| ConfigError::WriteFailed(e.to_string()))?;
        std::fs::write(path, content).map_err(|e| ConfigError::WriteFailed(e.to_string()))?;

        // Set restrictive permissions on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(path)
                .map_err(|e| ConfigError::WriteFailed(e.to_string()))?
                .permissions();
            perms.set_mode(0o600);
            std::fs::set_permissions(path, perms)
                .map_err(|e| ConfigError::WriteFailed(e.to_string()))?;
        }

        Ok(())
    }
}

/// Get the default configuration file path.
#[must_use]
pub fn config_path() -> PathBuf {
    let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("passkey-agent");
    path.push("config.json");
    path
}

/// Hex encoding/decoding for serde.
mod hex_bytes {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub(super) fn serialize<S: Serializer>(data: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
        let hex_str = hex::encode(data);
        hex_str.serialize(serializer)
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<[u8; 32], D::Error> {
        let hex_str = String::deserialize(deserializer)?;
        let bytes = hex::decode(&hex_str).map_err(serde::de::Error::custom)?;
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(arr)
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::NamedTempFile;

    use super::*;

    fn make_test_config() -> AgentConfig {
        AgentConfig {
            passkey_route: Some(PasskeyRoute {
                credential_id: CredentialIdWrapper(vec![1, 2, 3, 4]),
                salt: [0xAA; 32],
                encrypted_dek: vec![0xBB; 48],
            }),
            password_route: Some(PasswordRoute { salt: [0xCC; 32], encrypted_dek: vec![0xDD; 48] }),
            encrypted_age_public_key: "age1abc123".to_string(),
            encrypted_age_private_key: vec![0xEE; 64],
        }
    }

    #[test]
    fn config_roundtrip_serialization() {
        let config = make_test_config();
        let json = serde_json::to_string(&config).unwrap();
        let loaded: AgentConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.encrypted_age_public_key, loaded.encrypted_age_public_key);
        assert_eq!(config.encrypted_age_private_key, loaded.encrypted_age_private_key);
    }

    #[test]
    fn config_save_and_load() {
        let config = make_test_config();
        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_path_buf();

        config.save_to_path(&path).unwrap();
        let loaded = AgentConfig::load_from_path(&path).unwrap();

        assert_eq!(config.encrypted_age_public_key, loaded.encrypted_age_public_key);
        assert_eq!(
            config.passkey_route.as_ref().unwrap().salt,
            loaded.passkey_route.as_ref().unwrap().salt
        );
    }

    #[test]
    fn load_nonexistent_file_returns_error() {
        let result = AgentConfig::load_from_path(Path::new("/nonexistent/path/config.json"));
        assert!(result.is_err());
    }

    #[test]
    fn load_invalid_json_returns_error() {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "not valid json").unwrap();
        let result = AgentConfig::load_from_path(file.path());
        assert!(result.is_err());
    }
}
