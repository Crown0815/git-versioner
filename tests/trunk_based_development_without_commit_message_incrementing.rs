mod common;

use crate::common::{MAIN_BRANCH, TestRepo, VisualizableRepo};
use rstest::{fixture, rstest};

#[fixture]
fn repo(#[default(MAIN_BRANCH)] main_branch: &str) -> TestRepo {
    let mut repo = TestRepo::initialize(main_branch);
    repo.config.commit_message_incrementing = "Disabled".to_string();
    repo
}

#[fixture]
fn mermaid_repo(#[default(MAIN_BRANCH)] main_branch: &str) -> VisualizableRepo {
    let mut repo = VisualizableRepo::initialize(main_branch);
    repo.config().commit_message_incrementing = "Disabled".to_string();
    repo
}

/// # Trunk-based Development
///
/// %%MERMAID%%
#[rstest]
fn test_full_workflow(mermaid_repo: VisualizableRepo) {
    let repo = mermaid_repo;
    repo.commit_and_assert("0.1.0-pre.1");
    repo.commit_with_tag_and_assert("0.1.0-pre.2", "v", "1.0.0");
    repo.branch("release/1.0.0");

    repo.checkout(MAIN_BRANCH);
    repo.commit_and_assert("1.1.0-pre.1");

    repo.checkout("release/1.0.0");
    repo.commit_and_assert("1.0.1-pre.1");
    repo.commit_with_tag_and_assert("1.0.1-pre.2", "v", "1.0.1");
    repo.commit_and_assert("1.0.2-pre.1");
    repo.commit_with_tag_and_assert("1.0.2-pre.2", "v", "1.0.2");

    repo.checkout(MAIN_BRANCH);
    repo.commit_and_assert("1.1.0-pre.2");
    repo.branch("release/1.1.0");
    repo.checkout(MAIN_BRANCH);
    repo.commit_and_assert("1.2.0-pre.1");

    repo.checkout("release/1.1.0");
    repo.commit_and_assert("1.1.0-pre.3");
    repo.commit_with_tag_and_assert("1.1.0-pre.4", "v", "1.1.0");
    repo.commit_and_assert("1.1.1-pre.1");
    repo.commit_with_tag_and_assert("1.1.1-pre.2", "v", "1.1.1");

    repo.checkout(MAIN_BRANCH);
    repo.commit_and_assert("1.2.0-pre.2");
    repo.branch("release/1.2.0");
    repo.checkout(MAIN_BRANCH);
    repo.commit_and_assert("1.3.0-pre.1");

    repo.checkout("release/1.2.0");
    repo.commit_and_assert("1.2.0-pre.3");
    repo.commit_with_tag_and_assert("1.2.0-pre.4", "v", "1.2.0");
    repo.commit_and_assert("1.2.1-pre.1");
    repo.commit_with_tag_and_assert("1.2.1-pre.2", "v", "1.2.1");

    repo.checkout(MAIN_BRANCH);
    repo.commit_with_tag_and_assert("1.3.0-pre.2", "v", "1.3.0");
    repo.commit_and_assert("1.4.0-pre.1");
    repo.commit_and_assert("1.4.0-pre.2");

    repo.branch("release/2.0.0");
    repo.commit_and_assert("2.0.0-pre.1");

    repo.checkout(MAIN_BRANCH);
    repo.commit_and_assert("2.1.0-pre.1");

    repo.write_markdown(file!(), "test_full_workflow");
}

/// # Trunk-based Development with Feature Branches
///
/// %%MERMAID%%
#[rstest]
fn test_full_workflow_with_feature_branches(mermaid_repo: VisualizableRepo) {
    let repo = mermaid_repo;
    repo.commit_and_assert("0.1.0-pre.1");
    repo.branch("feature/feature1");
    repo.commit_and_assert("0.1.0-feature1.1");

    repo.checkout(MAIN_BRANCH);
    repo.merge_with_tag_and_assert("feature/feature1", "0.1.0-pre.3", "v", "1.0.0");
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

    repo.write_markdown(file!(), "test_full_workflow_with_feature_branches");
}

#[rstest]
fn test_result_on_detached_head_is_no_branch(repo: TestRepo) {
    let (sha, _) = repo.commit("commit");
    repo.checkout(&sha);

    repo.assert()
        .branch_name("(no branch)")
        .escaped_branch_name("-no-branch-");
}

#[rstest]
fn test_result_on_checked_out_version_tag_is_the_tag_version(repo: TestRepo) {
    let (sha, _) = repo.commit("commit");
    repo.tag("v1.0.0");
    repo.checkout("tags/v1.0.0");

    repo.assert()
        .full_sem_ver("1.0.0")
        .version_source_sha(&sha)
        .branch_name("(no branch)")
        .escaped_branch_name("-no-branch-");
}

#[rstest]
fn test_branch_name_on_main_branch(repo: TestRepo) {
    repo.commit("commit");
    repo.assert().branch_name(MAIN_BRANCH);
}

