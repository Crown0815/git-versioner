use anyhow::{Result, anyhow};
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
pub struct VersionTag {
    pub version: Version,
    pub commit_id: Oid,
}

pub struct GitVersioner {
    branch_type: BranchType,
    version_tags: Vec<VersionTag>,
}

impl GitVersioner {
    pub fn calculate_version<P: AsRef<Path>>(repo_path: P) -> Result<Version> {
        let repo = Repository::open(repo_path)?;
        let branch_type = Self::determine_current_branch_type(&repo)?;
        let version_tags = Self::collect_version_tags(&repo)?;
        let versioner = Self {
            branch_type,
            version_tags,
        };

        match &versioner.branch_type {
            BranchType::Trunk => versioner.calculate_trunk_version(),
            BranchType::Release(release_version) => {
                versioner.calculate_release_version(&release_version)
            }
            BranchType::Other(_) => Err(anyhow!(
                "Version calculation not supported for non-trunk/release branches"
            )),
        }
    }

    fn determine_current_branch_type(repo: &Repository) -> Result<BranchType> {
        match repo.head() {
            Ok(head) => Self::determine_branch_type_from(head),
            Err(error) => Err(anyhow!("Failed to get HEAD: {}", error)),
        }
    }

    fn determine_branch_type_from(reference: Reference) -> Result<BranchType> {
        if !reference.is_branch() {
            return Err(anyhow!("HEAD is not on a branch"));
        }

        let branch_name = reference.shorthand().unwrap_or("unknown");

        if branch_name == "trunk" || branch_name == "main" || branch_name == "master" {
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

    /// Collect all version tags from the repository
    fn collect_version_tags(repo: &Repository) -> Result<Vec<VersionTag>> {
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

                    version_tags.push(VersionTag { version, commit_id });
                }
            }
        }

        // Sort tags by version
        version_tags.sort_by(|a, b| a.version.cmp(&b.version));

