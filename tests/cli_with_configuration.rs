mod common;

use crate::common::MAIN_BRANCH;
use anyhow::{Result, anyhow};
use common::{TestRepo, cli};
use git_versioner::GitVersion;
use git_versioner::config::ConfigurationFile;
use rstest::{fixture, rstest};
use semver::Version;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const CUSTOM_MAIN_BRANCH: &str = "stem";
const DEFAULT_CONFIG: &str = ".git-versioner";

struct ConfiguredTestRepo {
    inner: TestRepo,
    cli_config: ConfigurationFile,
}

impl ConfiguredTestRepo {
    pub fn path(&self) -> &PathBuf {
        &self.inner.path
    }
    pub fn graph(&self) -> String {
        self.inner.graph()
    }

    pub fn create_config(&self, name: &str, extension: &str) -> Result<PathBuf> {
        let content = match extension {
            "toml" => toml::to_string(&self.cli_config)?,
            "yaml" => serde_yaml::to_string(&self.cli_config)?,
            &_ => return Err(anyhow!("Unsupported file extension {extension}")),
        };
        self.write(name, extension, content)
    }

    fn write(&self, filename: &str, extension: &str, content: String) -> Result<PathBuf> {
        let config_path = self.path().join(format!("{filename}.{extension}"));
        fs::write(&config_path, content)?;
        Ok(config_path)
    }

