use git2::Oid;
use git_versioner::config::ConfigurationFile;
use git_versioner::DefaultConfig;
use insta_cmd::get_cargo_bin;
use regex::Regex;
use rstest::fixture;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

pub const MAIN_BRANCH: &str = "trunk";

#[fixture]
pub fn repo(#[default(MAIN_BRANCH)] main: &str) -> TestRepo {
    let repo = TestRepo::initialize(main);
    repo.commit("0.1.0-rc.1");
    repo
}

#[fixture]
pub fn cli() -> Command {
    Command::new(get_cargo_bin(env!("CARGO_PKG_NAME")))
}

#[macro_export]
macro_rules! assert_repo_cmd_snapshot {
    ($repo:expr, $cmd:expr) => {
        with_settings!(
            { description => $repo.graph() },
            { assert_cmd_snapshot!($cmd); }
        );
    };
}

pub struct TestRepo {
    pub path: PathBuf,
    pub config: DefaultConfig,
    pub cli_config: ConfigurationFile,
    _temp_dir: tempfile::TempDir, // Keep the temp_dir to prevent it from being deleted
}

impl TestRepo {
    fn new() -> Self {
        let _temp_dir = tempfile::tempdir().unwrap();
        let path = _temp_dir.path().to_path_buf();
        let mut config = DefaultConfig::default();
        config.path = path.clone();
        let cli_config = ConfigurationFile::default();
        Self { path, config, cli_config, _temp_dir }
    }

    pub fn initialize(main_branch: &str) -> Self {
        let repo = TestRepo::new();
        repo.execute(&["init", &format!("--initial-branch={main_branch}")], "initialize repository");
        repo.execute(&["config", "user.name", "tester"], "configure user.name");
        repo.execute(&["config", "user.email", "tester@tests.com"], "configure user.email");
        repo
    }

    pub fn commit(&self, message: &str) -> Oid {
        self.execute(&["commit", "--allow-empty", "-m", message], &format!("commit {message}"));
        let output = self.execute(&["rev-parse", "HEAD"], "get commit hash");

        let commit_hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Oid::from_str(&commit_hash).unwrap()
    }

    pub fn branch(&self, name: &str) {
        self.execute(&["branch", name], &format!("branch {name}"));
        self.checkout(name);
    }

    pub fn checkout(&self, name: &str) {
        self.execute(&["checkout", name], &format!("checkout {name}"));
    }

    pub fn merge(&self, name: &str) {
        self.execute(&["merge", "--no-ff", name], &format!("merge {name}"));
    }

    pub fn tag(&self, name: &str) {
        self.execute(&["tag", name], &format!("create tag {name}"));
    }

    pub fn tag_annotated(&self, name: &str) {
        self.execute(&["tag", "-a", name, "-m", name], &format!("create tag {name}"));
    }

    pub fn graph(&self) -> String {
        let output = self.execute(&["log", "--graph", "--oneline", "--all", "--decorate"], "get commit graph");
        let raw = String::from_utf8_lossy(&output.stdout).to_string();

        let re = Regex::new(r"\b[[:xdigit:]]{7}\b").unwrap();
        re.replace_all(&raw, "##SHA##").to_string()
    }

    fn execute(&self, command: &[&str], description: &str) -> Output {
        let output = std::process::Command::new("git")
            .args(command)
            .current_dir(&self.path)
            .output()
            .expect(&format!("Failed to {description}"));

        if !output.status.success(){
            let error = String::from_utf8_lossy(&output.stderr);
            panic!("Failed to {description}, because: {error}")
        }
        output
    }

    pub fn create_default_toml_config(&self) -> PathBuf {
        self.create_toml_config(".git-versioner.toml")
    }

    pub fn create_toml_config(&self, filename: &str) -> PathBuf {
        let content = toml::to_string(&self.cli_config).expect("Failed to serialize config to TOML");
        self.write_config(filename, content)
    }

    pub fn create_default_yaml_config(&self) -> PathBuf {
        self.create_yaml_config(".git-versioner.yaml")
    }

    pub fn create_yaml_config(&self, filename: &str) -> PathBuf {
        let content = serde_yaml::to_string(&self.cli_config).expect("Failed to serialize config to YAML");
        self.write_config(filename, content)
    }

    fn write_config(&self, filename: &str, toml_content: String) -> PathBuf {
        let config_path = self.path.join(filename);
        fs::write(&config_path, toml_content).expect("Failed to write TOML config file");
        config_path
    }
}