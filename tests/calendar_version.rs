mod common;

use crate::common::{MAIN_BRANCH, TestRepo};
use rstest::{fixture, rstest};

mod with_commit_message_incrementing {
    use super::*;

    #[fixture]
    fn repo(#[default(MAIN_BRANCH)] main_branch: &str) -> TestRepo {
        let mut repo = TestRepo::initialize(main_branch);
        repo.config.commit_message_incrementing = "Enabled".to_string();
        repo
    }

    #[rstest]
    fn test_cal_ver_minor_is_one_before_the_first_feature_release_within_a_year(repo: TestRepo) {
        repo.commit_at("0.1.0-pre.1", "2023-12-31T12:00:00Z");
        repo.assert().cal_ver_minor(1);
    }

    #[rstest]
    fn test_cal_ver_minor_is_one_at_the_first_feature_release_within_a_year(repo: TestRepo) {
        repo.commit_at("0.1.0-pre.1", "2023-12-31T12:00:00Z");
        repo.tag("v0.1.0");
        repo.assert().cal_ver_minor(1);
    }

    #[rstest]
    fn test_cal_ver_minor_remains_one_after_the_first_feature_release_within_a_year(
        repo: TestRepo,
    ) {
        repo.commit_at("0.1.0-pre.1", "2023-12-31T12:00:00Z");
        repo.tag("v0.1.0");
        repo.commit_at("fix: 0.1.1-pre.1", "2023-12-31T12:00:00Z");
        repo.assert().cal_ver_minor(1);
    }

    #[rstest]
    fn test_cal_ver_minor_bumps_at_the_first_feature_after_a_release_within_a_year(repo: TestRepo) {
        repo.commit_at("0.1.0-pre.1", "2023-12-31T12:00:00Z");
        repo.tag("v0.1.0");
        repo.commit_at("feat: 0.2.0-pre.1", "2023-12-31T12:00:00Z");
        repo.assert().cal_ver_minor(2);
    }

    #[rstest]
    fn test_cal_ver_minor_remains_the_same_before_the_first_feature_commit_in_the_next_year(
        repo: TestRepo,
    ) {
        repo.commit_at("0.1.0-pre.1", "2023-12-31T12:00:00Z");
        repo.tag("v0.1.0");
        repo.commit_at("feat: 0.2.0-pre.1", "2023-12-31T12:00:00Z");
        repo.tag("v0.2.0");
        repo.assert().cal_ver_minor(2);
        repo.commit_at("fix: 0.2.1-pre.1", "2024-01-01T12:00:00Z");
        repo.assert().cal_ver_minor(2);
    }
}

mod without_commit_message_incrementing {
    use super::*;

    #[fixture]
    fn repo(#[default(MAIN_BRANCH)] main_branch: &str) -> TestRepo {
        let mut repo = TestRepo::initialize(main_branch);
        repo.config.commit_message_incrementing = "Disabled".to_string();
        repo
    }

    #[rstest]
    fn test_commit_date_parts_are_available_as_outputs(repo: TestRepo) {
        repo.commit_at("0.1.0-pre.1", "2024-03-09T12:34:56Z");

        repo.assert()
            .commit_year("2024")
            .commit_month("03")
            .commit_day("09");
    }

    #[rstest]
    fn test_commit_date_parts_are_available_for_assembly_informational_format(mut repo: TestRepo) {
        repo.config.assembly_informational_format =
            "{CommitYear}.{CommitMonth}.{CommitDay}.{SemVer}".to_string();

        repo.commit_at("0.1.0-pre.1", "2024-03-09T12:34:56Z");

        repo.assert()
            .informational_version("2024.03.09.0.1.0-pre.1");
    }

    #[rstest]
    fn test_cal_ver_minor_is_one_before_the_first_feature_release_within_a_year(repo: TestRepo) {
        repo.commit_at("0.1.0-pre.1", "2023-12-31T12:00:00Z");
        repo.assert().cal_ver_minor(1);
    }

    #[rstest]
    fn test_cal_ver_minor_is_one_at_the_first_feature_release_within_a_year(repo: TestRepo) {
        repo.commit_at("0.1.0-pre.1", "2023-12-31T12:00:00Z");
        repo.tag("v0.1.0");
        repo.assert().cal_ver_minor(1);
    }

    #[rstest]
    fn test_cal_ver_minor_is_two_after_the_first_feature_release_within_a_year(repo: TestRepo) {
        repo.commit_at("0.1.0-pre.1", "2023-12-31T12:00:00Z");
        repo.tag("v0.1.0");
        repo.commit_at("0.2.0-pre.1", "2023-12-31T12:00:00Z");
        repo.assert().cal_ver_minor(2);
    }

    #[rstest]
    fn test_cal_ver_minor_resets_to_one_with_the_first_commit_within_a_new_year(repo: TestRepo) {
        repo.commit_at("0.1.0-pre.1", "2023-12-31T12:00:00Z");
        repo.tag("v0.1.0");
        repo.commit_at("0.2.0-pre.1", "2024-01-01T12:00:00Z");
        repo.assert().cal_ver_minor(1);
    }

    #[rstest]
    fn test_cal_ver_minor_remains_equal_for_patches_to_a_feature_release(repo: TestRepo) {
        repo.commit_at("0.1.0-pre.1", "2023-12-31T12:00:00Z");
        repo.tag("v0.1.0");
        repo.commit_at("0.2.0-pre.1", "2023-12-31T12:00:00Z");
        repo.tag("v0.1.1");
        repo.assert().cal_ver_minor(1);
        repo.branch("release/0.1.0");
        repo.commit_at("0.1.2-pre.1", "2024-01-01T12:00:00Z");
        repo.assert().commit_year("2024").cal_ver_minor(1);
    }

    #[rstest]
    fn test_cal_ver_minor_remains_equal_for_patches_to_last_years_feature_release(repo: TestRepo) {
        repo.commit_at("0.1.0-pre.1", "2023-12-31T12:00:00Z");
        repo.tag("v0.1.0");
        repo.commit_at("0.2.0-pre.1", "2023-12-31T12:00:00Z");
        repo.tag("v0.2.0");
        repo.commit_at("0.2.1-pre.1", "2024-01-01T12:00:00Z");
        repo.tag("v0.2.1");
        repo.assert().commit_year("2024").cal_ver_minor(2);
    }

    #[rstest]
    fn test_cal_ver_minor_is_available_for_assembly_informational_format(mut repo: TestRepo) {
        repo.config.assembly_informational_format =
            "{CommitYear}.{CalVerMinor}.{Patch}{PreReleaseTagWithDash}".to_string();

        repo.commit_at("1.0.0-pre.1", "2024-01-10T12:00:00Z");

        repo.assert().informational_version("2024.1.0-pre.1");
    }
}
