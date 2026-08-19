#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
cargo run --locked -p relay-application --features test-gh-provider --example plan044b_actions_acceptance
if cargo run --locked -p relay-application --example plan044b_actions_acceptance >/dev/null 2>&1; then echo "044B acceptance example unexpectedly ran without test-gh-provider" >&2; exit 1; fi
for tool in workflow_list workflow_get workflow_run_list workflow_run_get workflow_run_jobs workflow_job_log_preview; do
  rg -q "\"$tool\"" packages/rust-tools/interfaces/src/mcp/catalog/forge.rs
  rg -q "\"$tool\"" packages/rust-tools/application/src/hooks/policy.rs
  rg -q "'$tool'" shared/utils/capability-policy.ts
done
! rg -q 'workflow_job_get|workflow_run_job_log' packages/rust-tools/interfaces/src/mcp/catalog/forge.rs
bash scripts/phase-039h-contract.sh
bash scripts/verify-044a-issue-contract.sh
echo "044B actions observability verification: PASS"
