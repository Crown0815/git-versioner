use git2::Oid;
use git_versioner::*;
use rstest::{fixture, rstest};
use semver::Version;
use std::path::PathBuf;
use std::process::Output;

struct TestRepo {
    path: PathBuf,
    _temp_dir: tempfile::TempDir, // Keep the temp_dir to prevent it from being deleted
}

impl TestRepo {
    fn new() -> Self {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().to_path_buf();
        Self { path, _temp_dir: temp_dir }
    }

    fn initialize(&self) {
        self.execute(&["init", "--initial-branch=trunk"], "initialize repository");
        self.execute(&["config", "user.name", "tester"], "configure user.name");
        self.execute(&["config", "user.email", "tester@tests.com"], "configure user.email");
    }

    fn commit(&self, message: &str) -> Oid {
        self.execute(&["commit", "--allow-empty", "-m", message], &format!("commit {message}"));
        let output = self.execute(&["rev-parse", "HEAD"], "get commit hash");

        let commit_hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Oid::from_str(&commit_hash).unwrap()
    }

    fn branch(&self, name: &str) {
        self.execute(&["branch", name], &format!("branch {name}"));
        self.checkout(name);
    }

    fn checkout(&self, name: &str) {
        self.execute(&["checkout", name], &format!("checkout {name}"));
    }

    fn merge(&self, name: &str) {
        self.execute(&["merge", "--no-ff", name], &format!("merge {name}"));
    }

    fn tag(&self, name: &str) {
        self.execute(&["tag", name], &format!("create tag {name}"));
    }

