---
layout: home

hero:
  name: Git Versioner
  text: Version numbers derived from Git history.
  tagline: A Rust CLI and GitHub Action for trunk-based repositories with release branches.
  image:
    src: /logo.png
    alt: Git Versioner logo
  actions:
    - theme: brand
      text: Get started
      link: "#install"
    - theme: alt
      text: View on GitHub
      link: https://github.com/Crown0815/git-versioner

features:
  - title: No committed version files
    details: Calculate semantic versions from branches, tags, commits, and repository state.
  - title: Built for CI/CD
    details: Emit GitVersion-style variables for GitHub Actions, GitLab, TeamCity, scripts, and release workflows.
  - title: Small configuration surface
    details: Configure branch patterns, tag prefixes, pre-release labels, and informational version templates.
---

## Why use it?

Git Versioner calculates semantic versions from Git history.
It is designed for trunk-based repositories where `trunk`, `main`, or `master` is the integration line and release branches stabilize individual versions.

- Calculate versions without committing version files.
- Support trunk, release branches, feature branches, and version tags.
- Emit GitVersion-style variables for CI/CD systems.
- Use semantic versions for release builds and pre-release versions for development builds.
- Keep configuration in TOML, YAML, or command-line flags.

## Install

::: code-group

```bash [Cargo]
cargo install git-versioner
git-versioner
```

```yaml [GitHub Action]
- uses: actions/checkout@v6
  with:
    fetch-depth: 0

- name: Determine Version
  id: versioner
  uses: crown0815/git-versioner@v1
```

:::

::: warning Fetch full history
Git Versioner needs tags and branch history. In GitHub Actions, set `fetch-depth: 0` on `actions/checkout`.
:::

## Command line

Run Git Versioner from the repository root:

```bash
git-versioner
```

Useful options:

| Option | Description |
| --- | --- |
| `-p, --path <PATH>` | Path to the repository to calculate the version for. |
| `--main-branch <MAIN_BRANCH>` | Regex to detect the main branch. |
| `--release-branch <RELEASE_BRANCH>` | Regex to detect release branches. |
| `--feature-branch <FEATURE_BRANCH>` | Regex to detect feature branches. |
| `--tag-prefix <TAG_PREFIX>` | Regex to detect version tags. |
| `--pre-release-tag <PRE_RELEASE_TAG>` | Label for pre-release versions, for example `pre`, `alpha`, `beta`, or `rc`. |
| `--patch-pre-release-tag <PATCH_PRE_RELEASE_TAG>` | Label for patch pre-release versions. Defaults to `PreReleaseTag`. |
| `--continuous-delivery` | Calculate the version using continuous delivery mode. |
| `--commit-message-incrementing <VALUE>` | Use `Enabled` or `Disabled` for conventional commit based increments. |
| `--assembly-informational-format <FORMAT>` | Format string for `InformationalVersion`. |
| `-a, --as-release` | Force release generation instead of pre-release generation. |
| `--show-config` | Print the effective configuration and exit. |
| `-v, --verbose` | Print the effective configuration before calculating the version. |
| `-c, --config <CONFIG_FILE>` | Path to a TOML or YAML configuration file. |

Run `git-versioner --help` for the full, authoritative list of options and their extended descriptions.

## CI outputs

Git Versioner always prints the calculated version as JSON to `stdout`, and additionally exports every field to the current CI system when the `CI` environment variable is `true`:

- **GitHub Actions** (`GITHUB_OUTPUT`): each field with a `GitVersion_` prefix (e.g. `GitVersion_SemVer`) and in PascalCase without the prefix (e.g. `SemVer`).
- **GitLab CI** (`GITLAB_ENV`): each field with a `GitVersion_` prefix.
- **TeamCity**: each field as `GitVersion.<Field>` and `system.GitVersion.<Field>` service messages.

```yaml
- name: Use Version
  run: echo "The version is ${{ steps.versioner.outputs.SemVer }}"
```

