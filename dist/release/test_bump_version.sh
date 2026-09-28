#!/usr/bin/env bash
# Tests for bump_version.sh. Builds a throwaway fixture tree, bumps it, asserts.
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
BUMP="$SCRIPT_DIR/bump_version.sh"
FAILURES=0

fail() {
    echo "FAIL: $*" >&2
    FAILURES=$((FAILURES + 1))
}

assert_contains() {
    local file="$1" needle="$2"
    if ! grep -qF -- "$needle" "$file"; then
        fail "expected '$needle' in $file, got:"
        cat "$file" >&2
    fi
}

assert_absent() {
    local file="$1" needle="$2"
    if grep -qF -- "$needle" "$file"; then
        fail "did not expect '$needle' in $file"
    fi
}

make_fixture() {
    local root="$1"
    mkdir -p "$root"
    cat > "$root/Cargo.toml" <<'FIXTURE'
[package]
name = "cucco"
version = "3.4.0"
edition = "2021"
rust-version = "1.98.1"

[dependencies]
clap = { version = "4", features = ["derive"] }
cocogitto = { version = "7", default-features = false }
FIXTURE
}

# -- valid bump --------------------------------------------------------------
ROOT="$(mktemp -d)"
trap 'rm -rf "$ROOT"' EXIT
make_fixture "$ROOT"
"$BUMP" 4.0.0 "$ROOT" > /dev/null

assert_contains "$ROOT/Cargo.toml" 'version = "4.0.0"'
assert_absent   "$ROOT/Cargo.toml" 'version = "3.4.0"'
# unrelated version keys must be untouched
assert_contains "$ROOT/Cargo.toml" 'rust-version = "1.98.1"'
assert_contains "$ROOT/Cargo.toml" 'clap = { version = "4", features = ["derive"] }'
assert_contains "$ROOT/Cargo.toml" 'cocogitto = { version = "7", default-features = false }'

# -- idempotency -------------------------------------------------------------
"$BUMP" 4.0.0 "$ROOT" > /dev/null
assert_contains "$ROOT/Cargo.toml" 'version = "4.0.0"'

# -- invalid versions are rejected -------------------------------------------
for bad in "v1.2.3" "1.2" "1.2.3-rc1" "01.2.3" "" "1.2.3.4"; do
    if "$BUMP" "$bad" "$ROOT" > /dev/null 2>&1; then
        fail "expected rejection of version '$bad'"
    fi
done

if [ "$FAILURES" -eq 0 ]; then
    echo "OK: all bump_version tests passed"
else
    echo "$FAILURES test(s) failed" >&2
    exit 1
fi
