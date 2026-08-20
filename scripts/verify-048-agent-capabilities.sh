#!/usr/bin/env bash
# Plan-048 acceptance: local CLI authentication discovery and live provider filtering.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

RUSTFLAGS='-D warnings' cargo run --manifest-path Cargo.toml --locked \
  -p relay-application --example plan048_agent_capability_acceptance --quiet
cargo run --quiet --locked -p relay-application --example plan045_tool_profiles_acceptance

if rg -n -- '--dangerously-skip-permissions|--dangerously-bypass-approvals-and-sandbox|--yolo|--no-sandbox|--api-key|--with-api-key' \
  packages/rust-tools/application/src/execution/agent_policy.rs; then
  echo 'plan048: provider invocation contains an unsafe bypass or API-key flag' >&2
  exit 1
fi

rg -n 'auth_probe_argv|auth-status|local login' \
  packages/rust-tools/application/src/execution/agent_capabilities.rs \
  packages/rust-tools/application/src/execution/agent_policy.rs >/dev/null

# The currently reviewed adapter must not regress to the removed legacy
# non-interactive shortcut; deterministic argv coverage above pins the safer
# workspace-write/automatic-review shape.
! rg -n -- '--full-auto' packages/rust-tools/application/src/execution/agent_policy.rs
echo 'plan048 authenticated CLI capability acceptance: PASS'
