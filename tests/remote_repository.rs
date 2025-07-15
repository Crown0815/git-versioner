mod common;

use crate::common::{MAIN_BRANCH, TestRepo};
use git_versioner::{GitVersioner, NO_BRANCH_NAME};
use rstest::{fixture, rstest};
use semver::Version;

impl TestRepo {
    fn assert_version(&self, expected: &str) {
        let actual = GitVersioner::calculate_version(&self.config).unwrap();
        let expected = Version::parse(expected).unwrap();
        let graph = self.graph();
        assert_eq!(
            actual, expected,
            "Expected HEAD version: {expected}, found: {actual}\n\n Git Graph:\n-------\n{}------",
            graph
        );
    }
}

#[fixture]
fn repo() -> TestRepo {
    TestRepo::initialize(MAIN_BRANCH)
}

#[rstest]
fn test_feature_branch_inherits_remote_main_branch_base_version(repo: TestRepo) {
    repo.commit("0.1.0-rc.1");
    repo.tag("v1.0.0");
    repo.branch("feature/feature");
    repo.commit("1.1.0-feature.1");

    let clone = TestRepo::clone(repo);
    clone.checkout("feature/feature");
    clone.assert_version("1.1.0-feature.1")
}

#[rstest]
fn test_feature_branch_inherits_remote_release_branch_base_version(repo: TestRepo) {
    repo.commit("0.1.0-rc.1");
    repo.branch("release/1.0.0");
    repo.commit("1.0.0-rc.1");
    repo.branch("feature/feature");
    repo.commit("1.0.0-feature.1");

    let clone = TestRepo::clone(repo);
    clone.checkout("feature/feature");
    clone.assert_version("1.0.0-feature.1")
}

#[rstest]
fn test_main_branch_considers_remote_release_branches_as_base_version(repo: TestRepo) {
    repo.commit("0.1.0-rc.1");
    repo.branch("release/1.0.0");
    repo.commit("1.0.0-rc.1");
    repo.checkout(MAIN_BRANCH);
    repo.commit("1.1.0-rc.1");

    let clone = TestRepo::clone(repo);
    clone.checkout(MAIN_BRANCH);
    clone.assert_version("1.1.0-rc.1")
}
