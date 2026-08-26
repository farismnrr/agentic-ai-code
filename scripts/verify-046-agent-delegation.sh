#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

bash scripts/phase-039h-contract.sh
test "$(jq 'length' .agents/contracts/039h-tool-catalog-v10.json)" = 101
test "$(jq 'length' .agents/contracts/039h-tool-catalog-v9.json)" = 100
cargo run --quiet --locked -p relay-application --example plan045_tool_profiles_acceptance
! rg -n 'agent_delegate|agent[-_]env|agent[-_]auth|allow[-_]agent[-_]network' \
  packages/rust-tools/application/src packages/rust-tools/core/src \
  packages/rust-tools/infrastructure/src packages/rust-tools/interfaces/src \
  packages/rust-tools/application/Cargo.toml
echo 'Plan 050 provider delegation removal verification: PASS'
