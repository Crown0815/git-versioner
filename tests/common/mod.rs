use git_versioner::config::{Configuration, DefaultConfig};
use git_versioner::{GitVersion, GitVersioner};
use rstest::fixture;
use std::path::PathBuf;
use std::process::{Command, Output};

pub const MAIN_BRANCH: &str = "trunk";

#[fixture]
pub fn repo(#[default(MAIN_BRANCH)] main: &str) -> TestRepo {
    let repo = TestRepo::initialize(main);
    repo.commit("0.1.0-pre.1");
    repo
}

pub struct TestRepo {
    pub config: TestConfig,
    _temp_dir: tempfile::TempDir, // Keep the temp_dir to prevent it from being deleted
}

pub struct TestConfig {
    pub path: PathBuf,
    pub main_branch: String,
    pub release_branch: String,
    pub feature_branch: String,
    pub tag_prefix: String,
    pub pre_release_tag: String,
    pub commit_message_incrementing: String,
    pub continuous_delivery: bool,
    pub as_release: bool,
}

macro_rules! config_getter {
    ($name:ident, $return:ty) => {
        fn $name(&self) -> &$return {
            &self.$name
        }
    };
}

impl Configuration for TestConfig {
    config_getter!(path, PathBuf);
    config_getter!(main_branch, str);
    config_getter!(release_branch, str);
    config_getter!(feature_branch, str);
    config_getter!(tag_prefix, str);
    config_getter!(pre_release_tag, str);
    config_getter!(commit_message_incrementing, str);
    config_getter!(continuous_delivery, bool);
    config_getter!(as_release, bool);
}

impl Default for TestConfig {
    fn default() -> Self {
        let default = DefaultConfig::default();
        Self {
            path: default.path,
            main_branch: default.main_branch,
            release_branch: default.release_branch,
            feature_branch: default.feature_branch,
            tag_prefix: default.tag_prefix,
            pre_release_tag: default.pre_release_tag,
            commit_message_incrementing: default.commit_message_incrementing,
            continuous_delivery: default.continuous_delivery,
            as_release: false,
        }
    }
}

impl TestRepo {
    pub fn new() -> Self {
        let _temp_dir = tempfile::tempdir().unwrap();
        let path = _temp_dir.path().to_path_buf();
        let config = TestConfig {
            path,
            ..Default::default()
        };
        Self { config, _temp_dir }
    }

    pub fn initialize(main_branch: &str) -> Self {
        let repo = TestRepo::new();
        repo.execute(
            &["init", &format!("--initial-branch={main_branch}")],
            "initialize repository",
        );
        repo.execute(&["config", "user.name", "tester"], "configure user.name");
        repo.execute(
            &["config", "user.email", "tester@tests.com"],
            "configure user.email",
        );
        repo
    }

    pub fn commit(&self, message: &str) -> (String, String) {
        self.execute(
            &["commit", "--allow-empty", "-m", message],
            &format!("commit {message}"),
        );
        self.read_head_sha_and_date()
    }

    pub fn branch(&self, name: &str) {
        self.execute(&["branch", name], &format!("branch {name}"));
        self.checkout(name);
    }

    pub fn checkout(&self, name: &str) {
        self.execute(&["checkout", name], &format!("checkout {name}"));
    }

    pub fn tag(&self, name: &str) -> (String, String) {
        self.execute(&["tag", name], &format!("create tag {name}"));
        self.read_head_sha_and_date()
    }

    pub fn graph(&self) -> String {
        let output = self.execute(
            &["log", "--graph", "--oneline", "--all", "--decorate"],
            "get commit graph",
        );
        String::from_utf8_lossy(&output.stdout).to_string()
    }

    pub fn execute(&self, command: &[&str], description: &str) -> Output {
        let output = Command::new("git")
            .args(command)
            .current_dir(&self.config.path)
            .output()
            .unwrap_or_else(|_| panic!("Failed to {description}"));

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            panic!("Failed to {description}, because: {error}")
        }
        output
    }

    pub fn assert(&self) -> Assertable {
        let result = GitVersioner::calculate_version(&self.config).unwrap();
        let context = format!("Git Graph:\n  {}", self.graph());
        Assertable { result, context }
    }

    fn read_head_sha_and_date(&self) -> (String, String) {
        let output = self.execute(&["rev-parse", "HEAD"], "get commit hash");
        let commit_sha = String::from_utf8_lossy(&output.stdout).trim().to_string();

        let output = self.execute(
            &["log", "-1", "--format=%cd", "--date=format:%Y-%m-%d"],
            "get commit date",
        );
        let commit_date = String::from_utf8_lossy(&output.stdout).trim().to_string();

        (commit_sha, commit_date)
    }
}

pub struct Assertable {
    pub result: GitVersion,
    pub context: String,
}

impl Assertable {
    pub fn version(self, expected: &str) -> Self {
        let actual = &self.result.full_sem_ver;
        assert_eq!(
            actual, expected,
            "Expected version: {expected}, found: {actual}\n{}",
            self.result,
        );
        self
    }

    pub fn branch_name(self, expected: &str) -> Self {
        let actual = &self.result.branch_name;
        assert_eq!(
            actual, expected,
            "Expected branch_name: {expected}, found: {actual}\n{}",
            self.context
        );
        self
    }

    pub fn escaped_branch_name(self, expected: &str) -> Self {
        let actual = &self.result.escaped_branch_name;
        assert_eq!(
            actual, expected,
            "Expected escaped_branch_name: {expected}, found: {actual}\n{}",
            self.context
        );
        self
    }

    pub fn weighted_pre_release_number(self, expected: u64) -> Self {
        let actual = self.result.weighted_pre_release_number;
        assert_eq!(
            actual, expected,
            "Expected weighted_pre_release_number: {expected}, found: {actual}\n{}",
            self.context
        );
        self
    }

    pub fn assembly_sem_ver(self, expected: &str) -> Self {
        let actual = &self.result.assembly_sem_ver;
        assert_eq!(
            actual, expected,
            "Expected assembly_sem_ver: {expected}, found: {actual}\n{}",
            self.context
        );
        self
    }

    pub fn assembly_sem_file_ver(self, expected: &str) -> Self {
        let actual = &self.result.assembly_sem_file_ver;
        assert_eq!(
            actual, expected,
            "Expected assembly_sem_file_ver: {expected}, found: {actual}\n{}",
            self.context
        );
        self
    }

    pub fn sha(self, expected: &str) -> Self {
        let actual = &self.result.sha;
        assert_eq!(
            actual, expected,
            "Expected sha: {expected}, found: {actual}\n{}",
            self.context
        );
        self
    }

    pub fn short_sha(self, expected: &str) -> Self {
        let actual = &self.result.short_sha;
        assert_eq!(
            actual, expected,
            "Expected short_sha: {expected}, found: {actual}\n{}",
            self.context
        );
        self
    }

    pub fn version_source_sha(self, expected: &str) -> Self {
        let actual = &self.result.version_source_sha;
        assert_eq!(
            actual, expected,
            "Expected version_source_sha: {expected}, found: {actual}\n{}",
            self.context
        );
        self
    }
}
