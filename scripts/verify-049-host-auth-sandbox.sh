#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

test -x "$root/target/release/ai-tools"
exec python3 - "$root/target/release/ai-tools" <<'PY'
import json
import os
import shlex
import socket
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request
from pathlib import Path


binary = Path(sys.argv[1]).resolve()


def free_port():
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


def wait_for_health(process, port):
    url = f"http://127.0.0.1:{port}/health"
    for _ in range(100):
        if process.poll() is not None:
            raise RuntimeError("relay exited before the acceptance health check")
        try:
            with urllib.request.urlopen(url, timeout=1) as response:
                if response.status == 200:
                    return
        except (urllib.error.URLError, TimeoutError, ConnectionError):
            time.sleep(0.05)
    raise RuntimeError("relay did not become healthy")


def rpc(port, name, arguments):
    body = {
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": name,
            "arguments": arguments,
            "_meta": {
                "io.modelcontextprotocol/protocolVersion": "2026-07-28",
                "io.modelcontextprotocol/clientCapabilities": {},
            },
        },
    }
    request = urllib.request.Request(
        f"http://127.0.0.1:{port}/mcp",
        data=json.dumps(body).encode(),
        headers={
            "Content-Type": "application/json",
            "Origin": "http://localhost:3333",
            "MCP-Protocol-Version": "2026-07-28",
            "Mcp-Method": "tools/call",
            "Mcp-Name": name,
        },
        method="POST",
    )
    with urllib.request.urlopen(request, timeout=15) as response:
        return json.loads(response.read())


def result_text(value):
    result = value.get("result", {})
    return "\n".join(
        item.get("text", "")
        for item in result.get("content", [])
        if item.get("type") == "text"
    )


