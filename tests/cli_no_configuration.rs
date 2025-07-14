mod common;

use common::{TestRepo, cli, repo};
use insta::with_settings;
use insta_cmd::assert_cmd_snapshot;
use rstest::rstest;
use std::process::Command;

#[rstest]
fn test_release_candidate_on_main_branch(repo: TestRepo, mut cli: Command) {
    assert_repo_cmd_snapshot!(repo, cli.current_dir(&repo.path));
}

#[rstest]
fn test_release_on_main_branch(repo: TestRepo, mut cli: Command) {
    repo.tag("0.1.0");
    assert_repo_cmd_snapshot!(repo, cli.current_dir(&repo.path));
}

#[rstest]
fn test_release_on_main_branch_with_custom_version_pattern(repo: TestRepo, mut cli: Command) {
    repo.tag("my/v0.1.0");
    assert_repo_cmd_snapshot!(
        repo,
        cli.current_dir(&repo.path)
            .args(&["--version-pattern", "my/v(?<Version>.*)"])
    );
}

#[rstest]
fn test_release_branch_with_custom_pattern(repo: TestRepo, mut cli: Command) {
    repo.commit("0.1.0-rc.1");
    repo.tag("v1.0.0");
    repo.branch("custom-release/1.0.0");
    repo.commit("1.0.1-rc.1");
    assert_repo_cmd_snapshot!(
        repo,
        cli.current_dir(&repo.path)
            .args(&["--release-branch", "custom-release/(?<BranchName>.*)"])
    );
}

#[rstest]
fn test_feature_branch_with_custom_pattern(repo: TestRepo, mut cli: Command) {
    repo.commit("0.1.0-rc.1");
    repo.branch("my-feature/feature");
    repo.commit("0.1.0-feature.1");
    assert_repo_cmd_snapshot!(
        repo,
        cli.current_dir(&repo.path)
            .args(&["--feature-branch", "my-feature/(?<BranchName>.*)"])
    );
}

#[rstest]
fn test_option_custom_main_branch(#[with("custom-main")] repo: TestRepo, mut cli: Command) {
    assert_repo_cmd_snapshot!(
        repo,
        cli.current_dir(&repo.path)
            .args(&["--main-branch", "custom-main"])
    );
}

#[rstest]
fn test_option_custom_repository_path(repo: TestRepo, mut cli: Command) {
    assert_repo_cmd_snapshot!(
        repo,
        cli.current_dir(".")
            .args(&["--path", repo.path.to_str().unwrap()])
    );
}

#[rstest]
fn test_help_text(mut cli: Command) {
    assert_cmd_snapshot!(cli.current_dir(".").args(&["--help"]));
}
