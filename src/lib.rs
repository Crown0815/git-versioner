use anyhow::{anyhow, Result};
use git2::{Oid, Reference, Repository};
use regex::Regex;
use semver::{Prerelease, Version};
use std::path::Path;

#[derive(Debug, PartialEq, Eq)]
pub enum BranchType {
    Trunk,            // Main development branch (trunk)
    Release(Version), // Release branch (e.g., release/1.0.0)
    Other(String),    // Feature branch or any other branch type
}

#[derive(Debug, Clone)]
struct VersionSource {
    version: Version,
    commit_id: Oid,
}

pub struct GitVersioner {
    version_tags: Vec<VersionSource>,
    version_branches: Vec<VersionSource>,
}

pub const TRUNK_BRANCH_REGEX: &str = r"^(trunk|main|master)$";

impl GitVersioner {
    pub fn calculate_version<P: AsRef<Path>>(repo_path: P, trunk_branch_regex: &str) -> Result<Version> {
        let repo = Repository::open(repo_path)?;

        let trunk_branch_regex = Regex::new(trunk_branch_regex)?;

        let branch_type = match repo.head() {
            Ok(head) => Self::determine_branch_type_from_regex(head, &trunk_branch_regex)?,
            Err(error) => return Err(anyhow!("Failed to get HEAD: {}", error)),
        };

        let versioner = Self {
            version_tags: Self::collect_version_tags(&repo)?,
            version_branches: Self::collect_sources_from_release_branches(&repo)?,
        };

        match branch_type {
            BranchType::Trunk => versioner.calculate_version_for_trunk(&repo),
            BranchType::Release(version) => versioner.calculate_release_version(&repo, &version),
            BranchType::Other(_) => Err(anyhow!("Version calculation not supported for non-trunk/release branches")),
        }
    }

    fn determine_branch_type_from_regex(reference: Reference, trunk_regex: &Regex) -> Result<BranchType> {
        if !reference.is_branch() {
            return Err(anyhow!("HEAD is not on a branch"));
        }

        let branch_name = reference.shorthand().unwrap_or("unknown");

        // Use the provided trunk branch regex
        if trunk_regex.is_match(branch_name) {
            return Ok(BranchType::Trunk);
        }

        // Check if it's a release branch
        let release_regex = Regex::new(r"^release/(\d+\.\d+\.\d+)$")?;
        if let Some(captures) = release_regex.captures(branch_name) {
            if let Some(version_str) = captures.get(1) {
                if let Ok(version) = Version::parse(version_str.as_str()) {
                    return Ok(BranchType::Release(version));
                }
            }
        }

        Ok(BranchType::Other(branch_name.to_string()))
    }

    fn collect_version_tags(repo: &Repository) -> Result<Vec<VersionSource>> {
        let mut version_tags = Vec::new();

        // Collect all tags
        let tag_names = repo.tag_names(None)?;

        for tag_name in tag_names.iter().flatten() {
            // Try to parse the tag as a version
            let version_str = tag_name.trim_start_matches('v');
            if let Ok(version) = Version::parse(version_str) {
                // Get the tag object
                if let Ok(tag_obj) = repo.revparse_single(&format!("refs/tags/{}", tag_name)) {
                    let commit_id = if let Some(tag) = tag_obj.as_tag() {
                        tag.target_id()
                    } else {
                        tag_obj.id()
                    };

                    version_tags.push(VersionSource { version, commit_id });
                }
            }
        }

        Ok(version_tags)
    }

    fn collect_sources_from_release_branches(repo: &Repository) -> Result<Vec<VersionSource>> {
        let release_regex = Regex::new(r"^release/(\d+\.\d+\.\d+)$")?;

        let mut matching_branches = Vec::new();

        // Iterate over local branches
        let branches = repo.branches(Some(git2::BranchType::Local))?;
        for branch in branches {
            let (branch, _) = branch?;
            if let Some(name) = branch.name()? {
                if let Some(captures) = release_regex.captures(name) {
                    if let Some(version_str) = captures.get(1) {
                        if let Ok(version) = Version::parse(version_str.as_str()) {
                            matching_branches.push(VersionSource {version, commit_id: branch.get().peel_to_commit()?.id()});
                        }
                    }
                }
            }
        }

        Ok(matching_branches)
    }

