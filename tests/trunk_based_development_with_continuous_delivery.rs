mod common;

use crate::common::{MAIN_BRANCH, TestRepo};
use rstest::{fixture, rstest};

#[fixture]
fn repo(#[default(MAIN_BRANCH)] main_branch: &str) -> TestRepo {
    let mut repo = TestRepo::initialize(main_branch);
    repo.config.commit_message_incrementing = "Disabled".to_string();
    repo.config.continuous_delivery = true;
    repo.commit("0.1.0+1");
    repo
}

#[rstest]
fn test_that_in_continuous_delivery_mode_with_custom_pre_release_tag_when_no_tags_exist_produces_pre_release_tag_1(
    mut repo: TestRepo,
) {
    repo.commit("0.1.0-pre.2");
    repo.config.pre_release_tag = "rc".to_string();

    repo.assert().version("0.1.0-rc.1").version_source_sha("");
}

#[rstest]
fn test_that_in_continuous_delivery_mode_with_custom_pre_release_tag_when_matching_tags_exist_produces_next_pre_release_tag(
    mut repo: TestRepo,
) {
    let (sha, _) = repo.tag("v0.1.0-rc.1");
    repo.commit("0.1.0-pre.2");
    repo.commit("0.1.0-pre.3");
    repo.config.pre_release_tag = "rc".to_string();
    repo.assert().version("0.1.0-rc.2").version_source_sha(&sha);
}

#[rstest]
fn test_that_in_continuous_delivery_mode_with_custom_pre_release_tag_when_non_matching_tags_exist_produces_pre_release_tag_1(
    mut repo: TestRepo,
) {
    repo.tag("v0.1.0-pre.1");
    repo.commit("0.1.0-pre.2");
    repo.config.pre_release_tag = "rc".to_string();

    repo.assert().version("0.1.0-rc.1").version_source_sha("");
}

#[rstest]
fn test_that_in_continuous_delivery_mode_when_no_tags_exist_produces_pre_release_tag_1(
    mut repo: TestRepo,
) {
    repo.commit("0.1.0-pre.2");

    repo.assert().version("0.1.0-pre.1").version_source_sha("");
}

#[rstest]
fn test_that_in_continuous_delivery_mode_when_matching_tags_exist_produces_next_pre_release_tag(
    mut repo: TestRepo,
) {
    let (sha, _) = repo.tag("v0.1.0-pre.1");
    repo.commit("0.1.0-pre.2");
    repo.commit("0.1.0-pre.3");

    repo.assert()
        .version("0.1.0-pre.2")
        .version_source_sha(&sha);
}

#[rstest]
fn test_that_in_continuous_delivery_mode_after_release_when_no_matching_tags_exist_produces_pre_release_tag_1(
    mut repo: TestRepo,
) {
    let (sha, _) = repo.tag("v1.0.0");
    repo.commit("1.1.0-pre.1");
    repo.commit("1.1.0-pre.2");

    repo.assert()
        .version("1.1.0-pre.1")
        .version_source_sha(&sha);
}

#[rstest]
fn test_that_in_continuous_delivery_mode_after_release_when_matching_tags_exist_produces_next_pre_release_tag(
    mut repo: TestRepo,
) {
    repo.tag("v1.0.0");
    repo.commit("1.1.0-pre.1");
    let (sha, _) = repo.tag("v1.1.0-pre.1");
    repo.commit("1.1.0-pre.2");
    repo.commit("1.1.0-pre.3");

    repo.assert()
        .version("1.1.0-pre.2")
        .version_source_sha(&sha);
}

#[rstest]
fn test_that_in_continuous_delivery_mode_on_release_branch_when_no_tags_exist_produces_pre_release_tag_1(
    mut repo: TestRepo,
) {
    repo.branch("release/1.0.0");
    repo.commit("1.0.0-pre.1");
    repo.commit("1.0.0-pre.2");

    repo.assert().version("1.0.0-pre.1").version_source_sha("");
}

#[rstest]
fn test_that_in_continuous_delivery_mode_on_release_branch_when_release_tags_exist_produces_pre_release_tag_1(
    mut repo: TestRepo,
) {
    repo.branch("release/1.0.0");
    repo.commit("1.0.0-pre.1");
    let (sha, _) = repo.tag("v1.0.0");
    repo.commit("1.0.1-pre.1");
    repo.commit("1.0.1-pre.2");

    repo.assert()
        .version("1.0.1-pre.1")
        .version_source_sha(&sha);
}

#[rstest]
fn test_that_in_continuous_delivery_mode_on_release_branch_when_release_branches_exist_produces_pre_release_tag_1(
    mut repo: TestRepo,
) {
    repo.branch("release/1.0.0");
    let (sha, _) = repo.commit("1.0.0-pre.1");
    repo.checkout(MAIN_BRANCH);
    repo.commit("1.1.0-pre.1");
    repo.branch("release/1.1.0");
    repo.commit("1.1.0-pre.2");
    repo.commit("1.1.0-pre.3");

    repo.assert()
        .version("1.1.0-pre.1")
        .version_source_sha(&sha);
}
