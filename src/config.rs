use anyhow::anyhow;
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const MAIN_BRANCH: &str = r"^(trunk|main|master)$";
pub const RELEASE_BRANCH: &str = r"^releases?[/-](?<BranchName>.+)$";
pub const FEATURE_BRANCH: &str = r"^features?[/-](?<BranchName>.+)$";
pub const VERSION_PATTERN: &str = r"^[vV]?(?<Version>.+)";
pub const TAG_PREFIX: &str = r"[vV]?";
pub const PRE_RELEASE_TAG: &str = "pre";

pub trait Configuration {
    fn repository_path(&self) -> &PathBuf;
    fn main_branch(&self) -> &str;
    fn release_branch(&self) -> &str;
    fn feature_branch(&self) -> &str;
    fn version_pattern(&self) -> &str;
    fn tag_prefix(&self) -> &str;
    fn pre_release_tag(&self) -> &str;
    fn verbose(&self) -> bool {
        false
    }
    fn show_config(&self) -> bool {
        false
    }

    fn print(&self) -> DefaultConfig {
        DefaultConfig {
            path: fs::canonicalize(self.repository_path()).unwrap(),
            main_branch: self.main_branch().to_string(),
            release_branch: self.release_branch().to_string(),
            feature_branch: self.feature_branch().to_string(),
            version_pattern: self.version_pattern().to_string(),
            tag_prefix: self.tag_prefix().to_string(),
            pre_release_tag: self.pre_release_tag().to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DefaultConfig {
    pub path: PathBuf,
    pub main_branch: String,
    pub release_branch: String,
    pub feature_branch: String,
    pub version_pattern: String,
    pub tag_prefix: String,
    pub pre_release_tag: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ConfigurationFile {
    pub main_branch: Option<String>,
    pub release_branch: Option<String>,
    pub feature_branch: Option<String>,
    pub version_pattern: Option<String>,
    pub pre_release_tag: Option<String>,
    pub tag_prefix: Option<String>,
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
    tag_prefix: Option<String>,

    #[arg(long, value_parser)]
    pre_release_tag: Option<String>,

    /// Outputs effective git-versioner config in toml format
    #[arg(long)]
    show_config: bool,

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
            tag_prefix: TAG_PREFIX.to_string(),
            pre_release_tag: PRE_RELEASE_TAG.to_string(),
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
    fn tag_prefix(&self) -> &str {
        &self.tag_prefix
    }
    fn pre_release_tag(&self) -> &str {
        &self.pre_release_tag
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
        if let Some(main_branch) = &self.args.main_branch {
            main_branch
        } else if let Some(main_branch) = &self.file.main_branch {
            main_branch
        } else {
            &self.config.main_branch
        }
    }
    fn release_branch(&self) -> &str {
        if let Some(release_branch) = &self.args.release_branch {
            release_branch
        } else if let Some(release_branch) = &self.file.release_branch {
            release_branch
        } else {
            &self.config.release_branch
        }
    }

    fn feature_branch(&self) -> &str {
        if let Some(feature_branch) = &self.args.feature_branch {
            feature_branch
        } else if let Some(feature_branch) = &self.file.feature_branch {
            feature_branch
        } else {
            &self.config.feature_branch
        }
    }

    fn version_pattern(&self) -> &str {
        if let Some(version_pattern) = &self.args.version_pattern {
            version_pattern
        } else if let Some(version_pattern) = &self.file.version_pattern {
            version_pattern
        } else {
            &self.config.version_pattern
        }
    }

    fn tag_prefix(&self) -> &str {
        if let Some(branch) = &self.args.tag_prefix {
            branch
        } else if let Some(tag_prefix) = &self.file.tag_prefix {
            tag_prefix
        } else {
            &self.config.tag_prefix
        }
    }

    fn pre_release_tag(&self) -> &str {
        if let Some(branch) = &self.args.pre_release_tag {
            branch
        } else if let Some(branch) = &self.file.pre_release_tag {
            branch
        } else {
            &self.config.pre_release_tag
        }
    }

    fn verbose(&self) -> bool {
        self.args.verbose
    }

    fn show_config(&self) -> bool {
        self.args.show_config
    }
}
