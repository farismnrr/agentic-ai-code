#!/usr/bin/env bash
# Current Plan-039H MCP contract gate.
#
# The v8 snapshot is captured from the candidate relay tools/list path. Historical
# v1-v7 artifacts remain immutable; v8 records Plan 043 workspace/worktree,
# broader structured Git, and timing-era catalog changes without rewriting history.
# Composed v9 canonical freeze is deferred to Plan 044D.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
catalog="$root/.agents/contracts/039h-tool-catalog-v8.json"
catalog_hash_file="$root/.agents/contracts/039h-tool-catalog-v8.sha256"
historical_v7="$root/.agents/contracts/039h-tool-catalog-v7.json"
historical_v7_hash="$root/.agents/contracts/039h-tool-catalog-v7.sha256"
historical_v6="$root/.agents/contracts/039h-tool-catalog-v6.json"
historical_v6_hash="$root/.agents/contracts/039h-tool-catalog-v6.sha256"
historical_v5="$root/.agents/contracts/039h-tool-catalog-v5.json"
historical_v5_hash="$root/.agents/contracts/039h-tool-catalog-v5.sha256"
historical_v4="$root/.agents/contracts/039h-tool-catalog-v4.json"
historical_v4_hash="$root/.agents/contracts/039h-tool-catalog-v4.sha256"
historical_v3="$root/.agents/contracts/039c-tool-catalog-v3.json"
historical_v3_hash="$root/.agents/contracts/039c-tool-catalog-v3.sha256"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

test -f "$catalog"
test -f "$catalog_hash_file"
test -f "$historical_v7"
test -f "$historical_v7_hash"
test -f "$historical_v6"
test -f "$historical_v6_hash"
test -f "$historical_v5"
test -f "$historical_v5_hash"
test -f "$historical_v4"
test -f "$historical_v4_hash"
test -f "$historical_v3"
test -f "$historical_v3_hash"
command -v jq >/dev/null
command -v sha256sum >/dev/null

validate_snapshot_integrity() {
  local snapshot="$1" hash_file="$2"
  jq -e 'type == "array" and all(.[]; (.name and .description and .inputSchema and .annotations and (.securitySchemes == [{"type":"oauth2","scopes":["relay.coding"]}]) and (has("security") | not)))' "$snapshot" >/dev/null
  local expected actual
  expected="$(jq -S -c . "$snapshot")"
  actual="$(printf '%s' "$expected" | sha256sum | awk '{print $1}')"
  if [[ "$actual" != "$(tr -d '[:space:]' < "$hash_file")" ]]; then return 1; fi
}

# Historical v3/v4/v5/v6/v7 integrity is checked independently of the live runtime.
v7_normalized="$(jq -S -c . "$historical_v7")"
v7_hash="$(printf '%s' "$v7_normalized" | sha256sum | awk '{print $1}')"
test "$v7_hash" = "$(tr -d '[:space:]' < "$historical_v7_hash")"
v6_normalized="$(jq -S -c . "$historical_v6")"
v6_hash="$(printf '%s' "$v6_normalized" | sha256sum | awk '{print $1}')"
test "$v6_hash" = "$(tr -d '[:space:]' < "$historical_v6_hash")"
v5_normalized="$(jq -S -c . "$historical_v5")"
v5_hash="$(printf '%s' "$v5_normalized" | sha256sum | awk '{print $1}')"
test "$v5_hash" = "$(tr -d '[:space:]' < "$historical_v5_hash")"
v4_normalized="$(jq -S -c . "$historical_v4")"
v4_hash="$(printf '%s' "$v4_normalized" | sha256sum | awk '{print $1}')"
test "$v4_hash" = "$(tr -d '[:space:]' < "$historical_v4_hash")"
v3_normalized="$(jq -S -c . "$historical_v3")"
v3_hash="$(printf '%s' "$v3_normalized" | sha256sum | awk '{print $1}')"
test "$v3_hash" = "$(tr -d '[:space:]' < "$historical_v3_hash")"

