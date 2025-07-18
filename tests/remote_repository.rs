mod common;

use common::{MAIN_BRANCH, TestRepo};
use rstest::{fixture, rstest};

impl TestRepo {
    pub fn path(&self) -> &str {
        self.path.to_str().unwrap()
    }

    pub fn clone(source: TestRepo) -> Self {
        let repo = TestRepo::new();
        repo.execute(
            &["clone", &format!(r"file://{}", source.path()), repo.path()],
            &format!("clone {}", source.path()),
        );
        repo
    }
}

#[fixture]
fn repo() -> TestRepo {
    TestRepo::initialize(MAIN_BRANCH)
}

#[rstest]
fn test_feature_branch_inherits_remote_main_branch_base_version(repo: TestRepo) {
    repo.commit("0.1.0-pre.1");
    repo.tag("v1.0.0");
    repo.branch("feature/feature");
    repo.commit("1.1.0-feature.1");

    let clone = TestRepo::clone(repo);
    clone.checkout("feature/feature");
    clone.assert_version("1.1.0-feature.1");
}

#[rstest]
fn test_feature_branch_inherits_remote_release_branch_base_version(repo: TestRepo) {
    repo.commit("0.1.0-pre.1");
    repo.branch("release/1.0.0");
    repo.commit("1.0.0-pre.1");
    repo.branch("feature/feature");
    repo.commit("1.0.0-feature.1");

    let clone = TestRepo::clone(repo);
    clone.checkout("feature/feature");
    clone.assert_version("1.0.0-feature.1");
}

#[rstest]
fn test_main_branch_considers_remote_release_branches_as_base_version(repo: TestRepo) {
    repo.commit("0.1.0-pre.1");
    repo.branch("release/1.0.0");
    repo.commit("1.0.0-pre.1");
    repo.checkout(MAIN_BRANCH);
    repo.commit("1.1.0-pre.1");

    let clone = TestRepo::clone(repo);
    clone.checkout(MAIN_BRANCH);
    clone.assert_version("1.1.0-pre.1");
}
