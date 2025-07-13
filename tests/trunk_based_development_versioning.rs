mod common;

use common::{TestRepo, MAIN_BRANCH};
use git_versioner::GitVersioner;
use rstest::{fixture, rstest};
use semver::Version;

impl TestRepo {
    fn commit_and_assert(&self, expected_version: &str) {
        self.commit(expected_version);
        self.assert_version(expected_version);
    }

    fn tag_and_assert(&self, prefix: &str, expected_version: &str) {
        self.tag(&format!("{}{}", prefix, expected_version));
        self.assert_version(expected_version);
    }

    fn tag_annotated_and_assert(&self, prefix: &str, expected_version: &str) {
        self.tag_annotated(&format!("{}{}", prefix, expected_version));
        self.assert_version(expected_version);
    }

    fn merge_and_assert(&self, branch_name: &str, expected_version: &str) {
        self.merge(branch_name);
        self.assert_version(expected_version);
    }

    fn assert_version(&self, expected: &str) {
        let actual = GitVersioner::calculate_version(&self.config).unwrap();
        let expected = Version::parse(expected).unwrap();
        let graph = self.graph();
        assert_eq!(actual, expected,
            "Expected HEAD version: {expected}, found: {actual}\n\n Git Graph:\n-------\n{}------",
            graph);
    }
}

#[fixture]
fn repo(#[default(MAIN_BRANCH)] main_branch: &str) -> TestRepo {
    TestRepo::initialize(main_branch)
}

#[rstest]
fn test_full_workflow(repo: TestRepo) {
    repo.commit_and_assert("0.1.0-rc.1");
    repo.commit_and_assert("0.1.0-rc.2");
    repo.tag("v1.0.0-rc.2"); // ignored
    repo.tag_and_assert("v", "1.0.0");
    repo.branch("release/1.0.0");

    repo.checkout(MAIN_BRANCH);
    repo.commit_and_assert("1.1.0-rc.1");

    repo.checkout("release/1.0.0");
    repo.commit_and_assert("1.0.1-rc.1");
    repo.commit_and_assert("1.0.1-rc.2");
    repo.tag_and_assert("v", "1.0.1");
    repo.commit_and_assert("1.0.2-rc.1");
    repo.commit_and_assert("1.0.2-rc.2");
    repo.tag_and_assert("v", "1.0.2");

    repo.checkout(MAIN_BRANCH);
    repo.commit_and_assert("1.1.0-rc.2");
    repo.branch("release/1.1.0");
    repo.checkout(MAIN_BRANCH);
    repo.commit_and_assert("1.2.0-rc.1");

    repo.checkout("release/1.1.0");
    repo.commit_and_assert("1.1.0-rc.3");
    repo.commit_and_assert("1.1.0-rc.4");
    repo.tag_annotated_and_assert("v", "1.1.0");
    repo.commit_and_assert("1.1.1-rc.1");
    repo.commit_and_assert("1.1.1-rc.2");
    repo.tag_and_assert("v", "1.1.1");

    repo.checkout(MAIN_BRANCH);
    repo.commit_and_assert("1.2.0-rc.2");
    repo.branch("release/1.2.0");
    repo.checkout(MAIN_BRANCH);
    repo.commit_and_assert("1.3.0-rc.1");

    repo.checkout("release/1.2.0");
    repo.commit_and_assert("1.2.0-rc.3");
    repo.commit_and_assert("1.2.0-rc.4");
    repo.tag_and_assert("v", "1.2.0");
    repo.commit_and_assert("1.2.1-rc.1");
    repo.commit_and_assert("1.2.1-rc.2");
    repo.tag_and_assert("v", "1.2.1");

    repo.checkout(MAIN_BRANCH);
    repo.commit_and_assert("1.3.0-rc.2");
    repo.tag_annotated_and_assert("v", "1.3.0");
    repo.commit_and_assert("1.4.0-rc.1");
    repo.commit_and_assert("1.4.0-rc.2");

    repo.branch("release/2.0.0");
    repo.commit_and_assert("2.0.0-rc.1");

    repo.checkout(MAIN_BRANCH);
    repo.commit_and_assert("2.1.0-rc.1");
}