RUSTFLAGS='-D warnings' cargo build --manifest-path "$root/Cargo.toml" --locked --bin ai-tools >/dev/null
python3 - "$root/target/debug/ai-tools" "$tmp/runtime.json" "$root" <<'PY'
import json, socket, subprocess, sys, tempfile, time, urllib.error, urllib.request
relay, output_path, root = sys.argv[1:]
def free_port():
    with socket.socket() as sock:
        sock.bind(('127.0.0.1', 0)); return sock.getsockname()[1]
with tempfile.TemporaryDirectory(prefix='relay-phase039h-') as workspace:
    port = free_port()
    process = subprocess.Popen([relay, 'relay', '--port', str(port), '--dir', workspace,
                                '--execution-root', workspace, '--origin', 'http://localhost:3333', '--mode', 'local'],
                               cwd=root, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    try:
        for _ in range(100):
            if process.poll() is not None: raise RuntimeError(process.stderr.read().strip())
            try:
                with urllib.request.urlopen(f'http://127.0.0.1:{port}/health', timeout=1) as response:
                    if response.status == 200: break
            except (urllib.error.URLError, ConnectionError): time.sleep(.1)
        else: raise RuntimeError('relay did not become healthy')
        body = {'jsonrpc':'2.0','id':1,'method':'tools/list','params':{'_meta':{
            'io.modelcontextprotocol/protocolVersion':'2026-07-28','io.modelcontextprotocol/clientCapabilities':{}}}}
        request = urllib.request.Request(f'http://127.0.0.1:{port}/mcp', data=json.dumps(body).encode(),
            headers={'Content-Type':'application/json','Origin':'http://localhost:3333',
                     'MCP-Protocol-Version':'2026-07-28','Mcp-Method':'tools/list'}, method='POST')
        with urllib.request.urlopen(request, timeout=5) as response:
            result = json.loads(response.read())
        if not isinstance(result.get('result', {}).get('tools'), list): raise AssertionError(result)
        with open(output_path, 'w', encoding='utf-8') as output:
            json.dump(result['result']['tools'], output, separators=(',', ':'))
    finally:
        process.terminate()
        try: process.wait(timeout=5)
        except subprocess.TimeoutExpired: process.kill(); process.wait(timeout=5)
PY

# Verify canonical v8 baseline tools are preserved identically in candidate runtime
python3 - "$catalog" "$tmp/runtime.json" <<'PY'
import json, sys
catalog_file, runtime_file = sys.argv[1:]
catalog = {t['name']: t for t in json.load(open(catalog_file))}
runtime = {t['name']: t for t in json.load(open(runtime_file))}
for name, tool in catalog.items():
    assert name in runtime, f"Baseline tool {name} missing from candidate runtime"
    assert tool == runtime[name], f"Baseline tool {name} modified in candidate runtime: {tool} vs {runtime[name]}"
PY

validate_snapshot_integrity "$catalog" "$catalog_hash_file"

# Prove checked-in current-contract artifacts fail closed when mutated,
# without modifying tracked files.
jq '.[0].title = "mutation"' "$catalog" > "$tmp/mutated-catalog.json"
if validate_snapshot_integrity "$tmp/mutated-catalog.json" "$catalog_hash_file"; then
  echo 'phase-039h: mutated catalog unexpectedly passed' >&2; exit 1
fi
current_hash="$(tr -d '[:space:]' < "$catalog_hash_file")"
first="${current_hash:0:1}"
replacement=0
if [[ "$first" == 0 ]]; then replacement=1; fi
printf '%s%s\n' "$replacement" "${current_hash:1}" > "$tmp/mutated-hash"
if validate_snapshot_integrity "$catalog" "$tmp/mutated-hash"; then
  echo 'phase-039h: mutated hash unexpectedly passed' >&2; exit 1
fi

for tool in directory_list file_search text_search git_diff git_log git_show code_symbols code_references code_implementations code_diagnostics; do
  jq -e --arg tool "$tool" 'any(.[]; .name == $tool and (.inputSchema.properties.continuation.type == "string"))' "$catalog" >/dev/null
done

echo "phase-039h current contract acceptance: pass ($(tr -d '[:space:]' < "$catalog_hash_file"))"
