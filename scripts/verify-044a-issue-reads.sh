#!/usr/bin/env bash
# Deterministic acceptance test for Plan 044A TASK-002: Bounded GitHub issue reads.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

RUSTFLAGS='-D warnings' cargo run --manifest-path "$root/Cargo.toml" --locked --example plan044a_issue_reads_acceptance

echo "044A issue reads acceptance: PASS"
