use anyhow::Result;
use git_versioner::config::load_configuration;
use git_versioner::*;

fn main() -> Result<()> {
    let config = load_configuration()?;
    let version = GitVersioner::calculate_version(&config)?;

    let json = serde_json::to_string_pretty(&version)?;
    println!("{}", json);

    Ok(())
}
