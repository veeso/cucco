#!/usr/bin/env bash
# Bump the package version in Cargo.toml.
# Usage: bump_version.sh <version> [root]
#
# Text substitution only. Cargo.lock is refreshed separately with
# `cargo update --workspace`, so this script stays testable without a registry.
set -euo pipefail

VERSION="${1:?usage: bump_version.sh <version> [root]}"
if [[ ! "$VERSION" =~ ^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$ ]]; then
    echo "invalid release version: $VERSION (expected MAJOR.MINOR.PATCH)" >&2
    exit 2
fi
ROOT="${2:-$(git rev-parse --show-toplevel)}"

# in-place substitution that works on both GNU and BSD/macOS
sedi() { perl -0777 -pi -e "$1" "$2"; }

# Only the first line-anchored `version = "..."` is the package version. The
# `version` keys inside dependency inline tables never start a line, and
# `rust-version` does not match `^version`.
sedi "s/^version = \"[0-9][0-9A-Za-z.\\-]*\"/version = \"$VERSION\"/m" "$ROOT/Cargo.toml"

echo "Bumped to $VERSION"
