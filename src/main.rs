use anyhow::Result;
use git_versioner::config::{Configuration, load_configuration};
use git_versioner::*;

fn main() -> Result<()> {
    let config = load_configuration()?;
    let versioner = GitVersioner::new(&config)?;
    if config.verbose() || config.show_config() {
        versioner.print_config();
        if config.show_config() {
            return Ok(());
        }
    }

    let version = GitVersioner::calculate_version(&config)?;

    let json = serde_json::to_string_pretty(&version)?;
    println!("{json}");

    Ok(())
}
