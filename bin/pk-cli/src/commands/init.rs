use age::{secrecy::ExposeSecret, x25519::Identity};
use indicatif::{ProgressBar, ProgressStyle};
use inquire::{Password, PasswordDisplayMode};
use pk_core::{
    config::{AgentConfig, PasskeyRoute, PasswordRoute},
    crypto::{derive_kek_password, encrypt_dek, encrypt_with_dek, generate_dek},
    fido::{generate_passkey_credential, generate_salt},
};

pub(crate) async fn init_command() -> eyre::Result<()> {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
            .template("{spinner} {msg}")?,
    );

    pb.set_message("Generating age keypair...");
    let age_identity = Identity::generate();
    let age_public_key = age_identity.to_public().to_string();

    pb.set_message("Generating DEK...");
    let dek = generate_dek()?;

    pb.set_message("Setting up passkey route (may fail gracefully)...");
    let salt_1 = generate_salt()?;
    let passkey_route = if let Ok((credential_id, _kek_1)) = generate_passkey_credential(&salt_1) {
        let encrypted_dek = encrypt_dek(&dek, &_kek_1)?;
        Some(PasskeyRoute { credential_id: credential_id.into(), salt: salt_1, encrypted_dek })
    } else {
        pb.println("Passkey route unavailable (FIDO2 not yet implemented).");
        None
    };

    pb.set_message("Setting up password route...");
    let master_password = Password::new("Enter master password:")
        .with_display_mode(PasswordDisplayMode::Masked)
        .prompt()?;

    let confirm_password = Password::new("Confirm master password:")
        .with_display_mode(PasswordDisplayMode::Masked)
        .prompt()?;

    if master_password != confirm_password {
        return Err(eyre::eyre!("Passwords do not match"));
    }

    let mut password_salt = [0u8; 32];
    getrandom::fill(&mut password_salt)
        .map_err(|e| eyre::eyre!("random generation failed: {e}"))?;

    let kek_2 = derive_kek_password(&master_password, &password_salt)?;
    let encrypted_dek_password = encrypt_dek(&dek, &kek_2)?;

    pb.set_message("Encrypting age private key...");
    let age_private_key_str = age_identity.to_string();
    let encrypted_age_key = encrypt_with_dek(age_private_key_str.expose_secret().as_bytes(), &dek)?;

    pb.set_message("Saving configuration...");
    let config = AgentConfig {
        passkey_route,
        password_route: Some(PasswordRoute {
            salt: password_salt,
            encrypted_dek: encrypted_dek_password,
        }),
        encrypted_age_public_key: age_public_key,
        encrypted_age_private_key: encrypted_age_key,
    };

    config.save()?;

    pb.finish_with_message("Agent initialized successfully.");
    println!("Configuration saved to: {}", pk_core::config::config_path().display());
    println!("Age public key: {}", config.encrypted_age_public_key);
    if config.passkey_route.is_some() {
        println!("Passkey route: configured");
    } else {
        println!("Passkey route: not available (run 'pk init' again after FIDO2 support)");
    }
    println!("Password route: configured");

    Ok(())
}
