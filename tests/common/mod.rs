use git_versioner::config::{Configuration, DefaultConfig};
use git_versioner::{GitVersion, GitVersioner};
use rstest::fixture;
use std::cell::RefCell;
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

#[allow(dead_code)]
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

    pub fn merge(&self, name: &str) {
        self.execute(&["merge", "--no-ff", name], &format!("merge {name}"));
    }

    pub fn tag_annotated(&self, name: &str) {
        self.execute(&["tag", "-a", name, "-m", name],&format!("create tag {name}"));
    }

    pub fn commit_and_assert(&self, expected: &str) -> Assertable {
        self.commit(expected);
        self.assert().full_sem_ver(expected)
    }

    pub fn tag_and_assert(&self, prefix: &str, expected: &str) -> Assertable {
        self.tag(&format!("{prefix}{expected}"));
        self.assert().full_sem_ver(expected)
    }

    pub fn tag_annotated_and_assert(&self, prefix: &str, expected_version: &str) -> Assertable {
        self.tag_annotated(&format!("{prefix}{expected_version}"));
        self.assert().full_sem_ver(expected_version)
    }

    pub fn merge_and_assert(&self, branch_name: &str, expected_version: &str) -> Assertable {
        self.merge(branch_name);
        self.assert().full_sem_ver(expected_version)
    }

    pub fn path(&self) -> &str {
        self.config.path.to_str().unwrap()
    }

    pub fn clone(source: &TestRepo) -> Self {
        let repo = TestRepo::new();
        repo.execute(
            &["clone", &format!(r"file://{}", source.path()), repo.path()],
            &format!("clone {}", source.path()),
        );
        repo
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

pub struct VisualizableRepo {
    test_repo: TestRepo,
    mermaid: RefCell<Vec<String>>,
}

#[allow(dead_code)]
impl VisualizableRepo {
    pub fn initialize(main_branch: &str) -> Self {
        Self {
            test_repo: TestRepo::initialize(main_branch),
            mermaid: RefCell::new(vec![format!(r#"---
config:
  theme: default
  gitGraph:
    mainBranchName: "{main_branch}"
---
gitGraph:
   checkout "{main_branch}""#)]),
        }
    }
    
    pub fn config(&mut self) -> &mut TestConfig {
        &mut self.test_repo.config
    }

    pub fn commit_and_assert(&self, message: &str) -> Assertable {
        self.mermaid
            .borrow_mut()
            .push(format!("   commit id: \"{}\"", message.replace('"', "'")));
        self.test_repo.commit_and_assert(message)
    }

    pub fn commit_with_tag_and_assert(&self, message: &str, prefix: &str, expected: &str) -> Assertable {
        self.mermaid
            .borrow_mut()
            .push(format!("   commit id: \"{}\" tag: \"{}{}\"",
                          message.replace('"', "'"),
                          prefix.replace('"', "'"),
                          expected.replace('"', "'")));
        self.test_repo.commit_and_assert(message);
        self.test_repo.tag_and_assert(prefix, expected)
    }

    pub fn branch(&self, name: &str) {
        self.mermaid.borrow_mut().push(format!("   branch \"{name}\""));
        self.test_repo.branch(name);
    }

    pub fn checkout(&self, name: &str) {
        self.mermaid
            .borrow_mut()
            .push(format!("   checkout \"{name}\""));
        self.test_repo.checkout(name);
    }

    pub fn merge_and_assert(&self, name: &str, expected_version: &str) -> Assertable {
        self.mermaid.borrow_mut().push(format!("   merge \"{name}\" id: \"{}\"",
                                               expected_version.replace('"', "'")));
        self.test_repo.merge_and_assert(name, expected_version)
    }

    pub fn merge_with_tag_and_assert(&self, name: &str, expected_version: &str, prefix: &str, expected: &str) -> Assertable {
        self.mermaid.borrow_mut().push(format!("   merge \"{name}\" id: \"{}\" tag: \"{}{}\"",
                                               expected_version.replace('"', "'"),
                                               prefix.replace('"', "'"),
                                               expected.replace('"', "'")));
        self.test_repo.merge_and_assert(name, expected_version);
        self.test_repo.tag_and_assert(prefix, expected)
    }

    pub fn draw(&self) -> String {
        self.mermaid.borrow().join("\n")
    }

    pub fn write_markdown(&self, file: &str, function: &str) {
        let path = std::path::Path::new(file);
        let file_name = path.file_stem().unwrap().to_str().unwrap();
        let content = std::fs::read_to_string(path).unwrap();
        let lines: Vec<&str> = content.lines().collect();

        let mut doc_comment = Vec::new();
        for i in 0..lines.len() {
            if lines[i].contains(&format!("fn {function}(")) {
                for j in (0..i).rev() {
                    let line = lines[j].trim();
                    if line.starts_with("///") {
                        doc_comment.push(line[3..].trim());
                    } else if line.starts_with("#[") {
                        continue;
                    } else {
                        break;
                    }
                }
                break;
            }
        }
        doc_comment.reverse();
        let doc_comment = doc_comment.join("\n");
        let mermaid = self.draw();
        let markdown = doc_comment.replace("%%MERMAID%%", &format!("```mermaid\n{}\n```", mermaid));

        let docs_dir = std::path::Path::new("docs");
        if !docs_dir.exists() {
            std::fs::create_dir_all(docs_dir).unwrap();
        }

        let output_file = docs_dir.join(format!("{}_{}.md", file_name, function));
        std::fs::write(output_file, markdown).unwrap();
    }
}

pub struct Assertable {
    pub result: GitVersion,
    pub context: String,
}


macro_rules! config_assertion {
    ($name:ident, &$expected:ty) => {
        pub fn $name(self, expected: &$expected) -> Self {
            let actual = &self.result.$name;
            let context = &self.context;
            let name = stringify!($name);
            assert_eq!(
                actual, expected,
                "Expected {name}: {expected}, found: {actual}\n{context}",
            );
            self
        }
    };
    ($name:ident, $expected:ty) => {
        pub fn $name(self, expected: $expected) -> Self {
            let actual = self.result.$name;
            let context = &self.context;
            let name = stringify!($name);
            assert_eq!(
                actual, expected,
                "Expected {name}: {expected}, found: {actual}\n{context}",
            );
            self
        }
    };
}


#[allow(dead_code)]
impl Assertable {
    config_assertion!(full_sem_ver, &str);
    config_assertion!(branch_name, &str);
    config_assertion!(escaped_branch_name, &str);
    config_assertion!(weighted_pre_release_number, u64);
    config_assertion!(assembly_sem_ver, &str);
    config_assertion!(assembly_sem_file_ver, &str);
    config_assertion!(sha, &str);
    config_assertion!(short_sha, &str);
    config_assertion!(version_source_sha, &str);
}
