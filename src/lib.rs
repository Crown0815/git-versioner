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
    config: GitVersionConfig,
    trunk_pattern: Regex,
    release_pattern: Regex,
}

pub struct GitVersionConfig {
    pub repo: Repository,
    pub version_tag_prefix: String,
}

pub const TRUNK_BRANCH_REGEX: &str = r"^(trunk|main|master)$";

impl GitVersioner {
    pub fn calculate_version<P: AsRef<Path>>(repo_path: P, trunk_branch_regex: &str) -> Result<Version> {
        let config = GitVersionConfig {
            repo: Repository::open(repo_path)?,
            version_tag_prefix: "[vV]?".to_string(),
        };

        let versioner = Self {
            config,
            trunk_pattern: Regex::new(trunk_branch_regex)?,
            release_pattern: Regex::new(r"^releases?[\\/-](?<BranchName>.+)$")?,
        };

        match versioner.determine_branch_at_head()? {
            BranchType::Trunk => versioner.calculate_version_for_trunk(),
            BranchType::Release(version) => versioner.calculate_version_for_release(&version),
            BranchType::Other(_) => Err(anyhow!("Version calculation not supported for non-trunk/release branches")),
        }
    }

    fn determine_branch_at_head(&self) -> Result<BranchType> {
        match self.head() {
            Ok(head) => self.determine_branch_at(head),
            Err(error) => Err(anyhow!("Failed to get HEAD: {}", error)),
        }
    }

    fn head(&self) -> Result<Reference, git2::Error> {
        self.config.repo.head()
    }

    fn determine_branch_at(&self, reference: Reference) -> Result<BranchType> {
        if !reference.is_branch() {
            return Err(anyhow!("HEAD is not on a branch"));
        }

        match reference.shorthand() {
            None => Err(anyhow!("Name for branch could not be determined")),
            Some(name) => Ok(self.determine_branch_type_by_name(name)),
        }
    }

    fn determine_branch_type_by_name(&self, name: &str) -> BranchType {
        if self.trunk_pattern.is_match(name) {
            return BranchType::Trunk;
        }

        if let Some(captures) = self.release_pattern.captures(name) {
            if let Some(version_str) = captures.get(1) {
                if let Ok(version) = Version::parse(version_str.as_str()) {
                    return BranchType::Release(version);
                }
            }
        }

        BranchType::Other(name.to_string())
    }

    fn collect_version_tags(&self) -> Result<Vec<VersionSource>> {
        let mut version_tags = Vec::new();

        let tag_names = self.config.repo.tag_names(None)?;
        let regex = Regex::new(&format!("^{}", &self.config.version_tag_prefix))?;

        for tag_name in tag_names.iter().flatten() {
            let version_str = regex.replacen(tag_name, 1, "");
            if let Ok(version) = Version::parse(&version_str) {
                if let Ok(tag_obj) = self.config.repo.revparse_single(&format!("refs/tags/{}", tag_name)) {
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

    fn collect_sources_from_release_branches(&self) -> Result<Vec<VersionSource>> {
        let mut matching_branches = Vec::new();

        // Iterate over local branches
        let branches = self.config.repo.branches(Some(git2::BranchType::Local))?;
        for branch in branches {
            let (branch, _) = branch?;
            if let Some(name) = branch.name()? {
                if let Some(captures) = self.release_pattern.captures(name) {
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

    fn calculate_version_for_trunk(&self) -> Result<Version> {
        let latest_trunk_tag = self.find_latest_trunk_tag()?;
        let repo = &self.config.repo;

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

    fn calculate_version_for_release(&self, release_version: &Version) -> Result<Version> {
        let latest_release_tag = self.find_latest_tag_for_release_branch(release_version)?;
        let repo = &self.config.repo;
        
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
        let mut released_tags = [&self.collect_version_tags()?[..], &self.collect_sources_from_release_branches()?[..]].concat()
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
            .collect_version_tags()?
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
            .collect_version_tags()?
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
