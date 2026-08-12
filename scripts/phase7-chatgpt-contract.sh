#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
catalog="$root/.agents/contracts/029-tool-catalog-v1.json"
manifest="$root/packages/rust-tools/Cargo.toml"

test -f "$catalog"
command -v jq >/dev/null
command -v sha256sum >/dev/null
command -v bwrap >/dev/null

jq -e 'type == "array" and all(.[]; (.name and .description and .inputSchema and .annotations and (.securitySchemes == [{"type":"oauth2","scopes":["relay.coding"]}]) and (has("security") | not)))' "$catalog" >/dev/null
test "$(jq -r '.[].name' "$catalog" | paste -sd' ' -)" = "terminal_exec http_fetch web_search"
test "$(jq -r '.[].title' "$catalog" | paste -sd' ' -)" = "Sandboxed Coding Terminal HTTP Fetch Web Search"

RUSTFLAGS='-D warnings' cargo build --manifest-path "$manifest" --locked --bin relay-agent

runtime_file="$(mktemp)"
trap 'rm -f "$runtime_file"' EXIT

python3 - "$root/target/debug/relay-agent" "$runtime_file" "$root" <<'PY'
import json
import os
import socket
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request

relay, output_path, root = sys.argv[1:]

def free_port():
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]

def post(url, body):
    request = urllib.request.Request(
        url,
        data=json.dumps(body).encode(),
        headers={
            "Content-Type": "application/json",
            "Origin": "http://localhost:3333",
            "MCP-Protocol-Version": "2026-07-28",
            "Mcp-Method": "tools/list",
        },
        method="POST",
    )
    with urllib.request.urlopen(request, timeout=5) as response:
        return json.loads(response.read())

with tempfile.TemporaryDirectory(prefix="relay-phase7-") as workspace:
    port = free_port()
    environment = os.environ.copy()
    process = subprocess.Popen(
        [relay, "--port", str(port), "--dir", workspace, "--execution-root", workspace,
         "--origin", "http://localhost:3333", "--mode", "local"],
        cwd=root,
        env=environment,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    try:
        for _ in range(50):
            if process.poll() is not None:
                raise RuntimeError(process.stderr.read().strip())
            try:
                with urllib.request.urlopen(f"http://127.0.0.1:{port}/health", timeout=1) as response:
                    if response.status == 200:
                        break
            except (urllib.error.URLError, ConnectionError):
                time.sleep(0.1)
        else:
            raise RuntimeError("relay did not become healthy")

        response = post(
            f"http://127.0.0.1:{port}/mcp",
            {"jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": {"_meta": {
                "io.modelcontextprotocol/protocolVersion": "2026-07-28",
                "io.modelcontextprotocol/clientCapabilities": {},
            }}},
        )
        if not isinstance(response.get("result", {}).get("tools"), list):
            raise AssertionError(f"tools/list response did not contain result.tools: {response}")
        with open(output_path, "w", encoding="utf-8") as output:
            json.dump(response, output, separators=(",", ":"))
    finally:
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=5)
PY

runtime_tools="$(jq -S -c '.result.tools' "$runtime_file")"
frozen_tools="$(jq -S -c . "$catalog")"
test "$runtime_tools" = "$frozen_tools"

hash="$(printf '%s' "$frozen_tools" | sha256sum | awk '{print $1}')"
recorded="$(sed -n 's/^catalogSha256: `\([^`]*\)`.*/\1/p' "$root/.agents/memories/029-phase7-published-app-lifecycle.md")"
test "$hash" = "$recorded"

echo "phase7 contract acceptance: pass ($hash)"