    pub fn assert_version(
        &self,
        version: &str,
        branch: &str,
        mut cmd: Command,
        config_path: PathBuf,
    ) {
        let output = cmd.output().unwrap();
        let stdout = str::from_utf8(&output.stdout).unwrap();
        let actual: GitVersion = serde_json::from_str(stdout).unwrap();

        let expected = GitVersion::new(Version::parse(version).unwrap(), branch.to_string());

        assert_eq!(
            actual,
            expected,
            "Expected HEAD version: {expected}, found: {actual}\n\
            Git Graph:\n  {}\n\
            Config ({}):\n  {}\n\
            Args:\n  {}\n",
            self.graph()
                .replace("\n", &format!("\n{}", "  "))
                .trim_end_matches(' '),
            config_path.file_name().unwrap().to_string_lossy(),
            fs::read_to_string(&config_path)
                .unwrap()
                .replace("\n", &format!("\n{}", "  "))
                .trim_end_matches(' '),
            cmd.get_args()
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
    }
}

#[fixture]
pub fn repo(#[default(MAIN_BRANCH)] main: &str) -> ConfiguredTestRepo {
    let repo = TestRepo::initialize(main);
    let cli_config = ConfigurationFile::default();
    repo.commit("0.1.0-pre.1");

    ConfiguredTestRepo {
        inner: repo,
        cli_config,
    }
}

#[rstest]
fn test_that_toml_config_file_overrides_default_main_branch_pattern(
    #[with(CUSTOM_MAIN_BRANCH)] mut repo: ConfiguredTestRepo,
    #[values("toml", "yaml")] extension: &str,
    mut cli: Command,
) {
    repo.cli_config.main_branch = Some(format!("^{CUSTOM_MAIN_BRANCH}$"));
    let config_file = repo.create_config(DEFAULT_CONFIG, extension).unwrap();
    cli.current_dir(repo.path());

    repo.assert_version("0.1.0-pre.1", CUSTOM_MAIN_BRANCH, cli, config_file);
}

#[rstest]
fn test_that_cli_argument_overrides_configuration_of_main_branch_pattern(
    #[with(CUSTOM_MAIN_BRANCH)] mut repo: ConfiguredTestRepo,
    #[values("toml", "yaml")] extension: &str,
    mut cli: Command,
) {
    repo.cli_config.main_branch = Some(format!("^{}$", "another_main_branch"));
    let config_file = repo.create_config(DEFAULT_CONFIG, extension).unwrap();
    cli.current_dir(repo.path())
        .args(["--main-branch", CUSTOM_MAIN_BRANCH]);

    repo.assert_version("0.1.0-pre.1", CUSTOM_MAIN_BRANCH, cli, config_file);
}

#[rstest]
fn test_that_toml_config_file_overrides_default_release_branch_pattern(
    mut repo: ConfiguredTestRepo,
    #[values("toml", "yaml")] extension: &str,
    mut cli: Command,
) {
    repo.cli_config.release_branch = Some("custom-release/(?<BranchName>.*)".to_string());
    let config_file = repo.create_config(DEFAULT_CONFIG, extension).unwrap();
    repo.inner.commit("0.1.0-pre.1");
    repo.inner.tag("v1.0.0");
    repo.inner.branch("custom-release/1.0.0");
    repo.inner.commit("1.0.1-pre.1");

    cli.current_dir(repo.path());

    repo.assert_version("1.0.1-pre.1", "custom-release/1.0.0", cli, config_file);
}

#[rstest]
fn test_that_cli_argument_overrides_configuration_of_release_branch_pattern(
    mut repo: ConfiguredTestRepo,
    #[values("toml", "yaml")] extension: &str,
    mut cli: Command,
) {
    repo.cli_config.release_branch = Some("whatever-release/(?<BranchName>.*)".to_string());
    let config_file = repo.create_config(DEFAULT_CONFIG, extension).unwrap();
    repo.inner.commit("0.1.0-pre.1");
    repo.inner.tag("v1.0.0");
    repo.inner.branch("custom-release/1.0.0");
    repo.inner.commit("1.0.1-pre.1");

    cli.current_dir(repo.path())
        .args(["--release-branch", "custom-release/(?<BranchName>.*)"]);

    repo.assert_version("1.0.1-pre.1", "custom-release/1.0.0", cli, config_file);
}

#[rstest]
fn test_that_toml_config_file_overrides_default_feature_branch_pattern(
    mut repo: ConfiguredTestRepo,
    #[values("toml", "yaml")] extension: &str,
    mut cli: Command,
) {
    repo.cli_config.feature_branch = Some("my-feature/(?<BranchName>.*)".to_string());
    let config_file = repo.create_config(DEFAULT_CONFIG, extension).unwrap();
    repo.inner.commit("0.1.0-pre.1");
    repo.inner.branch("my-feature/feature");
    repo.inner.commit("0.1.0-feature.1");

    cli.current_dir(repo.path());

    repo.assert_version("0.1.0-feature.1", "my-feature/feature", cli, config_file);
}

#[rstest]
fn test_that_cli_argument_overrides_configuration_of_feature_branch_pattern(
    mut repo: ConfiguredTestRepo,
    #[values("toml", "yaml")] extension: &str,
    mut cli: Command,
) {
    repo.cli_config.feature_branch = Some("whatever-feature/(?<BranchName>.*)".to_string());
    let config_file = repo.create_config(DEFAULT_CONFIG, extension).unwrap();
    repo.inner.commit("0.1.0-pre.1");
    repo.inner.branch("my-feature/feature");
    repo.inner.commit("0.1.0-feature.1");

    cli.current_dir(repo.path())
        .args(["--feature-branch", "my-feature/(?<BranchName>.*)"]);

    repo.assert_version("0.1.0-feature.1", "my-feature/feature", cli, config_file);
}

#[rstest]
fn test_that_toml_config_file_overrides_default_version_pattern(
    mut repo: ConfiguredTestRepo,
    #[values("toml", "yaml")] extension: &str,
    mut cli: Command,
) {
    repo.cli_config.version_pattern = Some("my/c(?<Version>.*)".to_string());
    let config_file = repo.create_config(DEFAULT_CONFIG, extension).unwrap();
    repo.inner.commit("0.1.0-pre.1");
    repo.inner.tag("my/c1.0.0");

    cli.current_dir(repo.path());

    repo.assert_version("1.0.0", MAIN_BRANCH, cli, config_file);
}

#[rstest]
fn test_that_cli_argument_overrides_configuration_of_version_pattern(
    mut repo: ConfiguredTestRepo,
    #[values("toml", "yaml")] extension: &str,
    mut cli: Command,
) {
    repo.cli_config.version_pattern = Some("my/c(?<Version>.*)".to_string());
    let config_file = repo.create_config(DEFAULT_CONFIG, extension).unwrap();
    repo.inner.commit("0.1.0-pre.1");
    repo.inner.tag("my/v1.0.0");

    cli.current_dir(repo.path())
        .args(["--version-pattern", "my/v(?<Version>.*)"]);

    repo.assert_version("1.0.0", MAIN_BRANCH, cli, config_file);
}
