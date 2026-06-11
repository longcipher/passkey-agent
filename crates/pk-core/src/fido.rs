//! FIDO2 Passkey interaction via the authenticator crate.
//!
//! Implements hmac-secret extension for KEK derivation.

use thiserror::Error;

/// Opaque credential identifier.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CredentialId(pub Vec<u8>);

/// Errors that can occur during FIDO2 operations.
#[derive(Debug, Error)]
pub enum FidoError {
    /// The platform authenticator is not available.
    #[error("authenticator not available: {0}")]
    AuthenticatorUnavailable(String),
    /// A protocol-level error occurred during communication.
    #[error("protocol error: {0}")]
    ProtocolError(String),
    /// The requested credential was not found.
    #[error("credential not found")]
    CredentialNotFound,
    /// The operation was cancelled by the user.
    #[error("operation cancelled by user")]
    UserCancelled,
    /// Random number generation failed.
    #[error("random generation failed")]
    RandomGenerationFailed,
}

/// Generate a 32-byte random salt for hmac-secret.
pub fn generate_salt() -> Result<[u8; 32], FidoError> {
    let mut salt = [0u8; 32];
    getrandom::fill(&mut salt).map_err(|_| FidoError::RandomGenerationFailed)?;
    Ok(salt)
}

/// Generate a passkey credential with hmac-secret extension.
///
/// This invokes the platform authenticator to create a new credential.
/// Returns the credential ID and the KEK derived from hmac-secret.
pub fn generate_passkey_credential(
    _salt: &[u8; 32],
) -> Result<(CredentialId, [u8; 32]), FidoError> {
    // TODO: Implement with authenticator crate
    // This requires platform-specific FIDO2 CTAP2 interaction.
    // The authenticator crate from Mozilla provides the necessary primitives.
    Err(FidoError::AuthenticatorUnavailable("FIDO2 authenticator not yet implemented".to_string()))
}

/// Authenticate with an existing passkey credential via hmac-secret.
///
/// Returns the KEK derived from the hmac-secret extension.
pub fn authenticate_passkey(
    _credential_id: &CredentialId,
    _salt: &[u8; 32],
) -> Result<[u8; 32], FidoError> {
    // TODO: Implement with authenticator crate
    Err(FidoError::AuthenticatorUnavailable("FIDO2 authenticator not yet implemented".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_salt_produces_32_bytes() {
        let salt = generate_salt().expect("OS RNG should not fail");
        assert_eq!(salt.len(), 32);
    }

    #[test]
    fn generate_salt_produces_different_values() {
        let salt1 = generate_salt().expect("OS RNG should not fail");
        let salt2 = generate_salt().expect("OS RNG should not fail");
        assert_ne!(salt1, salt2);
    }

    #[test]
    fn credential_id_newtype_wraps_vec() {
        let id = CredentialId(vec![1, 2, 3]);
        assert_eq!(id.0, vec![1, 2, 3]);
    }
}
