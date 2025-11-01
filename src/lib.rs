pub mod config;

use crate::config::Configuration;
use anyhow::{Result, anyhow};
use chrono::DateTime;
use chrono::offset::Utc;
pub use config::DefaultConfig;
use conventional_commit_parser::{commit::CommitType, parse};
use git2::{Oid, Reference, Repository};
use regex::Regex;
use semver::{Comparator, Op, Prerelease, Version};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt::{Debug, Display};
use std::time;

const BRANCH_NAME_ID: &str = "BranchName";
const VERSION_ID: &str = "Version";
const NO_BRANCH_NAME: &str = "(no branch)";
const IS_RELEASE_VERSION: fn(&&VersionSource) -> bool = |source| source.version.pre.is_empty();

const PRERELEASE_WEIGHT_MAIN: u64 = 55000;
const PRERELEASE_WEIGHT_RELEASE: u64 = PRERELEASE_WEIGHT_MAIN;
const PRERELEASE_WEIGHT_TAG: u64 = 60000;
const PRERELEASE_WEIGHT_FEATURE: u64 = 30000;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum BranchType {
    Trunk,            // Main development branch (trunk)
    Release(Version), // Release branch (e.g., release/1.0.0)
    Other(String),    // Feature branch or any other branch type
}

enum CommitBump {
    Major,
    Minor,
    Patch,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VersionSource {
    version: Version,
    commit_id: Oid,
    is_tag: bool,
}

pub struct GitVersioner {
    repo: Repository,
    trunk_pattern: Regex,
    release_pattern: Regex,
    feature_pattern: Regex,
    version_pattern: Regex,
    prerelease_tag: String,
    continuous_delivery: bool,
    is_commit_message_incrementing: bool,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct GitVersion {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    pub pre_release_tag: String,
    pub pre_release_tag_with_dash: String,
    pub pre_release_label: String,
    pub pre_release_label_with_dash: String,
    pub pre_release_number: u64,
    pub weighted_pre_release_number: u64,
    pub build_metadata: String,
    pub full_build_meta_data: String,
    pub major_minor_patch: String,
    pub sem_ver: String,
    pub assembly_sem_ver: String,
    pub assembly_sem_file_ver: String,
    pub informational_version: String,
    pub full_sem_ver: String,
    pub branch_name: String,
    pub escaped_branch_name: String,
    pub sha: String,
    pub short_sha: String,
    pub version_source_sha: String,
    pub commits_since_version_source: u64,
    pub commit_date: String,
    pub uncommitted_changes: u64,
}

struct FoundBranch {
    branch_type: BranchType,
    distance: i64,
}

impl GitVersioner {
    pub fn calculate_version<T: Configuration>(config: &T) -> Result<GitVersion> {
        let versioner = Self::new(config)?;

        let head = versioner.head()?;
        let branch_name = Self::branch_name_for(&head)?;
        let branch_type_at_head = versioner.determine_branch_type_by_name(&branch_name);

        let (mut version, source, mut prerelease_weight) = match branch_type_at_head {
            BranchType::Trunk => versioner.calculate_version_for_trunk(),
            BranchType::Release(version) => versioner.calculate_version_for_release(&version),
            BranchType::Other(name) => versioner.calculate_version_for_feature(&name),
        }?;

        if *config.as_release() {
            version.pre = Prerelease::EMPTY;
            prerelease_weight = PRERELEASE_WEIGHT_TAG;
        }

        Ok(GitVersion::new(
            version,
            branch_name,
            source.commit_id,
            prerelease_weight,
            head,
        ))
    }

    fn new<T: Configuration>(config: &T) -> Result<GitVersioner> {
        let versioner = Self {
            repo: Repository::open(config.path())?,
            trunk_pattern: Regex::new(config.main_branch())?,
            release_pattern: Regex::new(config.release_branch())?,
            feature_pattern: Regex::new(config.feature_branch())?,
            version_pattern: Regex::new(&format!("^{}(?<Version>.+)", config.tag_prefix()))?,
            prerelease_tag: config.pre_release_tag().to_string(),
            continuous_delivery: *config.continuous_delivery(),
            is_commit_message_incrementing: match config.commit_message_incrementing() {
                "Enabled" => true,
                "Disabled" => false,
                v => panic!(
                    r#"Invalid value "{}" for {}. Should be "Enabled" or "Disabled"."#,
                    v,
                    stringcase::pascal_case(get_method_name(T::commit_message_incrementing))
                ),
            },
        };
        Ok(versioner)
    }

