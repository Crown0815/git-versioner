mod common;

use common::{Assertable, MAIN_BRANCH, TestRepo};
use rstest::{fixture, rstest};

impl TestRepo {
    fn commit_and_assert(&self, expected: &str) -> Assertable {
        self.commit(expected);
        self.assert().version(expected)
    }

    fn tag_and_assert(&self, prefix: &str, expected: &str) -> Assertable {
        self.tag(&format!("{prefix}{expected}"));
        self.assert().version(expected)
    }

    pub fn tag_annotated(&self, name: &str) {
        self.execute(
            &["tag", "-a", name, "-m", name],
            &format!("create tag {name}"),
        );
    }

    fn tag_annotated_and_assert(&self, prefix: &str, expected_version: &str) -> Assertable {
        self.tag_annotated(&format!("{prefix}{expected_version}"));
        self.assert().version(expected_version)
    }

    pub fn merge(&self, name: &str) {
        self.execute(&["merge", "--no-ff", name], &format!("merge {name}"));
    }

    fn merge_and_assert(&self, branch_name: &str, expected_version: &str) -> Assertable {
        self.merge(branch_name);
        self.assert().version(expected_version)
    }
}

#[fixture]
fn repo(#[default(MAIN_BRANCH)] main_branch: &str) -> TestRepo {
    TestRepo::initialize(main_branch)
}

#[rstest]
fn test_full_workflow(repo: TestRepo) {
    repo.commit_and_assert("0.1.0-pre.1");
    repo.commit_and_assert("0.1.0-pre.2");
    repo.tag("v1.0.0-pre.2"); // ignored
    repo.tag_and_assert("v", "1.0.0");
    repo.branch("release/1.0.0");

    repo.checkout(MAIN_BRANCH);
    repo.commit_and_assert("1.1.0-pre.1");

    repo.checkout("release/1.0.0");
    repo.commit_and_assert("1.0.1-pre.1");
    repo.commit_and_assert("1.0.1-pre.2");
    repo.tag_and_assert("v", "1.0.1");
    repo.commit_and_assert("1.0.2-pre.1");
    repo.commit_and_assert("1.0.2-pre.2");
    repo.tag_and_assert("v", "1.0.2");

    repo.checkout(MAIN_BRANCH);
    repo.commit_and_assert("1.1.0-pre.2");
    repo.branch("release/1.1.0");
    repo.checkout(MAIN_BRANCH);
    repo.commit_and_assert("1.2.0-pre.1");

    repo.checkout("release/1.1.0");
    repo.commit_and_assert("1.1.0-pre.3");
    repo.commit_and_assert("1.1.0-pre.4");
    repo.tag_annotated_and_assert("v", "1.1.0");
    repo.commit_and_assert("1.1.1-pre.1");
    repo.commit_and_assert("1.1.1-pre.2");
    repo.tag_and_assert("v", "1.1.1");

    repo.checkout(MAIN_BRANCH);
    repo.commit_and_assert("1.2.0-pre.2");
    repo.branch("release/1.2.0");
    repo.checkout(MAIN_BRANCH);
    repo.commit_and_assert("1.3.0-pre.1");

    repo.checkout("release/1.2.0");
    repo.commit_and_assert("1.2.0-pre.3");
    repo.commit_and_assert("1.2.0-pre.4");
    repo.tag_and_assert("v", "1.2.0");
    repo.commit_and_assert("1.2.1-pre.1");
    repo.commit_and_assert("1.2.1-pre.2");
    repo.tag_and_assert("v", "1.2.1");

    repo.checkout(MAIN_BRANCH);
    repo.commit_and_assert("1.3.0-pre.2");
    repo.tag_annotated_and_assert("v", "1.3.0");
    repo.commit_and_assert("1.4.0-pre.1");
    repo.commit_and_assert("1.4.0-pre.2");

    repo.branch("release/2.0.0");
    repo.commit_and_assert("2.0.0-pre.1");

    repo.checkout(MAIN_BRANCH);
    repo.commit_and_assert("2.1.0-pre.1");
}

#[rstest]
fn test_full_workflow_with_feature_branches(repo: TestRepo) {
    repo.commit_and_assert("0.1.0-pre.1");
    repo.branch("feature/feature1");
    repo.commit_and_assert("0.1.0-feature1.1");

    repo.checkout(MAIN_BRANCH);
    repo.merge_and_assert("feature/feature1", "0.1.0-pre.3");
    repo.tag_and_assert("v", "1.0.0");
    repo.branch("release/1.0.0");

    repo.checkout(MAIN_BRANCH);
    repo.branch("feature/feature2");
    repo.commit_and_assert("1.1.0-feature2.1");
    repo.commit_and_assert("1.1.0-feature2.2");

    repo.checkout(MAIN_BRANCH);
    repo.merge_and_assert("feature/feature2", "1.1.0-pre.3");

    repo.checkout("release/1.0.0");
    repo.commit_and_assert("1.0.1-pre.1");
    repo.branch("feature/fix1");
    repo.commit_and_assert("1.0.1-fix1.1");
    repo.commit_and_assert("1.0.1-fix1.2");
    repo.checkout("release/1.0.0");
    repo.merge_and_assert("feature/fix1", "1.0.1-pre.4");

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
    repo.merge_and_assert("feature/feature3-2", "1.1.0-pre.6");
    repo.merge_and_assert("feature/feature3-1", "1.1.0-pre.9");
}

