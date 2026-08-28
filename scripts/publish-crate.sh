#!/usr/bin/env bash
# Publish mutanchor to crates.io.
#
# One-time setup: get a token from https://crates.io/settings/tokens
# and either:
#   1) `cargo login <TOKEN>` (writes ~/.cargo/credentials.toml), or
#   2) `export CARGO_REGISTRY_TOKEN=<TOKEN>` for this shell only.
#
# Then run:
#   scripts/publish-crate.sh
#
# The script verifies the crate packages cleanly (`cargo publish --dry-run`),
# runs the test suite one last time, and only then does the real publish.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

# 1. Verify tests are green.
echo "==> cargo test --all"
cargo test --all

# 2. Dry-run: package + verify build + compile the packaged crate.
echo "==> cargo publish --dry-run"
cargo publish --dry-run

# 3. Confirm before the irreversible real publish.
if [[ -z "${CARGO_REGISTRY_TOKEN:-}" ]]; then
  if [[ ! -f "$HOME/.cargo/credentials.toml" ]]; then
    echo "error: no crates.io token found." >&2
    echo "       run \`cargo login <TOKEN>\` or export CARGO_REGISTRY_TOKEN." >&2
    exit 1
  fi
fi

read -p "Publish mutanchor v$(grep '^version' Cargo.toml | head -1 | cut -d '"' -f 2) to crates.io? [y/N] " reply
if [[ "$reply" != "y" && "$reply" != "Y" ]]; then
  echo "aborted."
  exit 0
fi

echo "==> cargo publish"
cargo publish
echo "done. verify at https://crates.io/crates/mutanchor"