    fn head(&self) -> Result<Reference<'_>, git2::Error> {
        self.repo.head()
    }

    fn branch_name_for(reference: &Reference) -> Result<String> {
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

        if let Some(captures) = self.release_pattern.captures(name)
            && let Some(branch_name) = captures.name(BRANCH_NAME_ID)
            && let Some(version) = self.version_in(Self::loose(branch_name.as_str()))
        {
            return BranchType::Release(version);
        }

        if let Some(captures) = self.feature_pattern.captures(name)
            && let Some(branch_name) = captures.name(BRANCH_NAME_ID)
        {
            return BranchType::Other(branch_name.as_str().to_string());
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
            if let Some(version) = self.version_in(tag_name)
                && let Some(commit_id) = self.tag_id_for(tag_name)
            {
                version_tags.insert(VersionSource {
                    version,
                    commit_id,
                    is_tag: true,
                });
            }
        }

        Ok(version_tags)
    }

    fn pre_release_version_tags(
        &self,
        next_release_version: &Version,
    ) -> Result<HashSet<VersionSource>> {
        let mut version_tags = HashSet::new();

        let tag_names = self.repo.tag_names(None)?;
        for tag_name in tag_names.iter().flatten() {
            if let Some(version) = self.pre_release_version_in(tag_name)
                && let Some(commit_id) = self.tag_id_for(tag_name)
            {
                if let Some(version) = self.matching_pre_release(version, next_release_version) {
                    version_tags.insert(VersionSource {
                        version,
                        commit_id,
                        is_tag: true,
                    });
                }
            }
        }

        Ok(version_tags)
    }

    fn version_in<S: AsRef<str>>(&self, name: S) -> Option<Version> {
        if let Some(captures) = self.version_pattern.captures(name.as_ref())
            && let Some(version_str) = captures.name(VERSION_ID)
            && let Ok(version) = Version::parse(version_str.as_str())
            && version.pre.is_empty()
        {
            return Some(version);
        }
        None
    }

    fn pre_release_version_in<S: AsRef<str>>(&self, name: S) -> Option<Version> {
        if let Some(captures) = self.version_pattern.captures(name.as_ref())
            && let Some(version_str) = captures.name(VERSION_ID)
            && let Ok(version) = Version::parse(version_str.as_str())
            && !version.pre.is_empty()
        {
            return Some(version);
        }
        None
    }

