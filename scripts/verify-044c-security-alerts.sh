#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"; cd "$root"
for t in dependabot_alert_list dependabot_alert_get code_scanning_alert_list code_scanning_alert_get secret_scanning_alert_list secret_scanning_alert_get secret_scanning_alert_locations; do
  rg -q "\"$t\"" packages/rust-tools/interfaces/src/mcp/catalog/forge.rs
  rg -q "$t" packages/rust-tools/application/src/hooks/policy.rs
  rg -q "$t" shared/utils/capability-policy.ts
done
rg -q 'hide_secret.*true' packages/rust-tools/application/src/git/forge/security.rs
if rg -n 'pub\(in crate::git\) struct SecretAlert.*secret:' packages/rust-tools/application/src/git/forge/security.rs; then exit 1; fi
! rg -q 'github_api|arbitrary.*api' packages/rust-tools/interfaces/src/mcp/catalog/forge.rs
cargo check --workspace --all-targets --all-features --locked
bash scripts/verify-044b-actions-observability.sh
bash scripts/verify-044a-issue-contract.sh
echo '044C security alerts verification: PASS'
