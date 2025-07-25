use anyhow::Result;
use git_versioner::config::{Configuration, load_configuration};
use git_versioner::*;
use std::env;
use std::fs::OpenOptions;
use std::io::Write;

fn main() -> Result<()> {
    let config = load_configuration()?;
    if *config.verbose() || *config.show_config() {
        print(&config);
        if *config.show_config() {
            return Ok(());
        }
    }

    let version = GitVersioner::calculate_version(&config)?;

    let json = serde_json::to_string_pretty(&version)?;
    println!("{json}");

    if env::var_os("CI").is_some_and(|value| value.to_string_lossy().parse::<bool>().unwrap()) {
        if let Some(github_output_file) = env::var_os("GITHUB_OUTPUT") {
            let map = serde_json::to_value(&version)?;
            let map = map.as_object().unwrap();

            let mut file = OpenOptions::new()
                .append(true)
                .create(true)
                .open(github_output_file)?;

            for (key, raw_value) in map {
                let value = match raw_value {
                    serde_json::Value::String(s) => s.clone(),
                    _ => raw_value.to_string(),
                };
                writeln!(file, "GitVersion_{key}={value}")?;
            }
        }
    }

    Ok(())
}

fn print<T: Configuration>(config: &T) {
    println!("Configuration:");
    println!("{}", toml::to_string(&config.print()).unwrap());
}
