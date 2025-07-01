use anyhow::Result;
use clap::Parser;
use git_versioner::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, value_parser)]
    path: Option<PathBuf>,

    #[arg(long, value_parser, default_value = MAIN_BRANCH)]
    main_branch: String,

    #[arg(long, value_parser, default_value = RELEASE_BRANCH)]
    release_branch: String,

    #[arg(long, value_parser, default_value = FEATURE_BRANCH)]
    feature_branch: String,

    #[arg(long, value_parser, default_value = VERSION_PATTERN)]
    version_pattern: String,

    #[arg(short, long)]
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

    let config = Configuration {
        repo_path: args.path.unwrap_or_else(|| std::env::current_dir().unwrap()),
        main_branch: args.main_branch,
        release_branch: args.release_branch,
        feature_branch: args.feature_branch,
        version_pattern: args.version_pattern,
    };

    let version = GitVersioner::calculate_version(&config)?;

    if args.verbose {
        println!("Repository path: {}", config.repo_path.display());
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
