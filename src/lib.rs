pub mod config;

use crate::config::Configuration;
use anyhow::{Result, anyhow};
pub use config::DefaultConfig;
use git2::{Oid, Reference, Repository};
use regex::Regex;
use semver::{Comparator, Op, Prerelease, Version};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt::Display;

const BRANCH_NAME_ID: &str = "BranchName";
const VERSION_ID: &str = "Version";
pub const NO_BRANCH_NAME: &str = "(no branch)";
const IS_RELEASE_VERSION: fn(&&VersionSource) -> bool = |source| source.version.pre.is_empty();

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum BranchType {
    Trunk,            // Main development branch (trunk)
    Release(Version), // Release branch (e.g., release/1.0.0)
    Other(String),    // Feature branch or any other branch type
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
    prerelease_tag: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct GitVersion {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    pub major_minor_patch: String,
    pub pre_release_tag: String,
    pub pre_release_tag_with_dash: String,
    pub pre_release_label: String,
    pub pre_release_label_with_dash: String,
    pub pre_release_number: String,
    pub build_metadata: String,
    pub sem_ver: String,
    pub assembly_sem_ver: String,
    pub full_sem_ver: String,
    pub informational_version: String,
    pub branch_name: String,
    pub escaped_branch_name: String,
}

struct FoundBranch {
    branch_type: BranchType,
    distance: i64,
}

impl GitVersioner {
    pub fn calculate_version<T: Configuration>(config: &T) -> Result<GitVersion> {
        let versioner = Self {
            repo: Repository::open(config.repository_path())?,
            trunk_pattern: Regex::new(config.main_branch())?,
            release_pattern: Regex::new(config.release_branch())?,
            feature_pattern: Regex::new(config.feature_branch())?,
            version_pattern: Regex::new(config.version_pattern())?,
            prerelease_tag: config.prerelease_tag().to_string(),
        };

        if config.verbose() {
            versioner.print_effective_configuration();
        }

        let branch_name = Self::branch_name_for(versioner.head()?)?;
        let branch_type_at_head = versioner.determine_branch_type_by_name(&branch_name);

        let version = match branch_type_at_head {
            BranchType::Trunk => versioner.calculate_version_for_trunk(),
            BranchType::Release(version) => versioner.calculate_version_for_release(&version),
            BranchType::Other(name) => versioner.calculate_version_for_feature(&name),
        }?;

        Ok(GitVersion::new(version, branch_name))
    }

    fn print_effective_configuration(&self) {
        println!("Using configuration:");
        println!("  Repository path: {}", self.repo.path().display());
        println!("  Trunk pattern: {}", self.trunk_pattern);
        println!("  Release pattern: {}", self.release_pattern);
        println!("  Feature pattern: {}", self.feature_pattern);
        println!("  Version pattern: {}", self.version_pattern);
        println!();
    }

    fn head(&self) -> Result<Reference, git2::Error> {
        self.repo.head()
    }

    fn branch_name_for(reference: Reference) -> Result<String> {
        if !reference.is_branch() {
            return Ok(NO_BRANCH_NAME.to_string());
        }

        match reference.shorthand() {
            None => Err(anyhow!("Name for branch could not be determined")),
            Some(name) => Ok(name.to_string()),
        }
    }

    fn determine_branch_type_by_name(&self, name: &str) -> BranchType {
        if self.trunk_pattern.is_match(name) {
            return BranchType::Trunk;
        }

        if let Some(captures) = self.release_pattern.captures(name) {
            if let Some(branch_name) = captures.name(BRANCH_NAME_ID) {
                if let Some(version) = self.version_in(branch_name.as_str()) {
                    return BranchType::Release(version);
                }
            }
        }

        if let Some(captures) = self.feature_pattern.captures(name) {
            if let Some(branch_name) = captures.name(BRANCH_NAME_ID) {
                return BranchType::Other(branch_name.as_str().to_string());
            }
        }

        BranchType::Other(name.to_string())
    }

    fn escaped(name: &str) -> String {
        const ESCAPE_CHARACTER: &str = "-";
        name.replace(|c: char| !c.is_alphanumeric(), ESCAPE_CHARACTER)
    }

    fn version_tags(&self) -> Result<HashSet<VersionSource>> {
        let mut version_tags = HashSet::new();

        let tag_names = self.repo.tag_names(None)?;
        for tag_name in tag_names.iter().flatten() {
            if let Some(version) = self.version_in(tag_name) {
                if let Some(commit_id) = self.tag_id_for(tag_name) {
                    version_tags.insert(VersionSource { version, commit_id });
                }
            }
        }

        Ok(version_tags)
    }

    fn version_in(&self, name: &str) -> Option<Version> {
        if let Some(captures) = self.version_pattern.captures(name) {
            if let Some(version_str) = captures.name(VERSION_ID) {
                if let Ok(version) = Version::parse(version_str.as_str()) {
                    if version.pre.is_empty() {
                        return Some(version);
                    }
                }
            }
        }
        None
    }

    fn tag_id_for(&self, name: &str) -> Option<Oid> {
        match self.repo.revparse_single(&format!("refs/tags/{name}")) {
            Ok(tag_obj) => {
                if let Some(tag) = tag_obj.as_tag() {
                    Some(tag.target_id())
                } else {
                    Some(tag_obj.id())
                }
            }
            Err(_) => None,
        }
    }

    fn version_branches(&self) -> Result<HashSet<VersionSource>> {
        let mut version_branches = HashSet::new();

        let branches = self.repo.branches(Some(git2::BranchType::Local))?;
        for branch in branches {
            let (branch, _) = branch?;
            if let Some(name) = branch.name()? {
                if let BranchType::Release(version) = self.determine_branch_type_by_name(name) {
                    let commit = branch.get().peel_to_commit()?;
                    version_branches.insert(VersionSource {
                        version,
                        commit_id: commit.id(),
                    });
                }
            }
        }

        Ok(version_branches)
    }

    fn remote_version_branches(&self) -> Result<HashSet<VersionSource>> {
        let mut version_branches = HashSet::new();

        for remote in self.repo.remotes()?.iter().flatten() {
            let branches = self.repo.branches(Some(git2::BranchType::Remote))?;
            for branch in branches {
                let (branch, _) = branch?;
                if let Some(name) = branch.name()? {
                    let name = name.replace(&format!("{remote}/"), "");
                    if let BranchType::Release(version) = self.determine_branch_type_by_name(&name)
                    {
                        let commit = branch.get().peel_to_commit()?;
                        version_branches.insert(VersionSource {
                            version,
                            commit_id: commit.id(),
                        });
                    }
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
            version.pre = self.prerelease(count)?;
            Ok(version)
        } else {
            let count = self.count_commits_between(head_id, Oid::zero())?;

            let mut version = Version::new(0, 1, 0);
            version.pre = self.prerelease(count)?;
            Ok(version)
        }
    }

    fn prerelease(&self, count: i64) -> Result<Prerelease> {
        Ok(Prerelease::new(&format!(
            "{}.{}",
            self.prerelease_tag, count
        ))?)
    }

    fn calculate_version_for_release(&self, release_version: &Version) -> Result<Version> {
        let head_id = self.repo.head()?.peel_to_commit()?.id();
        let current_version = major_minor_comparator(release_version.major, release_version.minor);

        let previous_version = if release_version.minor > 0 {
            major_minor_comparator(release_version.major, release_version.minor - 1)
        } else {
            none_comparator()
        };

        if let Some(tag) = self.find_latest_tag_matching(&current_version)? {
            let merge_base_oid = self.repo.merge_base(head_id, tag.commit_id)?;
            let count = self.count_commits_between(head_id, merge_base_oid)?;
            if count == 0 {
                return Ok(tag.version);
            }

            let mut new_version = tag.version.clone();
            new_version.patch += 1;
            new_version.pre = self.prerelease(count)?;

            Ok(new_version)
        } else if let Some(tag) = self.find_latest_tag_matching(&previous_version)? {
            let merge_base_oid = self.repo.merge_base(head_id, tag.commit_id)?;
            let count = self.count_commits_between(head_id, merge_base_oid)?;
            if count == 0 {
                return Ok(tag.version);
            }

            let mut new_version = release_version.clone();
            new_version.patch += 0;
            new_version.pre = self.prerelease(count)?;
            Ok(new_version)
        } else {
            let mut found_branches = self.find_all_source_branches(head_id)?;

            found_branches.sort_by(|a, b| a.branch_type.cmp(&b.branch_type));
            let closest_branch = found_branches.first().unwrap();

            let count = closest_branch.distance;

            let mut version = release_version.clone();
            version.pre = self.prerelease(count)?;
            Ok(version)
        }
    }

    fn calculate_version_for_feature(&self, name: &str) -> Result<Version> {
        let head_id = self.repo.head()?.peel_to_commit()?.id();
        let mut found_branches = self.find_all_source_branches(head_id)?;

        found_branches.sort_by(|a, b| {
            a.distance
                .cmp(&b.distance)
                .then_with(|| a.branch_type.cmp(&b.branch_type))
        });
        let closest_branch = found_branches.first();

        let mut base_version = match closest_branch {
            None => Ok(Version::new(0, 1, 0)),
            Some(found_branch) => match &found_branch.branch_type {
                BranchType::Trunk => self.calculate_version_for_trunk(),
                BranchType::Release(version) => self.calculate_version_for_release(version),
                BranchType::Other(name) => panic!("Unexpected branch type: {name}"),
            },
        }
        .unwrap_or(Version::new(0, 1, 0));

        let distance = match closest_branch {
            None => self.count_commits_between(head_id, Oid::zero())?,
            Some(branch) => branch.distance,
        };

        base_version.pre = Prerelease::new(&format!("{}.{}", Self::escaped(name), distance))?;
        Ok(base_version)
    }

    fn find_all_source_branches(&self, count_reference: Oid) -> Result<Vec<FoundBranch>> {
        let mut found_branches = Vec::new();

        let branches = self.repo.branches(None)?;
        for branch in branches {
            let (branch, branch_type) = branch?;
            let cleaned_name = match branch_type {
                git2::BranchType::Local => branch.name()?,
                git2::BranchType::Remote => {
                    let branch_name = branch.name()?.unwrap();
                    match branch_name.split_once('/') {
                        None => return Err(anyhow!("Unexpected remote branch: {}", branch_name)),
                        Some((_, name)) => Some(name),
                    }
                }
            };
            if let Some(name) = cleaned_name {
                let branch_type = self.determine_branch_type_by_name(name);
                if let BranchType::Other(_) = branch_type {
                    continue;
                }

                let branch_id = branch.get().peel_to_commit()?.id();
                let merge_base = self.repo.merge_base(count_reference, branch_id)?;
                let distance = self.count_commits_between(count_reference, merge_base)?;

                found_branches.push(FoundBranch {
                    branch_type,
                    distance,
                });
            }
        }
        Ok(found_branches)
    }

    fn count_commits_between(&self, from: Oid, to: Oid) -> Result<i64> {
        let mut revision_walk = self.repo.revwalk()?;
        revision_walk.push(from)?;
        revision_walk.set_sorting(git2::Sort::TOPOLOGICAL)?;
        let mut count = 0;
        for oid in revision_walk {
            if oid? == to {
                break;
            } // Stop counting when the specific commit is reached
            count += 1;
        }

        Ok(count)
    }

    fn find_latest_trunk_version(&self) -> Result<Option<VersionSource>> {
        self.find_latest_version_source(true, &any_comparator())
    }

    fn find_latest_tag_matching(&self, comparator: &Comparator) -> Result<Option<VersionSource>> {
        self.find_latest_version_source(false, comparator)
    }

    fn find_latest_version_source(
        &self,
        track_release_branches: bool,
        comparator: &Comparator,
    ) -> Result<Option<VersionSource>> {
        let sources = if track_release_branches {
            self.version_tags()?
                .into_iter()
                .chain(self.version_branches()?)
                .chain(self.remote_version_branches()?)
                .collect()
        } else {
            self.version_tags()?
        };

        let mut matching_tags = sources
            .iter()
            .filter(IS_RELEASE_VERSION)
            .filter(|source: &&VersionSource| comparator.matches(&source.version))
            .cloned()
            .collect::<Vec<_>>();

        matching_tags.sort_by(|a, b| a.version.cmp(&b.version));
        Ok(matching_tags.last().cloned())
    }
}

fn none_comparator() -> Comparator {
    Comparator::parse("<=0.0.0-0").unwrap()
}

fn any_comparator() -> Comparator {
    Comparator::parse(">=0").unwrap()
}

fn major_minor_comparator(major: u64, minor: u64) -> Comparator {
    Comparator {
        op: Op::Exact,
        major,
        minor: Some(minor),
        patch: None,
        pre: Prerelease::EMPTY,
    }
}

impl GitVersion {
    pub fn new(version: Version, branch_name: String) -> Self {
        Self {
            major: version.major,
            minor: version.minor,
            patch: version.patch,
            major_minor_patch: format!("{}.{}.{}", version.major, version.minor, version.patch),
            pre_release_tag: version.pre.to_string(),
            pre_release_tag_with_dash: if version.pre.is_empty() {
                "".to_string()
            } else {
                format!("-{}", version.pre.as_str())
            },
            pre_release_label: version
                .pre
                .as_str()
                .split('.')
                .next()
                .unwrap_or("")
                .to_string(),
            pre_release_label_with_dash: if version.pre.is_empty() {
                "".to_string()
            } else {
                format!("-{}", version.pre.as_str().split('.').next().unwrap_or(""))
            },
            pre_release_number: version
                .pre
                .as_str()
                .split('.')
                .nth(1)
                .unwrap_or("")
                .to_string(),
            build_metadata: version.build.to_string(),
            sem_ver: version.to_string(),
            assembly_sem_ver: format!("{}.{}.{}", version.major, version.minor, version.patch),
            full_sem_ver: version.to_string(),
            informational_version: version.to_string(),
            escaped_branch_name: GitVersioner::escaped(&branch_name),
            branch_name,
        }
    }
}

impl Display for GitVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{} ({})", self.full_sem_ver, self.branch_name))
    }
}