    fn loose<S: AsRef<str> + ToString>(semantic_version_string: S) -> String {
        let version_string = semantic_version_string.as_ref();
        let meta_start = version_string
            .find(['-', '+'])
            .unwrap_or(version_string.len());
        let base = &version_string[..meta_start];
        let rest = &version_string[meta_start..];

        let components: Vec<&str> = base.split('.').collect();

        match components.len() {
            2 => format!("{base}.0{rest}"),
            _ => semantic_version_string.to_string(),
        }
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
            if let Some(name) = branch.name()?
                && let BranchType::Release(version) = self.determine_branch_type_by_name(name)
            {
                let commit = branch.get().peel_to_commit()?;
                version_branches.insert(VersionSource {
                    version,
                    commit_id: commit.id(),
                    is_tag: false,
                });
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
                            is_tag: false,
                        });
                    }
                }
            }
        }

        Ok(version_branches)
    }

    fn calculate_version_for_trunk(&self) -> Result<(Version, VersionSource, u64)> {
        let source = self.find_trunk_version_source()?.unwrap_or(no_source());
        let head_id = self.repo.head()?.peel_to_commit()?.id();

        let merge_base_oid = self.merge_base(head_id, source.commit_id)?;
        if head_id == merge_base_oid {
            return Ok(Self::version_from(&source, PRERELEASE_WEIGHT_MAIN));
        }

        let mut version = source.version.clone();

        if !self.is_commit_message_incrementing {
            version.minor += 1;
            version.patch = 0;
        } else {
            match self.determine_bump_between(head_id, merge_base_oid)? {
                CommitBump::Major => {
                    if version.major == 0 {
                        version.minor += 1;
                        version.patch = 0;
                    } else {
                        version.major += 1;
                        version.minor = 0;
                        version.patch = 0;
                    }
                }
                CommitBump::Minor => {
                    version.minor += 1;
                    version.patch = 0;
                }
                CommitBump::Patch => {
                    if version.major == 0 && version.minor == 0 {
                        version.minor += 1;
                        version.patch = 0;
                    } else {
                        version.patch += 1;
                    }
                }
            }
        }

        let (pre_release_number, source) = match self.continuous_delivery {
            true => {
                let highest_pre_release = self.find_latest_matching_pre_release(&version)?;
                let reference_pre_release = highest_pre_release.unwrap_or((0, source));
                (reference_pre_release.0 + 1, reference_pre_release.1)
            }
            false => {
                let commit_count = self.count_commits_between(head_id, merge_base_oid)?;
                (commit_count, source)
            }
        };

        version.pre = self.pre_release(pre_release_number)?;
        Ok((version, source, PRERELEASE_WEIGHT_MAIN))
    }

    fn find_latest_matching_pre_release(
        &self,
        version: &Version,
    ) -> Result<Option<(i64, VersionSource)>> {
        let pre_release_versions = self.pre_release_version_tags(version)?;

        let highest_prerelease = pre_release_versions
            .into_iter()
            .filter_map(|source| {
                Self::extract_pre_release_number(&source.version, &self.prerelease_tag)
                    .map(|number| (number, source))
            })
            .max_by_key(|(number, _)| *number);
        Ok(highest_prerelease)
    }

    fn extract_pre_release_number(version: &Version, pre_release_tag: &str) -> Option<i64> {
        let pre = version.pre.as_str();

        let expected_prefix = format!("{}.", pre_release_tag);
        if !pre.starts_with(&expected_prefix) {
            return None;
        }

        let integer_part = &pre[expected_prefix.len()..];
        integer_part.parse::<i64>().ok()
    }

    fn pre_release(&self, count: i64) -> Result<Prerelease> {
        Ok(Prerelease::new(&format!(
            "{}.{}",
            self.prerelease_tag, count
        ))?)
    }

    fn calculate_version_for_release(
        &self,
        release_version: &Version,
    ) -> Result<(Version, VersionSource, u64)> {
        let head_id = self.repo.head()?.peel_to_commit()?.id();
        let current_version = major_minor_comparator(release_version.major, release_version.minor);

        let previous_version = if release_version.minor > 0 {
            major_minor_comparator(release_version.major, release_version.minor - 1)
        } else {
            none_comparator()
        };

        if let Some(source) = self.find_latest_version_source(false, &current_version)? {
            let merge_base_oid = self.merge_base(head_id, source.commit_id)?;
            if head_id == merge_base_oid {
                return Ok(Self::version_from(&source, PRERELEASE_WEIGHT_RELEASE));
            }

            let mut new_version = source.version.clone();
            new_version.patch += 1;

            let (pre_release_number, source) = match self.continuous_delivery {
                true => {
                    let highest_pre_release =
                        self.find_latest_matching_pre_release(&source.version)?;
                    let reference_pre_release = highest_pre_release.unwrap_or((0, source));
                    (reference_pre_release.0 + 1, reference_pre_release.1)
                }
                false => {
                    let commit_count = self.count_commits_between(head_id, merge_base_oid)?;
                    (commit_count, source)
                }
            };

            new_version.pre = self.pre_release(pre_release_number)?;

            Ok((new_version, source, PRERELEASE_WEIGHT_RELEASE))
        } else if let Some(source) = self.find_latest_version_source(true, &previous_version)? {
            let merge_base_oid = self.merge_base(head_id, source.commit_id)?;
            if head_id == merge_base_oid {
                return Ok(Self::version_from(&source, PRERELEASE_WEIGHT_RELEASE));
            }

            let (pre_release_number, source) = match self.continuous_delivery {
                true => {
                    let highest_pre_release =
                        self.find_latest_matching_pre_release(&source.version)?;
                    let reference_pre_release = highest_pre_release.unwrap_or((0, source));
                    (reference_pre_release.0 + 1, reference_pre_release.1)
                }
                false => {
                    let commit_count = self.count_commits_between(head_id, merge_base_oid)?;
                    (commit_count, source)
                }
            };

            let mut new_version = release_version.clone();
            new_version.patch += 0;
            new_version.pre = self.pre_release(pre_release_number)?;
            Ok((new_version, source, PRERELEASE_WEIGHT_RELEASE))
        } else {
            let mut found_branches = self.find_all_source_branches(head_id)?;

            found_branches.sort_by(|a, b| a.branch_type.cmp(&b.branch_type));
            let closest_branch = found_branches.first().unwrap();

            let version = release_version.clone();
            let source = VersionSource {
                version,
                commit_id: Oid::zero(),
                is_tag: false,
            };

            let (pre_release_number, source) = match self.continuous_delivery {
                true => {
                    let highest_pre_release =
                        self.find_latest_matching_pre_release(&source.version)?;
                    let reference_pre_release = highest_pre_release.unwrap_or((0, source));
                    (reference_pre_release.0 + 1, reference_pre_release.1)
                }
                false => {
                    let commit_count = closest_branch.distance;
                    (commit_count, source)
                }
            };

            let mut version = source.version.clone();
            version.pre = self.pre_release(pre_release_number)?;
            Ok((version, source, PRERELEASE_WEIGHT_RELEASE))
        }
    }

    fn merge_base(&self, head_id: Oid, source_id: Oid) -> Result<Oid> {
        Ok(if source_id.is_zero() {
            source_id
        } else {
            self.repo.merge_base(head_id, source_id)?
        })
    }

    fn calculate_version_for_feature(&self, name: &str) -> Result<(Version, VersionSource, u64)> {
        let head_id = self.repo.head()?.peel_to_commit()?.id();
        let mut found_branches = self.find_all_source_branches(head_id)?;

        found_branches.sort_by(|a, b| {
            a.distance
                .cmp(&b.distance)
                .then_with(|| a.branch_type.cmp(&b.branch_type))
        });
        let closest_branch = found_branches.first();
        let fallback = (
            Version::new(0, 1, 0),
            VersionSource {
                version: Version::new(0, 1, 0),
                commit_id: Oid::zero(),
                is_tag: false,
            },
            0,
        );

        let base = match closest_branch {
            None => Ok(fallback.clone()),
            Some(found_branch) => match &found_branch.branch_type {
                BranchType::Trunk => self.calculate_version_for_trunk(),
                BranchType::Release(version) => self.calculate_version_for_release(version),
                BranchType::Other(name) => panic!("Unexpected branch type: {name}"),
            },
        }
        .unwrap_or(fallback);

        let distance = match closest_branch {
            None => self.count_commits_between(head_id, Oid::zero())?,
            Some(branch) => branch.distance,
        };

        if distance == 0 {
            return Ok(base);
        }

        let (mut base_version, source, _) = base;

        base_version.pre = Prerelease::new(&format!("{}.{}", Self::escaped(name), distance))?;
        Ok((base_version, source, PRERELEASE_WEIGHT_FEATURE))
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
                let merge_base = self.merge_base(count_reference, branch_id)?;
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
            let oid = oid?;
            if oid == to {
                break; // Stop counting when the specific commit is reached
            }
            count += 1;
        }

        Ok(count)
    }

    fn determine_bump_between(&self, from: Oid, to: Oid) -> Result<CommitBump> {
        let mut revision_walk = self.repo.revwalk()?;
        revision_walk.push(from)?;
        revision_walk.set_sorting(git2::Sort::TOPOLOGICAL)?;
        let mut commit_bump = CommitBump::Patch;
        for oid in revision_walk {
            let oid = oid?;
            if oid == to {
                break; // Stop counting when the specific commit is reached
            }
            if let CommitBump::Patch = commit_bump
                && let Ok(commit) = self.repo.find_commit(oid)
                && let Some(message) = commit.message()
                && let Ok(conventional_commit) = parse(message.trim())
            {
                if conventional_commit.is_breaking_change {
                    return Ok(CommitBump::Major);
                }
                if let CommitType::Feature = conventional_commit.commit_type {
                    commit_bump = CommitBump::Minor;
                }
            }
        }

        Ok(commit_bump)
    }

    fn find_trunk_version_source(&self) -> Result<Option<VersionSource>> {
        self.find_latest_version_source(true, &any_comparator())
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

        let mut all_sources = HashSet::from([no_source()]);
        all_sources.extend(sources);

        let mut matching_tags = all_sources
            .iter()
            .filter(IS_RELEASE_VERSION)
            .filter(|source: &&VersionSource| comparator.matches(&source.version))
            .cloned()
            .collect::<Vec<_>>();

        matching_tags.sort_by(|a, b| a.version.cmp(&b.version));
        Ok(matching_tags.last().cloned())
    }

    fn version_from(source: &VersionSource, fallback_weight: u64) -> (Version, VersionSource, u64) {
        let prerelease_weight = if source.is_tag {
            PRERELEASE_WEIGHT_TAG
        } else {
            fallback_weight
        };
        (source.version.clone(), source.clone(), prerelease_weight)
    }

    fn matching_pre_release(&self, pre: Version, release: &Version) -> Option<Version> {
        if pre.major == release.major && pre.minor == release.minor && pre.patch == release.patch {
            Some(pre)
        } else {
            None
        }
    }
}

