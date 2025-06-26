use anyhow::{anyhow, Result};
use clap::Parser;
use git_versioner::GitVersioner;
use std::path::PathBuf;

/// Git Versioner - Automatically calculate version numbers for trunk-based development
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Path to the Git repository (defaults to current directory)
    #[clap(short, long, value_parser)]
    repo_path: Option<PathBuf>,

    /// Print detailed version information
    #[clap(short, long)]
    verbose: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Use current directory if no path is specified
    let repo_path = args.repo_path.unwrap_or_else(|| std::env::current_dir().unwrap());

    // Calculate the version
    let versioner = GitVersioner::new(&repo_path)?;
    let version = versioner.calculate_version()?;

    if args.verbose {
        println!("Repository path: {}", repo_path.display());
        println!("Calculated version: {}", version);
    } else {
        println!("{}", version);
    }

    Ok(())
}
