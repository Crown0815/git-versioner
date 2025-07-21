mod cli;
mod common;

use crate::cli::{ConfiguredTestRepo, repo};
use crate::common::MAIN_BRANCH;
use rstest::rstest;

const CUSTOM_MAIN_BRANCH: &str = "stem";
const DEFAULT_CONFIG: &str = ".git-versioner";

#[rstest]
fn test_that_config_file_overrides_default_main_branch_pattern(
    #[with(CUSTOM_MAIN_BRANCH)] mut repo: ConfiguredTestRepo,
    #[values("toml", "yaml")] extension: &str,
) {
    repo.config_file.main_branch = Some(format!("^{CUSTOM_MAIN_BRANCH}$"));

    repo.assert([], Some((DEFAULT_CONFIG, extension)))
        .version("0.1.0-pre.1")
        .branch(CUSTOM_MAIN_BRANCH)
        .has_no_source();
}

#[rstest]
fn test_that_cli_argument_overrides_configuration_of_main_branch_pattern(
    #[with(CUSTOM_MAIN_BRANCH)] mut repo: ConfiguredTestRepo,
    #[values("toml", "yaml")] extension: &str,
) {
    repo.config_file.main_branch = Some(format!("^{}$", "another_main_branch"));

    repo.assert(
        ["--main-branch", CUSTOM_MAIN_BRANCH],
        Some((DEFAULT_CONFIG, extension)),
    )
    .version("0.1.0-pre.1")
    .branch(CUSTOM_MAIN_BRANCH)
    .has_no_source();
}

#[rstest]
fn test_that_config_file_overrides_default_release_branch_pattern(
    mut repo: ConfiguredTestRepo,
    #[values("toml", "yaml")] extension: &str,
) {
    repo.config_file.release_branch = Some("custom-release/(?<BranchName>.*)".to_string());
    let source = repo.inner.commit("0.1.0-pre.2");
    repo.inner.tag("v1.0.0");
    repo.inner.branch("custom-release/1.0.0");
    repo.inner.commit("1.0.1-pre.1");

    repo.assert([], Some((DEFAULT_CONFIG, extension)))
        .version("1.0.1-pre.1")
        .branch("custom-release/1.0.0")
        .source_id(source);
}

#[rstest]
fn test_that_cli_argument_overrides_configuration_of_release_branch_pattern(
    mut repo: ConfiguredTestRepo,
    #[values("toml", "yaml")] extension: &str,
) {
    repo.config_file.release_branch = Some("whatever-release/(?<BranchName>.*)".to_string());
    let source = repo.inner.commit("0.1.0-pre.1");
    repo.inner.tag("v1.0.0");
    repo.inner.branch("custom-release/1.0.0");
    repo.inner.commit("1.0.1-pre.1");

    repo.assert(
        ["--release-branch", "custom-release/(?<BranchName>.*)"],
        Some((DEFAULT_CONFIG, extension)),
    )
    .version("1.0.1-pre.1")
    .branch("custom-release/1.0.0")
    .source_id(source);
}

#[rstest]
fn test_that_config_file_overrides_default_feature_branch_pattern(
    mut repo: ConfiguredTestRepo,
    #[values("toml", "yaml")] extension: &str,
) {
    repo.config_file.feature_branch = Some("my-feature/(?<BranchName>.*)".to_string());
    repo.inner.commit("0.1.0-pre.1");
    repo.inner.branch("my-feature/feature");
    repo.inner.commit("0.1.0-feature.1");

    repo.assert([], Some((DEFAULT_CONFIG, extension)))
        .version("0.1.0-feature.1")
        .branch("my-feature/feature")
        .has_no_source();
}

#[rstest]
fn test_that_cli_argument_overrides_configuration_of_feature_branch_pattern(
    mut repo: ConfiguredTestRepo,
    #[values("toml", "yaml")] extension: &str,
) {
    repo.config_file.feature_branch = Some("whatever-feature/(?<BranchName>.*)".to_string());
    repo.inner.commit("0.1.0-pre.1");
    repo.inner.branch("my-feature/feature");
    repo.inner.commit("0.1.0-feature.1");

    repo.assert(
        ["--feature-branch", "my-feature/(?<BranchName>.*)"],
        Some((DEFAULT_CONFIG, extension)),
    )
    .version("0.1.0-feature.1")
    .branch("my-feature/feature")
    .has_no_source();
}

#[rstest]
fn test_that_config_file_overrides_default_version_pattern(
    mut repo: ConfiguredTestRepo,
    #[values("toml", "yaml")] extension: &str,
) {
    repo.config_file.version_pattern = Some("my/c(?<Version>.*)".to_string());
    let source = repo.inner.commit("0.1.0-pre.1");
    repo.inner.tag("my/c1.0.0");

    repo.assert([], Some((DEFAULT_CONFIG, extension)))
        .version("1.0.0")
        .branch(MAIN_BRANCH)
        .source_id(source);
}

#[rstest]
fn test_that_cli_argument_overrides_configuration_of_version_pattern(
    mut repo: ConfiguredTestRepo,
    #[values("toml", "yaml")] extension: &str,
) {
    repo.config_file.version_pattern = Some("my/c(?<Version>.*)".to_string());
    let source = repo.inner.commit("0.1.0-pre.1");
    repo.inner.tag("my/v1.0.0");

    repo.assert(
        ["--version-pattern", "my/v(?<Version>.*)"],
        Some((DEFAULT_CONFIG, extension)),
    )
    .version("1.0.0")
    .branch(MAIN_BRANCH)
    .source_id(source);
}

#[rstest]
fn test_that_config_file_overrides_default_prerelease_tag(
    mut repo: ConfiguredTestRepo,
    #[values("toml", "yaml")] extension: &str,
) {
    repo.config_file.prerelease_tag = Some("alpha".to_string());

    repo.assert([], Some((DEFAULT_CONFIG, extension)))
        .version("0.1.0-alpha.1")
        .branch(MAIN_BRANCH)
        .has_no_source();
}

#[rstest]
fn test_that_cli_argument_overrides_configuration_of_prerelease_tag(
    mut repo: ConfiguredTestRepo,
    #[values("toml", "yaml")] extension: &str,
) {
    repo.config_file.prerelease_tag = Some("whatever".to_string());

    repo.assert(
        ["--prerelease-tag", "alpha"],
        Some((DEFAULT_CONFIG, extension)),
    )
    .version("0.1.0-alpha.1")
    .branch(MAIN_BRANCH)
    .has_no_source();
}
