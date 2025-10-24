mod cli;
mod common;

use crate::cli::{ConfiguredTestRepo, cmd, repo};
use crate::common::MAIN_BRANCH;
use insta::assert_snapshot;
use insta_cmd::assert_cmd_snapshot;
use rstest::rstest;
// use std::env;
use std::process::Command;

impl ConfiguredTestRepo {}

#[rstest]
fn test_release_candidate_on_main_branch(mut repo: ConfiguredTestRepo) {
    repo.execute_and_assert([], None)
        .version("0.1.0-pre.1")
        .branch_name(MAIN_BRANCH)
        .version_source_sha("");
}

#[rstest]
fn test_release_tag_on_main_branch(mut repo: ConfiguredTestRepo) {
    let (source, _) = repo.inner.commit("tagged");
    repo.inner.tag("0.1.0");

    repo.execute_and_assert([], None)
        .version("0.1.0")
        .branch_name(MAIN_BRANCH)
        .version_source_sha(&source);
}

#[rstest]
fn test_release_request_on_main_branch(mut repo: ConfiguredTestRepo) {
    repo.execute_and_assert(["--as-release"], None)
        .version("0.1.0")
        .branch_name(MAIN_BRANCH)
        .version_source_sha("")
        .weighted_pre_release_number(60000);
}

#[rstest]
fn test_release_on_main_branch_with_custom_version_pattern(mut repo: ConfiguredTestRepo) {
    let (source, _) = repo.inner.commit("tagged");
    repo.inner.tag("my/v0.1.0");

    repo.execute_and_assert(["--tag-prefix", "my/v"], None)
        .version("0.1.0")
        .branch_name(MAIN_BRANCH)
        .version_source_sha(&source);
}

#[rstest]
fn test_release_branch_with_custom_pattern(mut repo: ConfiguredTestRepo) {
    let (source, _) = repo.inner.commit("tagged");
    repo.inner.tag("v1.0.0");
    repo.inner.branch("custom-release/1.0.0");
    repo.inner.commit("1.0.1-pre.1");

    repo.execute_and_assert(
        ["--release-branch", "custom-release/(?<BranchName>.*)"],
        None,
    )
    .version("1.0.1-pre.1")
    .branch_name("custom-release/1.0.0")
    .version_source_sha(&source);
}

#[rstest]
fn test_feature_branch_with_custom_pattern(mut repo: ConfiguredTestRepo) {
    repo.inner.commit("0.1.0-pre.1");
    repo.inner.branch("my-feature/feature");
    repo.inner.commit("0.1.0-feature.1");

    repo.execute_and_assert(["--feature-branch", "my-feature/(?<BranchName>.*)"], None)
        .version("0.1.0-feature.1")
        .branch_name("my-feature/feature")
        .version_source_sha("");
}

#[rstest]
fn test_option_custom_main_branch(#[with("custom-main")] mut repo: ConfiguredTestRepo) {
    repo.execute_and_assert(["--main-branch", "custom-main"], None)
        .version("0.1.0-pre.1")
        .branch_name("custom-main")
        .version_source_sha("");
}

#[rstest]
fn test_option_custom_repository_path(mut repo: ConfiguredTestRepo) {
    let path = repo.inner.path.to_string_lossy().to_string();

    repo.execute_and_assert(["--path", &path], None)
        .version("0.1.0-pre.1")
        .branch_name(MAIN_BRANCH)
        .version_source_sha("");
}

#[rstest]
fn test_argument_prerelease_tag(mut repo: ConfiguredTestRepo) {
    repo.execute_and_assert(["--pre-release-tag", "alpha"], None)
        .version("0.1.0-alpha.1")
        .branch_name(MAIN_BRANCH)
        .version_source_sha("");
}

