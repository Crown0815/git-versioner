use anyhow::Result;
use clap::Parser;
use git_versioner::GitVersioner;
use regex::Regex;
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

fn main() -> Result<()> {
    let args = Args::parse();

    let repo_path = args.repo_path.unwrap_or_else(|| std::env::current_dir().unwrap());
    let main_branch = args.main_branch.unwrap_or_else(|| Regex::new(MAIN_BRANCH).unwrap());
    let version = GitVersioner::calculate_version(&repo_path, main_branch)?;

    if args.verbose {
        println!("Repository path: {}", repo_path.display());
        println!("Calculated version: {}", version);
    } else {
        println!("{}", version);
    }

    Ok(())
}
