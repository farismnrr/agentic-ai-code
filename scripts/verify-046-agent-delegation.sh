#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

cargo run --quiet --locked -p relay-application --example plan046_agent_delegation_acceptance
bash scripts/phase-039h-contract.sh
jq -e 'any(.[]; .name == "agent_delegate" and .execution.taskSupport == "optional")' \
  .agents/contracts/039h-tool-catalog-v10.json >/dev/null
test "$(jq 'length' .agents/contracts/039h-tool-catalog-v10.json)" = 101
test "$(jq 'length' .agents/contracts/039h-tool-catalog-v9.json)" = 100
cargo run --quiet --locked -p relay-application --example plan045_tool_profiles_acceptance
! rg -n -- '--full-auto' packages/rust-tools/application/src/execution/agent_policy.rs
echo '046 agent delegation deterministic verification: PASS'
