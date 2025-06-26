use anyhow::Result;
use clap::Parser;
use git_versioner::GitVersioner;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long, value_parser)]
    repo_path: Option<PathBuf>,

    #[clap(short, long)]
    verbose: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let repo_path = args.repo_path.unwrap_or_else(|| std::env::current_dir().unwrap());

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
