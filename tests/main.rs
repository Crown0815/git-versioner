mod common;

use common::TestRepo;
use insta::with_settings;
use insta_cmd::{assert_cmd_snapshot, get_cargo_bin};
use rstest::{fixture, rstest};
use std::process::Command;

const MAIN_BRANCH: &str = "trunk";

macro_rules! assert_repo_cmd_snapshot {
    ($repo:expr, $cmd:expr) => {
        with_settings!(
            { description => $repo.graph() },
            { assert_cmd_snapshot!($cmd); }
        );
    };
}

#[fixture]
fn repo(#[default(MAIN_BRANCH)] main: &str) -> TestRepo {
    let repo = TestRepo::new();
    repo.initialize(main);
    repo.commit("0.1.0-rc.1");
    repo
}

#[fixture]
fn cli() -> Command {
    Command::new(get_cargo_bin(env!("CARGO_PKG_NAME")))
}

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
fn test_option_custom_main_branch(
    #[with("custom-main")] repo: TestRepo,
    mut cli: Command
) {
    assert_repo_cmd_snapshot!(repo, cli.current_dir(&repo.path).args(&["--main-branch", "custom-main"]));
}

#[rstest]
fn test_option_custom_repository_path(repo: TestRepo, mut cli: Command) {
    assert_repo_cmd_snapshot!(repo, cli.current_dir(".").args(&["--path", repo.path.to_str().unwrap()]));
}

#[rstest]
fn test_help_text(mut cli: Command) {
    assert_cmd_snapshot!(cli.current_dir(".").args(&["--help"]));
}