#[rstest]
fn test_support_of_custom_trunk_pattern(#[with("custom-trunk")] mut repo: TestRepo) {
    repo.config.main_branch = r"^custom-trunk$".to_string();

    repo.commit_and_assert("0.1.0-pre.1")
        .branch_name("custom-trunk")
        .version_source_sha("");
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
fn test_release_branches_may_only_define_partial_semantic_version(repo: TestRepo) {
    repo.commit_and_assert("0.1.0-pre.1");
    repo.branch("release/1.0");
    repo.checkout(MAIN_BRANCH);
    repo.commit_and_assert("1.1.0-pre.1");
}

#[rstest]
fn test_release_branches_matching_initial_trunk_version_continue_release_at_version_root(
    repo: TestRepo,
) {
    repo.commit_and_assert("0.1.0-pre.1");
    repo.branch("release/0.1.0");
    repo.commit_and_assert("0.1.0-pre.2").version_source_sha("");
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

    let (source, _) = repo.commit("0.1.0-pre.1");
    repo.branch("stabilize/my/1.0.0");
    repo.checkout(MAIN_BRANCH);
    repo.commit_and_assert("1.1.0-pre.1")
        .branch_name(MAIN_BRANCH)
        .version_source_sha(&source);
}

#[rstest]
fn test_release_branches_matching_custom_pattern_create_expected_prerelease(mut repo: TestRepo) {
    repo.config.release_branch = r"^stabilize/my/(?<BranchName>.+)$".to_string();

    repo.commit_and_assert("0.1.0-pre.1");
    let (source, _) = repo.tag("v1.0.0");
    repo.branch("stabilize/my/1.0.0");
    repo.commit_and_assert("1.0.1-pre.1")
        .branch_name("stabilize/my/1.0.0")
        .version_source_sha(&source);
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
    let (source_sha, _) = repo.commit("0.1.0+1");
    repo.tag_and_assert(prefix, "1.0.0")
        .version_source_sha(&source_sha);
}

#[rstest]
fn test_prerelease_tags_with_matching_version_tag_prefix_are_ignored(repo: TestRepo) {
    repo.commit_and_assert("0.1.0-pre.1");
    repo.tag("v1.0.0-pre.1");
    repo.assert().full_sem_ver("0.1.0-pre.1");
}

#[rstest]
fn test_release_tags_without_matching_version_tag_prefix_are_ignored(
    repo: TestRepo,
    #[values("a", "x", "p", "vv", "Vv")] prefix: &str,
) {
    repo.commit_and_assert("0.1.0-pre.1");
    repo.tag(&format!("{prefix}1.0.0"));
    repo.assert().full_sem_ver("0.1.0-pre.1");
}

#[rstest]
fn test_tags_with_matching_custom_version_tag_prefix_are_considered(mut repo: TestRepo) {
    repo.config.tag_prefix = "my/v".to_string();

    repo.commit_and_assert("0.1.0-pre.1");
    let (sha, _) = repo.tag("my/v1.0.0");

    repo.assert()
        .full_sem_ver("1.0.0")
        .branch_name(MAIN_BRANCH)
        .version_source_sha(&sha);
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
    let (source, _) = repo.tag("v1.0.0");
    repo.branch("feat/feature");
    repo.commit_and_assert("1.1.0-feature.1")
        .branch_name("feat/feature")
        .version_source_sha(&source);
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
fn test_weighted_prerelease_number_for_main_branch_as_release(mut repo: TestRepo) {
    repo.config.as_release = true;
    repo.commit_and_assert("0.1.0")
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
fn test_weighted_prerelease_number_for_release_branch_as_release_adds_60000(mut repo: TestRepo) {
    repo.commit_and_assert("0.1.0-pre.1");
    repo.branch("release/0.1.0");

    repo.config.as_release = true;
    repo.commit_and_assert("0.1.0")
        .weighted_pre_release_number(60000);
}

#[rstest]
fn test_weighted_prerelease_number_for_checked_out_release_tag_adds_60000(repo: TestRepo) {
    repo.commit_and_assert("0.1.0-pre.1");
    repo.commit_and_assert("0.1.0-pre.2");
    let (sha, _) = repo.tag("v0.1.0");
    repo.checkout("tags/v0.1.0");

    repo.assert()
        .full_sem_ver("0.1.0")
        .version_source_sha(&sha)
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
        .assembly_sem_file_ver("0.1.0.55001");
}

#[rstest]
fn test_sha_matches_head(repo: TestRepo) {
    let (sha, _) = repo.commit("0.1.0+1");
    repo.assert().sha(&sha);
}

#[rstest]
fn test_short_sha_is_first_7_chars_of_sha(repo: TestRepo) {
    let (sha, _) = repo.commit("0.1.0+1");
    repo.assert().short_sha(&sha[..7]);
}

#[rstest]
fn test_custom_prerelease_tag(mut repo: TestRepo) {
    repo.config.pre_release_tag = "alpha".to_string();
    repo.commit_and_assert("0.1.0-alpha.1")
        .branch_name(MAIN_BRANCH)
        .version_source_sha("");
}
