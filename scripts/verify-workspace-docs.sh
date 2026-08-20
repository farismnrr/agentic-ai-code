#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
catalog="$root/.agents/contracts/029-tool-catalog-v1.json"
client_docs=(
  "$root/docs/mcp-client.md"
  "$root/docs/getting-started.md"
  "$root/docs/configuration.md"
)
rust_readme="$root/packages/rust-tools/README.md"
root_readme="$root/README.md"

command -v jq >/dev/null
for name in directory_list file_search text_search file_read file_edit file_write terminal_exec http_fetch web_search terminal_job_start terminal_job_get terminal_job_cancel; do
  jq -e --arg name "$name" 'any(.[]; .name == $name)' "$catalog" >/dev/null
  rg -q "${name}" "${client_docs[@]}"
done

test "$(jq 'length' "$catalog")" = "12"
for name in directory_list file_search text_search file_read file_edit file_write; do
  rg -q "${name}" "$root_readme"
  rg -q "${name}" "$rust_readme"
done

# Defaults/hard caps documented for the agent-facing contracts must match the frozen runtime catalog.
test "$(jq -r '.[] | select(.name=="directory_list") | .inputSchema.properties.depth.default' "$catalog")" = "2"
test "$(jq -r '.[] | select(.name=="directory_list") | .inputSchema.properties.depth.maximum' "$catalog")" = "4"
test "$(jq -r '.[] | select(.name=="file_search") | .inputSchema.properties.max_results.default' "$catalog")" = "100"
test "$(jq -r '.[] | select(.name=="text_search") | .inputSchema.properties.regex.default' "$catalog")" = "false"
test "$(jq -r '.[] | select(.name=="text_search") | .inputSchema.properties.case_sensitive.default' "$catalog")" = "true"
test "$(jq -r '.[] | select(.name=="text_search") | .inputSchema.properties.max_results.default' "$catalog")" = "50"
test "$(jq -r '.[] | select(.name=="file_read") | .inputSchema.properties.offset_line.default' "$catalog")" = "1"
test "$(jq -r '.[] | select(.name=="file_read") | .inputSchema.properties.limit_lines.default' "$catalog")" = "200"
test "$(jq -r '.[] | select(.name=="file_read") | .inputSchema.properties.limit_lines.maximum' "$catalog")" = "1000"
test "$(jq -r '.[] | select(.name=="file_edit") | .inputSchema.properties.replace_all.default' "$catalog")" = "false"
test "$(jq -r '.[] | select(.name=="file_write") | .inputSchema.properties.create_parents.default' "$catalog")" = "false"
test "$(jq -r '.[] | select(.name=="file_write") | .inputSchema.properties.overwrite.default' "$catalog")" = "false"

# Prevent the pre-Workspace-v1 catalog count from creeping back into user-facing docs.
! rg -n 'current relay exposes six tools|healthy current relay exposes six tools' "$root/docs" "$root_readme" "$rust_readme"

echo "workspace documentation acceptance: PASS"
