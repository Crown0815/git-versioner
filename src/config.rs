use anyhow::anyhow;
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const MAIN_BRANCH: &str = r"^(trunk|main|master)$";
pub const RELEASE_BRANCH: &str = r"^releases?[/-](?<BranchName>.+)$";
pub const FEATURE_BRANCH: &str = r"^features?[/-](?<BranchName>.+)$";
pub const VERSION_PATTERN: &str = r"^[vV]?(?<Version>\d+\.\d+\.\d+)";

pub trait Configuration {
    fn repository_path(&self) -> &PathBuf;
    fn main_branch(&self) -> &str;
    fn release_branch(&self) -> &str;
    fn feature_branch(&self) -> &str;
    fn version_pattern(&self) -> &str;
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
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ConfigurationFile {
    pub main_branch: Option<String>,
    pub release_branch: Option<String>,
    pub feature_branch: Option<String>,
    pub version_pattern: Option<String>,
}

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    #[arg(short, long, value_parser)]
    path: Option<PathBuf>,

    #[arg(long, value_parser, default_value = MAIN_BRANCH)]
    main_branch: Option<String>,

    #[arg(long, value_parser, default_value = RELEASE_BRANCH)]
    release_branch: Option<String>,

    #[arg(long, value_parser, default_value = FEATURE_BRANCH)]
    feature_branch: Option<String>,

    #[arg(long, value_parser, default_value = VERSION_PATTERN)]
    version_pattern: Option<String>,

    #[arg(short, long)]
    verbose: bool,

    /// Path to a configuration file (TOML or YAML)
    #[arg(short = 'c', long = "config")]
    config_file: Option<PathBuf>,
}

impl Default for DefaultConfig {
    fn default() -> Self {
        Self {
            path: ".".into(),
            main_branch: MAIN_BRANCH.to_string(),
            release_branch: RELEASE_BRANCH.to_string(),
            feature_branch: FEATURE_BRANCH.to_string(),
            version_pattern: VERSION_PATTERN.to_string(),
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
}

impl ConfigurationFile {
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

pub struct ConfigurationLayers {
    args: Args,
    file: ConfigurationFile,
    config: DefaultConfig,
}

impl ConfigurationLayers {
    pub fn new(args: Args) -> anyhow::Result<Self> {
        let config = DefaultConfig::default();
        let file = match &args.config_file {
            None => ConfigurationFile::from_default_files(),
            Some(path) => ConfigurationFile::from_file(path),
        }.unwrap_or_default();
        Ok(Self { args, file, config })
    }
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
        if let Some(main_branch) = &self.args.main_branch {
            main_branch
        } else {
            &self.config.main_branch
        }
    }
    fn release_branch(&self) -> &str {
        if let Some(main_branch) = &self.args.release_branch {
            main_branch
        } else {
            &self.config.release_branch
        }
    }

    fn feature_branch(&self) -> &str {
        if let Some(main_branch) = &self.args.feature_branch {
            main_branch
        } else {
            &self.config.feature_branch
        }
    }

    fn version_pattern(&self) -> &str {
        if let Some(main_branch) = &self.args.version_pattern {
            main_branch
        } else {
            &self.config.version_pattern
        }
    }

    fn verbose(&self) -> bool {
        self.args.verbose
    }
}