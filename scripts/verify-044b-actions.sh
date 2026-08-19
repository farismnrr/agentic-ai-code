#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
cargo run --locked -p relay-application --features test-gh-provider --example plan044b_actions_acceptance
if cargo run --locked -p relay-application --example plan044b_actions_acceptance >/dev/null 2>&1; then
  echo "044B acceptance example unexpectedly ran without test-gh-provider" >&2; exit 1
fi
rg -q '"workflow_run_job_log"' packages/rust-tools/interfaces/src/mcp/catalog/forge.rs
rg -q 'workflow_run_job_log.*network_read' packages/rust-tools/application/src/hooks/policy.rs
rg -q "workflow_run_job_log" shared/utils/capability-policy.ts
bash scripts/phase-039h-contract.sh
echo "044B actions verification: PASS"
