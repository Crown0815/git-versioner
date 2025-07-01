use anyhow::{anyhow, Result};
use git2::{Oid, Reference, Repository};
use regex::Regex;
use semver::{Prerelease, Version};
use std::path::Path;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
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
    repo: Repository,
    trunk_pattern: Regex,
    release_pattern: Regex,
    feature_pattern: Regex,
    version_pattern: Regex,
}

const BRANCH_NAME_ID: &'static str = "BranchName";
const VERSION_ID: &'static str = "Version";

impl GitVersioner {
    pub fn calculate_version<P: AsRef<Path>>(repo_path: P, main_branch: Regex) -> Result<Version> {
        let versioner = Self {
            repo: Repository::open(repo_path)?,
            trunk_pattern: main_branch,
            release_pattern: Regex::new(r"^releases?[\\/-](?<BranchName>.+)$")?,
            feature_pattern: Regex::new(r"^features?[\\/-](?<BranchName>.+)$")?,
            version_pattern: Regex::new("^[vV]?(?<Version>.+)$")?,
        };

        match versioner.determine_branch_at_head()? {
            BranchType::Trunk => versioner.calculate_version_for_trunk(),
            BranchType::Release(version) => versioner.calculate_version_for_release(&version),
            BranchType::Other(name) => versioner.calculate_version_for_feature(&name),
        }
    }

    fn determine_branch_at_head(&self) -> Result<BranchType> {
        match self.head() {
            Ok(head) => self.determine_branch_at(head),
            Err(error) => Err(anyhow!("Failed to get HEAD: {}", error)),
        }
    }

    fn head(&self) -> Result<Reference, git2::Error> {
        self.repo.head()
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
            if let Some(version_str) = captures.name(BRANCH_NAME_ID) {
                if let Ok(version) = Version::parse(version_str.as_str()) {
                    return BranchType::Release(version);
                }
            }
        }

        if let Some(captures) = self.feature_pattern.captures(name) {
            if let Some(branch_name) = captures.name(BRANCH_NAME_ID) {
                return BranchType::Other(Self::escaped(branch_name.as_str()));
            }
        }

