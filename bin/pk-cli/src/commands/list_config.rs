use std::env;

use pk_core::sops_config::SopsConfig;

pub(crate) fn list_config_command() -> eyre::Result<()> {
    let cwd = env::current_dir()?;
    if let Ok((path, config)) = SopsConfig::find(&cwd) {
        println!("Found .sops.yaml at: {}", path.display());
        println!();
        println!("Creation Rules:");
        for (i, rule) in config.creation_rules.iter().enumerate() {
            println!("  {}:", i + 1);
            if let Some(ref regex) = rule.path_regex {
                println!("    path_regex: {}", regex);
            }
            if let Some(ref age) = rule.age {
                println!("    age: {}", age);
            }
            if let Some(ref suffix) = rule.encrypted_suffix {
                println!("    encrypted_suffix: {}", suffix);
            }
            if let Some(ref regex) = rule.encrypted_regex {
                println!("    encrypted_regex: {}", regex);
            }
        }
    } else {
        println!("No .sops.yaml found in current directory or parents");
    }
    Ok(())
}
