#!/usr/bin/env bash
set -euo pipefail

if [[ "${RELAY_TOOL_PROFILE:-}" != primary ]]; then
  echo "expected RELAY_TOOL_PROFILE=primary, got ${RELAY_TOOL_PROFILE:-unset}" >&2
  exit 1
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
expected_args=(
  relay
  --mode remote
  --trusted-proxy
  --trusted-proxy-cidr 127.0.0.1/32
  --port 47899
  --dir "$repo_root"
  --execution-root "$repo_root"
  --oauth-issuer https://auth.example.com/
  --oauth-audience https://mcp.example.com/mcp
  --oauth-owner-subject plan045-test-subject
)
actual_args=("$@")
if (( ${#actual_args[@]} != ${#expected_args[@]} )); then
  echo 'unexpected remote relay argument count' >&2
  exit 1
fi

for index in "${!expected_args[@]}"; do
  if [[ "${actual_args[$index]}" != "${expected_args[$index]}" ]]; then
    echo "unexpected remote relay argument at index $index" >&2
    exit 1
  fi
done

echo '045 relay deployment wrapper acceptance: PASS profile=primary'
