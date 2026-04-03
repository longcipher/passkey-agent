use std::{os::unix::process::CommandExt, process::Command};

use crate::key::ensure_key;

pub(crate) async fn run_command(args: &[String]) -> eyre::Result<()> {
    if args.is_empty() {
        return Err(eyre::eyre!("no command specified"));
    }

    let age_key = ensure_key().await?;

    let mut cmd = Command::new(&args[0]);
    cmd.env("SOPS_AGE_KEY", &age_key);
    if args.len() > 1 {
        cmd.args(&args[1..]);
    }

    let err = cmd.exec();
    Err(eyre::eyre!("failed to execute {}: {err}", args[0]))
}
