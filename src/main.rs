use anyhow::Result;
use git_versioner::GitVersioner;
use git_versioner::config::{Configuration, load_configuration};
use git_versioner::exporter::export_to_build_agent;

fn main() -> Result<()> {
    let config = load_configuration()?;
    if *config.show_config() {
        print(&config);
        return Ok(());
    }
    if *config.verbose() {
        print(&config);
    }

    let version = GitVersioner::calculate_version(&config)?;

    let json = serde_json::to_string_pretty(&version)?;
    println!("{json}");

    export_to_build_agent(&version)?;

    Ok(())
}

fn print<T: Configuration>(config: &T) {
    println!("Configuration:");
    println!("{}", toml::to_string(&config.print()).unwrap());
}
