mod cli;
mod common;

use crate::cli::{ConfiguredTestRepo, cmd, repo};
use crate::common::MAIN_BRANCH;
use insta::assert_snapshot;
use insta_cmd::assert_cmd_snapshot;
use rstest::rstest;
use std::path::PathBuf;
use std::process::Command;

#[rstest]
fn test_help_text(mut cmd: Command) {
    assert_cmd_snapshot!(cmd.current_dir(".").args(["--help"]));
}

#[rstest]
fn test_release_candidate_on_main_branch(mut repo: ConfiguredTestRepo) {
    repo.execute_and_assert([], None);
}

#[rstest]
fn test_release_tag_on_main_branch(mut repo: ConfiguredTestRepo) {
    repo.inner.tag("0.1.0");
    repo.execute_and_assert([], None);
}

#[rstest]
fn test_release_request_on_main_branch(mut repo: ConfiguredTestRepo) {
    repo.inner.config.as_release = true;
    repo.execute_and_assert(["--as-release"], None);
}

#[rstest]
fn test_release_on_main_branch_with_custom_version_pattern(mut repo: ConfiguredTestRepo) {
    repo.inner.tag("my/v0.1.0");

    repo.inner.config.tag_prefix = "my/v".to_string();
    repo.execute_and_assert(["--tag-prefix", "my/v"], None);
}

#[rstest]
fn test_release_branch_with_custom_pattern(mut repo: ConfiguredTestRepo) {
    repo.inner.tag("v1.0.0");
    repo.inner.branch("custom-release/1.0.0");
    repo.inner.commit("1.0.1+1");

    repo.inner.config.release_branch = "custom-release/(?<BranchName>.*)".to_string();
    repo.execute_and_assert(
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
    repo.execute_and_assert(["--feature-branch", "my-feature/(?<BranchName>.*)"], None);
}

#[rstest]
fn test_option_custom_main_branch(#[with("custom-main")] mut repo: ConfiguredTestRepo) {
    repo.inner.config.main_branch = "custom-main".to_string();
    repo.execute_and_assert(["--main-branch", "custom-main"], None);
}

#[rstest]
fn test_option_custom_repository_path(mut repo: ConfiguredTestRepo) {
    let path = repo.inner.path.to_string_lossy().to_string();
    // repo.inner.config.path = PathBuf::from(&path);

    repo.execute_and_assert(["--path", &path], None);
}

#[rstest]
fn test_argument_prerelease_tag(mut repo: ConfiguredTestRepo) {
    repo.inner.config.pre_release_tag = "alpha".to_string();
    repo.execute_and_assert(["--pre-release-tag", "alpha"], None);
}

#[rstest]
fn test_output_from_main_branch(mut repo: ConfiguredTestRepo) {
    insta::with_settings!({filters => vec![
        (r"\b[[:xdigit:]]{40}\b", "########################################"),
        (r"\b[[:xdigit:]]{7}\b", "#######"),
        (r"\b\d{4}-\d{2}-\d{2}\b", "####-##-##"),
    ]}, {
        assert_cmd_snapshot!(repo.cli.current_dir(repo.inner.path));
    });
}

#[rstest]
fn test_output_from_release_branch(mut repo: ConfiguredTestRepo) {
    repo.inner.commit("0.1.0+1");
    repo.inner.branch("release/0.1.0");

    insta::with_settings!({filters => vec![
        (r"\b[[:xdigit:]]{40}\b", "########################################"),
        (r"\b[[:xdigit:]]{7}\b", "#######"),
        (r"\b\d{4}-\d{2}-\d{2}\b", "####-##-##"),
    ]}, {
        assert_cmd_snapshot!(repo.cli.current_dir(repo.inner.path));
    });
}

#[rstest]
fn test_output_from_feature_branch(mut repo: ConfiguredTestRepo) {
    repo.inner.branch("feature/my-feature");
    repo.inner.commit("0.1.0+1");

    insta::with_settings!({filters => vec![
        (r"\b[[:xdigit:]]{40}\b", "########################################"),
        (r"\b[[:xdigit:]]{7}\b", "#######"),
        (r"\b\d{4}-\d{2}-\d{2}\b", "####-##-##"),
    ]}, {
        assert_cmd_snapshot!(repo.cli.current_dir(repo.inner.path));
    });
}

#[rstest]
fn test_output_from_tag_on_main_branch(mut repo: ConfiguredTestRepo) {
    repo.inner.commit("0.1.0+1");
    repo.inner.tag("0.1.0");

    insta::with_settings!({
        filters => vec![
            (r"\b[[:xdigit:]]{40}\b", "########################################"),
            (r"\b[[:xdigit:]]{7}\b", "#######"),
            (r"\b\d{4}-\d{2}-\d{2}\b", "####-##-##"),
        ]}, {
        assert_cmd_snapshot!(repo.cli.current_dir(repo.inner.path));
    });
}

#[rstest]
fn test_output_from_tag_on_release_branch(mut repo: ConfiguredTestRepo) {
    repo.inner.branch("release/0.1.0");
    repo.inner.commit("0.1.0+1");
    repo.inner.tag("0.1.0");

    insta::with_settings!({
        filters => vec![
            (r"\b[[:xdigit:]]{40}\b", "########################################"),
            (r"\b[[:xdigit:]]{7}\b", "#######"),
            (r"\b\d{4}-\d{2}-\d{2}\b", "####-##-##"),
        ]}, {
        assert_cmd_snapshot!(repo.cli.current_dir(repo.inner.path));
    });
}

#[rstest]
fn test_output_from_tag_checked_out(mut repo: ConfiguredTestRepo) {
    repo.inner.commit("0.1.0+1");
    repo.inner.tag("0.1.0");
    repo.inner.checkout("tags/0.1.0");

    insta::with_settings!({
        filters => vec![
            (r"\b[[:xdigit:]]{40}\b", "########################################"),
            (r"\b[[:xdigit:]]{7}\b", "#######"),
            (r"\b\d{4}-\d{2}-\d{2}\b", "####-##-##"),
        ]}, {
        assert_cmd_snapshot!(repo.cli.current_dir(repo.inner.path));
    });
}

#[rstest]
fn test_output_from_show_config(mut repo: ConfiguredTestRepo) {
    insta::with_settings!({filters => vec![
        (r#"Path = ["'][a-zA-Z0-9-_.~+=,:@%/\\]+["']"#, r#"Path = "<repository_path>""#),
    ]}, {
        assert_cmd_snapshot!(repo.cli.current_dir(repo.inner.path).args(["--show-config"]));
    });
}

#[rstest]
fn test_environment_variable_output_in_github_context(mut repo: ConfiguredTestRepo) {
    let github_output = tempfile::NamedTempFile::new().unwrap();

    let output = repo
        .cli
        .current_dir(repo.inner.path)
        .env_clear()
        .env("CI", "true")
        .env("GITHUB_OUTPUT", github_output.path())
        .output()
        .unwrap();
    assert!(output.status.success());

    let github_output = std::fs::read_to_string(github_output.path()).unwrap();

    insta::with_settings!({filters => vec![
        (r"\b[[:xdigit:]]{40}\b", "########################################"),
        (r"\b[[:xdigit:]]{7}\b", "#######"),
        (r"\b\d{4}-\d{2}-\d{2}\b", "####-##-##"),
    ]}, {
        assert_snapshot!(github_output);
    });
}
