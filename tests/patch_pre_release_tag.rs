mod common;

use crate::common::{MAIN_BRANCH, TestRepo};
use rstest::{fixture, rstest};

#[fixture]
fn repo(#[default(MAIN_BRANCH)] main_branch: &str) -> TestRepo {
    let mut repo = TestRepo::initialize(main_branch);
    repo.config.pre_release_tag = "pre".to_string();
    repo.config.commit_message_incrementing = "Enabled".to_string();
    repo.config.continuous_delivery = true;
    repo.commit("0.1.0+1");
    repo
}

#[rstest]
fn test_that_patch_version_uses_normal_pre_release_tag_if_patch_pre_release_tag_is_unspecified(
    repo: TestRepo,
) {
    repo.tag("v0.1.0");
    repo.commit("0.1.1+1");

    repo.assert().full_sem_ver("0.1.1-pre.1");
}

#[rstest]
fn test_that_patch_version_uses_patch_pre_release_tag_if_specified(mut repo: TestRepo) {
    repo.config.patch_pre_release_tag = "fix".to_string();

    repo.tag("v0.1.0");
    repo.commit("0.1.1+1");

    repo.assert().full_sem_ver("0.1.1-fix.1");
}

#[rstest]
fn test_that_minor_bump_uses_normal_pre_release_tag_even_if_patch_pre_release_tag_is_specified(
    mut repo: TestRepo,
) {
    repo.config.patch_pre_release_tag = "fix".to_string();

    repo.tag("v0.1.0");
    repo.commit("feat: new feature");

    repo.assert().full_sem_ver("0.2.0-pre.1");
}

#[rstest]
fn test_that_patch_pre_release_tag_increments_correctly_in_continuous_delivery(mut repo: TestRepo) {
    repo.config.patch_pre_release_tag = "fix".to_string();
    repo.tag("v0.1.0");

    repo.commit("fix: first fix");
    repo.assert().full_sem_ver("0.1.1-fix.1");

    repo.tag("v0.1.1-fix.1");
    repo.commit("fix: second fix");

    repo.assert().full_sem_ver("0.1.1-fix.2");
}

#[rstest]
fn test_that_patch_pre_release_tag_works_on_release_branch(mut repo: TestRepo) {
    repo.config.patch_pre_release_tag = "patch".to_string();

    repo.tag("v1.0.0");
    repo.branch("release/1.0.0");
    repo.commit("1.0.1-pre.1");

    repo.assert().full_sem_ver("1.0.1-patch.1");
}
