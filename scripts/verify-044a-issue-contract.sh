#!/usr/bin/env bash
# Deterministic Plan-044A issue lifecycle contract and MCP surface gate.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# 1. Build relay binary
RUSTFLAGS='-D warnings' cargo build --manifest-path "$root/Cargo.toml" --locked --bin ai-tools >/dev/null

# 2. Run deterministic contract acceptance via python
exec python3 - "$root/target/debug/ai-tools" "$root" <<'PY'
import json, os, socket, subprocess, sys, tempfile, time, urllib.error, urllib.request

RELAY, ROOT = sys.argv[1:]
P = '2026-07-28'
O = 'http://localhost:3333'

def port():
    s = socket.socket()
    s.bind(('127.0.0.1', 0))
    p = s.getsockname()[1]
    s.close()
    return p

def wait(p, proc):
    for _ in range(100):
        if proc.poll() is not None:
            raise AssertionError(proc.stderr.read())
        try:
            with urllib.request.urlopen(f'http://127.0.0.1:{p}/health', timeout=.5) as r:
                if r.status == 200:
                    return
        except Exception:
            time.sleep(.05)
    raise AssertionError('health timeout')

def rpc(url, method, params, i, name=None):
    params = dict(params)
    params['_meta'] = {'io.modelcontextprotocol/protocolVersion': P, 'io.modelcontextprotocol/clientCapabilities': {}}
    h = {'Accept': 'application/json', 'Content-Type': 'application/json', 'Origin': O, 'MCP-Protocol-Version': P, 'Mcp-Method': method}
    if name:
        h['Mcp-Name'] = name
    q = urllib.request.Request(url, data=json.dumps({'jsonrpc': '2.0', 'id': i, 'method': method, 'params': params}).encode(), headers=h, method='POST')
    try:
        with urllib.request.urlopen(q, timeout=5) as r:
            return r.status, json.loads(r.read())
    except urllib.error.HTTPError as e:
        raw = e.read().decode('utf-8', 'replace')
        try:
            return e.code, json.loads(raw)
        except Exception:
            return e.code, {'_raw': raw}

def expect_error(url, name, args, i):
    st, b = rpc(url, 'tools/call', {'name': name, 'arguments': args}, i, name)
    assert st in (200, 400), (name, st, b)
    if st == 200:
        assert b['result']['isError'] is True, (name, b)

with tempfile.TemporaryDirectory(prefix='relay-044a-') as owner:
    repo = os.path.join(owner, 'repo')
    os.makedirs(repo)
    subprocess.run(['git', '-C', repo, 'init', '-q', '-b', 'main'], check=True)
    with open(os.path.join(repo, 'a'), 'w') as f:
        f.write('a\n')
    subprocess.run(['git', '-C', repo, 'add', 'a'], check=True)
    subprocess.run(['git', '-C', repo, '-c', 'user.name=fixture', '-c', 'user.email=fixture@example.test', 'commit', '-qm', 'base'], check=True)
    subprocess.run(['git', '-C', repo, 'remote', 'add', 'origin', 'https://github.com/farismnrr/ai-code.git'], check=True)
    
    p = port()
    proc = subprocess.Popen([RELAY, 'relay', '--port', str(p), '--dir', repo, '--execution-root', owner, '--origin', O, '--mode', 'local'], cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    try:
        wait(p, proc)
        url = f'http://127.0.0.1:{p}/mcp'
        
        # 1. tools/list verification
        st, b = rpc(url, 'tools/list', {}, 1)
        assert st == 200, b
        tools_list = b['result']['tools']
        assert len(tools_list) >= 84, f"Expected at least the 84-tool 044A surface, got {len(tools_list)}"
        
        tools = {x['name']: x for x in tools_list}
        assert len(tools) == len(tools_list), "Duplicate tool names found in catalog"
        
        expected_issue_tools = [
            'issue_list',
            'issue_get',
            'issue_create',
            'issue_update',
            'issue_comment',
            'issue_close',
            'issue_reopen',
        ]
        
        for name in expected_issue_tools:
            assert name in tools, f"Missing tool {name}"
            schema = tools[name]['inputSchema']
            assert schema.get('additionalProperties') is False, f"additionalProperties must be false for {name}"
            props = schema.get('properties', {})
            for forbidden in ['url', 'repo', 'repository', 'owner', 'command', 'args', 'endpoint', 'api', 'admin', 'force', 'auto']:
                assert forbidden not in props, f"Forbidden field '{forbidden}' found in {name}"
        
        # 2. Annotations verification
        for name in ['issue_list', 'issue_get']:
            ann = tools[name]['annotations']
            assert ann['readOnlyHint'] is True, f"readOnlyHint must be true for {name}"
            assert ann['destructiveHint'] is False, f"destructiveHint must be false for {name}"
            assert ann['idempotentHint'] is True, f"idempotentHint must be true for {name}"
            assert ann['openWorldHint'] is True, f"openWorldHint must be true for {name}"
            
        for name in ['issue_create', 'issue_update', 'issue_comment', 'issue_close', 'issue_reopen']:
            ann = tools[name]['annotations']
            assert ann['readOnlyHint'] is False, f"readOnlyHint must be false for {name}"
            assert ann['destructiveHint'] is True, f"destructiveHint must be true for {name}"
            assert ann['idempotentHint'] is False, f"idempotentHint must be false for {name}"
            assert ann['openWorldHint'] is True, f"openWorldHint must be true for {name}"
            
        # 3. Schema and malformed-input enforcement
        expect_error(url, 'issue_list', {'cwd': 'repo', 'state': 'invalid_state'}, 10)
        expect_error(url, 'issue_list', {'cwd': 'repo', 'owner': 'evil'}, 11)
        expect_error(url, 'issue_get', {'cwd': 'repo'}, 20)
        expect_error(url, 'issue_get', {'cwd': 'repo', 'number': 0}, 21)
        expect_error(url, 'issue_get', {'cwd': 'repo', 'number': -5}, 22)
        expect_error(url, 'issue_create', {'cwd': 'repo'}, 30)
        expect_error(url, 'issue_create', {'cwd': 'repo', 'title': ''}, 31)
        expect_error(url, 'issue_update', {'cwd': 'repo'}, 40)
        expect_error(url, 'issue_update', {'cwd': 'repo', 'number': 0}, 41)
        expect_error(url, 'issue_comment', {'cwd': 'repo'}, 50)
        expect_error(url, 'issue_comment', {'cwd': 'repo', 'number': 1}, 51)
        expect_error(url, 'issue_comment', {'cwd': 'repo', 'number': 1, 'body': ''}, 52)
        expect_error(url, 'issue_close', {'cwd': 'repo', 'number': 1}, 60)
        expect_error(url, 'issue_close', {'cwd': 'repo', 'number': 1, 'reason': 'invalid_reason'}, 61)
        expect_error(url, 'issue_close', {'cwd': 'repo', 'number': 0, 'reason': 'completed'}, 62)
        expect_error(url, 'issue_reopen', {'cwd': 'repo'}, 70)
        expect_error(url, 'issue_reopen', {'cwd': 'repo', 'number': 0}, 71)

        # 4. Change request tools preserved
        cr_tools = ['change_request_list', 'change_request_get', 'change_request_create', 'change_request_update', 'change_request_checks', 'change_request_merge']
        for name in cr_tools:
            assert name in tools, f"Change request tool {name} missing"

        print("044A issue lifecycle contract acceptance: PASS")
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=3)
        except subprocess.TimeoutExpired:
            proc.kill()
            proc.wait(timeout=3)
PY