#[rstest]
fn test_support_of_custom_trunk_pattern(#[with("custom-trunk")] mut repo: TestRepo) {
    repo.config.main_branch = r"^custom-trunk$".to_string();

    repo.commit("Initial commit");
    repo.assert().version("0.1.0-pre.1");
}

#[rstest]
fn test_release_branches_with_matching_version_tag_prefix_affect_main_branch(
    repo: TestRepo,
    #[values("", "s")] casing: &str,
    #[values("/", "-")] separator: &str,
    #[values("v", "V", "")] prefix: &str,
) {
    repo.commit_and_assert("0.1.0-pre.1");
    repo.branch(&format!("release{casing}{separator}{prefix}1.0.0"));
    repo.checkout(MAIN_BRANCH);
    repo.commit_and_assert("1.1.0-pre.1");
}

#[rstest]
fn test_release_branches_matching_initial_trunk_version_continue_release_at_version_root(
    repo: TestRepo,
) {
    repo.commit_and_assert("0.1.0-pre.1");
    repo.branch("release/0.1.0");
    repo.commit_and_assert("0.1.0-pre.2").has_no_source();
}

#[rstest]
fn test_release_branches_matching_tag_incremented_trunk_version_continue_release_at_version_root(
    repo: TestRepo,
) {
    repo.commit_and_assert("0.1.0-pre.1");
    repo.tag_and_assert("v", "1.0.0");
    repo.commit_and_assert("1.1.0-pre.1");

    repo.branch("release/1.1.0");

    repo.commit_and_assert("1.1.0-pre.2");
}

#[rstest]
fn test_release_branches_matching_release_branch_incremented_trunk_version_continue_release_at_version_root(
    repo: TestRepo,
) {
    repo.commit_and_assert("0.1.0-pre.1");
    repo.branch("release/1.0.0");
    repo.checkout(MAIN_BRANCH);
    repo.commit_and_assert("1.1.0-pre.1");

    repo.branch("release/1.1.0");

    repo.commit_and_assert("1.1.0-pre.2");
}

#[rstest]
fn test_release_branches_matching_custom_pattern_affect_main_branch(mut repo: TestRepo) {
    repo.config.release_branch = r"^stabilize/my/(?<BranchName>.+)$".to_string();

    repo.commit_and_assert("0.1.0-pre.1");
    repo.branch("stabilize/my/1.0.0");
    repo.checkout(MAIN_BRANCH);
    repo.commit_and_assert("1.1.0-pre.1");
}

#[rstest]
fn test_release_branches_not_matching_current_trunk_version_start_new_release_at_their_root(
    repo: TestRepo,
) {
    repo.commit_and_assert("0.1.0-pre.1");
    repo.branch("release/1.0.0");
    repo.commit_and_assert("1.0.0-pre.1");
}

#[rstest]
fn test_release_branches_not_matching_current_trunk_version_increment_start_new_release_at_their_root(
    repo: TestRepo,
) {
    repo.commit_and_assert("0.1.0-pre.1");
    repo.tag_and_assert("v", "1.0.0");
    repo.commit_and_assert("1.1.0-pre.1");

    repo.branch("release/1.2.0");

    repo.commit_and_assert("1.2.0-pre.1");
}

#[rstest]
fn test_release_tags_with_matching_version_tag_prefix_are_considered(
    repo: TestRepo,
    #[values("v", "V", "")] prefix: &str,
) {
    let source = repo.commit("0.1.0-pre.1");
    repo.tag_and_assert(prefix, "1.0.0").source_id(source);
}

#[rstest]
fn test_prerelease_tags_with_matching_version_tag_prefix_are_ignored(repo: TestRepo) {
    repo.commit_and_assert("0.1.0-pre.1");
    repo.tag("v1.0.0-pre.1");
    repo.assert().version("0.1.0-pre.1");
}

#[rstest]
fn test_release_tags_without_matching_version_tag_prefix_are_ignored(
    repo: TestRepo,
    #[values("a", "x", "p", "vv", "Vv")] prefix: &str,
) {
    repo.commit_and_assert("0.1.0-pre.1");
    repo.tag(&format!("{prefix}1.0.0"));
    repo.assert().version("0.1.0-pre.1");
}

