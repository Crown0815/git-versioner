mod common;

use common::{MAIN_BRANCH, TestRepo};
use git_versioner::{GitVersioner, NO_BRANCH_NAME};
use rstest::{fixture, rstest};

impl TestRepo {
    fn assert_branch(&self, expected_name: &str, expected_escaped_name: &str) {
        let actual = GitVersioner::calculate_version(&self.config).unwrap();
        assert_eq!(actual.branch_name, expected_name.to_string());
        assert_eq!(
            actual.escaped_branch_name,
            expected_escaped_name.to_string()
        );
    }
}

#[fixture]
fn repo() -> TestRepo {
    TestRepo::initialize(MAIN_BRANCH)
}

#[rstest]
fn test_that_no_branch_name_is_no_branch_in_parenthesis() {
    assert_eq!(NO_BRANCH_NAME, "(no branch)");
}

#[rstest]
fn test_result_on_detached_head_is_no_branch(repo: TestRepo) {
    let (sha, _) = repo.commit("commit");
    repo.checkout(&sha);

    repo.assert_branch(NO_BRANCH_NAME, "-no-branch-");
}

#[rstest]
fn test_result_on_main_branch(repo: TestRepo) {
    repo.commit("commit");
    repo.assert_branch(MAIN_BRANCH, MAIN_BRANCH);
}
