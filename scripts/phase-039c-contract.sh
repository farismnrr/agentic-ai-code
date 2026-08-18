#!/usr/bin/env bash
# Plan-039C frozen MCP tool contract integrity acceptance.
#
# This gate verifies the Plan-039C v3 snapshot as immutable historical
# evidence. The current post-039H runtime is checked by
# scripts/phase-039h-contract.sh.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
catalog="$root/.agents/contracts/039c-tool-catalog-v3.json"
catalog_hash_file="$root/.agents/contracts/039c-tool-catalog-v3.sha256"

test -f "$catalog"
test -f "$catalog_hash_file"
command -v jq >/dev/null
command -v sha256sum >/dev/null
jq -e 'type == "array" and all(.[]; (.name and .description and .inputSchema and .annotations and (.securitySchemes == [{"type":"oauth2","scopes":["relay.coding"]}]) and (has("security") | not)))' "$catalog" >/dev/null
test "$(jq '. | length' "$catalog")" = "25"
test "$(jq -r '.[].name' "$catalog" | paste -sd' ' -)" = "terminal_exec http_fetch web_search directory_list file_search file_write file_edit file_read text_search git_status git_diff git_log git_show git_blame apply_patch code_symbols code_definition code_references code_implementations code_hover code_diagnostics code_rename_preview terminal_job_start terminal_job_get terminal_job_cancel"

frozen_tools="$(jq -S -c . "$catalog")"
frozen_hash="$(printf '%s' "$frozen_tools" | sha256sum | awk '{print $1}')"
test "$frozen_hash" = "$(tr -d '[:space:]' < "$catalog_hash_file")"

echo "phase-039c historical contract acceptance: pass ($frozen_hash)"
