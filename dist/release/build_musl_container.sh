#!/usr/bin/env sh
# Builds a static musl cucco binary. Runs INSIDE the Alpine container
# started by dist/release/build_musl.sh; /work is the mounted workspace.
#
# Required environment: TARGET, HOST_UID, HOST_GID.
set -eux

cleanup() {
  chown -R "$HOST_UID:$HOST_GID" /work
}
trap cleanup EXIT

# build-base, perl and pkgconf are what libgit2-sys, libz-sys and the
# tree-sitter grammars need to compile their vendored C sources.
apk add --no-cache \
  binutils \
  build-base \
  file \
  git \
  perl \
  pkgconf

rustup target add "$TARGET"
cargo fetch --locked

export RUSTFLAGS="-C target-feature=+crt-static"
cargo build --locked --release --target "$TARGET"

# -- prove the binary is static: no interpreter, no shared libraries
file "target/$TARGET/release/cucco"
readelf -l "target/$TARGET/release/cucco" > /tmp/program-headers.txt
cat /tmp/program-headers.txt
readelf -d "target/$TARGET/release/cucco" > /tmp/dynamic-section.txt
cat /tmp/dynamic-section.txt
if grep -q INTERP /tmp/program-headers.txt; then
  echo "binary has a program interpreter; it is not static" >&2
  exit 1
fi
if grep -q NEEDED /tmp/dynamic-section.txt; then
  echo "binary needs shared libraries; it is not static" >&2
  exit 1
fi
