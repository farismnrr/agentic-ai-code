#!/usr/bin/env bash
# Deterministic acceptance test for Plan 044A TASK-002: Bounded GitHub issue reads.
#
# The `test-gh-provider` feature is a non-default Cargo feature that enables the
# RELAY_TEST_GH_PATH override in forge_process::run_gh exclusively for this fixture.
# Ordinary debug builds, cargo test, and release builds do NOT enable it.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.."; pwd)"

# Proof 1: acceptance fixture works with the explicit test-only feature enabled.
RUSTFLAGS='-D warnings' cargo run --manifest-path "$root/Cargo.toml" --locked \
  --example plan044a_issue_reads_acceptance \
  --features relay-application/test-gh-provider

# Proof 2: test-gh-provider is non-default — the example refuses to compile without it.
if cargo run --manifest-path "$root/Cargo.toml" --locked \
     --example plan044a_issue_reads_acceptance 2>/dev/null; then
  echo "FAIL: example should require test-gh-provider feature but compiled without it"
  exit 1
fi
echo "test-gh-provider non-default: VERIFIED (example unavailable without feature)"

# Proof 3: ordinary default debug build never contains RELAY_TEST_GH_PATH.
# Build the default (no features) library and inspect the canonical output rlib.
RUSTFLAGS='-D warnings' cargo build --manifest-path "$root/Cargo.toml" --locked \
  -p relay-application --lib 2>/dev/null
if strings "$root/target/debug/librelay_application.rlib" | grep -q 'RELAY_TEST_GH_PATH'; then
  echo "FAIL: RELAY_TEST_GH_PATH found in default debug rlib"
  exit 1
fi
echo "RELAY_TEST_GH_PATH absent in default debug build: VERIFIED"

echo "044A issue reads acceptance: PASS"
