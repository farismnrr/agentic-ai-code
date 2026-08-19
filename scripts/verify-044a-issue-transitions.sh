#!/usr/bin/env bash
# Deterministic acceptance test for Plan 044A TASK-004 issue close/reopen transitions.
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.."; pwd)"
RUSTFLAGS='-D warnings' cargo run --manifest-path "$root/Cargo.toml" --locked \
  --example plan044a_issue_transitions_acceptance \
  --features relay-application/test-gh-provider
if cargo run --manifest-path "$root/Cargo.toml" --locked \
     --example plan044a_issue_transitions_acceptance 2>/dev/null; then
  echo "FAIL: example should require test-gh-provider feature but compiled without it"
  exit 1
fi
echo "test-gh-provider non-default: VERIFIED (transition example unavailable without feature)"
echo "044A issue transitions acceptance: PASS"
