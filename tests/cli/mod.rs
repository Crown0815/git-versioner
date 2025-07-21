use crate::common::{MAIN_BRANCH, TestRepo};
use anyhow::anyhow;
use git_versioner::GitVersion;
use git_versioner::config::ConfigurationFile;
use git2::Oid;
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

    pub fn assert<'a, I: IntoIterator<Item = &'a str>>(
        &mut self,
        args: I,
        config_file: Option<(&str, &str)>,
    ) -> Assertable {
        let config_path = match config_file {
            None => PathBuf::new(),
            Some((name, ext)) => self.write_config(name, ext).unwrap(),
        };

        let output = self.cli.args(args).output().unwrap();

        let stdout = str::from_utf8(&output.stdout).unwrap();
        let result: GitVersion = serde_json::from_str(stdout).unwrap();
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

        fn shifted(raw: String) -> String {
            raw.replace("\n", "\n  ").trim_end_matches(' ').to_string()
        }

        Assertable { result, context }
    }
}

pub struct Assertable {
    result: GitVersion,
    context: String,
}

impl Assertable {
    pub fn version(self, expected: &str) -> Self {
        let actual = &self.result.full_sem_ver;
        assert_eq!(
            actual, expected,
            "Expected version: {expected}, found: {actual}\n{}",
            self.result,
        );
        self
    }

    pub fn branch(self, expected: &str) -> Self {
        let actual = &self.result.branch_name;
        assert_eq!(
            actual, expected,
            "Expected branch: {expected}, found: {actual}\n{}",
            self.context
        );
        self
    }

    pub fn source_id(self, expected: Oid) -> Self {
        self.source_sha(&expected.to_string())
    }

    pub fn has_no_source(self) -> Self {
        self.source_sha("")
    }

    pub fn source_sha(self, expected: &str) -> Self {
        let actual = &self.result.version_source_sha;
        assert_eq!(
            actual, expected,
            "Expected source_id: {expected}, found: {actual}\n{}",
            self.context
        );
        self
    }
}
