use git_versioner::{DefaultConfig, GitVersion, GitVersioner};
use git2::Oid;
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
    pub path: PathBuf,
    pub config: DefaultConfig,
    _temp_dir: tempfile::TempDir, // Keep the temp_dir to prevent it from being deleted
}

impl TestRepo {
    pub fn new() -> Self {
        let _temp_dir = tempfile::tempdir().unwrap();
        let path = _temp_dir.path().to_path_buf();
        let config = DefaultConfig {
            path: path.clone(),
            ..Default::default()
        };
        Self {
            path,
            config,
            _temp_dir,
        }
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

    pub fn commit(&self, message: &str) -> Oid {
        self.execute(
            &["commit", "--allow-empty", "-m", message],
            &format!("commit {message}"),
        );
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

    pub fn tag(&self, name: &str) {
        self.execute(&["tag", name], &format!("create tag {name}"));
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
            .current_dir(&self.path)
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
            "Expected branch: {expected}, found: {actual}\n{}",
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

    pub fn source_id(self, expected: Oid) -> Self {
        self.source_sha(&expected.to_string())
    }

    pub fn has_no_source(self) -> Self {
        self.source_sha("")
    }

    pub fn source_sha(self, expected: &str) -> Self {
        let actual = &self.result.version_source_sha;
        assert_eq!(
            actual, expected,
            "Expected source_id: {expected}, found: {actual}\n{}",
            self.context
        );
        self
    }
}
