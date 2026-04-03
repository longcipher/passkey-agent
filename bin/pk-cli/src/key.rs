use pk_core::{
    config::AgentConfig,
    crypto::{decrypt_dek, decrypt_with_dek, derive_kek_password},
    fido::{CredentialId, authenticate_passkey},
    ipc::AgentClient,
};

/// Ensure the daemon is unlocked and return the age key.
pub(crate) async fn ensure_key() -> eyre::Result<String> {
    let client = AgentClient::new();

    let is_unlocked = match client.status().await {
        Ok((state, _)) => matches!(state, pk_core::ipc::AgentState::Unlocked),
        Err(_) => false,
    };

    if !is_unlocked {
        let config = AgentConfig::load().map_err(|e| eyre::eyre!("failed to load config: {e}"))?;
        let age_key = match try_unlock_passkey(&config).await {
            Ok(key) => key,
            Err(_) => try_unlock_password(&config).await?,
        };
        client.unlock(age_key).await.map_err(|e| eyre::eyre!("failed to unlock daemon: {e}"))?;
    }

    client.get_key().await.map_err(|e| eyre::eyre!("failed to get key from daemon: {e}"))
}

async fn try_unlock_passkey(config: &AgentConfig) -> eyre::Result<String> {
    let passkey_route =
        config.passkey_route.as_ref().ok_or_else(|| eyre::eyre!("no passkey route configured"))?;

    let credential_id = CredentialId(passkey_route.credential_id.0.clone());
    let kek_1 = authenticate_passkey(&credential_id, &passkey_route.salt)?;
    let dek = decrypt_dek(&passkey_route.encrypted_dek, &kek_1)?;
    let age_key_bytes = decrypt_with_dek(&config.encrypted_age_private_key, &dek)?;
    let age_key =
        String::from_utf8(age_key_bytes).map_err(|_| eyre::eyre!("invalid age key encoding"))?;

    Ok(age_key)
}

async fn try_unlock_password(config: &AgentConfig) -> eyre::Result<String> {
    let password_route = config
        .password_route
        .as_ref()
        .ok_or_else(|| eyre::eyre!("no password route configured"))?;

    let master_password = inquire::Password::new("Enter master password:")
        .prompt()
        .map_err(|e| eyre::eyre!("failed to read password: {e}"))?;

    let kek_2 = derive_kek_password(&master_password, &password_route.salt)?;
    let dek = decrypt_dek(&password_route.encrypted_dek, &kek_2)?;
    let age_key_bytes = decrypt_with_dek(&config.encrypted_age_private_key, &dek)?;
    let age_key =
        String::from_utf8(age_key_bytes).map_err(|_| eyre::eyre!("invalid age key encoding"))?;

    Ok(age_key)
}