    fn calculate_version_for_trunk(&self, repo: &Repository) -> Result<Version> {
        let latest_trunk_tag = self.find_latest_trunk_tag()?;


        // If we have a tag, increase the minor version and add rc.1
        if let Some(tag) = latest_trunk_tag {
            let head_id = repo.head()?.peel_to_commit()?.id();
            if head_id == tag.commit_id {
                return Ok(tag.version);
            }

            let merge_base_oid = repo.merge_base(head_id, tag.commit_id)?;

            let mut revwalk = repo.revwalk()?;
            revwalk.push_head()?;
            revwalk.set_sorting(git2::Sort::TOPOLOGICAL)?;
            let mut count = 0;
            for oid in revwalk {
                let oid = oid?;
                if oid == merge_base_oid {
                    break; // Stop counting when the specific commit is reached
                }
                count += 1;
            }

            let mut version = tag.version.clone();
            version.minor += 1;
            version.patch = 0;
            version.pre = Prerelease::new(&format!("rc.{}", count))?;
            Ok(version)
        } else {
            let mut revwalk = repo.revwalk()?;
            revwalk.push_head()?;
            revwalk.set_sorting(git2::Sort::TOPOLOGICAL)?;
            let count = revwalk.count();

            let mut version = Version::new(0, 1, 0);
            version.pre = Prerelease::new(&format!("rc.{}", count))?;
            Ok(version)
        }
    }

    /// Calculate the version for the release branch
    fn calculate_release_version(&self, repo: &Repository, release_version: &Version) -> Result<Version> {
        // Find the latest tag on this release branch
        let latest_release_tag = self.find_latest_tag_for_release_branch(release_version)?;

        if let Some(tag) = latest_release_tag {

            let mut revwalk = repo.revwalk()?;
            revwalk.push_head()?;
            revwalk.set_sorting(git2::Sort::TOPOLOGICAL)?;
            let mut count = 0;
            for oid in revwalk {
                let oid = oid?;
                if oid == tag.commit_id {
                    break; // Stop counting when the specific commit is reached
                }
                count += 1;
            }

            if count == 0 {
                return Ok(tag.version);
            }

            let mut new_version = tag.version.clone();
            new_version.patch += 1;
            new_version.pre = Prerelease::new(&format!("rc.{}", count))?;

            Ok(new_version)
        } else {
            let tag = self.find_latest_tag_base_for_release_branch()?.unwrap();

            let head_id = repo.head()?.peel_to_commit()?.id();
            if head_id == tag.commit_id {
                return Ok(tag.version);
            }

            let merge_base_oid = repo.merge_base(head_id, tag.commit_id)?;

            let mut revwalk = repo.revwalk()?;
            revwalk.push_head()?;
            revwalk.set_sorting(git2::Sort::TOPOLOGICAL)?;
            let mut count = 0;
            for oid in revwalk {
                let oid = oid?;
                if oid == merge_base_oid {
                    break; // Stop counting when the specific commit is reached
                }
                count += 1;
            }

            // No tags on this release branch yet, so use the release version with rc.1
            let mut version = release_version.clone();
            version.pre = Prerelease::new(&format!("rc.{}", count))?;
            Ok(version)
        }
    }

    /// Find the latest version tag on the trunk branch
    fn find_latest_trunk_tag(&self) -> Result<Option<VersionSource>> {
        let mut released_tags = [&self.version_tags[..], &self.version_branches[..]].concat()
            .iter()
            .filter(|tag| tag.version.pre.is_empty())
            .cloned()
            .collect::<Vec<_>>();

        // Sort by version (highest last)
        released_tags.sort_by(|a, b| a.version.cmp(&b.version));
        Ok(released_tags.last().cloned())
    }

    /// Find the latest version tag on a specific release branch
    fn find_latest_tag_for_release_branch(&self, release_version: &Version) -> Result<Option<VersionSource>> {
        let mut matching_tags = self
            .version_tags
            .iter()
            .filter(|tag| {
                tag.version.major == release_version.major
                    && tag.version.minor == release_version.minor
                    && tag.version.pre.is_empty()
            })
            .cloned()
            .collect::<Vec<_>>();

        matching_tags.sort_by(|a, b| a.version.cmp(&b.version));
        Ok(matching_tags.last().cloned())
    }

