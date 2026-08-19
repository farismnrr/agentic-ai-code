#!/usr/bin/env bash
# Deterministic acceptance test for Plan 044A TASK-003: Issue create, update, and comment mutations.
#
# The `test-gh-provider` feature is a non-default Cargo feature that enables the
# RELAY_TEST_GH_PATH override in forge_process::run_gh exclusively for this fixture.
# Ordinary debug builds, cargo test, and release builds do NOT enable it.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.."; pwd)"

# Proof 1: acceptance fixture works with the explicit test-only feature enabled.
RUSTFLAGS='-D warnings' cargo run --manifest-path "$root/Cargo.toml" --locked \
  --example plan044a_issue_mutations_acceptance \
  --features relay-application/test-gh-provider

# Proof 2: test-gh-provider is non-default — the example refuses to compile without it.
if cargo run --manifest-path "$root/Cargo.toml" --locked \
     --example plan044a_issue_mutations_acceptance 2>/dev/null; then
  echo "FAIL: example should require test-gh-provider feature but compiled without it"
  exit 1
fi
echo "test-gh-provider non-default: VERIFIED (mutations example unavailable without feature)"

echo "044A issue mutations acceptance: PASS"
