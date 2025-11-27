mod cli;
mod common;

use crate::cli::{ConfiguredTestRepo, repo};
use rstest::rstest;

#[rstest]
fn test_release_candidate_on_main_branch(mut repo: ConfiguredTestRepo) {
    repo.execute_and_verify([], None);
}

#[rstest]
fn test_release_tag_on_main_branch(mut repo: ConfiguredTestRepo) {
    repo.inner.tag("0.1.0");
    repo.execute_and_verify([], None);
}

#[rstest]
fn test_release_request_on_main_branch(mut repo: ConfiguredTestRepo) {
    repo.inner.config.as_release = true;
    repo.execute_and_verify(["--as-release"], None);
}

#[rstest]
fn test_release_on_main_branch_with_custom_version_pattern(mut repo: ConfiguredTestRepo) {
    repo.inner.tag("my/v0.1.0");

    repo.inner.config.tag_prefix = "my/v".to_string();
    repo.execute_and_verify(["--tag-prefix", "my/v"], None);
}

#[rstest]
fn test_release_branch_with_custom_pattern(mut repo: ConfiguredTestRepo) {
    repo.inner.tag("v1.0.0");
    repo.inner.branch("custom-release/1.0.0");
    repo.inner.commit("1.0.1+1");

    repo.inner.config.release_branch = "custom-release/(?<BranchName>.*)".to_string();
    repo.execute_and_verify(
        ["--release-branch", "custom-release/(?<BranchName>.*)"],
        None,
    );
}

#[rstest]
fn test_feature_branch_with_custom_pattern(mut repo: ConfiguredTestRepo) {
    repo.inner.commit("0.1.0+1");
    repo.inner.branch("my-feature/feature");
    repo.inner.commit("0.1.0-feature.1");

    repo.inner.config.feature_branch = "my-feature/(?<BranchName>.*)".to_string();
    repo.execute_and_verify(["--feature-branch", "my-feature/(?<BranchName>.*)"], None);
}

#[rstest]
fn test_option_custom_main_branch(#[with("custom-main")] mut repo: ConfiguredTestRepo) {
    repo.inner.config.main_branch = "custom-main".to_string();
    repo.execute_and_verify(["--main-branch", "custom-main"], None);
}

#[rstest]
fn test_option_custom_repository_path(mut repo: ConfiguredTestRepo) {
    let path = repo.inner.config.path.to_string_lossy().to_string();
    repo.execute_and_verify(["--path", &path], None);
}

#[rstest]
fn test_argument_prerelease_tag(mut repo: ConfiguredTestRepo) {
    repo.inner.config.pre_release_tag = "alpha".to_string();
    repo.execute_and_verify(["--pre-release-tag", "alpha"], None);
}