#[rstest]
fn test_full_workflow_with_feature_branches(repo: TestRepo) {
    repo.commit_and_assert("0.1.0-rc.1");
    repo.branch("feature/feature1");
    repo.commit_and_assert("0.1.0-feature1.1");

    repo.checkout(MAIN_BRANCH);
    repo.merge_and_assert("feature/feature1", "0.1.0-rc.3");
    repo.tag_and_assert("v", "1.0.0");
    repo.branch("release/1.0.0");

    repo.checkout(MAIN_BRANCH);
    repo.branch("feature/feature2");
    repo.commit_and_assert("1.1.0-feature2.1");
    repo.commit_and_assert("1.1.0-feature2.2");

    repo.checkout(MAIN_BRANCH);
    repo.merge_and_assert("feature/feature2", "1.1.0-rc.3");

    repo.checkout("release/1.0.0");
    repo.commit_and_assert("1.0.1-rc.1");
    repo.branch("feature/fix1");
    repo.commit_and_assert("1.0.1-fix1.1");
    repo.commit_and_assert("1.0.1-fix1.2");
    repo.checkout("release/1.0.0");
    repo.merge_and_assert("feature/fix1", "1.0.1-rc.4");

    repo.checkout(MAIN_BRANCH);
    repo.branch("feature/feature3-1");
    repo.commit_and_assert("1.1.0-feature3-1.1");
    repo.checkout(MAIN_BRANCH);
    repo.branch("feature/feature3-2");
    repo.commit_and_assert("1.1.0-feature3-2.1");
    repo.commit_and_assert("1.1.0-feature3-2.2");
    repo.checkout("feature/feature3-1");
    repo.commit_and_assert("1.1.0-feature3-1.2");

    repo.checkout(MAIN_BRANCH);
    repo.merge_and_assert("feature/feature3-2", "1.1.0-rc.6");
    repo.merge_and_assert("feature/feature3-1", "1.1.0-rc.9");
}

#[rstest]
fn test_support_of_custom_trunk_pattern(#[with("custom-trunk")] mut repo: TestRepo) {
    repo.config.main_branch = r"^custom-trunk$".to_string();

    repo.commit("Initial commit");
    repo.assert_version("0.1.0-rc.1");
}

#[rstest]
fn test_release_branches_with_matching_version_tag_prefix_affect_main_branch(
    repo: TestRepo,
    #[values("", "s")] casing: &str,
    #[values("/", "-")] separator: &str,
    #[values("v", "V", "")] prefix: &str,
) {
    repo.commit_and_assert("0.1.0-rc.1");
    repo.branch(&format!("release{}{}{}1.0.0", casing, separator, prefix));
    repo.checkout(MAIN_BRANCH);
    repo.commit_and_assert("1.1.0-rc.1");
}

#[rstest]
fn test_release_branches_matching_custom_pattern_affect_main_branch(mut repo: TestRepo) {
    repo.config.release_branch = r"^stabilize/my/(?<BranchName>.+)$".to_string();

    repo.commit_and_assert("0.1.0-rc.1");
    repo.branch("stabilize/my/1.0.0");
    repo.checkout(MAIN_BRANCH);
    repo.commit_and_assert("1.1.0-rc.1");
}

#[rstest]
fn test_release_branches_not_matching_current_trunk_start_new_release_at_their_root(repo: TestRepo) {
    repo.commit_and_assert("0.1.0-rc.1");
    repo.branch("release/1.0.0");
    repo.commit_and_assert("1.0.0-rc.1");
}