    fn graph(&self) -> String {
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

    fn commit_and_assert(&self, expected_version: &str) {
        self.commit(expected_version);
        assert_version(&self, expected_version);
    }

    fn tag_and_assert(&self, prefix: &str, expected_version: &str) {
        self.tag(&format!("{}{}", prefix, expected_version));
        assert_version(&self, expected_version);
    }

    fn merge_and_assert(&self, branch_name: &str, expected_version: &str) {
        self.merge(branch_name);
        assert_version(&self, expected_version);
    }
}

#[fixture]
fn repo() -> TestRepo {
    let repo = TestRepo::new();
    repo.initialize();
    repo
}

#[rstest]
fn test_full_workflow(repo: TestRepo) {
    repo.commit_and_assert("0.1.0-rc.1");
    repo.commit_and_assert("0.1.0-rc.2");
    repo.tag("v1.0.0-rc.2"); // ignored
    repo.tag_and_assert("v", "1.0.0");
    repo.branch("release/1.0.0");

    repo.checkout("trunk");
    repo.commit_and_assert("1.1.0-rc.1");

    repo.checkout("release/1.0.0");
    repo.commit_and_assert("1.0.1-rc.1");
    repo.commit_and_assert("1.0.1-rc.2");
    repo.tag_and_assert("v", "1.0.1");
    repo.commit_and_assert("1.0.2-rc.1");
    repo.commit_and_assert("1.0.2-rc.2");
    repo.tag_and_assert("v", "1.0.2");

    repo.checkout("trunk");
    repo.commit_and_assert("1.1.0-rc.2");
    repo.branch("release/1.1.0");
    repo.checkout("trunk");
    repo.commit_and_assert("1.2.0-rc.1");

    repo.checkout("release/1.1.0");
    repo.commit_and_assert("1.1.0-rc.3");
    repo.commit_and_assert("1.1.0-rc.4");
    repo.tag_and_assert("v", "1.1.0");
    repo.commit_and_assert("1.1.1-rc.1");
    repo.commit_and_assert("1.1.1-rc.2");
    repo.tag_and_assert("v", "1.1.1");

    repo.checkout("trunk");
    repo.commit_and_assert("1.2.0-rc.2");
    repo.branch("release/1.2.0");
    repo.checkout("trunk");
    repo.commit_and_assert("1.3.0-rc.1");

    repo.checkout("release/1.2.0");
    repo.commit_and_assert("1.2.0-rc.3");
    repo.commit_and_assert("1.2.0-rc.4");
    repo.tag_and_assert("v", "1.2.0");
    repo.commit_and_assert("1.2.1-rc.1");
    repo.commit_and_assert("1.2.1-rc.2");
    repo.tag_and_assert("v", "1.2.1");

    repo.checkout("trunk");
    repo.commit_and_assert("1.3.0-rc.2");
    repo.tag_and_assert("v", "1.3.0");
    repo.commit_and_assert("1.4.0-rc.1");
}

#[rstest]
fn test_full_workflow_with_feature_branches(repo: TestRepo) {
    repo.commit("0.1.0-rc.1");
    repo.branch("feature/feature1");
    repo.commit_and_assert("0.1.0-feature1.1");

    repo.checkout("trunk");
    repo.merge_and_assert("feature/feature1", "0.1.0-rc.3");
    repo.tag_and_assert("v", "1.0.0");
    repo.branch("release/1.0.0");

    repo.checkout("trunk");
    repo.branch("feature/feature2");
    repo.commit_and_assert("1.1.0-feature2.1");
    repo.commit_and_assert("1.1.0-feature2.2");

    repo.checkout("trunk");
    repo.merge_and_assert("feature/feature2", "1.1.0-rc.3");

    repo.checkout("release/1.0.0");
    repo.commit_and_assert("1.0.1-rc.1");
    repo.branch("feature/fix1");
    repo.commit_and_assert("1.0.1-fix1.1");
    repo.commit_and_assert("1.0.1-fix1.2");
    repo.checkout("release/1.0.0");
    repo.merge_and_assert("feature/fix1", "1.0.1-rc.4");

    repo.checkout("trunk");
    repo.branch("feature/feature3-1");
    repo.commit_and_assert("1.1.0-feature3-1.1");
    repo.checkout("trunk");
    repo.branch("feature/feature3-2");
    repo.commit_and_assert("1.1.0-feature3-2.1");
    repo.commit_and_assert("1.1.0-feature3-2.2");
    repo.checkout("feature/feature3-1");
    repo.commit_and_assert("1.1.0-feature3-1.2");

    repo.checkout("trunk");
    repo.merge_and_assert("feature/feature3-2", "1.1.0-rc.6");
    repo.merge_and_assert("feature/feature3-1", "1.1.0-rc.9");
}

#[rstest]
fn test_support_of_custom_trunk_pattern(repo: TestRepo) {
    repo.commit("Initial commit");
    repo.branch("custom-trunk");
    repo.execute(&["branch", "-D", "trunk"], "delete trunk branch");

    assert_version(&repo, "0.1.0-custom-trunk.1");
    assert_version_with_custom_trunk(&repo, "0.1.0-rc.1", r"^custom-trunk$");
}

#[rstest]
fn test_tags_with_matching_version_tag_prefix_are_considered(
    repo: TestRepo,
    #[values("v", "V", "")] prefix: &str,
) {
    repo.commit_and_assert("0.1.0-rc.1");
    repo.tag_and_assert(prefix, "1.0.0");
}

#[rstest]
fn test_tags_without_matching_version_tag_prefix_are_ignored(
    repo: TestRepo,
    #[values("a", "x", "p", "vv", "Vv")] prefix: &str,
) {
    repo.commit_and_assert("0.1.0-rc.1");
    repo.tag(&format!("{}1.0.0",  prefix));
    assert_version(&repo, "0.1.0-rc.1");
}

#[rstest]
// These symbols were not tested because they are invalid in branch names:
// '\', '^', '~', ' ', ':', '?', '*', '['
// see https://git-scm.com/docs/git-check-ref-format
fn test_valid_feature_branch_symbols_incompatible_with_semantic_versions_are_replaces_with_dash(
    repo: TestRepo,
    #[values('_', '/', ',', '!', '`', ']', '{', '}', '😁')] incompatible_symbol: char,
) {
    repo.commit("irrelevant");
    repo.branch(&format!("feature/a{}a",  incompatible_symbol));
    repo.commit_and_assert("0.1.0-a-a.1");
}

fn assert_version(repo: &TestRepo, expected: &str) {
    assert_version_with_custom_trunk(repo, expected, TRUNK_BRANCH_REGEX);
}

fn assert_version_with_custom_trunk(repo: &TestRepo, expected: &str, trunk_branch_regex: &str) {
    let actual = GitVersioner::calculate_version(&repo.path, trunk_branch_regex).unwrap();
    let expected = Version::parse(expected).unwrap();
    assert_eq!(
        actual,
        expected,
        "Expected HEAD version: {}, found: {}\n\n Git Graph:\n-------\n{}------",
        expected,
        actual,
        repo.graph());
}