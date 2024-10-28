use std::collections::HashMap;
use std::path::Path;
use std::{env, fs};

use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, PartialEq, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    pub profiles: HashMap<String, Profile>,
}

#[derive(Debug, PartialEq, Default, Deserialize)]
pub struct Profile {
    pub repo: String,
    #[serde(default = "default_add_params")]
    pub add_params: Vec<String>,
    #[serde(default = "default_rm_params")]
    pub rm_params: Vec<String>,
    #[serde(default = "default_require_signature")]
    pub require_signature: bool,
    #[serde(default = "default_backup")]
    pub backup: bool,
    pub backup_dir: Option<String>,
    /// The number of package versions to keep in the backup directory
    pub backup_num: Option<usize>,
    pub debug_dir: Option<String>,
    #[serde(default = "default_interactive")]
    pub interactive: bool,
    // Add the reference_repo field
    pub reference_repo: Option<String>,
}

pub fn parse_config_file(filepath: &str) -> Result<Config> {
    let file_content = fs::read_to_string(filepath)?;
    parse_config_content(&file_content)
}

pub fn get_config_path() -> Result<String> {
    // Search for config file in home and system directories
    let home_env = env::var("HOME").expect("Failed to get HOME environment");

    let home_config_path = format!("{home_env}/{}", ".config/repo-manage/config.toml");

    let check_paths = [home_config_path, "/etc/repo-manage/config.toml".to_owned()];
    for check_path in check_paths {
        if !Path::new(&check_path).exists() {
            continue;
        }
        // we found config path
        return Ok(check_path);
    }

    anyhow::bail!("Failed to find config!");
}

fn parse_config_content(file_content: &str) -> Result<Config> {
    if file_content.is_empty() {
        anyhow::bail!("The config file is empty!")
    }
    let config: Config = toml::from_str(file_content)?;
    Ok(config)
}

fn default_add_params() -> Vec<String> {
    vec!["--sign".to_string(), "--include-sigs".to_string(), "--verify".to_string()]
}

fn default_rm_params() -> Vec<String> {
    vec!["--sign".to_string()]
}

fn default_require_signature() -> bool {
    true
}

fn default_backup() -> bool {
    false
}

fn default_interactive() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use crate::config::*;

    #[test]
    fn parse_example_config() {
        let prof_path = "example-config.toml";
        let parsed_config = parse_config_file(prof_path);
        assert!(parsed_config.is_ok());

        let expected_config = Config {
            profiles: HashMap::from([
                (
                    "repof".to_string(),
                    Profile {
                        repo: "/home/testuser/repos/x86_64/os/repof/repof.db.tar.zst".to_string(),
                        add_params: vec!["--sign".to_string(), "--include-sigs".to_string()],
                        rm_params: vec!["--sign".to_string()],
                        require_signature: true,
                        backup: true,
                        backup_num: None,
                        backup_dir: Some("/home/testuser/backup_repos/repof".to_string()),
                        debug_dir: Some("/home/testuser/debug_repos/repof".to_string()),
                        interactive: false,
                        reference_repo: None,
                    },
                ),
                (
                    "reposecond".to_string(),
                    Profile {
                        repo: "/home/testuser/repos/x86_64/os/reposecond/reposecond.db.tar.zst"
                            .to_string(),
                        add_params: vec!["--sign".to_string(), "--include-sigs".to_string()],
                        rm_params: vec!["--sign".to_string()],
                        require_signature: true,
                        backup: true,
                        backup_num: None,
                        backup_dir: Some("/home/testuser/backup_repos/reposecond".to_string()),
                        debug_dir: Some("/home/testuser/debug_repos/reposecond".to_string()),
                        interactive: false,
                        reference_repo: None,
                    },
                ),
            ]),
        };

        assert_eq!(parsed_config.unwrap(), expected_config);
    }

    #[test]
    fn test_missing_required_field() {
        let config_str = r#"
[profiles.repof]
backup_dir = "/home/testuser/backup_repos/repof"
debug_package_dir = "/home/testuser/debug_repos/repof"
"#;

        let result = parse_config_content(config_str);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_config() {
        let config_str = "";
        let result = parse_config_content(config_str);
        assert!(result.is_err());
    }
}
