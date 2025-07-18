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
    cli: Command,
}

impl ConfiguredTestRepo {
    pub fn write_config(&self, name: &str, extension: &str) -> Result<PathBuf> {
        let content = match extension {
            "toml" => toml::to_string(&self.cli_config)?,
            "yaml" => serde_yaml::to_string(&self.cli_config)?,
            &_ => return Err(anyhow!("Unsupported file extension {extension}")),
        };
        self.write(name, extension, content)
    }

    fn write(&self, filename: &str, extension: &str, content: String) -> Result<PathBuf> {
        let file_path = self.inner.path.join(format!("{filename}.{extension}"));
        fs::write(&file_path, content)?;
        Ok(file_path)
    }

    pub fn assert_version<'a, I: IntoIterator<Item = &'a str>>(
        &mut self,
        version: &str,
        branch: &str,
        args: I,
        config_name: &str,
        config_extension: &str,
    ) {
        let config_path = self.write_config(config_name, config_extension).unwrap();
        let output = self.cli.args(args).output().unwrap();

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
            self.inner
                .graph()
                .replace("\n", &format!("\n{}", "  "))
                .trim_end_matches(' '),
            config_path.file_name().unwrap().to_string_lossy(),
            fs::read_to_string(&config_path)
                .unwrap()
                .replace("\n", &format!("\n{}", "  "))
                .trim_end_matches(' '),
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
    }
}

#[fixture]
pub fn repo(#[default(MAIN_BRANCH)] main: &str, mut cli: Command) -> ConfiguredTestRepo {
    let repo = TestRepo::initialize(main);
    let cli_config = ConfigurationFile::default();
    repo.commit("0.1.0-pre.1");
    cli.current_dir(&repo.path);

    ConfiguredTestRepo {
        inner: repo,
        cli_config,
        cli,
    }
}

#[rstest]
fn test_that_toml_config_file_overrides_default_main_branch_pattern(
    #[with(CUSTOM_MAIN_BRANCH)] mut repo: ConfiguredTestRepo,
    #[values("toml", "yaml")] extension: &str,
) {
    repo.cli_config.main_branch = Some(format!("^{CUSTOM_MAIN_BRANCH}$"));

    repo.assert_version(
        "0.1.0-pre.1",
        CUSTOM_MAIN_BRANCH,
        [],
        DEFAULT_CONFIG,
        extension,
    );
}

#[rstest]
fn test_that_cli_argument_overrides_configuration_of_main_branch_pattern(
    #[with(CUSTOM_MAIN_BRANCH)] mut repo: ConfiguredTestRepo,
    #[values("toml", "yaml")] extension: &str,
) {
    repo.cli_config.main_branch = Some(format!("^{}$", "another_main_branch"));

    repo.assert_version(
        "0.1.0-pre.1",
        CUSTOM_MAIN_BRANCH,
        ["--main-branch", CUSTOM_MAIN_BRANCH],
        DEFAULT_CONFIG,
        extension,
    );
}

#[rstest]
fn test_that_toml_config_file_overrides_default_release_branch_pattern(
    mut repo: ConfiguredTestRepo,
    #[values("toml", "yaml")] extension: &str,
) {
    repo.cli_config.release_branch = Some("custom-release/(?<BranchName>.*)".to_string());
    repo.inner.commit("0.1.0-pre.1");
    repo.inner.tag("v1.0.0");
    repo.inner.branch("custom-release/1.0.0");
    repo.inner.commit("1.0.1-pre.1");

    repo.assert_version(
        "1.0.1-pre.1",
        "custom-release/1.0.0",
        [],
        DEFAULT_CONFIG,
        extension,
    );
}

#[rstest]
fn test_that_cli_argument_overrides_configuration_of_release_branch_pattern(
    mut repo: ConfiguredTestRepo,
    #[values("toml", "yaml")] extension: &str,
) {
    repo.cli_config.release_branch = Some("whatever-release/(?<BranchName>.*)".to_string());
    repo.inner.commit("0.1.0-pre.1");
    repo.inner.tag("v1.0.0");
    repo.inner.branch("custom-release/1.0.0");
    repo.inner.commit("1.0.1-pre.1");

    repo.assert_version(
        "1.0.1-pre.1",
        "custom-release/1.0.0",
        ["--release-branch", "custom-release/(?<BranchName>.*)"],
        DEFAULT_CONFIG,
        extension,
    );
}

#[rstest]
fn test_that_toml_config_file_overrides_default_feature_branch_pattern(
    mut repo: ConfiguredTestRepo,
    #[values("toml", "yaml")] extension: &str,
) {
    repo.cli_config.feature_branch = Some("my-feature/(?<BranchName>.*)".to_string());
    repo.inner.commit("0.1.0-pre.1");
    repo.inner.branch("my-feature/feature");
    repo.inner.commit("0.1.0-feature.1");

    repo.assert_version(
        "0.1.0-feature.1",
        "my-feature/feature",
        [],
        DEFAULT_CONFIG,
        extension,
    );
}

#[rstest]
fn test_that_cli_argument_overrides_configuration_of_feature_branch_pattern(
    mut repo: ConfiguredTestRepo,
    #[values("toml", "yaml")] extension: &str,
) {
    repo.cli_config.feature_branch = Some("whatever-feature/(?<BranchName>.*)".to_string());
    repo.inner.commit("0.1.0-pre.1");
    repo.inner.branch("my-feature/feature");
    repo.inner.commit("0.1.0-feature.1");

    repo.assert_version(
        "0.1.0-feature.1",
        "my-feature/feature",
        ["--feature-branch", "my-feature/(?<BranchName>.*)"],
        DEFAULT_CONFIG,
        extension,
    );
}

#[rstest]
fn test_that_toml_config_file_overrides_default_version_pattern(
    mut repo: ConfiguredTestRepo,
    #[values("toml", "yaml")] extension: &str,
) {
    repo.cli_config.version_pattern = Some("my/c(?<Version>.*)".to_string());
    repo.inner.commit("0.1.0-pre.1");
    repo.inner.tag("my/c1.0.0");

    repo.assert_version("1.0.0", MAIN_BRANCH, [], DEFAULT_CONFIG, extension);
}

#[rstest]
fn test_that_cli_argument_overrides_configuration_of_version_pattern(
    mut repo: ConfiguredTestRepo,
    #[values("toml", "yaml")] extension: &str,
) {
    repo.cli_config.version_pattern = Some("my/c(?<Version>.*)".to_string());
    repo.inner.commit("0.1.0-pre.1");
    repo.inner.tag("my/v1.0.0");

    repo.assert_version(
        "1.0.0",
        MAIN_BRANCH,
        ["--version-pattern", "my/v(?<Version>.*)"],
        DEFAULT_CONFIG,
        extension,
    );
}