#[rstest]
fn test_help_text(mut cmd: Command) {
    assert_cmd_snapshot!(cmd.current_dir(".").args(["--help"]));
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
    repo.inner.commit("0.1.0-pre.1");
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
    repo.inner.commit("0.1.0-pre.1");

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
    repo.inner.commit("0.1.0-pre.1");
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
    repo.inner.commit("0.1.0-pre.1");
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
    repo.inner.commit("0.1.0-pre.1");
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

#[rstest]
fn test_that_in_continuous_delivery_mode_with_custom_pre_release_tag_when_no_tags_exist_produces_pre_release_tag_1(
    mut repo: ConfiguredTestRepo,
) {
    repo.inner.commit("0.1.0-pre.2");
    repo.execute_and_assert(["--pre-release-tag", "rc", "--continuous-delivery"], None)
        .version("0.1.0-rc.1")
        .version_source_sha("");
}

#[rstest]
fn test_that_in_continuous_delivery_mode_with_custom_pre_release_tag_when_matching_tags_exist_produces_next_pre_release_tag(
    mut repo: ConfiguredTestRepo,
) {
    let (sha, _) = repo.inner.tag("v0.1.0-rc.1");
    repo.inner.commit("0.1.0-pre.2");
    repo.inner.commit("0.1.0-pre.3");
    repo.execute_and_assert(["--pre-release-tag", "rc", "--continuous-delivery"], None)
        .version("0.1.0-rc.2")
        .version_source_sha(&sha);
}

#[rstest]
fn test_that_in_continuous_delivery_mode_with_custom_pre_release_tag_when_non_matching_tags_exist_produces_pre_release_tag_1(
    mut repo: ConfiguredTestRepo,
) {
    repo.inner.tag("v0.1.0-pre.1");
    repo.inner.commit("0.1.0-pre.2");
    repo.execute_and_assert(["--pre-release-tag", "rc", "--continuous-delivery"], None)
        .version("0.1.0-rc.1")
        .version_source_sha("");
}

#[rstest]
fn test_that_in_continuous_delivery_mode_when_no_tags_exist_produces_pre_release_tag_1(
    mut repo: ConfiguredTestRepo,
) {
    repo.inner.commit("0.1.0-pre.2");
    repo.execute_and_assert(["--continuous-delivery"], None)
        .version("0.1.0-pre.1")
        .version_source_sha("");
}

#[rstest]
fn test_that_in_continuous_delivery_mode_when_matching_tags_exist_produces_next_pre_release_tag(
    mut repo: ConfiguredTestRepo,
) {
    let (sha, _) = repo.inner.tag("v0.1.0-pre.1");
    repo.inner.commit("0.1.0-pre.2");
    repo.inner.commit("0.1.0-pre.3");
    repo.execute_and_assert(["--continuous-delivery"], None)
        .version("0.1.0-pre.2")
        .version_source_sha(&sha);
}

#[rstest]
fn test_that_in_continuous_delivery_mode_after_release_when_no_matching_tags_exist_produces_pre_release_tag_1(
    mut repo: ConfiguredTestRepo,
) {
    let (sha, _) = repo.inner.tag("v1.0.0");
    repo.inner.commit("1.1.0-pre.1");
    repo.inner.commit("1.1.0-pre.2");
    repo.execute_and_assert(["--continuous-delivery"], None)
        .version("1.1.0-pre.1")
        .version_source_sha(&sha);
}

#[rstest]
fn test_that_in_continuous_delivery_mode_after_release_when_matching_tags_exist_produces_next_pre_release_tag(
    mut repo: ConfiguredTestRepo,
) {
    repo.inner.tag("v1.0.0");
    repo.inner.commit("1.1.0-pre.1");
    let (sha, _) = repo.inner.tag("v1.1.0-pre.1");
    repo.inner.commit("1.1.0-pre.2");
    repo.inner.commit("1.1.0-pre.3");
    repo.execute_and_assert(["--continuous-delivery"], None)
        .version("1.1.0-pre.2")
        .version_source_sha(&sha);
}

#[rstest]
fn test_that_in_continuous_delivery_mode_on_release_branch_when_no_tags_exist_produces_pre_release_tag_1(
    mut repo: ConfiguredTestRepo,
) {
    repo.inner.branch("release/1.0.0");
    repo.inner.commit("1.0.0-pre.1");
    repo.inner.commit("1.0.0-pre.2");
    repo.execute_and_assert(["--continuous-delivery"], None)
        .version("1.0.0-pre.1")
        .version_source_sha("");
}

#[rstest]
fn test_that_in_continuous_delivery_mode_on_release_branch_when_release_tags_exist_produces_pre_release_tag_1(
    mut repo: ConfiguredTestRepo,
) {
    repo.inner.branch("release/1.0.0");
    repo.inner.commit("1.0.0-pre.1");
    let (sha, _) = repo.inner.tag("v1.0.0");
    repo.inner.commit("1.0.1-pre.1");
    repo.inner.commit("1.0.1-pre.2");
    repo.execute_and_assert(["--continuous-delivery"], None)
        .version("1.0.1-pre.1")
        .version_source_sha(&sha);
}

#[rstest]
fn test_that_in_continuous_delivery_mode_on_release_branch_when_release_branches_exist_produces_pre_release_tag_1(
    mut repo: ConfiguredTestRepo,
) {
    repo.inner.branch("release/1.0.0");
    let (sha, _) = repo.inner.commit("1.0.0-pre.1");
    repo.inner.checkout(MAIN_BRANCH);
    repo.inner.commit("1.1.0-pre.1");
    repo.inner.branch("release/1.1.0");
    repo.inner.commit("1.1.0-pre.2");
    repo.inner.commit("1.1.0-pre.3");
    repo.execute_and_assert(["--continuous-delivery"], None)
        .version("1.1.0-pre.1")
        .version_source_sha(&sha);
}
