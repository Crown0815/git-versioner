use anyhow::anyhow;
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const MAIN_BRANCH: &str = r"^(trunk|main|master)$";
pub const RELEASE_BRANCH: &str = r"^releases?[/-](?<BranchName>.+)$";
pub const FEATURE_BRANCH: &str = r"^features?[/-](?<BranchName>.+)$";
pub const VERSION_PATTERN: &str = r"^[vV]?(?<Version>.+)";
pub const PRERELEASE_TAG: &str = "pre";

pub trait Configuration {
    fn repository_path(&self) -> &PathBuf;
    fn main_branch(&self) -> &str;
    fn release_branch(&self) -> &str;
    fn feature_branch(&self) -> &str;
    fn version_pattern(&self) -> &str;
    fn prerelease_tag(&self) -> &str;
    fn verbose(&self) -> bool {
        false
    }
}

#[derive(Debug)]
pub struct DefaultConfig {
    pub path: PathBuf,
    pub main_branch: String,
    pub release_branch: String,
    pub feature_branch: String,
    pub version_pattern: String,
    pub prerelease_tag: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ConfigurationFile {
    pub main_branch: Option<String>,
    pub release_branch: Option<String>,
    pub feature_branch: Option<String>,
    pub version_pattern: Option<String>,
    pub prerelease_tag: Option<String>,
}

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    #[arg(short, long, value_parser)]
    path: Option<PathBuf>,

    #[arg(long, value_parser)]
    main_branch: Option<String>,

    #[arg(long, value_parser)]
    release_branch: Option<String>,

    #[arg(long, value_parser)]
    feature_branch: Option<String>,

    #[arg(long, value_parser)]
    version_pattern: Option<String>,

    #[arg(long, value_parser)]
    prerelease_tag: Option<String>,

    #[arg(short, long)]
    verbose: bool,

    /// Path to a configuration file (TOML or YAML)
    #[arg(short = 'c', long = "config")]
    config_file: Option<PathBuf>,
}

#[derive(Debug)]
pub struct ConfigurationLayers {
    args: Args,
    file: ConfigurationFile,
    config: DefaultConfig,
}

impl Default for DefaultConfig {
    fn default() -> Self {
        Self {
            path: ".".into(),
            main_branch: MAIN_BRANCH.to_string(),
            release_branch: RELEASE_BRANCH.to_string(),
            feature_branch: FEATURE_BRANCH.to_string(),
            version_pattern: VERSION_PATTERN.to_string(),
            prerelease_tag: PRERELEASE_TAG.to_string(),
        }
    }
}

impl Configuration for DefaultConfig {
    fn repository_path(&self) -> &PathBuf {
        &self.path
    }
    fn main_branch(&self) -> &str {
        &self.main_branch
    }
    fn release_branch(&self) -> &str {
        &self.release_branch
    }
    fn feature_branch(&self) -> &str {
        &self.feature_branch
    }
    fn version_pattern(&self) -> &str {
        &self.version_pattern
    }
    fn prerelease_tag(&self) -> &str {
        &self.prerelease_tag
    }
}

impl ConfigurationFile {
    pub fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let path = path.as_ref();
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .ok_or_else(|| anyhow!("File has no extension"))?;

        match extension.to_lowercase().as_str() {
            "toml" => Self::from_toml_file(path),
            "yaml" | "yml" => Self::from_yaml_file(path),
            _ => Err(anyhow!("Unsupported file format: {}", extension)),
        }
    }

    pub fn from_toml_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn from_yaml_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: Self = serde_yaml::from_str(&content)?;
        Ok(config)
    }

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

pub fn load_configuration() -> anyhow::Result<ConfigurationLayers> {
    let args = Args::parse();
    let config = DefaultConfig::default();
    let file = match &args.config_file {
        None => ConfigurationFile::from_default_files(),
        Some(path) => ConfigurationFile::from_file(path),
    }
    .unwrap_or_default();
    Ok(ConfigurationLayers { args, file, config })
}

impl Configuration for ConfigurationLayers {
    fn repository_path(&self) -> &PathBuf {
        if let Some(path) = &self.args.path {
            path
        } else {
            &self.config.path
        }
    }
    fn main_branch(&self) -> &str {
        if let Some(branch) = &self.args.main_branch {
            branch
        } else if let Some(branch) = &self.file.main_branch {
            branch
        } else {
            &self.config.main_branch
        }
    }
    fn release_branch(&self) -> &str {
        if let Some(branch) = &self.args.release_branch {
            branch
        } else if let Some(branch) = &self.file.release_branch {
            branch
        } else {
            &self.config.release_branch
        }
    }

    fn feature_branch(&self) -> &str {
        if let Some(branch) = &self.args.feature_branch {
            branch
        } else if let Some(branch) = &self.file.feature_branch {
            branch
        } else {
            &self.config.feature_branch
        }
    }

    fn version_pattern(&self) -> &str {
        if let Some(branch) = &self.args.version_pattern {
            branch
        } else if let Some(branch) = &self.file.version_pattern {
            branch
        } else {
            &self.config.version_pattern
        }
    }

    fn prerelease_tag(&self) -> &str {
        if let Some(branch) = &self.args.prerelease_tag {
            branch
        } else if let Some(branch) = &self.file.prerelease_tag {
            branch
        } else {
            &self.config.prerelease_tag
        }
    }

    fn verbose(&self) -> bool {
        self.args.verbose
    }
}