fn no_source() -> VersionSource {
    VersionSource {
        version: Version::parse("0.0.0").unwrap(),
        commit_id: Oid::zero(),
        is_tag: false,
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
    fn new(
        version: Version,
        branch_name: String,
        source: Oid,
        prerelease_weight: u64,
        head: Reference,
    ) -> Self {
        let pre_release_number = version
            .pre
            .as_str()
            .split('.')
            .nth(1)
            .unwrap_or("0")
            .parse()
            .unwrap();

        let weighted_pre_release_number = pre_release_number + prerelease_weight;

        let commit = head.peel_to_commit().unwrap();
        let sha = commit.id().to_string();
        let short_sha = sha[..7].to_string();
        let seconds_since_epoch = time::Duration::from_secs(commit.time().seconds() as u64);
        let commit_date_time: DateTime<Utc> = (time::UNIX_EPOCH + seconds_since_epoch).into();
        let commit_date = commit_date_time.format("%Y-%m-%d").to_string();

        let version_source_sha = if source.is_zero() {
            "".to_string()
        } else {
            source.to_string()
        };
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
            pre_release_number,
            weighted_pre_release_number,
            build_metadata: version.build.to_string(),
            sem_ver: version.to_string(),
            assembly_sem_ver: format!("{}.{}.{}.0", version.major, version.minor, version.patch),
            assembly_sem_file_ver: format!(
                "{}.{}.{}.{}",
                version.major, version.minor, version.patch, weighted_pre_release_number
            ),
            full_sem_ver: version.to_string(),
            informational_version: version.to_string(),
            escaped_branch_name: GitVersioner::escaped(&branch_name),
            sha,
            short_sha,
            version_source_sha,
            commits_since_version_source: 0,
            commit_date,
            branch_name,
            full_build_meta_data: "".to_string(),
            uncommitted_changes: 0,
        }
    }
}

impl Display for GitVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{} ({})", self.full_sem_ver, self.branch_name))
    }
}

fn get_method_name<R, O, F>(_: F) -> &'static str
where
    F: for<'a> Fn(&'a R) -> &'a O,
    O: ?Sized,
{
    let full_name = std::any::type_name::<F>();
    full_name.rsplit("::").next().unwrap_or(full_name)
}
