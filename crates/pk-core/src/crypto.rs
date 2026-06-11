//! Cryptographic primitives for DEK/KEK key hierarchy.
//!
//! Implements XChaCha20-Poly1305 encryption and Argon2id key derivation.

use chacha20poly1305::{
    XChaCha20Poly1305, XNonce,
    aead::{Aead, KeyInit},
};
use thiserror::Error;

/// Errors that can occur during cryptographic operations.
#[derive(Debug, Error)]
pub enum CryptoError {
    /// Decryption failed due to invalid key or corrupted ciphertext.
    #[error("decryption failed: invalid key or corrupted ciphertext")]
    DecryptionFailed,
    /// Key derivation failed with the given parameters.
    #[error("key derivation failed: {0}")]
    KeyDerivationFailed(String),
    /// Random number generation failed.
    #[error("random generation failed")]
    RandomGenerationFailed,
}

/// Generate a 32-byte Data Encryption Key (DEK) using the OS RNG.
pub fn generate_dek() -> Result<[u8; 32], CryptoError> {
    let mut dek = [0u8; 32];
    getrandom::fill(&mut dek).map_err(|_| CryptoError::RandomGenerationFailed)?;
    Ok(dek)
}

/// Encrypt a DEK with a KEK using XChaCha20-Poly1305.
///
/// Returns nonce || ciphertext.
pub fn encrypt_dek(dek: &[u8; 32], kek: &[u8; 32]) -> Result<Vec<u8>, CryptoError> {
    encrypt_data(dek.as_slice(), kek)
}

/// Decrypt a DEK with a KEK using XChaCha20-Poly1305.
///
/// Expects nonce || ciphertext format.
pub fn decrypt_dek(ciphertext: &[u8], kek: &[u8; 32]) -> Result<[u8; 32], CryptoError> {
    let plaintext = decrypt_data(ciphertext, kek)?;
    let mut dek = [0u8; 32];
    dek.copy_from_slice(&plaintext);
    Ok(dek)
}

/// Derive a KEK from a password using Argon2id.
pub fn derive_kek_password(password: &str, salt: &[u8; 32]) -> Result<[u8; 32], CryptoError> {
    use argon2::{Argon2, Params, Version};

    let argon2 = Argon2::new(
        argon2::Algorithm::Argon2id,
        Version::V0x13,
        Params::new(
            Params::DEFAULT_M_COST,
            Params::DEFAULT_T_COST,
            Params::DEFAULT_P_COST,
            Some(32),
        )
        .map_err(|e| CryptoError::KeyDerivationFailed(e.to_string()))?,
    );

    let mut kek = [0u8; 32];
    argon2
        .hash_password_into(password.as_bytes(), salt, &mut kek)
        .map_err(|e| CryptoError::KeyDerivationFailed(e.to_string()))?;
    Ok(kek)
}

/// Encrypt arbitrary plaintext with a DEK using XChaCha20-Poly1305.
///
/// Returns nonce || ciphertext.
pub fn encrypt_with_dek(plaintext: &[u8], dek: &[u8; 32]) -> Result<Vec<u8>, CryptoError> {
    encrypt_data(plaintext, dek)
}

/// Decrypt arbitrary ciphertext with a DEK using XChaCha20-Poly1305.
///
/// Expects nonce || ciphertext format.
pub fn decrypt_with_dek(ciphertext: &[u8], dek: &[u8; 32]) -> Result<Vec<u8>, CryptoError> {
    decrypt_data(ciphertext, dek)
}

// --- Internal helpers ---

fn generate_nonce() -> Result<[u8; 24], CryptoError> {
    let mut nonce = [0u8; 24];
    getrandom::fill(&mut nonce).map_err(|_| CryptoError::RandomGenerationFailed)?;
    Ok(nonce)
}

