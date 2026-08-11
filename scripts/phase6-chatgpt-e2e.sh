#!/usr/bin/env bash
set -euo pipefail

# Phase 6 acceptance harness. Static checks always run; live ChatGPT evidence is
# opt-in because this repository cannot drive a user's ChatGPT workspace.
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
relay="$root/packages/rust-tools/src/relay_agent"

required=(
  "${relay}/mcp.rs"
  "${relay}/transport.rs"
  "${relay}/execution.rs"
  "${relay}/security.rs"
)
for file in "${required[@]}"; do test -f "$file"; done

# Behavioral protocol and security checks run in phase4-black-box.sh. Keep this
# gate limited to structural compatibility invariants that do not claim an HTTP
# behavior merely because a string exists in source.
! rg -q '(/sse|session_id|Mcp-Session-Id)' "${relay}/transport.rs"

if [[ -n "${PHASE6_MCP_URL:-}" ]]; then
  command -v curl >/dev/null
  curl --fail --silent --show-error \
    -H 'Accept: application/json' \
    "${PHASE6_MCP_URL%/}/.well-known/oauth-protected-resource" >/dev/null
else
  echo 'LIVE_CHATGPT_E2E=unavailable (set PHASE6_MCP_URL for deployment metadata probe)' >&2
fi

echo 'phase6 static acceptance: pass'
