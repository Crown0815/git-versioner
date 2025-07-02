mod common;

use common::{cli, repo, TestRepo};
use indoc::formatdoc;
use insta::with_settings;
use insta_cmd::assert_cmd_snapshot;
use rstest::rstest;
use std::fs;
use std::process::Command;

const CUSTOM_MAIN_BRANCH: &str = "stem";

#[macro_export]
macro_rules! assert_configured_repo_cmd_snapshot {
    ($repo:expr, $config:expr, $cmd:expr) => {
        with_settings!(
            { description =>
                formatdoc!(
                    "
                    Git Graph:
                        {graph}
                    Configuration ({path}):
                        {config}",
                    graph = $repo.graph(),
                    path = $config.file_name().unwrap().to_string_lossy(),
                    config = fs::read_to_string(&$config).unwrap()
                )
            },
            { assert_cmd_snapshot!($cmd); }
        );
    };
}

#[rstest]
fn test_release_candidate_on_toml_configured_main_branch(
    #[with(CUSTOM_MAIN_BRANCH)] mut repo: TestRepo,
    mut cli: Command
) {
    repo.cli_config.main_branch = Some(format!("^{}$", CUSTOM_MAIN_BRANCH));
    let config_file = repo.create_default_toml_config();

    assert_configured_repo_cmd_snapshot!(repo, config_file, cli.current_dir(&repo.path));
}
