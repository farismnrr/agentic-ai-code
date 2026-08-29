#!/usr/bin/env bash
# Historical Plan-029/Plan-039B frozen contract regression check.
#
# This gate is immutable evidence: it proves the Plan-029 (v1, 12 tools) and
# Plan-039B (v2, 18 tools) frozen catalogs on disk have not been altered
# since they were recorded. It intentionally does NOT compare either frozen
# catalog against the live runtime tool list any more — the runtime catalog
# has grown past both historical snapshots (Plan-039C added code_* tools),
# so that comparison would fail by design, not by regression. The current
# runtime catalog is verified separately by scripts/phase-039h-contract.sh;
# Plan-039C v3 is retained as historical evidence by phase-039c-contract.sh.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
catalog="$root/.agents/contracts/039b-tool-catalog-v2.json"
catalog_hash_file="$root/.agents/contracts/039b-tool-catalog-v2.sha256"
legacy_catalog="$root/.agents/contracts/029-tool-catalog-v1.json"
legacy_hash_file="$root/.agents/contracts/029-tool-catalog-v1.sha256"

test -f "$catalog"
test -f "$catalog_hash_file"
test -f "$legacy_catalog"
test -f "$legacy_hash_file"
command -v jq >/dev/null
command -v sha256sum >/dev/null

jq -e 'type == "array" and all(.[]; (.name and .description and .inputSchema and .annotations and (.securitySchemes == [{"type":"oauth2","scopes":["relay.coding"]}]) and (has("security") | not)))' "$catalog" >/dev/null
test "$(jq -r '.[].name' "$catalog" | paste -sd' ' -)" = "terminal_exec http_fetch web_search directory_list file_search file_write file_edit file_read text_search git_status git_diff git_log git_show git_blame apply_patch terminal_job_start terminal_job_get terminal_job_cancel"
tools="$(jq -S -c . "$catalog")"
hash="$(printf '%s' "$tools" | sha256sum | awk '{print $1}')"
test "$hash" = "$(tr -d '[:space:]' < "$catalog_hash_file")"

legacy_tools="$(jq -S -c . "$legacy_catalog")"
legacy_hash="$(printf '%s' "$legacy_tools" | sha256sum | awk '{print $1}')"
test "$legacy_hash" = "$(tr -d '[:space:]' < "$legacy_hash_file")"

echo "phase7 historical contract regression: pass (029=$legacy_hash 039b=$hash)"
