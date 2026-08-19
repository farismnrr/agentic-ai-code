#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUSTFLAGS='-D warnings' cargo build --manifest-path "$root/Cargo.toml" --locked --bin ai-tools >/dev/null

exec python3 - "$root/target/debug/ai-tools" "$root" <<'PY'
import json, os, socket, subprocess, sys, tempfile, time, urllib.error, urllib.request

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
        'io.modelcontextprotocol/clientCapabilities': {},
    }
    headers = {
        'Accept': 'application/json',
        'Content-Type': 'application/json',
        'Origin': ORIGIN,
        'MCP-Protocol-Version': PROTOCOL,
        'Mcp-Method': method,
    }
    if name:
        headers['Mcp-Name'] = name
    request = urllib.request.Request(
        url,
        data=json.dumps({'jsonrpc':'2.0','id':request_id,'method':method,'params':params}).encode(),
        headers=headers,
        method='POST',
    )
    try:
        with urllib.request.urlopen(request, timeout=5) as response:
            body = json.loads(response.read())
            return response.status, body
    except urllib.error.HTTPError as error:
        raw = error.read().decode('utf-8', 'replace')
        try:
            body = json.loads(raw)
        except Exception:
            body = {'_raw': raw}
        return error.code, body


def call(url, name, arguments, request_id):
    status, body = rpc(url, 'tools/call', {'name':name, 'arguments':arguments}, request_id, name)
    assert status == 200, (name, status, body)
    return body['result']


def payload(result):
    assert result['resultType'] == 'complete', result
    assert result.get('isError') is False, result
    text = next(item['text'] for item in result['content'] if item.get('type') == 'text')
    return json.loads(text)


def expect_tool_error(url, name, arguments, label, request_id):
    status, body = rpc(url, 'tools/call', {'name':name, 'arguments':arguments}, request_id, name)
    assert status in (200, 400), (label, status, body)
    if status == 200:
        assert body['result']['isError'] is True, (label, body)
    else:
        assert body['error']['code'] in (-32602, -32600), (label, body)


with tempfile.TemporaryDirectory(prefix='relay-040b-branches-') as owner:
    repo = os.path.join(owner, 'repo')
    os.makedirs(repo)
    subprocess.run(['git','-C',repo,'init','-q','-b','main'], check=True)
    open(os.path.join(repo, 'file.txt'), 'w').write('base\n')
    subprocess.run(['git','-C',repo,'add','file.txt'], check=True)
    subprocess.run(['git','-C',repo,'config','user.email','fixture@example.test'], check=True)
    subprocess.run(['git','-C',repo,'config','user.name','fixture'], check=True)
    subprocess.run(['git','-C',repo,'commit','-qm','base'], check=True)
    head = subprocess.check_output(['git','-C',repo,'rev-parse','HEAD'], text=True).strip()
    subprocess.run(['git','-C',repo,'branch','-r'], check=True)

    port = free_port()
    proc = subprocess.Popen([
        RELAY, 'relay', '--port', str(port), '--dir', repo,
        '--execution-root', owner, '--origin', ORIGIN, '--mode', 'local'
    ], cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    try:
        wait_health(port, proc)
        url = f'http://127.0.0.1:{port}/mcp'

        status, listed = rpc(url, 'tools/list', {}, 1)
        assert status == 200, listed
        tools = {tool['name']: tool for tool in listed['result']['tools']}
        for name in ['git_branch_list', 'git_branch_create', 'git_branch_switch', 'git_stage', 'git_unstage', 'git_commit', 'git_operation_status', 'git_merge_start', 'git_merge_continue', 'git_merge_abort', 'git_rebase_start', 'git_rebase_continue', 'git_rebase_abort', 'git_branch_delete']:
            assert name in tools, name
        assert tools['git_branch_list']['annotations']['readOnlyHint'] is True
        assert tools['git_operation_status']['annotations']['readOnlyHint'] is True
        assert tools['git_branch_create']['annotations']['readOnlyHint'] is False
        assert tools['git_branch_switch']['annotations']['readOnlyHint'] is False
        assert tools['git_branch_delete']['annotations']['destructiveHint'] is True

        branches = payload(call(url, 'git_branch_list', {'cwd':'repo'}, 10))
        assert branches['repository_root'] == 'repo', branches
        assert branches['branches'] == [{'name':'main','head':head,'current':True}], branches

        created = payload(call(url, 'git_branch_create', {'cwd':'repo','name':'feat/040b-fixture'}, 20))
        assert created == {'repository_root':'repo','operation':'create','branch':'feat/040b-fixture','head':head}, created
        assert subprocess.check_output(['git','-C',repo,'branch','--show-current'], text=True).strip() == 'main'

        branches = payload(call(url, 'git_branch_list', {'cwd':'repo'}, 30))
        by_name = {branch['name']: branch for branch in branches['branches']}
        assert by_name['main']['current'] is True and by_name['feat/040b-fixture']['current'] is False, branches

        switched = payload(call(url, 'git_branch_switch', {'cwd':'repo','name':'feat/040b-fixture'}, 40))
        assert switched == {'repository_root':'repo','operation':'switch','branch':'feat/040b-fixture','head':head}, switched
        assert subprocess.check_output(['git','-C',repo,'branch','--show-current'], text=True).strip() == 'feat/040b-fixture'

        expect_tool_error(url, 'git_branch_create', {'cwd':'repo','name':'feat/040b-fixture'}, 'duplicate branch', 50)
        for invalid in ['-danger', '../escape', 'bad ref', 'refs/heads/x', '.hidden']:
            expect_tool_error(url, 'git_branch_create', {'cwd':'repo','name':invalid}, f'invalid branch {invalid}', 60)
        expect_tool_error(url, 'git_branch_switch', {'cwd':'repo','name':'origin/main'}, 'remote-only branch switch', 70)

        open(os.path.join(repo, 'file.txt'), 'a').write('dirty\n')
        expect_tool_error(url, 'git_branch_switch', {'cwd':'repo','name':'main'}, 'dirty worktree switch', 80)
        assert subprocess.check_output(['git','-C',repo,'branch','--show-current'], text=True).strip() == 'feat/040b-fixture'
        subprocess.run(['git','-C',repo,'reset','--hard','-q','HEAD'], check=True)

        switched_back = payload(call(url, 'git_branch_switch', {'cwd':'repo','name':'main'}, 90))
        assert switched_back['branch'] == 'main' and switched_back['head'] == head, switched_back
        print('040B Git branch foundation black-box: PASS')
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=3)
        except subprocess.TimeoutExpired:
            proc.kill()
            proc.wait(timeout=3)
PY
