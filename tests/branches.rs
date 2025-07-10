mod common;

use git_versioner::NO_BRANCH_NAME;
use rstest::rstest;

#[rstest]
fn test_that_no_branch_name_is_no_branch_in_parenthesis() {
    assert_eq!(NO_BRANCH_NAME, "(no branch)");
}