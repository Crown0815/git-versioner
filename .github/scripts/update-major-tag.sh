#!/usr/bin/env bash
# Move one or more major tags (v0, v1, …) to the commit of an existing release tag.
#
# Usage:
#   update-major-tag.sh --to-release <release-tag> <major-tag> [<major-tag>...]
#
# Environment:
#   GITHUB_REPOSITORY  owner/repo (set automatically on GitHub Actions)
#   GH_TOKEN           token with contents: write (use the workflow GITHUB_TOKEN)
#
# The script is idempotent: if a major tag already points at the target commit,
# it is left unchanged.

set -euo pipefail

usage() {
  echo "Usage: $0 --to-release <release-tag> <major-tag> [<major-tag>...]" >&2
  exit 2
}

if [[ "${1:-}" != "--to-release" ]] || [[ $# -lt 3 ]]; then
  usage
fi

RELEASE_TAG="$2"
shift 2
MAJOR_TAGS=("$@")

is_release_tag() {
  [[ "$1" =~ ^v?[0-9]+\.[0-9]+\.[0-9]+([+-][0-9A-Za-z.-]+)*$ ]]
}

is_major_tag() {
  [[ "$1" =~ ^v[0-9]+$ ]]
}

is_prerelease_tag() {
  [[ "$1" =~ ^v?[0-9]+\.[0-9]+\.[0-9]+[-+] ]]
}

if ! is_release_tag "$RELEASE_TAG"; then
  echo "::error::Invalid release tag '$RELEASE_TAG' (expected vMAJOR.MINOR.PATCH)" >&2
  exit 1
fi

if is_prerelease_tag "$RELEASE_TAG"; then
  echo "::error::Refusing to move a major tag to pre-release '$RELEASE_TAG'" >&2
  exit 1
fi

for tag in "${MAJOR_TAGS[@]}"; do
  if ! is_major_tag "$tag"; then
    echo "::error::Refusing to update '$tag' (major tags must match v<MAJOR>, e.g. v1)" >&2
    exit 1
  fi
done

REPO="${GITHUB_REPOSITORY:?GITHUB_REPOSITORY is required}"

# Returns the commit SHA a tag points at, peeling annotated tags.
peel_to_commit() {
  local tag="$1"
  local type sha

  type="$(gh api "repos/${REPO}/git/ref/tags/${tag}" --jq .object.type)"
  sha="$(gh api "repos/${REPO}/git/ref/tags/${tag}" --jq .object.sha)"

  if [[ "$type" == "tag" ]]; then
    sha="$(gh api "repos/${REPO}/git/tags/${sha}" --jq .object.sha)"
  fi

  printf '%s' "$sha"
}

# Prints the commit SHA of an existing major tag, or prints nothing if missing.
existing_major_commit() {
  local tag="$1"
  local type sha

  if ! type="$(gh api "repos/${REPO}/git/ref/tags/${tag}" --jq .object.type 2>/dev/null)"; then
    return 0
  fi

  sha="$(gh api "repos/${REPO}/git/ref/tags/${tag}" --jq .object.sha)"

  if [[ "$type" == "tag" ]]; then
    sha="$(gh api "repos/${REPO}/git/tags/${sha}" --jq .object.sha)"
  fi

  printf '%s' "$sha"
}

TARGET_SHA="$(peel_to_commit "$RELEASE_TAG")"
if [[ -z "$TARGET_SHA" ]]; then
  echo "::error::Could not resolve commit for release tag '$RELEASE_TAG'" >&2
  exit 1
fi

echo "Target release ${RELEASE_TAG} is at ${TARGET_SHA}"

for tag in "${MAJOR_TAGS[@]}"; do
  current="$(existing_major_commit "$tag")"
  if [[ "$current" == "$TARGET_SHA" ]]; then
    echo "Tag ${tag} already points at ${TARGET_SHA}; leaving it unchanged"
    continue
  fi

  if [[ -n "$current" ]]; then
    gh api --method PATCH "repos/${REPO}/git/refs/tags/${tag}" \
      -f sha="$TARGET_SHA" \
      -F force=true >/dev/null
    echo "Moved ${tag} from ${current} to ${TARGET_SHA} (${RELEASE_TAG})"
  else
    gh api --method POST "repos/${REPO}/git/refs" \
      -f ref="refs/tags/${tag}" \
      -f sha="$TARGET_SHA" >/dev/null
    echo "Created ${tag} at ${TARGET_SHA} (${RELEASE_TAG})"
  fi
done
