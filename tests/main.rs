mod lib;

use lib::TestRepo;
use rstest::{fixture, rstest};
use std::path::Path;

use assert_cmd::Command;

const TRUNK: &str = "trunk";

#[fixture]
fn repo(#[default(TRUNK)] main: &str) -> TestRepo {
    let repo = TestRepo::new();
    repo.initialize(main);
    repo.commit("0.1.0-rc.1");
    repo
}

#[rstest]
fn test_default_main_branch_name(repo: TestRepo) {
    assert_version(&repo.path, &[], "0.1.0-rc.1")
}

#[rstest]
fn test_custom_main_branch(#[with("custom-main")] repo: TestRepo) {
    assert_version(&repo.path, &["--main-branch", "custom-main"], "0.1.0-rc.1")
}

#[rstest]
fn test_repository_not_in_working_directory(repo: TestRepo) {
    assert_version(".", &["--repo-path", repo.path.to_str().unwrap()], "0.1.0-rc.1")
}

fn assert_version<P: AsRef<Path>>(cd: P, args: &[&str], expected: &str) {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))
        .expect("CLI binary not found");

    cmd.args(args)
        .current_dir(cd)
        .assert()
        .success()
        .stdout(format!("{expected}\n"));
}