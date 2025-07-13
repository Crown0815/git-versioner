pub mod config;

use crate::config::Configuration;
use anyhow::{anyhow, Result};
pub use config::DefaultConfig;
use git2::{Oid, Reference, Repository};
use regex::Regex;
use semver::{Comparator, Op, Prerelease, Version};

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

pub struct GitVersion{
    pub version: Version,
    pub branch_name: String,
    pub escaped_branch_name: String,
}

struct FoundBranch {
    branch_type: BranchType,
    distance: i64,
}

const BRANCH_NAME_ID: &'static str = "BranchName";
const VERSION_ID: &'static str = "Version";
pub const NO_BRANCH_NAME: &'static str = "(no branch)";
const IS_RELEASE_VERSION: fn(&&VersionSource) -> bool = |source| source.version.pre.is_empty();

impl GitVersioner {
    pub fn calculate_version<T: Configuration>(config: &T) -> Result<Version> {
        Ok(Self::calculate_version2(config)?.version)
    }

    pub fn calculate_version2<T: Configuration>(config: &T) -> Result<GitVersion> {
        let versioner = Self {
            repo: Repository::open(config.repository_path())?,
            trunk_pattern: Regex::new(&config.main_branch())?,
            release_pattern: Regex::new(&config.release_branch())?,
            feature_pattern: Regex::new(&config.feature_branch())?,
            version_pattern: Regex::new(&config.version_pattern())?,
        };

        if config.verbose() {
            versioner.print_effective_configuration();
        }

        let branch_name = Self::branch_name_for(versioner.head()?)?;
        let branch_type_at_head = versioner.determine_branch_type_by_name(&branch_name);

        let result = match branch_type_at_head {
            BranchType::Trunk => versioner.calculate_version_for_trunk(),
            BranchType::Release(version) => versioner.calculate_version_for_release(&version),
            BranchType::Other(name) => versioner.calculate_version_for_feature(&name),
        };

        match result {
            Err(e) => {Err(e)}
            Ok(version) => {
                Ok(GitVersion{
                    version,
                    escaped_branch_name: Self::escaped(&branch_name),
                    branch_name,
                })
            }
        }
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
            return Ok(NO_BRANCH_NAME.to_string())
        }

        match reference.shorthand() {
            None => Err(anyhow!("Name for branch could not be determined")),
            Some(name) => Ok(name.to_string())
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
        const ESCAPE_CHARACTER: char = '-';
        name.chars().map(|c| {
            if c.is_ascii_alphanumeric() { c } 
            else { ESCAPE_CHARACTER }
        }).collect()
    }

    fn collect_version_tags(&self) -> Result<Vec<VersionSource>> {
        let mut version_tags = Vec::new();

        let tag_names = self.repo.tag_names(None)?;
        for tag_name in tag_names.iter().flatten() {
            if let Some(version) = self.version_in(tag_name){
                if let Some(commit_id) = self.tag_id_for(tag_name) {
                    version_tags.push(VersionSource { version, commit_id });
                }
            }
        }

        Ok(version_tags)
    }

    fn version_in(&self, name: &str) -> Option<Version> {
        if let Some(captures) = self.version_pattern.captures(name){
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
        match self.repo.revparse_single(&format!("refs/tags/{}", name)) {
            Ok(tag_obj) => if let Some(tag) = tag_obj.as_tag() {
                Some(tag.target_id())
            } else {
                Some(tag_obj.id())
            }
            Err(_) => None
        }
    }

    fn collect_sources_from_release_branches(&self) -> Result<Vec<VersionSource>> {
        let mut version_branches = Vec::new();

        let branches = self.repo.branches(Some(git2::BranchType::Local))?;
        for branch in branches {
            let (branch, _) = branch?;
            if let Some(name) = branch.name()? {
                if let BranchType::Release(version) = self.determine_branch_type_by_name(name) {
                    let commit = branch.get().peel_to_commit()?;
                    version_branches.push(VersionSource {version, commit_id: commit.id()});
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
        let head_id = self.repo.head()?.peel_to_commit()?.id();
        let current_version = major_minor_comparator(release_version.major, release_version.minor);

        let previous_version = if release_version.minor > 0 {
            major_minor_comparator(release_version.major, release_version.minor - 1)
        } else{
            none_comparator()
        };


        if let Some(tag) = self.find_latest_tag_matching(&current_version)? {
            let merge_base_oid = (&self.repo).merge_base(head_id, tag.commit_id)?;
            let count = self.count_commits_between(head_id, merge_base_oid)?;
            if count == 0 {
                return Ok(tag.version);
            }

            let mut new_version = tag.version.clone();
            new_version.patch += 1;
            new_version.pre = Prerelease::new(&format!("rc.{}", count))?;

            Ok(new_version)
        } else if let Some(tag) = self.find_latest_tag_matching(&previous_version)? {
            let merge_base_oid = (&self.repo).merge_base(head_id, tag.commit_id)?;
            let count = self.count_commits_between(head_id, merge_base_oid)?;
            if count == 0 {
                return Ok(tag.version);
            }

            let mut new_version = release_version.clone();
            new_version.patch += 0;
            new_version.pre = Prerelease::new(&format!("rc.{}", count))?;
            Ok(new_version)
        } else {
            let mut found_branches= self.find_all_source_branches(head_id)?;

            found_branches.sort_by(|a, b| a.branch_type.cmp(&b.branch_type));
            let closest_branch = found_branches.first().unwrap();

            let count = closest_branch.distance;

            let mut version = release_version.clone();
            version.pre = Prerelease::new(&format!("rc.{}", count))?;
            Ok(version)
        }
    }

    fn calculate_version_for_feature(&self, name: &str) -> Result<Version> {
        let head_id = self.repo.head()?.peel_to_commit()?.id();
        let mut found_branches= self.find_all_source_branches(head_id)?;

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

        base_version.pre = Prerelease::new(
            &format!("{}.{}", Self::escaped(name), distance))?;
        Ok(base_version)
    }

    fn find_all_source_branches(&self, count_reference: Oid) -> Result<Vec<FoundBranch>> {
        let mut found_branches = Vec::new();

        let branches = self.repo.branches(Some(git2::BranchType::Local))?;
        for branch in branches {
            let (branch, _) = branch?;
            if let Some(name) = branch.name()? {
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

    fn find_latest_trunk_version(&self) -> Result<Option<VersionSource>> {
        self.find_latest_version_source(true, &any_comparator())
    }

    fn find_latest_tag_matching(&self, comparator: &Comparator) -> Result<Option<VersionSource>> {
        self.find_latest_version_source(false, comparator)
    }

    fn find_latest_version_source(&self, track_release_branches: bool, comparator: &Comparator) -> Result<Option<VersionSource>> {
        let mut sources = self.collect_version_tags()?;
        if track_release_branches {
            sources.append(&mut self.collect_sources_from_release_branches()?);
        }
        
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
    Comparator{
        op: Op::Exact,
        major,
        minor: Some(minor),
        patch: None,
        pre: Prerelease::EMPTY,
    }
}
