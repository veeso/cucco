#!/usr/bin/env sh
# Builds a static musl cucco release binary for the given target triple
# inside a pinned Alpine container.
#
# Usage: dist/release/build_musl.sh <target-triple>
set -eu

# Keep the Rust version equal to rust-toolchain.toml so rustup inside the
# container does not try to download another toolchain.
IMAGE="rust:1.98.1-alpine3.22"

TARGET="${1:-}"
if [ -z "$TARGET" ]; then
  echo "usage: $0 <target-triple>" >&2
  exit 2
fi

case "$TARGET" in
  x86_64-unknown-linux-musl) PLATFORM="linux/amd64" ;;
  aarch64-unknown-linux-musl) PLATFORM="linux/arm64" ;;
  *)
    echo "unsupported target: $TARGET" >&2
    exit 2
    ;;
esac

WORKSPACE="$(CDPATH='' cd -- "$(dirname -- "$0")/../.." && pwd)"

HOST_UID="$(id -u)"
HOST_GID="$(id -g)"
export TARGET HOST_UID HOST_GID

docker run --rm \
  --platform "$PLATFORM" \
  --env TARGET \
  --env HOST_UID \
  --env HOST_GID \
  --volume "$WORKSPACE:/work" \
  --workdir /work \
  "$IMAGE" \
  sh /work/dist/release/build_musl_container.sh
