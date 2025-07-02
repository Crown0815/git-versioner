use git2::Oid;
use git_versioner::Configuration;
use std::path::PathBuf;
use std::process::Output;

pub struct TestRepo {
    pub path: PathBuf,
    pub config: Configuration,
    _temp_dir: tempfile::TempDir, // Keep the temp_dir to prevent it from being deleted
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

impl TestRepo {
    pub fn new() -> Self {
        let _temp_dir = tempfile::tempdir().unwrap();
        let path = _temp_dir.path().to_path_buf();
        let mut config = Configuration::default();
        config.repo_path = path.clone();
        Self { path, config, _temp_dir }
    }

    pub fn initialize(&self, main_branch: &str) {
        self.execute(&["init", &format!("--initial-branch={main_branch}")], "initialize repository");
        self.execute(&["config", "user.name", "tester"], "configure user.name");
        self.execute(&["config", "user.email", "tester@tests.com"], "configure user.email");
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
        String::from_utf8_lossy(&output.stdout).to_string()
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
}