#[rstest]
fn test_release_branches_matching_current_trunk_increment_continue_release_at_version_root(repo: TestRepo) {
    repo.commit_and_assert("0.1.0-rc.1");
    repo.tag_and_assert("v", "1.0.0");
    repo.commit_and_assert("1.1.0-rc.1");

    repo.branch("release/1.1.0");

    repo.commit_and_assert("1.1.0-rc.2");
}

#[rstest]
fn test_release_branches_not_matching_current_trunk_increment_start_new_release_at_their_root(repo: TestRepo) {
    repo.commit_and_assert("0.1.0-rc.1");
    repo.tag_and_assert("v", "1.0.0");
    repo.commit_and_assert("1.1.0-rc.1");

    repo.branch("release/1.2.0");

    repo.commit_and_assert("1.2.0-rc.1");
}

#[rstest]
fn test_release_tags_with_matching_version_tag_prefix_are_considered(
    repo: TestRepo,
    #[values("v", "V", "")] prefix: &str,
) {
    repo.commit_and_assert("0.1.0-rc.1");
    repo.tag_and_assert(prefix, "1.0.0");
}

#[rstest]
fn test_prerelease_tags_with_matching_version_tag_prefix_are_ignored(repo: TestRepo) {
    repo.commit_and_assert("0.1.0-rc.1");
    repo.tag("v1.0.0-rc.1");
    repo.assert_version("0.1.0-rc.1");
}

#[rstest]
fn test_release_tags_without_matching_version_tag_prefix_are_ignored(
    repo: TestRepo,
    #[values("a", "x", "p", "vv", "Vv")] prefix: &str,
) {
    repo.commit_and_assert("0.1.0-rc.1");
    repo.tag(&format!("{}1.0.0",  prefix));
    repo.assert_version("0.1.0-rc.1");
}

#[rstest]
fn test_tags_with_matching_custom_version_tag_prefix_are_considered(mut repo: TestRepo) {
    repo.config.version_pattern = "my/v(?<Version>.+)".to_string();

    repo.commit_and_assert("0.1.0-rc.1");
    repo.tag_and_assert("my/v", "1.0.0");
}

#[rstest]
fn test_feature_branches_inherits_main_branch_base_version(
    repo: TestRepo,
    #[values("/", "-", "s/", "s-")] case: &str,
) {
    repo.commit_and_assert("0.1.0-rc.1");
    repo.tag_and_assert("v", "1.0.0");
    repo.branch(&format!("feature{}feature", case));
    repo.commit_and_assert("1.1.0-feature.1");
}

#[rstest]
fn test_feature_branches_matching_custom_pattern_inherit_source_branch_base_version(mut repo: TestRepo) {
    repo.config.feature_branch = r"^feat(ure)?/(?<BranchName>.+)$".to_string();
    
    repo.commit_and_assert("0.1.0-rc.1");
    repo.tag_and_assert("v", "1.0.0");
    repo.branch("feat/feature");
    repo.commit_and_assert("1.1.0-feature.1");
}

#[rstest]
// These symbols were not tested because they are invalid in branch names:
// '\', '^', '~', ' ', ':', '?', '*', '['
// see https://git-scm.com/docs/git-check-ref-format
fn test_valid_feature_branch_symbols_incompatible_with_semantic_versions_are_replaces_with_dash(
    repo: TestRepo,
    #[values('_', '/', ',', '!', '`', ']', '{', '}', '😁')] incompatible_symbol: char,
) {
    repo.commit("irrelevant");
    repo.branch(&format!("feature/a{}a",  incompatible_symbol));
    repo.commit_and_assert("0.1.0-a-a.1");
}

#[rstest]
fn test_non_matching_branches_are_treated_as_feature_branches(repo: TestRepo) {
    repo.commit_and_assert("0.1.0-rc.1");
    repo.tag_and_assert("v", "1.0.0");
    repo.branch("refactor/abc");
    repo.commit_and_assert("1.1.0-refactor-abc.1");
}