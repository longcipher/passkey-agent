use std::{env, os::unix::process::CommandExt, process::Command};

use crate::key;

pub(crate) async fn edit_command(file: &str) -> eyre::Result<()> {
    let age_key = key::ensure_key().await?;

    let editor = env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());

    let mut cmd = Command::new("sops");
    cmd.env("EDITOR", &editor);
    cmd.env("SOPS_AGE_KEY", &age_key);
    cmd.arg("edit");
    cmd.arg(file);

    let err = cmd.exec();
    Err(eyre::eyre!("failed to execute sops edit: {err}"))
}
