const fs = require('fs');
const https = require('https');
const os = require('os');
const path = require('path');
const { spawn } = require('child_process');

const CLI_NAME = 'git-versioner';

function info(message) {
  console.log(message);
}

function fail(message) {
  console.error(`::error::${message}`);
  process.exitCode = 1;
}

function getInput(name) {
  const key = `INPUT_${name.replace(/-/g, '_').toUpperCase()}`;
  return (process.env[key] || '').trim();
}

function isTrue(value) {
  return value.toLowerCase() === 'true';
}

function targetTriple() {
  const platform = process.platform;
  const arch = process.arch;

  if (platform === 'linux') {
    if (arch === 'x64') return 'x86_64-unknown-linux-gnu';
    if (arch === 'ia32') return 'i686-unknown-linux-gnu';
    if (arch === 'arm64') return 'aarch64-unknown-linux-gnu';
  }

  if (platform === 'darwin') {
    if (arch === 'x64') return 'x86_64-apple-darwin';
    if (arch === 'arm64') return 'aarch64-apple-darwin';
  }

  if (platform === 'win32') {
    if (arch === 'x64') return 'x86_64-pc-windows-msvc';
    if (arch === 'ia32') return 'i686-pc-windows-msvc';
    if (arch === 'arm64') return 'aarch64-pc-windows-msvc';
  }

  throw new Error(`Unsupported runner platform: ${platform}/${arch}`);
}

function normalizeActionRef(ref) {
  if (!ref) return '';
  return ref.replace(/^refs\/tags\//, '').replace(/^refs\/heads\//, '');
}

function releaseDownloadUrl(ownerRepo, tag, assetName) {
  const [owner, repo] = ownerRepo.split('/');
  if (!owner || !repo) {
    throw new Error(`Invalid GitHub repository name: ${ownerRepo}`);
  }

  const encodedOwner = encodeURIComponent(owner);
  const encodedRepo = encodeURIComponent(repo);
  const encodedAsset = encodeURIComponent(assetName);

  if (tag) {
    return `https://github.com/${encodedOwner}/${encodedRepo}/releases/download/${encodeURIComponent(tag)}/${encodedAsset}`;
  }

  return `https://github.com/${encodedOwner}/${encodedRepo}/releases/latest/download/${encodedAsset}`;
}

function download(url, destination, redirectCount = 0) {
  if (redirectCount > 10) {
    return Promise.reject(new Error('Too many redirects while downloading release asset'));
  }

  const token = process.env.GITHUB_TOKEN || process.env.GH_TOKEN || '';
  const headers = {
    'User-Agent': 'git-versioner-action',
    Accept: 'application/octet-stream',
  };

  if (token) {
    headers.Authorization = `Bearer ${token}`;
  }

  return new Promise((resolve, reject) => {
    const request = https.get(url, { headers }, (response) => {
      const status = response.statusCode || 0;

      if ([301, 302, 303, 307, 308].includes(status)) {
        response.resume();
        const location = response.headers.location;
        if (!location) {
          reject(new Error(`Release asset redirect did not include a Location header (${status})`));
          return;
        }

        const nextUrl = new URL(location, url).toString();
        download(nextUrl, destination, redirectCount + 1).then(resolve, reject);
        return;
      }

      if (status < 200 || status >= 300) {
        response.resume();
        reject(new Error(`Failed to download release asset: HTTP ${status}`));
        return;
      }

      const file = fs.createWriteStream(destination, { mode: 0o755 });
      response.pipe(file);
      file.on('finish', () => file.close(resolve));
      file.on('error', reject);
    });

    request.on('error', reject);
  });
}

function buildArgs() {
  const args = [];

  const pathInput = getInput('path');
  if (pathInput) args.push('--path', pathInput);

  const mainBranch = getInput('main-branch');
  if (mainBranch) args.push('--main-branch', mainBranch);

  const releaseBranch = getInput('release-branch');
  if (releaseBranch) args.push('--release-branch', releaseBranch);

  const featureBranch = getInput('feature-branch');
  if (featureBranch) args.push('--feature-branch', featureBranch);

  const tagPrefix = getInput('tag-prefix');
  if (tagPrefix) args.push('--tag-prefix', tagPrefix);

  const preReleaseTag = getInput('pre-release-tag');
  if (preReleaseTag) args.push('--pre-release-tag', preReleaseTag);

  const patchPreReleaseTag = getInput('patch-pre-release-tag');
  if (patchPreReleaseTag) args.push('--patch-pre-release-tag', patchPreReleaseTag);

  if (isTrue(getInput('continuous-delivery'))) args.push('--continuous-delivery');

  const commitMessageIncrementing = getInput('commit-message-incrementing');
  if (commitMessageIncrementing) {
    args.push('--commit-message-incrementing', commitMessageIncrementing);
  }

  const assemblyInformationalFormat = getInput('assembly-informational-format');
  if (assemblyInformationalFormat) {
    args.push('--assembly-informational-format', assemblyInformationalFormat);
  }

  if (isTrue(getInput('as-release'))) args.push('--as-release');

  if (isTrue(getInput('show-config'))) args.push('--show-config');

  if (isTrue(getInput('verbose'))) args.push('--verbose');

  const config = getInput('config');
  if (config) args.push('--config', config);

  return args;
}

function run(binary, args) {
  return new Promise((resolve, reject) => {
    const child = spawn(binary, args, {
      env: process.env,
      stdio: 'inherit',
      windowsHide: true,
    });

    child.on('error', reject);
    child.on('close', (code, signal) => {
      if (signal) {
        reject(new Error(`${CLI_NAME} exited due to signal ${signal}`));
        return;
      }

      resolve(code || 0);
    });
  });
}

async function main() {
  const target = targetTriple();
  const executableSuffix = process.platform === 'win32' ? '.exe' : '';
  const assetName = `${CLI_NAME}-${target}${executableSuffix}`;
  const ownerRepo = process.env.GITHUB_ACTION_REPOSITORY || process.env.GITHUB_REPOSITORY || 'Crown0815/git-versioner';
  const actionRef = normalizeActionRef(process.env.GITHUB_ACTION_REF || '');
  const url = releaseDownloadUrl(ownerRepo, actionRef, assetName);
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), `${CLI_NAME}-`));
  const binary = path.join(tempDir, `${CLI_NAME}${executableSuffix}`);

  if (actionRef) {
    info(`Downloading ${assetName} from ${ownerRepo}@${actionRef}`);
  } else {
    info(`Downloading ${assetName} from the latest ${ownerRepo} release`);
  }

  await download(url, binary);

  if (process.platform !== 'win32') {
    fs.chmodSync(binary, 0o755);
  }

  const code = await run(binary, buildArgs());
  process.exitCode = code;
}

main().catch((error) => {
  fail(error instanceof Error ? error.message : String(error));
});
