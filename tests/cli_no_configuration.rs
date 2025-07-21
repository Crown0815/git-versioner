mod cli;
mod common;

use crate::cli::{ConfiguredTestRepo, cmd, repo};
use crate::common::MAIN_BRANCH;
use common::TestRepo;
use git2::Oid;
use insta::with_settings;
use insta_cmd::assert_cmd_snapshot;
use regex::{Captures, Regex, Replacer};
use rstest::rstest;
use std::process::Command;

macro_rules! assert_repo_cmd_snapshot {
    ($repo:expr, $cmd:expr) => {
        with_settings!(
            { description => redacted_graph(&$repo.graph()) },
            { assert_cmd_snapshot!($cmd); }
        );
    };
}

//noinspection RegExpDuplicateCharacterInClass
//noinspection RegExpRedundantNestedCharacterClass
//noinspection RegExpSimplifiable
pub fn redacted_graph(graph: &str) -> String {
    #[derive(Default)]
    struct ShaReplacer {
        counter: usize,
    }

    impl Replacer for ShaReplacer {
        fn replace_append(&mut self, _caps: &Captures<'_>, dst: &mut String) {
            self.counter += 1;
            dst.push_str(&format!("#SHA-{}#", self.counter));
        }
    }

    fn reverse_lines(input: &str) -> String {
        let mut lines: Vec<&str> = input.lines().collect();
        lines.reverse();
        lines.join("\n")
    }

    let re = Regex::new(r"\b[[:xdigit:]]{7}\b").unwrap();
    reverse_lines(
        re.replace_all(&reverse_lines(graph), ShaReplacer::default())
            .as_ref(),
    )
}

#[rstest]
fn test_release_candidate_on_main_branch(mut repo: ConfiguredTestRepo) {
    repo.assert_version("0.1.0-pre.1", MAIN_BRANCH, [], Oid::zero());
}

#[rstest]
fn test_release_on_main_branch(mut repo: ConfiguredTestRepo) {
    let source = repo.inner.commit("tagged");
    repo.inner.tag("0.1.0");
    repo.assert_version("0.1.0", MAIN_BRANCH, [], source);
}

#[rstest]
fn test_release_on_main_branch_with_custom_version_pattern(mut repo: ConfiguredTestRepo) {
    let source = repo.inner.commit("tagged");
    repo.inner.tag("my/v0.1.0");

    repo.assert_version(
        "0.1.0",
        MAIN_BRANCH,
        ["--version-pattern", "my/v(?<Version>.*)"],
        source,
    );
}

#[rstest]
fn test_release_branch_with_custom_pattern(mut repo: ConfiguredTestRepo) {
    let source = repo.inner.commit("tagged");
    repo.inner.tag("v1.0.0");
    repo.inner.branch("custom-release/1.0.0");
    repo.inner.commit("1.0.1-pre.1");

    repo.assert_version(
        "1.0.1-pre.1",
        "custom-release/1.0.0",
        ["--release-branch", "custom-release/(?<BranchName>.*)"],
        source,
    );
}

#[rstest]
fn test_feature_branch_with_custom_pattern(mut repo: ConfiguredTestRepo) {
    repo.inner.commit("0.1.0-pre.1");
    repo.inner.branch("my-feature/feature");
    repo.inner.commit("0.1.0-feature.1");

    repo.assert_version(
        "0.1.0-feature.1",
        "my-feature/feature",
        ["--feature-branch", "my-feature/(?<BranchName>.*)"],
        Oid::zero(),
    );
}

#[rstest]
fn test_option_custom_main_branch(#[with("custom-main")] mut repo: ConfiguredTestRepo) {
    repo.assert_version(
        "0.1.0-pre.1",
        "custom-main",
        ["--main-branch", "custom-main"],
        Oid::zero(),
    );
}

#[rstest]
fn test_option_custom_repository_path(mut repo: ConfiguredTestRepo) {
    let path = repo.inner.path.to_string_lossy().to_string();
    repo.assert_version("0.1.0-pre.1", MAIN_BRANCH, ["--path", &path], Oid::zero());
}

#[rstest]
fn test_argument_prerelease_tag(mut repo: ConfiguredTestRepo) {
    let path = repo.inner.path.to_string_lossy().to_string();
    repo.assert_version(
        "0.1.0-alpha.1",
        MAIN_BRANCH,
        ["--prerelease-tag", "alpha"],
        Oid::zero(),
    );
}

#[rstest]
fn test_help_text(mut cmd: Command) {
    assert_cmd_snapshot!(cmd.current_dir(".").args(["--help"]));
}
