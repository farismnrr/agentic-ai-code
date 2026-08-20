#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

relay="${AI_TOOLS_BIN:-$root/target/release/ai-tools}"
test -x "$relay"
command -v python3 >/dev/null

python3 - "$relay" "$root" <<'PY'
import json
import os
import socket
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request
from pathlib import Path

relay, root = sys.argv[1:]


def free_port():
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


def request(url, method, params):
    body = json.dumps({"jsonrpc": "2.0", "id": method, "method": method, "params": params}).encode()
    headers = {
        "Content-Type": "application/json",
        "Origin": "http://localhost:3333",
        "MCP-Protocol-Version": "2026-07-28",
        "Mcp-Method": method,
    }
    if method == "tools/call":
        headers["Mcp-Name"] = params["name"]
    req = urllib.request.Request(
        url,
        data=body,
        headers=headers,
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=10) as response:
            return json.loads(response.read())
    except urllib.error.HTTPError as error:
        detail = error.read().decode("utf-8", errors="replace")
        raise RuntimeError(f"MCP {method} returned HTTP {error.code}: {detail}") from error


with tempfile.TemporaryDirectory(prefix="relay-050-agent-") as temp:
    root_path = Path(temp)
    workspace = root_path / "workspace"
    provider_bin = root_path / "provider-bin"
    home = root_path / "home"
    workspace.mkdir()
    provider_bin.mkdir()
    home.mkdir()
    (workspace / "README.md").write_text("# MCP result fixture\n", encoding="utf-8")
    codex = provider_bin / "codex"
    codex.write_text(
        "#!/bin/sh\n"
        'if [ "${1:-}" = "login" ] && [ "${2:-}" = "status" ]; then exit 0; fi\n'
        "printf 'provider=codex\\nbranch=fix/host-github-auth\\nREADME=true\\ntoken=fixture-secret\\n'\n",
        encoding="utf-8",
    )
    codex.chmod(0o755)
    port = free_port()
    environment = os.environ.copy()
    environment["HOME"] = str(home)
    process = subprocess.Popen(
        [
            relay,
            "relay",
            "--mode",
            "local",
            "--port",
            str(port),
            "--dir",
            str(workspace),
            "--execution-root",
            str(root_path),
            "--origin",
            "http://localhost:3333",
            "--toolchain-path",
            str(provider_bin),
        ],
        cwd=root,
        env=environment,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    try:
        endpoint = f"http://127.0.0.1:{port}/mcp"
        health = f"http://127.0.0.1:{port}/health"
        for _ in range(100):
            if process.poll() is not None:
                raise RuntimeError("relay exited before MCP wire acceptance")
            try:
                with urllib.request.urlopen(health, timeout=1) as response:
                    if response.status == 200:
                        break
            except (urllib.error.URLError, ConnectionError):
                time.sleep(0.1)
        else:
            raise RuntimeError("relay did not become healthy")

        listed = request(
            endpoint,
            "tools/list",
            {
                "_meta": {
                    "io.modelcontextprotocol/protocolVersion": "2026-07-28",
                    "io.modelcontextprotocol/clientCapabilities": {},
                }
            },
        )
        tools = listed["result"]["tools"]
        agent_tool = next(tool for tool in tools if tool["name"] == "agent_delegate")
        assert agent_tool["inputSchema"]["properties"]["providers"]["items"]["enum"] == ["codex"]

        called = request(
            endpoint,
            "tools/call",
            {
                "name": "agent_delegate",
                "arguments": {
                    "prompt": "report provider identity, current branch, and whether README exists",
                    "providers": ["codex"],
                    "cwd": str(workspace),
                    "timeout_ms": 5_000,
                    "fallback": False,
                },
                "_meta": {
                    "io.modelcontextprotocol/protocolVersion": "2026-07-28",
                    "io.modelcontextprotocol/clientCapabilities": {},
                },
            },
        )
        result = called["result"]
        assert result["isError"] is False, called
        agent_result = json.loads(result["content"][0]["text"])
        assert agent_result["provider"] == "codex"
        assert agent_result["message"] == "delegation completed"
        assert agent_result["workspace_changed"] is False
        output = agent_result["output"]
        assert "provider=codex" in output
        assert "branch=fix/host-github-auth" in output
        assert "README=true" in output
        assert "token=[REDACTED]" in output
        assert "fixture-secret" not in json.dumps(called)
        assert agent_result["output_redacted"] is True
        assert "output_truncated" not in agent_result
        print("plan050 MCP agent result propagation acceptance: PASS")
    finally:
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=5)
PY
