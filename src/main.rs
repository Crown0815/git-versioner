use anyhow::Result;
use clap::Parser;
use git_versioner::GitVersioner;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const MAIN_BRANCH: &str = r"^(trunk|main|master)$";

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long, value_parser)]
    repo_path: Option<PathBuf>,

    #[clap(short, long, value_parser)]
    main_branch: Option<Regex>,

    #[clap(short, long)]
    verbose: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Output {
    major: u64,
    minor: u64,
    patch: u64,
    major_minor_patch: String,
    pre_release_tag: String,
    pre_release_tag_with_dash: String,
    pre_release_label: String,
    pre_release_label_with_dash: String,
    pre_release_number: String,
    build_metadata: String,
    sem_ver: String,
    assembly_sem_ver: String,
    full_sem_ver: String,
    informational_version: String,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let repo_path = args.repo_path.unwrap_or_else(|| std::env::current_dir().unwrap());
    let main_branch = args.main_branch.unwrap_or_else(|| Regex::new(MAIN_BRANCH).unwrap());
    let version = GitVersioner::calculate_version(&repo_path, main_branch)?;

    if args.verbose {
        println!("Repository path: {}", repo_path.display());
    }

    let output = Output {
        major: version.major,
        minor: version.minor,
        patch: version.patch,
        major_minor_patch: format!("{}.{}.{}", version.major, version.minor, version.patch),
        pre_release_tag: version.pre.to_string(),
        pre_release_tag_with_dash: if version.pre.is_empty() {"".to_string()} else {format!("-{}", version.pre.as_str())},
        pre_release_label: version.pre.as_str().split('.').nth(0).unwrap_or("").to_string(),
        pre_release_label_with_dash: if version.pre.is_empty() {"".to_string()} else {format!("-{}", version.pre.as_str().split('.').nth(0).unwrap_or(""))},
        pre_release_number: version.pre.as_str().split('.').nth(1).unwrap_or("").to_string(),
        build_metadata: version.build.to_string(),
        sem_ver: version.to_string(),
        assembly_sem_ver: format!("{}.{}.{}", version.major, version.minor, version.patch),
        full_sem_ver: version.to_string(),
        informational_version: version.to_string(),
    };

    let json = serde_json::to_string_pretty(&output)?;
    println!("{}", json);

    Ok(())
}