#[rstest]
fn test_tags_with_matching_custom_version_tag_prefix_are_considered(mut repo: TestRepo) {
    repo.config.version_pattern = "my/v(?<Version>.+)".to_string();

    repo.commit_and_assert("0.1.0-pre.1");
    repo.tag_and_assert("my/v", "1.0.0");
}

#[rstest]
fn test_feature_branches_from_main_branch_inherits_main_branch_base_version(
    repo: TestRepo,
    #[values("/", "-", "s/", "s-")] case: &str,
) {
    repo.commit_and_assert("0.1.0-pre.1");
    repo.tag_and_assert("v", "1.0.0");
    repo.branch(&format!("feature{case}feature"));
    repo.commit_and_assert("1.1.0-feature.1");
}

#[rstest]
fn test_feature_branches_from_release_branch_inherits_main_branch_base_version(repo: TestRepo) {
    repo.commit_and_assert("0.1.0-pre.1");
    repo.branch("release/0.2.0");
    repo.commit_and_assert("0.2.0-pre.1");
    repo.branch("feature/feature");
    repo.commit_and_assert("0.2.0-feature.1");
}

#[rstest]
fn test_feature_branches_from_feature_branches_extend_source_feature_branch(repo: TestRepo) {
    repo.commit_and_assert("0.1.0-pre.1");
    repo.tag_and_assert("v", "1.0.0");
    repo.branch("feature/feature-A");
    repo.commit_and_assert("1.1.0-feature-A.1");
    repo.branch("feature/feature-B");
    repo.commit_and_assert("1.1.0-feature-B.2"); // feature 2 has 2 ahead of main
}

#[rstest]
fn test_feature_branches_matching_custom_pattern_inherit_source_branch_base_version(
    mut repo: TestRepo,
) {
    repo.config.feature_branch = r"^feat(ure)?/(?<BranchName>.+)$".to_string();

    repo.commit_and_assert("0.1.0-pre.1");
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
    let branch_name = &format!("feature/a{incompatible_symbol}a");

    repo.commit("irrelevant");
    repo.branch(branch_name);

    repo.commit_and_assert("0.1.0-a-a.1")
        .branch_name(branch_name);
}

#[rstest]
fn test_non_matching_branches_are_treated_as_feature_branches(repo: TestRepo) {
    repo.commit_and_assert("0.1.0-pre.1");
    repo.tag_and_assert("v", "1.0.0");
    repo.branch("refactor/abc");
    repo.commit_and_assert("1.1.0-refactor-abc.1");
}

#[rstest]
fn test_weighted_prerelease_number_for_main_branch_adds_55000(repo: TestRepo) {
    repo.commit_and_assert("0.1.0-pre.1")
        .weighted_pre_release_number(55001);
}

#[rstest]
fn test_weighted_prerelease_number_for_main_branch_release_tag_adds_60000(repo: TestRepo) {
    repo.commit_and_assert("0.1.0-pre.1");
    repo.tag_and_assert("v", "1.0.0")
        .weighted_pre_release_number(60000);
}

#[rstest]
fn test_weighted_prerelease_number_for_release_branch_adds_55000(repo: TestRepo) {
    repo.commit_and_assert("0.1.0-pre.1");
    repo.branch("release/0.1.0");

    repo.commit_and_assert("0.1.0-pre.2")
        .weighted_pre_release_number(55002);
}

#[rstest]
fn test_weighted_prerelease_number_for_release_branch_release_tag_adds_60000(repo: TestRepo) {
    repo.commit_and_assert("0.1.0-pre.1");
    repo.branch("release/0.1.0");

    repo.commit_and_assert("0.1.0-pre.2");
    repo.tag_and_assert("v", "0.1.0")
        .weighted_pre_release_number(60000);
}

#[rstest]
fn test_weighted_prerelease_number_for_feature_branch_adds_30000(repo: TestRepo) {
    repo.commit_and_assert("0.1.0-pre.1");
    repo.branch("feature/feature-A");

    repo.commit_and_assert("0.1.0-feature-A.1")
        .weighted_pre_release_number(30001);
}

#[rstest]
fn test_assembly_sem_ver_is_major_minor_patch_dot_zero(repo: TestRepo) {
    repo.commit_and_assert("0.1.0-pre.1")
        .assembly_sem_ver("0.1.0.0");
}

#[rstest]
fn test_assembly_sem_file_ver_is_major_minor_patch_dot_weighted_pre_release_number(repo: TestRepo) {
    repo.commit_and_assert("0.1.0-pre.1")
        .assembly_sem_ver("0.1.0.55001");
}
