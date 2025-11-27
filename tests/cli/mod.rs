use crate::common::{MAIN_BRANCH, TestRepo};
use anyhow::anyhow;
use git_versioner::GitVersion;
use git_versioner::config::ConfigurationFile;
use insta_cmd::get_cargo_bin;
use rstest::fixture;
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
    cmd.current_dir(&repo.config.path);

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
        let content = self.serialize_config(extension)?;
        self.write(name, extension, content)
    }

    pub fn serialize_config(&self, extension: &str) -> anyhow::Result<String> {
        match extension {
            "toml" => Ok(toml::to_string(&self.config_file)?),
            "yaml" | "yml" => Ok(serde_yaml::to_string(&self.config_file)?),
            &_ => Err(anyhow!("Unsupported file extension {extension}")),
        }
    }

    fn write(&self, filename: &str, extension: &str, content: String) -> anyhow::Result<PathBuf> {
        let file_path = self
            .inner
            .config
            .path
            .join(format!("{filename}.{extension}"));
        fs::write(&file_path, content)?;
        Ok(file_path)
    }

    pub fn execute_and_verify<'a, I: IntoIterator<Item = &'a str>>(
        &mut self,
        args: I,
        config_file: Option<(&str, &str)>,
    ) {
        let config_path = match config_file {
            None => PathBuf::new(),
            Some((name, ext)) => self.write_config(name, ext).unwrap(),
        };
        let output = self.cli.args(args).env_clear().output().unwrap();

        let context = format!(
            "Git Graph:\n  {}\nConfig ({}):\n  {}\nArgs:\n  {}\n",
            shifted(self.inner.graph()),
            config_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy(),
            shifted(fs::read_to_string(&config_path).unwrap_or_default()),
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

        assert!(
            output.status.success(),
            "{context}\n{stderr}",
            context = context,
            stderr = String::from_utf8_lossy(&output.stderr)
        );
        let stdout = str::from_utf8(&output.stdout).unwrap();
        let actual: GitVersion = serde_json::from_str(stdout).unwrap();

        let expected = self.inner.assert().result;
        assert_eq!(
            &expected, &actual,
            "Expected {expected} does not match actual {actual}\n{context}"
        );

        fn shifted(raw: String) -> String {
            raw.replace("\n", "\n  ").trim_end_matches(' ').to_string()
        }
    }
}
