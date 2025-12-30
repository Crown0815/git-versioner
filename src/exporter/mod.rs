use crate::GitVersion;
use anyhow::Result;
use inflection_rs::inflection;
use std::env;
use std::fs::OpenOptions;
use std::io::Write;

pub trait Exporter {
    fn export(&self, version: &GitVersion) -> Result<()>;
}

pub struct GitHubExporter;

impl Exporter for GitHubExporter {
    fn export(&self, version: &GitVersion) -> Result<()> {
        if let Some(github_output_file) = env::var_os("GITHUB_OUTPUT") {
            let map = serde_json::to_value(version)?;
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
                writeln!(file, "{}={value}", inflection::camelize_upper(key, false))?;
            }
        }
        Ok(())
    }
}

pub struct GitLabExporter;

impl Exporter for GitLabExporter {
    fn export(&self, version: &GitVersion) -> Result<()> {
        if let Some(gitlab_env_file) = env::var_os("GITLAB_ENV") {
            let map = serde_json::to_value(version)?;
            let map = map.as_object().unwrap();

            let mut file = OpenOptions::new()
                .append(true)
                .create(true)
                .open(gitlab_env_file)?;

            for (key, raw_value) in map {
                let value = match raw_value {
                    serde_json::Value::String(s) => s.clone(),
                    _ => raw_value.to_string(),
                };
                writeln!(file, "GitVersion_{key}={value}")?;
            }
        }
        Ok(())
    }
}

pub struct TeamCityExporter;

impl Exporter for TeamCityExporter {
    fn export(&self, version: &GitVersion) -> Result<()> {
        let map = serde_json::to_value(version)?;
        let map = map.as_object().unwrap();

        for (key, raw_value) in map {
            let value = match raw_value {
                serde_json::Value::String(s) => s.clone(),
                _ => raw_value.to_string(),
            };
            println!("##teamcity[setParameter name='GitVersion.{key}' value='{value}']");
            println!("##teamcity[setParameter name='system.GitVersion.{key}' value='{value}']");
        }
        Ok(())
    }
}

pub fn export_to_build_agent(version: &GitVersion) -> Result<()> {
    if !env::var_os("CI")
        .is_some_and(|value| value.to_string_lossy().parse::<bool>().unwrap_or(false))
    {
        return Ok(());
    }

    if env::var_os("GITHUB_ACTIONS").is_some() {
        GitHubExporter.export(version)?;
    }

    if env::var_os("GITLAB_CI").is_some() {
        GitLabExporter.export(version)?;
    }

    if env::var_os("TEAMCITY_VERSION").is_some() {
        TeamCityExporter.export(version)?;
    }

    Ok(())
}
