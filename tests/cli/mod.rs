use crate::common::{MAIN_BRANCH, TestRepo};
use anyhow::anyhow;
use git_versioner::GitVersion;
use git_versioner::config::ConfigurationFile;
use git2::Oid;
use insta_cmd::get_cargo_bin;
use rstest::fixture;
use semver::Version;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[fixture]
pub fn cmd() -> Command {
    Command::new(get_cargo_bin(env!("CARGO_PKG_NAME")))
}

#[fixture]
pub fn repo(#[default(MAIN_BRANCH)] main: &str, mut cmd: Command) -> ConfiguredTestRepo {
    let repo = TestRepo::initialize(main);
    let config_file = ConfigurationFile::default();
    repo.commit("0.1.0-pre.1");
    cmd.current_dir(&repo.path);

    ConfiguredTestRepo {
        inner: repo,
        config_file,
        cli: cmd,
    }
}

pub struct ConfiguredTestRepo {
    pub inner: TestRepo,
    pub config_file: ConfigurationFile,
    pub cli: Command,
}

impl ConfiguredTestRepo {
    pub fn write_config(&self, name: &str, extension: &str) -> anyhow::Result<PathBuf> {
        let content = match extension {
            "toml" => toml::to_string(&self.config_file)?,
            "yaml" => serde_yaml::to_string(&self.config_file)?,
            &_ => return Err(anyhow!("Unsupported file extension {extension}")),
        };
        self.write(name, extension, content)
    }

    fn write(&self, filename: &str, extension: &str, content: String) -> anyhow::Result<PathBuf> {
        let file_path = self.inner.path.join(format!("{filename}.{extension}"));
        fs::write(&file_path, content)?;
        Ok(file_path)
    }

    pub fn assert_version<'a, I: IntoIterator<Item = &'a str>>(
        &mut self,
        version: &str,
        branch: &str,
        args: I,
        source_id: Oid,
    ) {
        let output = self.cli.args(args).output().unwrap();

        let stdout = str::from_utf8(&output.stdout).unwrap();
        let actual: GitVersion = serde_json::from_str(stdout).unwrap();

        let expected = GitVersion::new(
            Version::parse(version).unwrap(),
            branch.to_string(),
            source_id,
        );

        assert_eq!(
            actual,
            expected,
            "Expected HEAD version: {expected}, found: {actual}\n\
            Git Graph:\n  {}\n\
            Config ({}):\n  {}\n\
            Args:\n  {}\n",
            shifted(self.inner.graph()),
            "None",
            "",
            self.cli
                .get_args()
                .map(|s| {
                    let arg = s.to_string_lossy();
                    if arg.contains(' ') || arg.is_empty() {
                        format!("\"{arg}\"")
                    } else {
                        arg.into_owned()
                    }
                })
                .collect::<Vec<_>>()
                .join(" "),
        );

        fn shifted(raw: String) -> String {
            raw.replace("\n", "\n  ").trim_end_matches(' ').to_string()
        }
    }

    pub fn assert_configured_version<'a, I: IntoIterator<Item = &'a str>>(
        &mut self,
        version: &str,
        branch: &str,
        args: I,
        config_name: &str,
        config_extension: &str,
        source_id: Oid,
    ) {
        let config_path = self.write_config(config_name, config_extension).unwrap();
        let output = self.cli.args(args).output().unwrap();

        let stdout = str::from_utf8(&output.stdout).unwrap();
        let actual: GitVersion = serde_json::from_str(stdout).unwrap();

        let expected = GitVersion::new(
            Version::parse(version).unwrap(),
            branch.to_string(),
            source_id,
        );

        assert_eq!(
            actual,
            expected,
            "Expected HEAD version: {expected}, found: {actual}\n\
            Git Graph:\n  {}\n\
            Config ({}):\n  {}\n\
            Args:\n  {}\n",
            shifted(self.inner.graph()),
            config_path.file_name().unwrap().to_string_lossy(),
            shifted(fs::read_to_string(&config_path).unwrap()),
            self.cli
                .get_args()
                .map(|s| {
                    let arg = s.to_string_lossy();
                    if arg.contains(' ') || arg.is_empty() {
                        format!("\"{arg}\"")
                    } else {
                        arg.into_owned()
                    }
                })
                .collect::<Vec<_>>()
                .join(" "),
        );

        fn shifted(raw: String) -> String {
            raw.replace("\n", "\n  ").trim_end_matches(' ').to_string()
        }
    }
}
