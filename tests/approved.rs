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
fn test_long_help_text(mut cmd: Command) {
    assert_cmd_snapshot!(cmd.current_dir(".").args(["--help"]));
}

#[rstest]
fn test_help_text(mut cmd: Command) {
    assert_cmd_snapshot!(cmd.current_dir(".").args(["-h"]));
}

macro_rules! with_masked_unpredictable_values {
    ($($block:tt)*) => {
        insta::with_settings!({
            filters => vec![
                (r"\b[[:xdigit:]]{40}\b", "########################################"), // SHA1
                (r"\b[[:xdigit:]]{7}\b", "#######"), // Short SHA1
                (r"\b\d{4}-\d{2}-\d{2}\b", "####-##-##"), // Date
            ]
        }, {
            $($block)*
        });
    };
}

#[rstest]
fn test_output_from_main_branch(mut repo: ConfiguredTestRepo) {
    with_masked_unpredictable_values! {
        assert_cmd_snapshot!(repo.cli.current_dir(repo.inner.config.path));
    }
}

#[rstest]
fn test_output_from_release_branch(mut repo: ConfiguredTestRepo) {
    repo.inner.commit("0.1.0+1");
    repo.inner.branch("release/0.1.0");

    with_masked_unpredictable_values! {
        assert_cmd_snapshot!(repo.cli.current_dir(repo.inner.config.path));
    }
}

#[rstest]
fn test_output_from_feature_branch(mut repo: ConfiguredTestRepo) {
    repo.inner.branch("feature/my-feature");
    repo.inner.commit("0.1.0+1");

    with_masked_unpredictable_values! {
        assert_cmd_snapshot!(repo.cli.current_dir(repo.inner.config.path));
    }
}

#[rstest]
fn test_output_from_tag_on_main_branch(mut repo: ConfiguredTestRepo) {
    repo.inner.commit("0.1.0+1");
    repo.inner.tag("0.1.0");

    with_masked_unpredictable_values! {
        assert_cmd_snapshot!(repo.cli.current_dir(repo.inner.config.path));
    }
}

#[rstest]
fn test_output_from_tag_on_release_branch(mut repo: ConfiguredTestRepo) {
    repo.inner.branch("release/0.1.0");
    repo.inner.commit("0.1.0+1");
    repo.inner.tag("0.1.0");

    with_masked_unpredictable_values! {
        assert_cmd_snapshot!(repo.cli.current_dir(repo.inner.config.path));
    }
}

#[rstest]
fn test_output_from_tag_checked_out(mut repo: ConfiguredTestRepo) {
    repo.inner.commit("0.1.0+1");
    repo.inner.tag("0.1.0");
    repo.inner.checkout("tags/0.1.0");

    with_masked_unpredictable_values! {
        assert_cmd_snapshot!(repo.cli.current_dir(repo.inner.config.path));
    }
}

#[rstest]
fn test_environment_variable_output_in_github_context(mut repo: ConfiguredTestRepo) {
    let github_output = tempfile::NamedTempFile::new().unwrap();

    let output = repo
        .cli
        .current_dir(repo.inner.config.path)
        .env_clear()
        .env("CI", "true")
        .env("GITHUB_OUTPUT", github_output.path())
        .output()
        .unwrap();
    assert!(output.status.success());

    let github_output = std::fs::read_to_string(github_output.path()).unwrap();

    with_masked_unpredictable_values! {
        assert_snapshot!(github_output);
    }
}

#[rstest]
fn test_output_from_show_config(mut repo: ConfiguredTestRepo) {
    insta::with_settings!({filters => vec![
        (r#"Path = ["'][a-zA-Z0-9-_.~+=,:@%/\\]+["']"#, r#"Path = "<repository_path>""#),
    ]}, {
        assert_cmd_snapshot!(repo.cli.current_dir(repo.inner.config.path).args(["--show-config"]));
    });
}
