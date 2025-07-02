use crate::{FEATURE_BRANCH, MAIN_BRANCH, RELEASE_BRANCH, VERSION_PATTERN};
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
pub struct Configuration{
    pub repo_path: PathBuf,
    pub main_branch: String,
    pub release_branch: String,
    pub feature_branch: String,
    pub version_pattern: String,
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            repo_path: ".".into(),
            main_branch: MAIN_BRANCH.to_string(),
            release_branch: RELEASE_BRANCH.to_string(),
            feature_branch: FEATURE_BRANCH.to_string(),
            version_pattern: VERSION_PATTERN.to_string(),
        }
    }
}

pub struct ConfigurationFile {
    pub main_branch: Option<String>,
    pub release_branch: Option<String>,
    pub feature_branch: Option<String>,
    pub version_pattern: Option<String>,
}

impl Configuration {
    /// Attempts to load configuration from a file with the given path.
    /// The file format is determined by the file extension.
    pub fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let path = path.as_ref();
        let extension = path.extension()
            .and_then(|ext| ext.to_str())
            .ok_or_else(|| anyhow!("File has no extension"))?;

        match extension.to_lowercase().as_str() {
            "toml" => Self::from_toml_file(path),
            "yaml" | "yml" => Self::from_yaml_file(path),
            _ => Err(anyhow!("Unsupported file format: {}", extension)),
        }
    }

    /// Loads configuration from a TOML file.
    pub fn from_toml_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    /// Loads configuration from a YAML file.
    pub fn from_yaml_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: Self = serde_yaml::from_str(&content)?;
        Ok(config)
    }

    /// Attempts to load configuration from default configuration files.
    /// Looks for .git-versioner.toml and .git-versioner.yaml in the current directory.
    pub fn from_default_files() -> anyhow::Result<Self> {
        let toml_path = Path::new(".git-versioner.toml");
        let yaml_path = Path::new(".git-versioner.yaml");
        let yml_path = Path::new(".git-versioner.yml");

        if toml_path.exists() {
            return Self::from_toml_file(toml_path);
        } else if yaml_path.exists() {
            return Self::from_yaml_file(yaml_path);
        } else if yml_path.exists() {
            return Self::from_yaml_file(yml_path);
        }

        Err(anyhow!("No configuration file found"))
    }
}