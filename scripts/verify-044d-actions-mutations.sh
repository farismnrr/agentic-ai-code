#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"; cd "$root"
for t in workflow_dispatch workflow_run_rerun workflow_run_cancel; do
 rg -q "\"$t\"" packages/rust-tools/interfaces/src/mcp/catalog/forge.rs
 rg -q "$t" packages/rust-tools/application/src/hooks/policy.rs
 rg -q "$t" shared/utils/capability-policy.ts
done
! rg -q 'workflow_(enable|disable|delete)|artifact.*delete|cache.*delete|environment.*approve' packages/rust-tools/interfaces/src/mcp/catalog/forge.rs
cargo check --workspace --all-targets --all-features --locked
bash scripts/verify-044c-security-alerts.sh
echo '044D actions mutations deterministic verification: PASS'
