#!/usr/bin/env bash
# Plan-047 acceptance: incremental file edits, contract rotation, and neutral docs.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

test -f docs/mcp-client.md
test -f docs/oauth-provider.md
test ! -e docs/chatgpt.md
test ! -e docs/keycloak.md
if rg -n -i 'chatgpt|openai|claude|codex|antigravity|gemini|cloudflare|keycloak|auth0|github|ghcr' docs --glob '*.md'; then
  echo 'plan047 docs contain a client/provider-specific reference' >&2
  exit 1
fi

bash scripts/verify-file-edit.sh
bash scripts/verify-workspace-docs.sh
bash scripts/phase-039h-contract.sh

python3 - "$root" <<'PY'
import json
import sys

root = sys.argv[1]
with open(f'{root}/.agents/contracts/039h-tool-catalog-v10.json', encoding='utf-8') as handle:
    v10 = json.load(handle)
with open(f'{root}/.agents/contracts/039h-tool-catalog-v11.json', encoding='utf-8') as handle:
    v11 = json.load(handle)
assert len(v10) == len(v11) == 101
old_file_edit = next(tool for tool in v10 if tool['name'] == 'file_edit')
new_file_edit = next(tool for tool in v11 if tool['name'] == 'file_edit')
assert 'edits' not in old_file_edit['inputSchema']['properties']
assert 'edits' in new_file_edit['inputSchema']['properties']
assert any(tool['name'] == 'agent_delegate' for tool in v10)
assert any(tool['name'] == 'agent_delegate' for tool in v11)
print('plan047 editing/docs acceptance: PASS')
PY
