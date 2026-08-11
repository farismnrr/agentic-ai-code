#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
catalog="$root/.agents/contracts/029-tool-catalog-v1.json"
relay="$root/packages/rust-tools/src/relay_agent/mcp.rs"
test -f "$catalog"; command -v jq >/dev/null; command -v sha256sum >/dev/null
expected='terminal_exec http_fetch web_search'
actual="$(jq -r '.tools[].name' "$catalog" | paste -sd' ' -)"
test "$actual" = "$expected"
for tool in $expected; do rg -q "name: \"$tool\"" "$relay"; done
hash="$(jq -S -c . "$catalog" | sha256sum | awk '{print $1}')"
recorded="$(sed -n 's/^catalogSha256: `\([^`]*\)`.*/\1/p' "$root/.agents/memories/029-phase7-published-app-lifecycle.md")"
test "$hash" = "$recorded"
jq -e 'all(.tools[]; (.name and (.required|type == "array") and (.security|type == "object") and (.security|has("readOnly") and has("destructive") and has("idempotent") and has("openWorld"))))' "$catalog" >/dev/null
echo "phase7 contract acceptance: pass ($hash)"
