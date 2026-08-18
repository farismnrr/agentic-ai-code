#!/usr/bin/env bash
set -euo pipefail
root=$(git rev-parse --show-toplevel)
contract="$root/.agents/contracts/039i-mcp-surface-v5.json"
hash_file="$root/.agents/contracts/039i-mcp-surface-v5.sha256"
test -f "$contract" -a -f "$hash_file"
actual=$(sha256sum "$contract" | awk '{print $1}')
expected=$(tr -d '[:space:]' < "$hash_file")
test "$actual" = "$expected"
test "$(python3 - "$contract" <<'PY'
import json, sys
v=json.load(open(sys.argv[1]))
assert v['protocolVersion'] == '2026-07-28'
assert v['capabilities'] == {'resources': {}}
assert v['templates'] is False and v['subscriptions'] is False
assert v['methods']['resources/list']['cacheScope'] == 'private'
assert v['methods']['resources/read']['ttlMs'] == 0
print('ok')
PY
)" = ok
tmp=$(mktemp)
trap 'rm -f "$tmp"' EXIT
cp "$contract" "$tmp"
python3 - "$tmp" <<'PY'
import json, sys
p=sys.argv[1]
v=json.load(open(p)); v['subscriptions']=True
open(p,'w').write(json.dumps(v, separators=(',', ':'))+'\n')
PY
test "$(sha256sum "$tmp" | awk '{print $1}')" != "$expected"
echo "phase-039i current contract acceptance: pass ($expected)"
