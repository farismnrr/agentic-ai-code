#!/usr/bin/env bash
# Plan-048 acceptance: local CLI authentication discovery and live provider filtering.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

RUSTFLAGS='-D warnings' cargo run --manifest-path Cargo.toml --locked \
  -p relay-application --example plan048_agent_capability_acceptance --quiet

if rg -n -- '--dangerously-skip-permissions|--yolo|--no-sandbox|--api-key|--with-api-key' \
  packages/rust-tools/application/src/execution/agent_policy.rs; then
  echo 'plan048: provider invocation contains an unsafe bypass or API-key flag' >&2
  exit 1
fi

rg -n 'auth_probe_argv|auth-status|local login' \
  packages/rust-tools/application/src/execution/agent_capabilities.rs \
  packages/rust-tools/application/src/execution/agent_policy.rs >/dev/null

echo 'plan048 authenticated CLI capability acceptance: PASS'