def start_relay(root, home, workspace, toolchain, allow_auth, enable_hooks=False):
    port = free_port()
    environment = os.environ.copy()
    environment["HOME"] = str(home)
    for key in (
        "RELAY_ALLOW_HOST_GITHUB_AUTH",
        "RELAY_ALLOW_TERMINAL_NETWORK",
        "RELAY_ALLOW_AGENT_NETWORK",
        "RELAY_ENABLE_AGENT_HOOKS",
    ):
        environment.pop(key, None)
    command = [
        str(binary),
        "relay",
        "--mode",
        "local",
        "--port",
        str(port),
        "--dir",
        str(workspace),
        "--execution-root",
        str(root),
        "--origin",
        "http://localhost:3333",
        "--toolchain-path",
        str(toolchain),
    ]
    if allow_auth:
        command.append("--allow-host-github-auth")
    if enable_hooks:
        command.extend([
            "--enable-agent-hooks",
            "--lsp-server",
            "typescript=host-auth-lsp",
        ])
    process = subprocess.Popen(
        command,
        cwd=root,
        env=environment,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    wait_for_health(process, port)
    return process, port


def stop_relay(process):
    process.terminate()
    try:
        process.wait(timeout=5)
    except subprocess.TimeoutExpired:
        process.kill()
        process.wait(timeout=5)


with tempfile.TemporaryDirectory(prefix="relay-plan049-") as temporary:
    root = Path(temporary).resolve()
    home = root / "home"
    workspace = root / "workspace"
    toolchain = root / "toolchain"
    gh_config = home / ".config" / "gh"
    workspace_agents = workspace / ".agents"
    for directory in (
        gh_config,
        home / ".ssh",
        home / ".config" / "gcloud",
        home / ".docker",
        workspace_agents,
        toolchain,
    ):
        directory.mkdir(parents=True, exist_ok=True)
    (gh_config / "hosts.yml").write_text("github.com: fixture-auth\n")
    (home / ".ssh" / "id_rsa").write_text("protected\n")
    (home / ".git-credentials").write_text("protected\n")
    (home / ".config" / "gcloud" / "credentials.db").write_text("protected\n")
    (home / ".docker" / "config.json").write_text("protected\n")
    (workspace / "main.ts").write_text("export const value = 1;\n")
    subprocess.run(["git", "init", "-q", str(workspace)], check=True)

    gh_path = toolchain / "gh"
    gh_path.write_text("""#!/bin/sh
if test "$1" = auth && test -s "$HOME/.config/gh/hosts.yml"; then
  echo AUTHENTICATED
  exit 0
fi
echo AUTH_MISSING
exit 1
""")
    gh_path.chmod(0o755)

    lsp_host_path = repr(str(gh_config / "hosts.yml"))
    lsp_script = """#!/usr/bin/python3
import json
import os
import sys

HOST_AUTH = __HOST_AUTH__

def read_message():
    length = None
    while True:
        line = sys.stdin.buffer.readline()
        if not line:
            return None
        if line in (b'\\r\\n', b'\\n'):
            break
        key, _, value = line.decode().partition(':')
        if key.lower() == 'content-length':
            length = int(value.strip())
    if length is None:
        return None
    return json.loads(sys.stdin.buffer.read(length))

def write_message(value):
    encoded = json.dumps(value, separators=(',', ':')).encode()
    sys.stdout.buffer.write(f'Content-Length: {len(encoded)}\\r\\n\\r\\n'.encode())
    sys.stdout.buffer.write(encoded)
    sys.stdout.buffer.flush()

while True:
    message = read_message()
    if message is None:
        break
    method = message.get('method')
    request_id = message.get('id')
    if request_id is None:
        continue
    if method == 'initialize':
        write_message({'jsonrpc':'2.0','id':request_id,'result':{'capabilities':{'documentSymbolProvider':True,'textDocumentSync':{'openClose':True,'change':1}}}})
    elif method == 'textDocument/documentSymbol':
        visible = os.path.isfile(HOST_AUTH) and os.path.getsize(HOST_AUTH) > 0
        name = 'HOST_AUTH_VISIBLE' if visible else 'HOST_AUTH_MASKED'
        write_message({'jsonrpc':'2.0','id':request_id,'result':[{'name':name,'kind':13,'range':{'start':{'line':0,'character':0},'end':{'line':0,'character':1}},'selectionRange':{'start':{'line':0,'character':0},'end':{'line':0,'character':1}}}]})
    elif method == 'shutdown':
        write_message({'jsonrpc':'2.0','id':request_id,'result':None})
"""
    lsp_script = lsp_script.replace("__HOST_AUTH__", lsp_host_path)
    lsp_path = toolchain / "host-auth-lsp"
    lsp_path.write_text(lsp_script)
    lsp_path.chmod(0o755)

    hook_host_path = shlex.quote(str(gh_config / "hosts.yml"))
    hook_path = toolchain / "host-auth-hook"
    hook_path.write_text(
        "#!/bin/sh\n"
        f"if test -s {hook_host_path}; then exit 11; fi\n"
        "exit 0\n"
    )
    hook_path.chmod(0o755)
    identity = f"{workspace}|{(workspace / '.git').resolve()}"
    (workspace_agents / "hooks.json").write_text(
        json.dumps(
            {
                "repository_identity": identity,
                "handlers": [
                    {
                        "event": "pre_tool_use",
                        "command": ["host-auth-hook"],
                        "class": "security",
                        "tool": "terminal_exec",
                        "effect_class": "process_exec",
                        "timeout_ms": 1000,
                    }
                ],
            }
        )
    )

    no_auth_process, no_auth_port = start_relay(
        root, home, workspace, toolchain, allow_auth=False
    )
    try:
        no_auth = rpc(
            no_auth_port,
            "terminal_exec",
            {"command": "gh", "args": ["auth", "status"]},
        )
        assert "AUTH_MISSING" in result_text(no_auth), "host auth leaked without opt-in"
    finally:
        stop_relay(no_auth_process)

    auth_process, auth_port = start_relay(
        root, home, workspace, toolchain, allow_auth=True, enable_hooks=True
    )
    try:
        read_only = rpc(
            auth_port,
            "terminal_exec",
            {"command": "gh", "args": ["auth", "status"]},
        )
        assert "AUTHENTICATED" in result_text(read_only), "read-only terminal did not see opt-in auth"

        writable = rpc(
            auth_port,
            "terminal_exec",
            {"command": "sh", "args": ["-c", "gh auth status; touch writable-proof"]},
        )
        assert "AUTHENTICATED" in result_text(writable), "writable terminal did not see opt-in auth"
        assert (workspace / "writable-proof").is_file(), "writable terminal fixture did not remain writable"

        protected = rpc(
            auth_port,
            "terminal_exec",
            {
                "command": "sh",
                "args": [
                    "-c",
                    'for p in "$HOME/.ssh/id_rsa" "$HOME/.git-credentials" "$HOME/.config/gcloud/credentials.db" "$HOME/.docker/config.json"; do test ! -s "$p" || exit 17; done',
                ],
            },
        )
        assert "Exit: 0" in result_text(protected), "unrelated protected credentials were visible"

        lsp = rpc(
            auth_port,
            "code_symbols",
            {"cwd": str(workspace), "path": "main.ts"},
        )
        lsp_text = result_text(lsp)
        assert "HOST_AUTH_MASKED" in lsp_text and "HOST_AUTH_VISIBLE" not in lsp_text, "LSP received host auth"

        hook_probe = rpc(auth_port, "terminal_exec", {"command": "true"})
        assert not hook_probe.get("result", {}).get("isError", False), "hook received host auth"
    finally:
        stop_relay(auth_process)

print("plan049 host-auth Bubblewrap acceptance: PASS")
PY