    /// Find the latest version tag on a specific release branch
    fn find_latest_tag_base_for_release_branch(&self) -> Result<Option<VersionSource>> {
        let mut matching_tags = self
            .version_tags
            .iter()
            .filter(|tag| {
                    tag.version.pre.is_empty()
            })
            .cloned()
            .collect::<Vec<_>>();

        matching_tags.sort_by(|a, b| a.version.cmp(&b.version));
        Ok(matching_tags.last().cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;
    use std::path::PathBuf;
    use std::process::Output;

    struct TestRepo {
        path: PathBuf,
        _temp_dir: tempfile::TempDir, // Keep the temp_dir to prevent it from being deleted
    }

    impl TestRepo {
        fn new() -> Self {
            let temp_dir = tempfile::tempdir().unwrap();
            let path = temp_dir.path().to_path_buf();
            Self { path, _temp_dir: temp_dir }
        }

        fn initialize(&self) {
            self.execute(&["init", "--initial-branch=trunk"], "initialize repository");
            self.execute(&["config", "user.name", "tester"], "configure user.name");
            self.execute(&["config", "user.email", "tester@test.com"], "configure user.email");
        }

        fn commit(&self, message: &str) -> Oid {
            self.execute(&["commit", "--allow-empty", "-m", message], &format!("commit {message}"));
            let output = self.execute(&["rev-parse", "HEAD"], "get commit hash");

            let commit_hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Oid::from_str(&commit_hash).unwrap()
        }

        fn branch(&self, name: &str) {
            self.execute(&["branch", name], &format!("branch {name}"));
            self.checkout(name);
        }

        fn checkout(&self, name: &str) {
            self.execute(&["checkout", name], &format!("checkout {name}"));
        }

        fn tag(&self, name: &str) {
            self.execute(&["tag", name], &format!("create tag {name}"));
        }

        fn graph(&self) -> String {
            let output = self.execute(&["log", "--graph", "--oneline", "--all", "--decorate"], "get commit graph");
            String::from_utf8_lossy(&output.stdout).to_string()
        }

        fn execute(&self, command: &[&str], description: &str) -> Output {
            std::process::Command::new("git")
                .args(command)
                .current_dir(&self.path)
                .output()
                .expect(&format!("Failed to {description}"))
        }

        fn commit_and_assert(&self, expected_version: &str) {
            self.commit(expected_version);
            assert_version(&self, expected_version);
        }
    }

    #[fixture]
    fn repo() -> TestRepo {
        let repo = TestRepo::new();
        repo.initialize();
        repo
    }

    #[rstest]
    fn test_full_workflow(repo: TestRepo) {
        repo.commit_and_assert("0.1.0-rc.1");
        repo.commit_and_assert("0.1.0-rc.2");
        repo.tag("v1.0.0-rc.2"); // ignored
        repo.tag("v1.0.0");
        assert_version(&repo, "1.0.0");
        repo.branch("release/1.0.0");

        repo.checkout("trunk");
        repo.commit_and_assert("1.1.0-rc.1");

        repo.checkout("release/1.0.0");
        repo.commit_and_assert("1.0.1-rc.1");
        repo.commit_and_assert("1.0.1-rc.2");
        repo.tag("v1.0.1");
        assert_version(&repo, "1.0.1");
        repo.commit_and_assert("1.0.2-rc.1");
        repo.commit_and_assert("1.0.2-rc.2");
        repo.tag("v1.0.2");
        assert_version(&repo, "1.0.2");

        repo.checkout("trunk");
        repo.commit_and_assert("1.1.0-rc.2");
        repo.branch("release/1.1.0");
        repo.checkout("trunk");
        repo.commit_and_assert("1.2.0-rc.1");

        repo.checkout("release/1.1.0");
        repo.commit_and_assert("1.1.0-rc.3");
        repo.commit_and_assert("1.1.0-rc.4");
        repo.tag("v1.1.0");
        assert_version(&repo, "1.1.0");
        repo.commit_and_assert("1.1.1-rc.1");
        repo.commit_and_assert("1.1.1-rc.2");
        repo.tag("v1.1.1");
        assert_version(&repo, "1.1.1");

        repo.checkout("trunk");
        repo.commit_and_assert("1.2.0-rc.2");
        repo.branch("release/1.2.0");
        repo.checkout("trunk");
        repo.commit_and_assert("1.3.0-rc.1");

        repo.checkout("release/1.2.0");
        repo.commit_and_assert("1.2.0-rc.3");
        repo.commit_and_assert("1.2.0-rc.4");
        repo.tag("v1.2.0");
        assert_version(&repo, "1.2.0");
        repo.commit_and_assert("1.2.1-rc.1");
        repo.commit_and_assert("1.2.1-rc.2");
        repo.tag("v1.2.1");
        assert_version(&repo, "1.2.1");

        repo.checkout("trunk");
        repo.commit_and_assert("1.3.0-rc.2");
        repo.tag("v1.3.0");
        assert_version(&repo, "1.3.0");
        repo.commit_and_assert("1.4.0-rc.1");
    }

    #[rstest]
    fn test_custom_trunk_feature(repo: TestRepo) {
        repo.commit("Initial commit");
        repo.branch("custom-trunk");
        repo.execute(&["branch", "-D", "trunk"], "delete trunk branch");

        let result = GitVersioner::calculate_version(&repo.path, TRUNK_BRANCH_REGEX);
        assert!(result.is_err(), "Expected error with default trunk regex, but got: {:?}", result);

        assert_version_with_custom_trunk(&repo, "0.1.0-rc.1", r"^custom-trunk$");
    }

    fn assert_version(repo: &TestRepo, expected: &str) {
        assert_version_with_custom_trunk(repo, expected, TRUNK_BRANCH_REGEX);
    }

    fn assert_version_with_custom_trunk(repo: &TestRepo, expected: &str, trunk_branch_regex: &str) {
        let actual = GitVersioner::calculate_version(&repo.path, trunk_branch_regex).unwrap();
        let expected = Version::parse(expected).unwrap();
        assert_eq!(
            actual,
            expected,
            "Expected HEAD version: {}, found: {}\n\n Git Graph:\n-------\n{}------",
            expected,
            actual,
            repo.graph());
    }
}