        BranchType::Other(name.to_string())
    }

    fn escaped(name: &str) -> String {
        const ESCAPE_CHARACTER: char = '-';
        name.chars().map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                ESCAPE_CHARACTER
            }
        }).collect()
    }

    fn collect_version_tags(&self) -> Result<Vec<VersionSource>> {
        let mut version_tags = Vec::new();

        let tag_names = self.repo.tag_names(None)?;
        for tag_name in tag_names.iter().flatten() {
            if let Some(captures) = self.version_pattern.captures(tag_name){
                if let Some(version_str) = captures.name(VERSION_ID) {
                    if let Ok(version) = Version::parse(version_str.as_str()) {
                        if let Ok(tag_obj) = self.repo.revparse_single(&format!("refs/tags/{}", tag_name)) {
                            let commit_id = if let Some(tag) = tag_obj.as_tag() {
                                tag.target_id()
                            } else {
                                tag_obj.id()
                            };

                            version_tags.push(VersionSource { version, commit_id });
                        }
                    }
                }
            }
        }

        Ok(version_tags)
    }

    fn collect_sources_from_release_branches(&self) -> Result<Vec<VersionSource>> {
        let mut version_branches = Vec::new();

        let branches = self.repo.branches(Some(git2::BranchType::Local))?;
        for branch in branches {
            let (branch, _) = branch?;
            if let Some(name) = branch.name()? {
                if let BranchType::Release(version) = self.determine_branch_type_by_name(name) {
                    version_branches.push(VersionSource {version, commit_id: branch.get().peel_to_commit()?.id()});
                }
            }
        }

        Ok(version_branches)
    }

    fn calculate_version_for_trunk(&self) -> Result<Version> {
        let latest_trunk_tag = self.find_latest_trunk_version()?;
        let head_id = self.repo.head()?.peel_to_commit()?.id();

        if let Some(tag) = latest_trunk_tag {
            let merge_base_oid = self.repo.merge_base(head_id, tag.commit_id)?;
            let count = self.count_commits_between(head_id, merge_base_oid)?;
            if count == 0 {
                return Ok(tag.version);
            }

            let mut version = tag.version.clone();
            version.minor += 1;
            version.patch = 0;
            version.pre = Prerelease::new(&format!("rc.{}", count))?;
            Ok(version)
        } else {
            let count = self.count_commits_between(head_id, Oid::zero())?;

            let mut version = Version::new(0, 1, 0);
            version.pre = Prerelease::new(&format!("rc.{}", count))?;
            Ok(version)
        }
    }

    fn count_commits_between(&self, from: Oid, to: Oid) -> Result<i64> {

        let mut revision_walk = self.repo.revwalk()?;
        revision_walk.push(from)?;
        revision_walk.set_sorting(git2::Sort::TOPOLOGICAL)?;
        let mut count = 0;
        for oid in revision_walk {
            if oid? == to { break; } // Stop counting when the specific commit is reached
            count += 1;
        }

        Ok(count)
    }

    fn calculate_version_for_release(&self, release_version: &Version) -> Result<Version> {
        let latest_release_tag = self.find_latest_tag_for_release_branch(release_version)?;
        let head_id = self.repo.head()?.peel_to_commit()?.id();

        if let Some(tag) = latest_release_tag {
            let count = self.count_commits_between(head_id, tag.commit_id)?;
            if count == 0 {
                return Ok(tag.version);
            }

            let mut new_version = tag.version.clone();
            new_version.patch += 1;
            new_version.pre = Prerelease::new(&format!("rc.{}", count))?;

            Ok(new_version)
        } else {
            let tag = self.find_latest_tag_base_for_release_branch()?.unwrap();
            let merge_base_oid = (&self.repo).merge_base(head_id, tag.commit_id)?;
            let count = self.count_commits_between(head_id, merge_base_oid)?;
            if count == 0 {
                return Ok(tag.version);
            }

            let mut version = release_version.clone();
            version.pre = Prerelease::new(&format!("rc.{}", count))?;
            Ok(version)
        }
    }

    fn calculate_version_for_feature(&self, name: &str) -> Result<Version> {
        struct FoundBranch {
            branch_type: BranchType,
            distance: i64,
        }

        let mut found_branches = Vec::new();
        let head_id = self.repo.head()?.peel_to_commit()?.id();

        let branches = self.repo.branches(Some(git2::BranchType::Local))?;
        for branch in branches {
            let (branch, _) = branch?;
            if let Some(name) = branch.name()? {
                let branch_type = self.determine_branch_type_by_name(name);
                if let BranchType::Other(_) = branch_type {
                    continue;
                }

                let branch_id = branch.get().peel_to_commit()?.id();
                let merge_base = self.repo.merge_base(head_id, branch_id)?;
                let distance = self.count_commits_between(head_id, merge_base)?;

                found_branches.push(FoundBranch {
                    branch_type,
                    distance,
                });
            }
        }

        found_branches.sort_by(
            |a, b| a.distance.cmp(&b.distance)
                .then_with(|| a.branch_type.cmp(&b.branch_type)));
        let closest_branch = found_branches.first();

        let mut base_version = match closest_branch {
            None => Ok(Version::new(0,1,0)),
            Some(found_branch) => match &found_branch.branch_type {
                BranchType::Trunk => self.calculate_version_for_trunk(),
                BranchType::Release(version) => self.calculate_version_for_release(&version),
                BranchType::Other(name) => panic!("Unexpected branch type: {}", name),
            }
        }.unwrap_or(Version::new(0,1,0));

        let distance = match closest_branch {
            None => self.count_commits_between(head_id, Oid::zero())?,
            Some(branch) => branch.distance,
        };

        base_version.pre = Prerelease::new(&format!("{}.{}", name, distance))?;
        Ok(base_version)
    }

    /// Find the latest version tag on the trunk branch
    fn find_latest_trunk_version(&self) -> Result<Option<VersionSource>> {
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
