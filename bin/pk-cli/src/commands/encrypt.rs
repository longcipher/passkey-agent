use std::{os::unix::process::CommandExt, process::Command};

use crate::key;

pub(crate) async fn encrypt_command(file: &str, output: Option<&str>) -> eyre::Result<()> {
    let age_key = key::ensure_key().await?;

    let mut cmd = Command::new("sops");
    cmd.env("SOPS_AGE_KEY", &age_key);
    cmd.arg("encrypt");
    cmd.arg(file);

    if let Some(out) = output {
        cmd.arg("--output");
        cmd.arg(out);
    }

    let err = cmd.exec();
    Err(eyre::eyre!("failed to execute sops encrypt: {err}"))
}