fn encrypt_data(plaintext: &[u8], key: &[u8; 32]) -> Result<Vec<u8>, CryptoError> {
    let nonce = generate_nonce()?;
    let cipher =
        XChaCha20Poly1305::new_from_slice(key).map_err(|_| CryptoError::RandomGenerationFailed)?;
    let ciphertext = cipher
        .encrypt(&XNonce::from(nonce), plaintext)
        .map_err(|_| CryptoError::DecryptionFailed)?;

    let mut result = Vec::with_capacity(24 + ciphertext.len());
    result.extend_from_slice(&nonce);
    result.extend_from_slice(&ciphertext);
    Ok(result)
}

fn decrypt_data(ciphertext_with_nonce: &[u8], key: &[u8; 32]) -> Result<Vec<u8>, CryptoError> {
    if ciphertext_with_nonce.len() < 24 {
        return Err(CryptoError::DecryptionFailed);
    }
    let (nonce, ciphertext) = ciphertext_with_nonce.split_at(24);
    let cipher =
        XChaCha20Poly1305::new_from_slice(key).map_err(|_| CryptoError::RandomGenerationFailed)?;
    cipher.decrypt(XNonce::from_slice(nonce), ciphertext).map_err(|_| CryptoError::DecryptionFailed)
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    #[test]
    fn generate_dek_produces_32_bytes() {
        let dek = generate_dek().unwrap();
        assert_eq!(dek.len(), 32);
    }

    #[test]
    fn encrypt_decrypt_dek_roundtrip() {
        let dek = generate_dek().unwrap();
        let kek = generate_dek().unwrap();
        let encrypted = encrypt_dek(&dek, &kek).unwrap();
        let decrypted = decrypt_dek(&encrypted, &kek).unwrap();
        assert_eq!(dek, decrypted);
    }

    #[test]
    fn encrypt_decrypt_data_roundtrip() {
        let dek = generate_dek().unwrap();
        let plaintext = b"hello, world!";
        let encrypted = encrypt_with_dek(plaintext, &dek).unwrap();
        let decrypted = decrypt_with_dek(&encrypted, &dek).unwrap();
        assert_eq!(plaintext.as_slice(), decrypted.as_slice());
    }

    #[test]
    fn decrypt_with_wrong_key_fails() {
        let dek = generate_dek().unwrap();
        let wrong_key = generate_dek().unwrap();
        let plaintext = b"secret data";
        let encrypted = encrypt_with_dek(plaintext, &dek).unwrap();
        assert!(decrypt_with_dek(&encrypted, &wrong_key).is_err());
    }

    #[test]
    fn derive_kek_password_produces_32_bytes() {
        let salt = [0u8; 32];
        let kek = derive_kek_password("test-password", &salt).unwrap();
        assert_eq!(kek.len(), 32);
    }

    #[test]
    fn derive_kek_password_different_salts_produce_different_keks() {
        let salt1 = [0u8; 32];
        let salt2 = [1u8; 32];
        let kek1 = derive_kek_password("test", &salt1).unwrap();
        let kek2 = derive_kek_password("test", &salt2).unwrap();
        assert_ne!(kek1, kek2);
    }

    proptest! {
        #[test]
        fn encrypt_decrypt_roundtrip_arbitrary_data(
            data in prop::collection::vec(any::<u8>(), 0..256),
            key_bytes in prop::collection::vec(any::<u8>(), 32),
        ) {
            let mut key = [0u8; 32];
            key.copy_from_slice(&key_bytes);
            let encrypted = encrypt_with_dek(&data, &key).unwrap();
            let decrypted = decrypt_with_dek(&encrypted, &key).unwrap();
            prop_assert_eq!(data, decrypted);
        }

        #[test]
        fn encrypt_decrypt_roundtrip_arbitrary_dek(
            dek_bytes in prop::collection::vec(any::<u8>(), 32),
            kek_bytes in prop::collection::vec(any::<u8>(), 32),
        ) {
            let mut dek = [0u8; 32];
            dek.copy_from_slice(&dek_bytes);
            let mut kek = [0u8; 32];
            kek.copy_from_slice(&kek_bytes);
            let encrypted = encrypt_dek(&dek, &kek).unwrap();
            let decrypted = decrypt_dek(&encrypted, &kek).unwrap();
            prop_assert_eq!(dek, decrypted);
        }
    }
}
