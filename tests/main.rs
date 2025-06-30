mod lib_integration;
use lib_integration::TestRepo;
use rstest::{fixture, rstest};

use assert_cmd::Command;

#[fixture]
fn repo(
    #[default("trunk")] main: &str
) -> TestRepo {
    let repo = TestRepo::new();
    repo.initialize(main);

    repo.commit("0.1.0-rc.1");
    repo.branch("feature/feature1");
    repo.commit("0.1.0-feature1.1");

    repo.checkout(main);
    repo.merge("feature/feature1");
    repo.tag("v1.0.0");
    repo.branch("release/1.0.0");

    repo.checkout(main);
    repo.branch("feature/feature2");
    repo.commit("1.1.0-feature2.1");
    repo.commit("1.1.0-feature2.2");

    repo.checkout(main);
    repo.merge("feature/feature2");

    repo.checkout("release/1.0.0");
    repo.commit("1.0.1-rc.1");
    repo.branch("feature/fix1");
    repo.commit("1.0.1-fix1.1");
    repo.commit("1.0.1-fix1.2");
    repo.checkout("release/1.0.0");
    repo.merge("feature/fix1");

    repo
}

#[rstest]
fn test_custom_main_branch_name(#[with("custom-trunk")] repo: TestRepo) {
    repo.checkout("custom-trunk");
    assert_version(&repo, "1.1.0-rc.3", "custom-trunk")
}

fn assert_version(repo: &TestRepo, expected: &str, main_branch: &str) {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))
        .expect("CLI binary not found");

    cmd.args(&["--main-branch", main_branch])
        .current_dir(&repo.path)
        .assert()
        .success()
        .stdout(format!("{expected}\n"));
}