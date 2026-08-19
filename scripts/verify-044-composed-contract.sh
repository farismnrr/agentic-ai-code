#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"; cd "$root"
bash scripts/phase-039h-contract.sh
jq -e 'length == 100 and ([.[].name] | unique | length) == 100' .agents/contracts/039h-tool-catalog-v9.json >/dev/null
for t in issue_list issue_get issue_create issue_update issue_comment issue_close issue_reopen workflow_list workflow_get workflow_run_list workflow_run_get workflow_run_jobs workflow_job_log_preview dependabot_alert_list dependabot_alert_get code_scanning_alert_list code_scanning_alert_get secret_scanning_alert_list secret_scanning_alert_get secret_scanning_alert_locations workflow_dispatch workflow_run_rerun workflow_run_cancel; do jq -e --arg t "$t" '([.[]|select(.name==$t)]|length)==1' .agents/contracts/039h-tool-catalog-v9.json >/dev/null; done
! jq -r '.[].name' .agents/contracts/039h-tool-catalog-v9.json | grep -Eq '^(github_api|gh|api)$'
echo '044 composed v9 contract: PASS'
