use crate::config::{DefaultConfig, FEATURE_BRANCH, MAIN_BRANCH, RELEASE_BRANCH, VERSION_PATTERN};
use anyhow::{anyhow, Result};
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

    /// Path to a configuration file (TOML or YAML)
    #[arg(short = 'c', long = "config")]
    config_file: Option<PathBuf>,
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

    // Try to load configuration from a file if specified
    let mut config = if let Some(config_path) = &args.config_file {
        match DefaultConfig::from_file(config_path) {
            Ok(config) => {
                if args.verbose {
                    println!("Loaded configuration from {}", config_path.display());
                }
                config
            },
            Err(err) => {
                return Err(anyhow!("Failed to load configuration from {}: {}", config_path.display(), err));
            }
        }
    } else {
        // Try to load from default configuration files
        match DefaultConfig::from_default_files() {
            Ok(config) => {
                if args.verbose {
                    println!("Loaded configuration from default file");
                }
                config
            },
            Err(_) => {
                // Fall back to CLI arguments
                DefaultConfig::default()
            }
        }
    };

    // Override with CLI arguments if provided
    if let Some(path) = args.path {
        config.repo_path = path;
    }

    // Only override pattern values if they're different from the defaults
    if args.main_branch != MAIN_BRANCH {
        config.main_branch = args.main_branch;
    }
    if args.release_branch != RELEASE_BRANCH {
        config.release_branch = args.release_branch;
    }
    if args.feature_branch != FEATURE_BRANCH {
        config.feature_branch = args.feature_branch;
    }
    if args.version_pattern != VERSION_PATTERN {
        config.version_pattern = args.version_pattern;
    }

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
