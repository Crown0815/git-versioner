use anyhow::anyhow;
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const MAIN_BRANCH: &str = r"^(trunk|main|master)$";
pub const RELEASE_BRANCH: &str = r"^releases?[/-](?<BranchName>.+)$";
pub const FEATURE_BRANCH: &str = r"^features?[/-](?<BranchName>.+)$";
pub const TAG_PREFIX: &str = r"[vV]?";
pub const PRE_RELEASE_TAG: &str = "pre";
pub const COMMIT_MESSAGE_INCREMENTING: &str = "Disabled";

pub trait Configuration {
    fn path(&self) -> &PathBuf;
    fn main_branch(&self) -> &str;
    fn release_branch(&self) -> &str;
    fn feature_branch(&self) -> &str;
    fn tag_prefix(&self) -> &str;
    fn pre_release_tag(&self) -> &str;
    fn commit_message_incrementing(&self) -> &str;
    fn verbose(&self) -> &bool {
        &false
    }
    fn show_config(&self) -> &bool {
        &false
    }

    fn print(&self) -> DefaultConfig {
        DefaultConfig {
            path: fs::canonicalize(self.path()).unwrap(),
            main_branch: self.main_branch().to_string(),
            release_branch: self.release_branch().to_string(),
            feature_branch: self.feature_branch().to_string(),
            tag_prefix: self.tag_prefix().to_string(),
            pre_release_tag: self.pre_release_tag().to_string(),
            commit_message_incrementing: self.commit_message_incrementing().to_string(),
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
    pub tag_prefix: String,
    pub pre_release_tag: String,
    pub commit_message_incrementing: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ConfigurationFile {
    pub main_branch: Option<String>,
    pub release_branch: Option<String>,
    pub feature_branch: Option<String>,
    pub tag_prefix: Option<String>,
    pub pre_release_tag: Option<String>,
    // pub commit_message_incrementing: Option<String>,
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
    tag_prefix: Option<String>,

    #[arg(long, value_parser)]
    pre_release_tag: Option<String>,

    // #[arg(long, value_parser)]
    // commit_message_incrementing: Option<String>,
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
            tag_prefix: TAG_PREFIX.to_string(),
            pre_release_tag: PRE_RELEASE_TAG.to_string(),
            commit_message_incrementing: COMMIT_MESSAGE_INCREMENTING.to_string(),
        }
    }
}

impl Configuration for DefaultConfig {
    fn path(&self) -> &PathBuf {
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
    fn tag_prefix(&self) -> &str {
        &self.tag_prefix
    }
    fn pre_release_tag(&self) -> &str {
        &self.pre_release_tag
    }
    fn commit_message_incrementing(&self) -> &str {
        &self.commit_message_incrementing
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

macro_rules! config_getter {
    ($name:ident, $return:ty, arg>file>default) => {
        fn $name(&self) -> &$return {
            if let Some(value) = &self.args.$name {
                value
            } else if let Some(value) = &self.file.$name {
                value
            } else {
                &self.config.$name
            }
        }
    };
    ($name:ident, $return:ty, arg>default) => {
        fn $name(&self) -> &$return {
            if let Some(value) = &self.args.$name {
                value
            } else {
                &self.config.$name
            }
        }
    };
    ($name:ident, $return:ty, arg) => {
        fn $name(&self) -> &$return {
            &self.args.$name
        }
    };
}

impl Configuration for ConfigurationLayers {
    config_getter!(main_branch, str, arg > file > default);
    config_getter!(release_branch, str, arg > file > default);
    config_getter!(feature_branch, str, arg > file > default);
    config_getter!(tag_prefix, str, arg > file > default);
    config_getter!(pre_release_tag, str, arg > file > default);
    fn commit_message_incrementing(&self) -> &str {
        &self.config.commit_message_incrementing
    }
    config_getter!(path, PathBuf, arg > default);
    config_getter!(verbose, bool, arg);
    config_getter!(show_config, bool, arg);
}
