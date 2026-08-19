#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUSTFLAGS='-D warnings' cargo build --manifest-path "$root/Cargo.toml" --locked --bin ai-tools >/dev/null

exec python3 - "$root/target/debug/ai-tools" "$root" <<'PY'
import json, socket, subprocess, sys, tempfile, time, urllib.error, urllib.request

RELAY, ROOT = sys.argv[1:]
PROTOCOL = '2026-07-28'
ORIGIN = 'http://localhost:3333'


def free_port():
    with socket.socket() as sock:
        sock.bind(('127.0.0.1', 0))
        return sock.getsockname()[1]


def wait_health(port, proc):
    deadline = time.monotonic() + 5
    while time.monotonic() < deadline:
        if proc.poll() is not None:
            raise AssertionError(proc.stderr.read())
        try:
            with urllib.request.urlopen(f'http://127.0.0.1:{port}/health', timeout=0.5) as response:
                if response.status == 200:
                    return
        except Exception:
            time.sleep(0.05)
    raise AssertionError('relay health timeout')


def rpc(url, method, params, request_id, name=None):
    params = dict(params)
    params['_meta'] = {
        'io.modelcontextprotocol/protocolVersion': PROTOCOL,
        'io.modelcontextprotocol/clientCapabilities': {
            'extensions': {'io.modelcontextprotocol/tasks': {}}
        },
    }
    headers = {
        'Accept': 'application/json',
        'Content-Type': 'application/json',
        'Origin': ORIGIN,
        'MCP-Protocol-Version': PROTOCOL,
        'Mcp-Method': method,
    }
    routing_name = name if name else params.get('taskId') if method.startswith('tasks/') else None
    if routing_name:
        headers['Mcp-Name'] = routing_name
    request = urllib.request.Request(
        url,
        data=json.dumps({'jsonrpc': '2.0', 'id': request_id, 'method': method, 'params': params}).encode(),
        headers=headers,
        method='POST',
    )
    try:
        with urllib.request.urlopen(request, timeout=3) as response:
            assert response.status == 200
            return json.loads(response.read())['result']
    except urllib.error.HTTPError as error:
        body = error.read().decode('utf-8', 'replace')
        raise AssertionError(f'{method} {name or ""} HTTP {error.code}: {body[:1000]}') from error


def task_tool(url, name, arguments, request_id):
    created = rpc(url, 'tools/call', {'name': name, 'arguments': arguments}, request_id, name)
    assert created['resultType'] == 'task', (name, created)
    assert created['status'] == 'working', (name, created)
    assert created['pollIntervalMs'] >= 250, (name, created)
    task_id = created['taskId']
    deadline = time.monotonic() + 5
    while time.monotonic() < deadline:
        current = rpc(url, 'tasks/get', {'taskId': task_id}, request_id + 100)
        if current['status'] != 'working':
            return current
        time.sleep(min(current.get('pollIntervalMs', 250), 250) / 1000)
    raise AssertionError(f'{name} task did not settle')


with tempfile.TemporaryDirectory(prefix='relay-040a-task-') as workspace:
    port = free_port()
    proc = subprocess.Popen([
        RELAY, 'relay', '--port', str(port), '--dir', workspace,
        '--execution-root', workspace, '--origin', ORIGIN, '--mode', 'local'
    ], cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    try:
        wait_health(port, proc)
        url = f'http://127.0.0.1:{port}/mcp'

        http_task = task_tool(url, 'http_fetch', {'url': 'http://127.0.0.1:1/'}, 10)
        assert http_task['status'] in ('completed', 'failed', 'cancelled'), http_task

        search_task = task_tool(url, 'web_search', {'query': '040a deterministic task canary'}, 20)
        assert search_task['status'] in ('completed', 'failed', 'cancelled'), search_task

        # External mutations do not become durable tasks until a later plan adds
        # request-level idempotency/deduplication for ambiguous response loss.
        post = rpc(url, 'tools/call', {
            'name': 'http_fetch',
            'arguments': {'url': 'http://127.0.0.1:1/', 'method': 'POST', 'data': 'synthetic'}
        }, 25, 'http_fetch')
        assert post['resultType'] == 'complete', post

        # Fast native reads remain synchronous even when the client negotiates Tasks.
        read = rpc(url, 'tools/call', {'name': 'directory_list', 'arguments': {'path': '.', 'depth': 0}}, 30, 'directory_list')
        assert read['resultType'] == 'complete', read
        print('040A MCP task transport black-box: PASS')
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=3)
        except subprocess.TimeoutExpired:
            proc.kill()
            proc.wait(timeout=3)
PY