The example below shows a representative set of the exported GitVersion-style variables (additional fields such as `Sha`, `ShortSha`, `VersionSourceSha`, and `UncommittedChanges` are also emitted):

```text
GitVersion_AssemblySemFileVer=0.1.0.55001
GitVersion_AssemblySemVer=0.1.0.0
GitVersion_BranchName=trunk
GitVersion_BuildMetadata=
GitVersion_CalVerDay=09
GitVersion_CalVerMinor=1
GitVersion_CalVerMonth=03
GitVersion_CalVerYear=2024
GitVersion_CommitDate=2024-03-09
GitVersion_CommitDay=09
GitVersion_CommitMonth=03
GitVersion_CommitYear=2024
GitVersion_CommitsSinceVersionSource=0
GitVersion_EscapedBranchName=trunk
GitVersion_FullBuildMetaData=
GitVersion_FullSemVer=0.1.0-pre.1
GitVersion_InformationalVersion=0.1.0-pre.1
GitVersion_Major=0
GitVersion_MajorMinorPatch=0.1.0
GitVersion_Minor=1
GitVersion_Patch=0
GitVersion_PreReleaseLabel=pre
GitVersion_PreReleaseLabelWithDash=-pre
GitVersion_PreReleaseNumber=1
GitVersion_PreReleaseTag=pre.1
GitVersion_PreReleaseTagWithDash=-pre.1
GitVersion_SemVer=0.1.0-pre.1
GitVersion_WeightedPreReleaseNumber=55001
```

## Configuration

Create `.git-versioner.toml`, `.git-versioner.yaml`, or `.git-versioner.yml` in the repository root.
All fields are optional.

```yaml
MainBranch: ^(trunk|main|master)$
ReleaseBranch: ^releases?[/-](?<BranchName>.+)$
FeatureBranch: ^features?[/-](?<BranchName>.+)$
TagPrefix: '[vV]?'
PreReleaseTag: pre
PatchPreReleaseTag: ''
CommitMessageIncrementing: Disabled
AssemblyInformationalFormat: '{InformationalVersion}'
```

| Field | Description | Default |
| --- | --- | --- |
| `MainBranch` | Regex to detect the main branch. | `^(trunk\|main\|master)$` |
| `ReleaseBranch` | Regex to detect release branches. | `^releases?[/-](?<BranchName>.+)$` |
| `FeatureBranch` | Regex to detect feature branches. | `^features?[/-](?<BranchName>.+)$` |
| `TagPrefix` | Regex prefix for version tags. | `[vV]?` |
| `PreReleaseTag` | Label for pre-release versions. | `pre` |
| `PatchPreReleaseTag` | Label for patch (Patch > 0) pre-release versions. | value of `PreReleaseTag` |
| `CommitMessageIncrementing` | Conventional-commit increments: `Disabled` or `Enabled`. | `Disabled` |
| `AssemblyInformationalFormat` | Template for `InformationalVersion`. | `{InformationalVersion}` |

The same option can also be set in kebab case for TOML and YAML compatibility:

```yaml
assembly-informational-format: "{InformationalVersion}"
```

## Informational version templates

`AssemblyInformationalFormat` supports GitVersion-style placeholders and environment variables.

```text
{InformationalVersion}
{Major}.{Minor}.{Patch}.{WeightedPreReleaseNumber ?? 0}
{Major}.{Minor}.{Patch}.{env:BUILD_NUMBER}
{Major}.{Minor}.{Patch}.{env:BUILD_NUMBER ?? 42}
```

## Documentation stack

This site is generated with VitePress from Markdown source files.
That keeps documentation authoring simple while providing a polished static site, navigation, local search, code groups, custom containers, and GitHub Pages deployment.

## Build locally

```bash
cd docs
npm install
npm run docs:dev
```

The local documentation site will be available at `http://localhost:5173/git-versioner/`.
