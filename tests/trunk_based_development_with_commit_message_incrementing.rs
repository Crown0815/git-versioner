mod common;

use crate::common::{MAIN_BRANCH, TestRepo};
use rstest::{fixture, rstest};

#[fixture]
fn repo(#[default(MAIN_BRANCH)] main_branch: &str) -> TestRepo {
    let mut repo = TestRepo::initialize(main_branch);
    repo.config.commit_message_incrementing = "Enabled".to_string();
    repo
}

#[rstest]
#[should_panic(
    expected = r#"Invalid value "foo" for CommitMessageIncrementing. Should be "Enabled" or "Disabled"."#
)]
fn test_providing_non_disabled_or_enabled_string_to_commit_message_incrementing_panics(
    mut repo: TestRepo,
) {
    repo.config.commit_message_incrementing = "foo".to_string();
    repo.commit_and_assert("0.0.1-pre.1")
        .branch_name(MAIN_BRANCH)
        .version_source_sha("");
}

#[rstest]
fn test_on_main_branch_starts_with_version_0_1_0(repo: TestRepo) {
    repo.commit_and_assert("0.1.0-pre.1");
}

#[rstest]
fn test_on_main_branch_when_encountering_feature_commit_bumps_minor_version(repo: TestRepo) {
    repo.commit("feat: foo");
    repo.commit_and_assert("0.1.0-pre.2");
}

#[rstest]
fn test_after_feature_release_tag_on_main_branch_only_bumps_patch_version(repo: TestRepo) {
    repo.commit_and_assert("0.1.0-pre.1");
    repo.tag_and_assert("v", "1.0.0");
    repo.commit_and_assert("1.0.1-pre.1");
}

#[rstest]
fn test_after_a_feature_release_when_encountering_feature_commit_on_main_branch_bumps_minor_version(
    repo: TestRepo,
) {
    repo.commit_and_assert("0.1.0-pre.1");
    repo.tag_and_assert("v", "1.0.0");
    repo.commit("feat: foo");
    repo.commit_and_assert("1.1.0-pre.2");
}

#[rstest]
fn test_after_patch_release_tag_on_main_branch_only_bumps_patch_version(repo: TestRepo) {
    repo.commit_and_assert("0.1.0-pre.1");
    repo.tag_and_assert("v", "1.0.1");
    repo.commit_and_assert("1.0.2-pre.1");
}

#[rstest]
fn test_after_a_patch_release_when_encountering_feature_commit_on_main_branch_bumps_minor_version(
    repo: TestRepo,
) {
    repo.commit_and_assert("0.1.0-pre.1");
    repo.tag_and_assert("v", "1.0.1");
    repo.commit("feat: foo");
    repo.commit_and_assert("1.1.0-pre.2");
}

#[rstest]
fn test_on_main_branch_with_major_version_zero_when_encountering_breaking_change_commit_bumps_minor_version(
    repo: TestRepo,
) {
    repo.commit("fix!: foo");
    repo.commit_and_assert("0.1.0-pre.2");
}

#[rstest]
fn test_on_main_branch_with_major_version_zero_when_encountering_commit_with_breaking_change_footer_bumps_minor_version(
    repo: TestRepo,
) {
    repo.commit("fix: foo\n\nBody\n\nBREAKING CHANGE: bar");
    repo.commit_and_assert("0.1.0-pre.2");
}

#[rstest]
fn test_on_main_branch_with_major_version_greater_than_zero_when_encountering_breaking_change_commit_bumps_major_version(
    repo: TestRepo,
) {
    repo.commit_and_assert("0.1.0-pre.1");
    repo.tag_and_assert("v", "1.0.0");
    repo.commit("fix!: foo");
    repo.commit_and_assert("2.0.0-pre.2");
}

#[rstest]
fn test_on_main_branch_with_major_version_greater_than_zero_when_encountering_commit_with_breaking_change_footer_bumps_major_version(
    repo: TestRepo,
) {
    repo.commit_and_assert("0.1.0-pre.1");
    repo.tag_and_assert("v", "1.0.0");
    repo.commit("fix: foo\n\nBody\n\nBREAKING CHANGE: bar");
    repo.commit_and_assert("2.0.0-pre.2");
}
