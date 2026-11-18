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
  const key = `INPUT_${name.toUpperCase()}`;
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

function isMajorOnlyTag(tag) {
  return /^v?\d+$/.test(tag);
}

function isStableSemverTag(tag) {
  return /^v?\d+\.\d+\.\d+$/.test(tag);
}

function majorPrefix(tag) {
  const match = /^v?(\d+)/.exec(tag);
  return match ? `v${match[1]}` : '';
}

function parseStableSemver(tag) {
  const match = /^v?(\d+)\.(\d+)\.(\d+)$/.exec(tag);
  if (!match) return null;
  return {
    major: Number(match[1]),
    minor: Number(match[2]),
    patch: Number(match[3]),
  };
}

function compareStableSemverTags(left, right) {
  const a = parseStableSemver(left);
  const b = parseStableSemver(right);
  if (!a || !b) return 0;
  return a.major - b.major || a.minor - b.minor || a.patch - b.patch;
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

function githubRequest(url, accept) {
  const token = process.env.GITHUB_TOKEN || process.env.GH_TOKEN || '';
  const headers = {
    'User-Agent': 'git-versioner-action',
    Accept: accept,
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
          reject(new Error(`Request redirect did not include a Location header (${status})`));
          return;
        }

        githubRequest(new URL(location, url).toString(), accept).then(resolve, reject);
        return;
      }

      const chunks = [];
      response.on('data', (chunk) => chunks.push(chunk));
      response.on('end', () => {
        resolve({ status, body: Buffer.concat(chunks) });
      });
      response.on('error', reject);
    });

    request.on('error', reject);
  });
}

async function githubApi(ownerRepo, pathname) {
  const [owner, repo] = ownerRepo.split('/');
  if (!owner || !repo) {
    throw new Error(`Invalid GitHub repository name: ${ownerRepo}`);
  }

  const url = `https://api.github.com/repos/${encodeURIComponent(owner)}/${encodeURIComponent(repo)}${pathname}`;
  const { status, body } = await githubRequest(url, 'application/vnd.github+json');

  if (status === 404) {
    return null;
  }

  if (status < 200 || status >= 300) {
    throw new Error(`GitHub API request failed: HTTP ${status} for ${pathname}`);
  }

  return JSON.parse(body.toString('utf8'));
}

async function peelTagToCommitSha(ownerRepo, tag) {
  const ref = await githubApi(ownerRepo, `/git/ref/tags/${encodeURIComponent(tag)}`);
  if (!ref || !ref.object) {
    return '';
  }

  if (ref.object.type === 'tag') {
    const tagObject = await githubApi(ownerRepo, `/git/tags/${encodeURIComponent(ref.object.sha)}`);
    return (tagObject && tagObject.object && tagObject.object.sha) || '';
  }

  return ref.object.sha || '';
}

function tagNameFromRef(entry) {
  return String((entry && entry.ref) || '').replace(/^refs\/tags\//, '');
}

function matchesMajor(tag, prefix) {
  const unprefixed = prefix.replace(/^v/, '');
  return tag === prefix || tag.startsWith(`${prefix}.`) || tag === unprefixed || tag.startsWith(`${unprefixed}.`);
}

async function commitShaForRefEntry(ownerRepo, entry) {
  if (!entry || !entry.object || !entry.object.sha) {
    return '';
  }

  if (entry.object.type === 'tag') {
    const tagObject = await githubApi(ownerRepo, `/git/tags/${encodeURIComponent(entry.object.sha)}`);
    return (tagObject && tagObject.object && tagObject.object.sha) || '';
  }

  return entry.object.sha;
}

async function latestStableReleaseForMajor(ownerRepo, prefix) {
  const releases = (await githubApi(ownerRepo, '/releases?per_page=100')) || [];
  const matching = releases
    .filter((release) => release && !release.draft && !release.prerelease && isStableSemverTag(release.tag_name || ''))
    .filter((release) => matchesMajor(release.tag_name, prefix))
    .map((release) => release.tag_name);

  matching.sort(compareStableSemverTags);
  return matching.length ? matching[matching.length - 1] : '';
}

async function resolveReleaseTag(ownerRepo, actionRef) {
  if (!actionRef || !isMajorOnlyTag(actionRef)) {
    return actionRef;
  }

  const prefix = majorPrefix(actionRef);
  const commitSha = (await peelTagToCommitSha(ownerRepo, actionRef))
    || (actionRef.startsWith('v') ? '' : await peelTagToCommitSha(ownerRepo, prefix));

  if (commitSha) {
    const matches = (await githubApi(ownerRepo, `/git/matching-refs/tags/${encodeURIComponent(`${prefix}.`)}`)) || [];
    const stableOnCommit = [];

    for (const entry of matches) {
      const name = tagNameFromRef(entry);
      if (!isStableSemverTag(name)) continue;

      const sha = await commitShaForRefEntry(ownerRepo, entry);
      if (sha === commitSha) {
        stableOnCommit.push(name);
      }
    }

    if (stableOnCommit.length > 0) {
      stableOnCommit.sort(compareStableSemverTags);
      return stableOnCommit[stableOnCommit.length - 1];
    }
  }

  const fallback = await latestStableReleaseForMajor(ownerRepo, prefix);
  if (fallback) {
    return fallback;
  }

  throw new Error(
    `Could not resolve moving tag ${actionRef} to a stable ${prefix}.x release of ${ownerRepo}`,
  );
}

function downloadStream(url, destination, redirectCount = 0) {
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
        downloadStream(nextUrl, destination, redirectCount + 1).then(resolve, reject);
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

  [
    'path',
    'main-branch',
    'release-branch',
    'feature-branch',
    'feature-branch-name-limit',
    'tag-prefix',
    'pre-release-tag',
    'patch-pre-release-tag',
    'commit-message-incrementing',
    'assembly-informational-format',
    'config',
  ].forEach((name) => {
    const value = getInput(name);
    if (value) args.push(`--${name}`, value);
  });

  [
    'continuous-delivery',
    'as-release',
    'show-config',
    'verbose',
  ].forEach((name) => {
    if (isTrue(getInput(name))) args.push(`--${name}`);
  });

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
  const releaseTag = await resolveReleaseTag(ownerRepo, actionRef);
  const url = releaseDownloadUrl(ownerRepo, releaseTag, assetName);
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), `${CLI_NAME}-`));
  const binary = path.join(tempDir, `${CLI_NAME}${executableSuffix}`);

  if (releaseTag && releaseTag !== actionRef) {
    info(`Resolved moving tag ${actionRef} to ${ownerRepo}@${releaseTag}`);
  }

  if (releaseTag) {
    info(`Downloading ${assetName} from ${ownerRepo}@${releaseTag}`);
  } else {
    info(`Downloading ${assetName} from the latest ${ownerRepo} release`);
  }

  await downloadStream(url, binary);

  if (process.platform !== 'win32') {
    fs.chmodSync(binary, 0o755);
  }

  const args = buildArgs();

  info(`Running ${binary} with args ${args.join(' ')}`);
  process.exitCode = await run(binary, args);
}

module.exports = {
  isMajorOnlyTag,
  isStableSemverTag,
  majorPrefix,
  compareStableSemverTags,
  resolveReleaseTag,
  releaseDownloadUrl,
  normalizeActionRef,
};

if (require.main === module) {
  main().catch((error) => {
    fail(error instanceof Error ? error.message : String(error));
  });
}
