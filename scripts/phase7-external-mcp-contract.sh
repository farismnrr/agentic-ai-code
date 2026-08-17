#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
catalog="$root/.agents/contracts/039b-tool-catalog-v2.json"
catalog_hash_file="$root/.agents/contracts/039b-tool-catalog-v2.sha256"
legacy_catalog="$root/.agents/contracts/029-tool-catalog-v1.json"
legacy_hash_file="$root/.agents/contracts/029-tool-catalog-v1.sha256"
manifest="$root/Cargo.toml"

test -f "$catalog"
test -f "$catalog_hash_file"
test -f "$legacy_catalog"
test -f "$legacy_hash_file"
command -v jq >/dev/null
command -v sha256sum >/dev/null
command -v bwrap >/dev/null

jq -e 'type == "array" and all(.[]; (.name and .description and .inputSchema and .annotations and (.securitySchemes == [{"type":"oauth2","scopes":["relay.coding"]}]) and (has("security") | not)))' "$catalog" >/dev/null
test "$(jq -r '.[].name' "$catalog" | paste -sd' ' -)" = "terminal_exec http_fetch web_search directory_list file_search file_write file_edit file_read text_search git_status git_diff git_log git_show git_blame apply_patch terminal_job_start terminal_job_get terminal_job_cancel"
legacy_tools="$(jq -S -c . "$legacy_catalog")"
legacy_hash="$(printf '%s' "$legacy_tools" | sha256sum | awk '{print $1}')"
test "$legacy_hash" = "$(tr -d '[:space:]' < "$legacy_hash_file")"

RUSTFLAGS='-D warnings' cargo build --manifest-path "$manifest" --locked --bin ai-tools

runtime_file="$(mktemp)"
trap 'rm -f "$runtime_file"' EXIT

python3 - "$root/target/debug/ai-tools" "$runtime_file" "$root" <<'PY'
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
        [relay, "relay", "--port", str(port), "--dir", workspace, "--execution-root", workspace,
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
recorded="$(tr -d '[:space:]' < "$catalog_hash_file")"
test "$hash" = "$recorded"

echo "phase7 contract acceptance: pass ($hash)"
