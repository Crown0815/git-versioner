mod cli;
mod common;

use crate::cli::{repo, ConfiguredTestRepo as TestRepo};
use rstest::rstest;
use rstest_reuse::{apply, template};

const CUSTOM_MAIN_BRANCH: &str = "stem";
const DEFAULT_CONFIG: &str = ".git-versioner";

#[template]
#[rstest]
fn default(repo: TestRepo, #[values("toml", "yaml", "yml")] ext: &str) {}

#[apply(default)]
fn test_that_config_file_overrides_default_main_branch_pattern(
    #[with(CUSTOM_MAIN_BRANCH)] mut repo: TestRepo,
    ext: &str,
) {
    repo.config_file.main_branch = Some(format!("^{CUSTOM_MAIN_BRANCH}$"));
    repo.inner.config.main_branch = format!("^{CUSTOM_MAIN_BRANCH}$").to_string();

    repo.execute_and_verify([], Some((DEFAULT_CONFIG, ext)));
}

#[apply(default)]
fn test_that_cli_argument_overrides_configuration_of_main_branch_pattern(
    #[with(CUSTOM_MAIN_BRANCH)] mut repo: TestRepo,
    ext: &str,
) {
    repo.config_file.main_branch = Some(format!("^{}$", "another_main_branch"));

    repo.inner.config.main_branch = CUSTOM_MAIN_BRANCH.to_string();
    repo.execute_and_verify(
        ["--main-branch", CUSTOM_MAIN_BRANCH],
        Some((DEFAULT_CONFIG, ext)),
    );
}

#[apply(default)]
fn test_that_config_file_overrides_default_release_branch_pattern(mut repo: TestRepo, ext: &str) {
    repo.config_file.release_branch = Some("custom-release/(?<BranchName>.*)".to_string());
    let (source, _) = repo.inner.commit("0.1.0+2");
    repo.inner.tag("v1.0.0");
    repo.inner.branch("custom-release/1.0.0");
    repo.inner.commit("1.0.1+1");

    repo.inner.config.release_branch = "custom-release/(?<BranchName>.*)".to_string();
    repo.execute_and_verify([], Some((DEFAULT_CONFIG, ext)));
}

#[apply(default)]
fn test_that_cli_argument_overrides_configuration_of_release_branch_pattern(
    mut repo: TestRepo,
    ext: &str,
) {
    repo.config_file.release_branch = Some("whatever-release/(?<BranchName>.*)".to_string());
    let (source, _) = repo.inner.commit("0.1.0+1");
    repo.inner.tag("v1.0.0");
    repo.inner.branch("custom-release/1.0.0");
    repo.inner.commit("1.0.1+1");

    repo.inner.config.release_branch = "custom-release/(?<BranchName>.*)".to_string();
    repo.execute_and_verify(
        ["--release-branch", "custom-release/(?<BranchName>.*)"],
        Some((DEFAULT_CONFIG, ext)),
    );
}

#[apply(default)]
fn test_that_config_file_overrides_default_feature_branch_pattern(mut repo: TestRepo, ext: &str) {
    repo.config_file.feature_branch = Some("my-feature/(?<BranchName>.*)".to_string());
    repo.inner.commit("0.1.0+1");
    repo.inner.branch("my-feature/feature");
    repo.inner.commit("0.1.0-feature.1");

    repo.inner.config.feature_branch = "my-feature/(?<BranchName>.*)".to_string();
    repo.execute_and_verify([], Some((DEFAULT_CONFIG, ext)));
}

#[apply(default)]
fn test_that_cli_argument_overrides_configuration_of_feature_branch_pattern(
    mut repo: TestRepo,
    ext: &str,
) {
    repo.config_file.feature_branch = Some("whatever-feature/(?<BranchName>.*)".to_string());
    repo.inner.commit("0.1.0+1");
    repo.inner.branch("my-feature/feature");
    repo.inner.commit("0.1.0-feature.1");

    repo.inner.config.feature_branch = "my-feature/(?<BranchName>.*)".to_string();
    repo.execute_and_verify(
        ["--feature-branch", "my-feature/(?<BranchName>.*)"],
        Some((DEFAULT_CONFIG, ext)),
    );
}

#[apply(default)]
fn test_that_config_file_overrides_default_version_pattern(mut repo: TestRepo, ext: &str) {
    repo.config_file.tag_prefix = Some("my/v".to_string());
    repo.inner.commit("0.1.0+1");
    repo.inner.tag("my/v1.0.0");

    repo.inner.config.tag_prefix = "my/v".to_string();
    repo.execute_and_verify([], Some((DEFAULT_CONFIG, ext)));
}

#[apply(default)]
fn test_that_cli_argument_overrides_configuration_of_version_pattern(
    mut repo: TestRepo,
    ext: &str,
) {
    repo.config_file.tag_prefix = Some("my/c".to_string());
    repo.inner.commit("0.1.0+1");
    repo.inner.tag("my/v1.0.0");

    repo.inner.config.tag_prefix = "my/v".to_string();
    repo.execute_and_verify(["--tag-prefix", "my/v"], Some((DEFAULT_CONFIG, ext)));
}

#[apply(default)]
fn test_that_config_file_overrides_default_prerelease_tag(mut repo: TestRepo, ext: &str) {
    repo.config_file.pre_release_tag = Some("alpha".to_string());

    repo.inner.config.pre_release_tag = "alpha".to_string();
    repo.execute_and_verify([], Some((DEFAULT_CONFIG, ext)));
}

#[apply(default)]
fn test_that_cli_argument_overrides_configuration_of_prerelease_tag(mut repo: TestRepo, ext: &str) {
    repo.config_file.pre_release_tag = Some("whatever".to_string());

    repo.inner.config.pre_release_tag = "alpha".to_string();
    repo.execute_and_verify(["--pre-release-tag", "alpha"], Some((DEFAULT_CONFIG, ext)));
}

#[apply(default)]
fn test_commit_message_incrementing(mut repo: TestRepo, ext: &str) {
    repo.inner.commit("0.1.0-pre.1");
    repo.inner.tag("v1.0.1");
    repo.inner.commit("1.0.2-pre.1");

    repo.config_file.commit_message_incrementing = Some("Enabled".to_string());
    repo.inner.config.commit_message_incrementing = "Enabled".to_string();
    repo.execute_and_verify([], Some((DEFAULT_CONFIG, ext)));
}

#[apply(default)]
fn test_that_cli_argument_overrides_configuration_of_commit_message_incrementing(
    mut repo: TestRepo,
    ext: &str,
) {
    repo.inner.commit("0.1.0-pre.1");
    repo.inner.tag("v1.0.1");
    repo.inner.commit("1.0.2-pre.1");

    repo.config_file.commit_message_incrementing = Some("Disabled".to_string());
    repo.inner.config.commit_message_incrementing = "Enabled".to_string();
    repo.execute_and_verify(
        ["--commit-message-incrementing", "Enabled"],
        Some((DEFAULT_CONFIG, ext)),
    );
}
