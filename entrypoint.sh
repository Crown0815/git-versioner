#!/bin/sh -l

set -e

# Construct arguments array
ARGS=""

if [ -n "$INPUT_PATH" ]; then
  ARGS="$ARGS --path $INPUT_PATH"
fi

if [ -n "$INPUT_MAIN_BRANCH" ]; then
  ARGS="$ARGS --main-branch $INPUT_MAIN_BRANCH"
fi

if [ -n "$INPUT_RELEASE_BRANCH" ]; then
  ARGS="$ARGS --release-branch $INPUT_RELEASE_BRANCH"
fi

if [ -n "$INPUT_FEATURE_BRANCH" ]; then
  ARGS="$ARGS --feature-branch $INPUT_FEATURE_BRANCH"
fi

if [ -n "$INPUT_TAG_PREFIX" ]; then
  ARGS="$ARGS --tag-prefix $INPUT_TAG_PREFIX"
fi

if [ -n "$INPUT_PRE_RELEASE_TAG" ]; then
  ARGS="$ARGS --pre-release-tag $INPUT_PRE_RELEASE_TAG"
fi

if [ "$INPUT_CONTINUOUS_DELIVERY" = "true" ]; then
  ARGS="$ARGS --continuous-delivery"
fi

if [ -n "$INPUT_COMMIT_MESSAGE_INCREMENTING" ]; then
  ARGS="$ARGS --commit-message-incrementing $INPUT_COMMIT_MESSAGE_INCREMENTING"
fi

if [ "$INPUT_AS_RELEASE" = "true" ]; then
  ARGS="$ARGS --as-release"
fi

if [ -n "$INPUT_CONFIG" ]; then
  ARGS="$ARGS --config $INPUT_CONFIG"
fi

# Run the tool
# shellcheck disable=SC2086
git-versioner $ARGS