        Ok(version_tags)
    }

    /// Calculate version for trunk branch
    fn calculate_trunk_version(&self) -> Result<Version> {
        // Find the latest version tag on the trunk
        let latest_trunk_tag = self.find_latest_trunk_tag()?;

        // If we have a tag, increase the minor version and add rc.1
        if let Some(tag) = latest_trunk_tag {
            let mut new_version = tag.version.clone();
            new_version.minor += 1;
            new_version.patch = 0;

            // Check if we already have rc tags for this version
            let rc_number = self.get_next_rc_number(&new_version)?;
            new_version.pre = Prerelease::new(&format!("rc.{}", rc_number))?;

            Ok(new_version)
        } else {
            // If no tags exist, start with 0.1.0-rc.0
            let mut version = Version::new(0, 1, 0);
            version.pre = Prerelease::new("rc.0")?;
            Ok(version)
        }
    }

    /// Calculate the version for the release branch
    fn calculate_release_version(&self, release_version: &Version) -> Result<Version> {
        // Find the latest tag on this release branch
        let latest_release_tag = self.find_latest_release_tag(release_version)?;

        if let Some(tag) = latest_release_tag {
            let mut new_version = tag.version.clone();

            // If the tag has no pre-release component, it's a released version.
            // So we increment the patch version for the next release candidate
            if new_version.pre.is_empty() {
                new_version.patch += 1;
                new_version.pre = Prerelease::new("rc.1")?;
            } else {
                // It's already a release candidate, so increment the rc number
                let rc_number = self.get_next_rc_number(&new_version)?;
                new_version.pre = Prerelease::new(&format!("rc.{}", rc_number))?;
            }

            Ok(new_version)
        } else {
            // No tags on this release branch yet, so use the release version with rc.1
            let mut version = release_version.clone();
            version.pre = Prerelease::new("rc.1")?;
            Ok(version)
        }
    }

    /// Find the latest version tag on the trunk branch
    fn find_latest_trunk_tag(&self) -> Result<Option<VersionTag>> {
        // Get all tags that are reachable from the trunk but don't have pre-release components
        let mut released_tags = self
            .version_tags
            .iter()
            .filter(|tag| tag.version.pre.is_empty())
            .cloned()
            .collect::<Vec<_>>();

        // Sort by version (highest last)
        released_tags.sort_by(|a, b| a.version.cmp(&b.version));

        // Return the highest version
        Ok(released_tags.last().cloned())
    }

    /// Find the latest version tag on a specific release branch
    fn find_latest_release_tag(&self, release_version: &Version) -> Result<Option<VersionTag>> {
        // Get all tags that match the major.minor of the release version
        let mut matching_tags = self
            .version_tags
            .iter()
            .filter(|tag| {
                tag.version.major == release_version.major
                    && tag.version.minor == release_version.minor
            })
            .cloned()
            .collect::<Vec<_>>();

        // Sort by version (highest last)
        matching_tags.sort_by(|a, b| a.version.cmp(&b.version));

        // Return the highest version
        Ok(matching_tags.last().cloned())
    }

    /// Get the next release candidate number for a version
    fn get_next_rc_number(&self, version: &Version) -> Result<u64> {
        // Find all rc tags for this version
        let rc_regex = Regex::new(r"^rc\.(\d+)$")?;

        let mut max_rc = 0;

        for tag in &self.version_tags {
            // Check if the tag matches our version's major.minor.patch
            if tag.version.major == version.major
                && tag.version.minor == version.minor
                && tag.version.patch == version.patch
            {
                // Check if the tag is a release candidate
                if let Some(captures) = rc_regex.captures(tag.version.pre.as_str()) {
                    if let Some(rc_str) = captures.get(1) {
                        if let Ok(rc_num) = rc_str.as_str().parse::<u64>() {
                            max_rc = max_rc.max(rc_num);
                        }
                    }
                }
            }
        }

        // Return the next rc number
        Ok(max_rc + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::process::Output;

    struct TestRepo {
        path: PathBuf,
        _temp_dir: tempfile::TempDir, // Keep the temp_dir to prevent it from being deleted
    }

    impl TestRepo {
        fn new() -> Self {
            let repo = Self::create_empty_path();
            repo.initialize();
            repo
        }

        fn create_empty_path() -> Self {
            let temp_dir = tempfile::tempdir().unwrap();
            let path = temp_dir.path().to_path_buf();
            Self {
                path,
                _temp_dir: temp_dir,
            }
        }

        fn initialize(&self) {
            self.execute(&["init"], "initialize repository");
            self.execute(&["config", "user.name", "tester"], "configure user.name");
            self.execute(
                &["config", "user.email", "tester@test.com"],
                "configure user.email",
            );
            self.commit("initial commit");
            self.create_branch("trunk");
        }

        fn commit(&self, message: &str) -> Oid {
            self.execute(
                &["commit", "--allow-empty", "-m", message],
                format!("commit {message}").as_str(),
            );
            let output = self.execute(&["rev-parse", "HEAD"], "get commit hash");

            let commit_hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Oid::from_str(&commit_hash).unwrap()
        }

        fn create_branch(&self, name: &str) {
            self.execute(&["branch", name], format!("create branch {name}").as_str());
        }

        fn checkout_branch(&self, name: &str) {
            self.execute(
                &["switch", name],
                format!("checkout branch {name}").as_str(),
            );
        }

        fn tag(&self, name: &str) {
            self.execute(&["tag", name], format!("create tag {name}").as_str());
        }

        fn execute(&self, command: &[&str], description: &str) -> Output {
            std::process::Command::new("git")
                .args(command)
                .current_dir(&self.path)
                .output()
                .expect(format!("Failed to {description}").as_str())
        }
    }

    #[test]
    fn test_trunk_versioning() {
        let test_repo = TestRepo::new();

        test_repo.checkout_branch("trunk");

        // Calculate version - should be 0.1.0-rc.0 as no tags exist
        let version = GitVersioner::calculate_version(&test_repo.path).unwrap();
        let expected = Version::parse("0.1.0-rc.0").unwrap();
        assert_eq!(version, expected);

        // Add a tag and check version calculation
        test_repo.tag("v0.1.0");
        let version = GitVersioner::calculate_version(&test_repo.path).unwrap();
        let mut expected = Version::new(0, 2, 0);
        expected.pre = Prerelease::new("rc.1").unwrap();
        assert_eq!(version, expected);
    }

    #[test]
    fn test_release_branch_versioning() {
        let test_repo = TestRepo::new();

        // Use the trunk branch that was created in TestRepo::new
        test_repo.checkout_branch("trunk");

        // First commit on trunk
        test_repo.commit("Initial trunk commit");
        test_repo.tag("v1.0.0");

        // Create a release branch
        test_repo.create_branch("release/1.0.0");
        test_repo.checkout_branch("release/1.0.0");

        // First commit on release branch
        test_repo.commit("First release commit");

        // Calculate version - should be 1.0.1-rc.1
        let version = GitVersioner::calculate_version(&test_repo.path).unwrap();
        let mut expected = Version::new(1, 0, 1);
        expected.pre = Prerelease::new("rc.1").unwrap();
        assert_eq!(version, expected);

        // Tag as rc.1
        test_repo.tag("v1.0.1-rc.1");

        // Another commit
        test_repo.commit("Second release commit");

        // Calculate version - should be 1.0.1-rc.2
        let version = GitVersioner::calculate_version(&test_repo.path).unwrap();
        let mut expected = Version::new(1, 0, 1);
        expected.pre = Prerelease::new("rc.2").unwrap();
        assert_eq!(version, expected);

        // Tag as final release
        test_repo.tag("v1.0.1");

        // Another commit
        test_repo.commit("Third release commit");

        // Calculate version - should be 1.0.2-rc.1
        let version = GitVersioner::calculate_version(&test_repo.path).unwrap();
        let mut expected = Version::new(1, 0, 2);
        expected.pre = Prerelease::new("rc.1").unwrap();
        assert_eq!(version, expected);
    }

    #[test]
    fn test_complex_workflow() {
        let test_repo = TestRepo::new();

        // Use the trunk branch that was created in TestRepo::new
        test_repo.checkout_branch("trunk");

        // First commits on trunk
        test_repo.commit("Initial trunk commit");
        test_repo.tag("v0.1.0-rc.0");
        test_repo.commit("Second trunk commit");
        test_repo.tag("v0.1.0-rc.1");

        // Tag a final release to make the next version increment the minor version
        test_repo.tag("v0.1.0");

        // Create the first release branch
        test_repo.create_branch("release/1.0.0");

        // Continue on the trunk
        test_repo.commit("Third trunk commit");

        // Calculate version on trunk - should be 0.2.0-rc.1
        let version = GitVersioner::calculate_version(&test_repo.path).unwrap();
        let mut expected = Version::new(0, 2, 0);
        expected.pre = Prerelease::new("rc.1").unwrap();
        assert_eq!(version, expected);

        // Switch to the release branch
        test_repo.checkout_branch("release/1.0.0");

        // Commit on the release branch
        test_repo.commit("First release commit");
        test_repo.tag("v1.0.0-rc.1");
        test_repo.commit("Second release commit");
        test_repo.tag("v1.0.0-rc.2");
        test_repo.tag("v1.0.0"); // Final release

        // Calculate version on the release branch - should be 1.0.1-rc.1
        let version = GitVersioner::calculate_version(&test_repo.path).unwrap();
        let mut expected = Version::new(1, 0, 1);
        expected.pre = Prerelease::new("rc.1").unwrap();
        assert_eq!(version, expected);
    }
}
