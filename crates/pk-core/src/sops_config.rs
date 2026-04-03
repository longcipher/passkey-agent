//! SOPS configuration file parsing.
//!
//! Provides support for reading `.sops.yaml` files, walking up the directory
//! tree to locate them, and matching creation rules against file paths.

use std::path::{Path, PathBuf};

use regex::Regex;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur when loading or parsing `.sops.yaml` files.
#[derive(Debug, Error)]
pub enum SopsConfigError {
    /// Failed to read the `.sops.yaml` file from disk.
    #[error("failed to read .sops.yaml: {0}")]
    ReadFailed(String),
    /// Failed to parse the `.sops.yaml` file contents.
    #[error("failed to parse .sops.yaml: {0}")]
    ParseFailed(String),
    /// No `.sops.yaml` file was found while walking up.
    #[error(".sops.yaml not found")]
    NotFound,
}

/// Parsed `.sops.yaml` configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SopsConfig {
    /// Ordered list of creation rules.
    #[serde(default)]
    pub creation_rules: Vec<CreationRule>,
    /// Optional 1Password item reference (opsops-specific).
    #[serde(rename = "onepassworditem", default)]
    pub onepassword_item: Option<String>,
}

/// A single creation rule from the `creation_rules` list.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreationRule {
    /// Regex pattern for matching file paths.
    pub path_regex: Option<String>,
    /// Age public key(s) for encryption.
    pub age: Option<String>,
    /// Suffix that indicates encrypted fields.
    pub encrypted_suffix: Option<String>,
    /// Regex for encrypted field names.
    pub encrypted_regex: Option<String>,
    /// Prefix that indicates encrypted fields.
    pub encrypted_prefix: Option<String>,
    /// GPG fingerprints.
    pub pgp: Option<String>,
    /// KMS ARNs.
    pub kms: Option<String>,
    /// GCP KMS key rings.
    pub gcp_kms: Option<String>,
    /// Azure Key Vault.
    pub azure_keyvault: Option<String>,
    /// HashiCorp Vault.
    pub hc_vault: Option<String>,
}

impl SopsConfig {
    /// Find `.sops.yaml` by walking up from the given directory.
    ///
    /// Returns the path to the found file and the parsed configuration.
    pub fn find(start_dir: &Path) -> Result<(PathBuf, Self), SopsConfigError> {
        let mut current = if start_dir.is_absolute() {
            start_dir.to_path_buf()
        } else {
            std::env::current_dir()
                .map_err(|e| SopsConfigError::ReadFailed(e.to_string()))?
                .join(start_dir)
        };

        loop {
            let candidate = current.join(".sops.yaml");
            if candidate.is_file() {
                let config = Self::load(&candidate)?;
                return Ok((candidate, config));
            }

            if !current.pop() {
                return Err(SopsConfigError::NotFound);
            }
        }
    }

    /// Load and parse a `.sops.yaml` file from a specific path.
    pub fn load(path: &Path) -> Result<Self, SopsConfigError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| SopsConfigError::ReadFailed(e.to_string()))?;
        serde_yaml::from_str(&content).map_err(|e| SopsConfigError::ParseFailed(e.to_string()))
    }

    /// Find the matching creation rule for a given file path.
    ///
    /// Returns the first rule whose `path_regex` matches the file path.
    /// Rules with no `path_regex` or an invalid regex are skipped.
    pub fn matching_rule(&self, file_path: &Path) -> Option<&CreationRule> {
        let path_str = file_path.to_string_lossy();
        for rule in &self.creation_rules {
            if let Some(ref pattern) = rule.path_regex &&
                let Ok(re) = Regex::new(pattern) &&
                re.is_match(&path_str)
            {
                return Some(rule);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn make_temp_dir_with_sops_yaml(content: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(".sops.yaml"), content).unwrap();
        dir
    }

    #[test]
    fn test_load_valid_config() {
        let yaml = r#"
creation_rules:
  - path_regex: .*\.ya?ml
    age: age1xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
    encrypted_suffix: _secret
  - path_regex: .*\.json
    age: age1yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy
    encrypted_regex: "^(data|stringData)$"
onepassworditem: op://Personal/test/Private Key
"#;
        let dir = make_temp_dir_with_sops_yaml(yaml);
        let config = SopsConfig::load(&dir.path().join(".sops.yaml")).unwrap();

        assert_eq!(config.creation_rules.len(), 2);
        assert_eq!(config.creation_rules[0].path_regex, Some(".*\\.ya?ml".to_string()));
        assert_eq!(
            config.creation_rules[0].age,
            Some("age1xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx".to_string())
        );
        assert_eq!(config.creation_rules[0].encrypted_suffix, Some("_secret".to_string()));
        assert_eq!(
            config.creation_rules[1].encrypted_regex,
            Some("^(data|stringData)$".to_string())
        );
        assert_eq!(config.onepassword_item, Some("op://Personal/test/Private Key".to_string()));
    }

    #[test]
    fn test_find_walks_up() {
        let yaml = r#"
creation_rules:
  - path_regex: .*\.yaml
    age: age1test
"#;
        let parent = make_temp_dir_with_sops_yaml(yaml);
        let child = parent.path().join("subdir");
        fs::create_dir_all(&child).unwrap();

        let (found_path, config) = SopsConfig::find(&child).unwrap();
        assert!(found_path.ends_with(".sops.yaml"));
        assert_eq!(config.creation_rules.len(), 1);
    }

    #[test]
    fn test_find_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let result = SopsConfig::find(dir.path());
        assert!(matches!(result, Err(SopsConfigError::NotFound)));
    }

    #[test]
    fn test_matching_rule_yaml() {
        let yaml = r#"
creation_rules:
  - path_regex: .*\.ya?ml
    age: age1yaml
  - path_regex: .*\.json
    age: age1json
"#;
        let dir = make_temp_dir_with_sops_yaml(yaml);
        let config = SopsConfig::load(&dir.path().join(".sops.yaml")).unwrap();

        let rule = config.matching_rule(Path::new("config.yaml"));
        assert!(rule.is_some());
        assert_eq!(rule.unwrap().age, Some("age1yaml".to_string()));

        let rule = config.matching_rule(Path::new("config.yml"));
        assert!(rule.is_some());
        assert_eq!(rule.unwrap().age, Some("age1yaml".to_string()));

        let rule = config.matching_rule(Path::new("data.json"));
        assert!(rule.is_some());
        assert_eq!(rule.unwrap().age, Some("age1json".to_string()));

        let rule = config.matching_rule(Path::new("data.txt"));
        assert!(rule.is_none());
    }

    #[test]
    fn test_matching_rule_no_path_regex() {
        let yaml = r#"
creation_rules:
  - age: age1default
"#;
        let dir = make_temp_dir_with_sops_yaml(yaml);
        let config = SopsConfig::load(&dir.path().join(".sops.yaml")).unwrap();

        let rule = config.matching_rule(Path::new("anything.txt"));
        assert!(rule.is_none());
    }

    #[test]
    fn test_matching_rule_invalid_regex() {
        let yaml = r#"
creation_rules:
  - path_regex: "[invalid"
    age: age1bad
"#;
        let dir = make_temp_dir_with_sops_yaml(yaml);
        let config = SopsConfig::load(&dir.path().join(".sops.yaml")).unwrap();

        let rule = config.matching_rule(Path::new("test.yaml"));
        assert!(rule.is_none());
    }
}
