#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
manifest="$root/Cargo.toml"

command -v cargo >/dev/null
command -v python3 >/dev/null
command -v bwrap >/dev/null

RUSTFLAGS='-D warnings' cargo run --manifest-path "$manifest" --locked --quiet --package relay-application --example file_search_limits
RUSTFLAGS='-D warnings' cargo build --manifest-path "$manifest" --locked --bin ai-tools

exec python3 - "$root/target/debug/ai-tools" "$root" <<'PY'
import json
import os
import socket
import subprocess
import sys
import threading
import tempfile
import time
import urllib.error
import urllib.request

RELAY, ROOT = sys.argv[1:]
PROTOCOL = "2026-07-28"
ORIGIN = "http://localhost:3333"


def free_port():
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


def request(url, headers, body):
    req = urllib.request.Request(url, data=json.dumps(body).encode(), headers=headers, method="POST")
    try:
        with urllib.request.urlopen(req, timeout=10) as response:
            return response.status, json.loads(response.read())
    except urllib.error.HTTPError as error:
        return error.code, json.loads(error.read())


def headers(method, name=None):
    result = {
        "Content-Type": "application/json",
        "Origin": ORIGIN,
        "MCP-Protocol-Version": PROTOCOL,
        "Mcp-Method": method,
    }
    if name:
        result["Mcp-Name"] = name
    return result


def mcp(method, params, request_id=1):
    params = dict(params)
    params["_meta"] = {
        "io.modelcontextprotocol/protocolVersion": PROTOCOL,
        "io.modelcontextprotocol/clientCapabilities": {},
    }
    return {"jsonrpc": "2.0", "id": request_id, "method": method, "params": params}


def wait_health(port, process):
    for _ in range(80):
        if process.poll() is not None:
            raise AssertionError(process.stderr.read())
        try:
            with urllib.request.urlopen(f"http://127.0.0.1:{port}/health", timeout=1) as response:
                if response.status == 200:
                    return
        except (urllib.error.URLError, ConnectionError):
            pass
        time.sleep(0.05)
    raise AssertionError("relay did not become healthy")


def tool_call(url, arguments, request_id=10):
    status, body = request(
        url,
        headers("tools/call", "file_search"),
        mcp("tools/call", {"name": "file_search", "arguments": arguments}, request_id),
    )
    if status != 200:
        raise AssertionError(f"file_search HTTP {status}: {body}")
    result = body["result"]
    text = next(item["text"] for item in result["content"] if item.get("type") == "text")
    return result, json.loads(text)


def expect_tool_error(url, arguments, label):
    status, body = request(
        url,
        headers("tools/call", "file_search"),
        mcp("tools/call", {"name": "file_search", "arguments": arguments}, 11),
    )
    if status == 400:
        if body.get("error", {}).get("code") != -32602:
            raise AssertionError(f"{label}: unexpected schema rejection: {body}")
        return {"schemaError": body["error"]}
    if status != 200:
        raise AssertionError(f"{label}: unexpected HTTP {status}: {body}")
    result = body["result"]
    if result["isError"] is not True:
        raise AssertionError(f"{label}: expected tool error, got {result}")
    return result


def tool_call_raw(url, arguments, request_id=11):
    status, body = request(
        url,
        headers("tools/call", "file_search"),
        mcp("tools/call", {"name": "file_search", "arguments": arguments}, request_id),
    )
    if status != 200:
        raise AssertionError(f"file_search HTTP {status}: {body}")
    result = body["result"]
    if result["isError"]:
        return result, None
    text = next(item["text"] for item in result["content"] if item.get("type") == "text")
    return result, json.loads(text)


with tempfile.TemporaryDirectory(prefix="relay-file-search-") as base:
    workspace = os.path.join(base, "workspace")
    external = os.path.join(base, "external")
    os.makedirs(os.path.join(workspace, "src", "nested"))
    os.makedirs(os.path.join(workspace, "src", "nested", "more"))
    os.makedirs(os.path.join(workspace, ".secret"))
    os.makedirs(external)

    files = {
        "Cargo.toml": "root",
        "src/lib.rs": "lib",
        "src/auth_service.rs": "auth",
        "src/nested/auth.rs": "nested",
        "src/nested/more/lex-02.txt": "nested",
        "src/nested/more/lex-01.txt": "nested",
        "src/nested/lex-03.txt": "nested",
        ".hidden.rs": "hidden",
        ".secret/inside.rs": "hidden-dir",
        "ignored.rs": "gitignored-but-visible",
    }
    for relative, content in files.items():
        path = os.path.join(workspace, relative)
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "w", encoding="utf-8") as output:
            output.write(content)
    with open(os.path.join(workspace, ".gitignore"), "w", encoding="utf-8") as output:
        output.write("ignored.rs\n")

    for generated in (".git", "node_modules", "target", ".nuxt", ".output"):
        path = os.path.join(workspace, generated)
        os.makedirs(path)
        with open(os.path.join(path, "generated.rs"), "w", encoding="utf-8") as output:
            output.write("generated")
    os.makedirs(os.path.join(workspace, "src", "target"))
    with open(os.path.join(workspace, "src", "target", "nested-generated.rs"), "w", encoding="utf-8") as output:
        output.write("generated")

    with open(os.path.join(external, "CANARY-EXTERNAL-038.rs"), "w", encoding="utf-8") as output:
        output.write("external")
    os.symlink(os.path.join(external, "CANARY-EXTERNAL-038.rs"), os.path.join(workspace, "external-file-link.rs"))
    os.symlink(external, os.path.join(workspace, "external-dir-link"))
    os.symlink("loop-b", os.path.join(workspace, "loop-a"))
    os.symlink("loop-a", os.path.join(workspace, "loop-b"))
    os.makedirs(os.path.join(workspace, "contained-dir"))
    with open(os.path.join(workspace, "contained-dir", "not-visible.rs"), "w", encoding="utf-8") as output:
        output.write("contained")
    os.symlink("contained-dir", os.path.join(workspace, "contained-dir-link"))
    os.symlink("src/lib.rs", os.path.join(workspace, "contained-file-link.rs"))
    os.mkfifo(os.path.join(workspace, "named-pipe"))

    os.makedirs(os.path.join(workspace, "many"))
    for index in range(105):
        with open(os.path.join(workspace, "many", f"match-{index:03}.txt"), "w", encoding="utf-8") as output:
            output.write("x")

    os.makedirs(os.path.join(workspace, "target", "scan-cap"))
    for index in range(4097):
        with open(os.path.join(workspace, "target", "scan-cap", f"entry-{index:04}.dat"), "w", encoding="utf-8") as output:
            output.write("x")

    total_cap = os.path.join(workspace, "target", "total-cap")
    os.makedirs(total_cap)
    # 17 x 4095 = 69,615 entries, below the per-directory cap but above the
    # whole-call traversal cap. Directories themselves also count as entries.
    for directory_index in range(17):
        directory = os.path.join(total_cap, f"d-{directory_index:02}")
        os.makedirs(directory)
        for index in range(4095):
            with open(os.path.join(directory, f"entry-{index:04}.dat"), "w", encoding="utf-8") as output:
                output.write("x")

    wide_root = os.path.join(workspace, "target", "wide")
    wide = wide_root
    for index in range(12):
        wide = os.path.join(wide, f"segment-{index}-" + "x" * 220)
    os.makedirs(wide)
    for index in range(100):
        with open(os.path.join(wide, f"match-{index:03}-" + "y" * 220 + ".result"), "w", encoding="utf-8") as output:
            output.write("x")

    deep_root = os.path.join(workspace, "target", "deep")
    deep = deep_root
    for index in range(15):
        deep = os.path.join(deep, f"depth-{index}-" + "z" * 230)
    os.makedirs(deep)
    with open(os.path.join(deep, "too-deep.dat"), "w", encoding="utf-8") as output:
        output.write("x")

    depth_root = os.path.join(workspace, "target", "depth-check")
    depth_bound = depth_root
    for index in range(65):
        depth_bound = os.path.join(depth_bound, f"d-{index:02}")
    os.makedirs(depth_bound)
    with open(os.path.join(depth_bound, "depth-bound.dat"), "w", encoding="utf-8") as output:
        output.write("x")
    invalid_utf8 = os.path.join(os.fsencode(workspace), b"invalid-\xff.rs")
    with open(invalid_utf8, "wb") as output:
        output.write(b"invalid")

    port = free_port()
    process = subprocess.Popen(
        [RELAY, "relay", "--port", str(port), "--dir", workspace, "--execution-root", workspace,
         "--origin", ORIGIN, "--mode", "local"],
        cwd=ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    try:
        wait_health(port, process)
        url = f"http://127.0.0.1:{port}/mcp"

        status, body = request(url, headers("tools/list"), mcp("tools/list", {}))
        assert status == 200
        tool = next(tool for tool in body["result"]["tools"] if tool["name"] == "file_search")
        schema = tool["inputSchema"]
        assert schema["additionalProperties"] is False
        assert schema["properties"]["pattern"]["maxLength"] == 4096
        assert schema["properties"]["cwd"]["maxLength"] == 4096
        assert schema["properties"]["max_results"]["maximum"] == 100
        assert tool["annotations"] == {
            "readOnlyHint": True, "destructiveHint": False,
            "idempotentHint": True, "openWorldHint": False,
        }
        assert tool["securitySchemes"] == [{"type": "oauth2", "scopes": ["relay.coding"]}]

        result, search = tool_call(url, {"pattern": "Cargo.toml"})
        assert result["isError"] is False
        assert search["matches"] == ["Cargo.toml"] and search["count"] == 1 and search["truncated"] is False

        _, search = tool_call(url, {"pattern": "**/*.rs"})
        expected = [".hidden.rs", ".secret/inside.rs", "contained-dir/not-visible.rs", "ignored.rs", "src/auth_service.rs", "src/lib.rs", "src/nested/auth.rs"]
        assert search["matches"] == expected, search
        assert "CANARY-EXTERNAL-038" not in json.dumps(search)
        assert "generated.rs" not in json.dumps(search)
        assert "nested-generated.rs" not in json.dumps(search)

        _, search = tool_call(url, {"pattern": "**/*auth*"})
        assert search["matches"] == ["src/auth_service.rs", "src/nested/auth.rs"]

        _, search = tool_call(url, {"pattern": "**/lex-*.txt", "max_results": 3})
        assert search["matches"] == [
            "src/nested/lex-03.txt",
            "src/nested/more/lex-01.txt",
            "src/nested/more/lex-02.txt",
        ]

        _, search = tool_call(url, {"pattern": "*.rs", "cwd": workspace})
        assert search["matches"] == expected
        expect_tool_error(url, {"pattern": "*.rs", "cwd": external}, "external absolute cwd")

        _, search = tool_call(url, {"pattern": "aut?.rs", "cwd": "src/nested"})
        assert search["matches"] == ["auth.rs"]

        _, search = tool_call(url, {"pattern": "missing.file"})
        assert search["matches"] == [] and search["count"] == 0 and search["truncated"] is False

        _, search = tool_call(url, {"pattern": "match-*.txt", "cwd": "many", "max_results": 5})
        assert search["matches"] == [f"match-{index:03}.txt" for index in range(5)]
        assert search["count"] == 5 and search["truncated"] is True

        _, search = tool_call(url, {"pattern": "ignored.rs"})
        assert search["matches"] == ["ignored.rs"], ".gitignore must not be silently implemented"

        _, search = tool_call(url, {"pattern": "**/*.result", "cwd": os.path.relpath(wide_root, workspace)})
        assert search["truncated"] is True and len(search["matches"]) < 100
        assert len(json.dumps(search).encode()) <= 256 * 1024

        _, search = tool_call(url, {"pattern": "**/*.rs"})
        assert "contained-dir-link/not-visible.rs" not in json.dumps(search)
        assert "contained-file-link.rs" not in json.dumps(search)
        assert "named-pipe" not in json.dumps(search)
        assert "invalid-" not in json.dumps(search)

        # Exercise replacement while traversal is in progress. A secure
        # result may omit the swapped directory or report an operation-time
        # error, but it must never contain the external canary.
        race_safe = os.path.join(workspace, "race-safe")
        race_backup = os.path.join(workspace, "race-safe-backup")
        race_link = os.path.join(workspace, "race-dir")
        race_external = os.path.join(external, "race-external")
        os.makedirs(os.path.join(race_safe, "nested"))
        os.makedirs(race_external)
        with open(os.path.join(race_external, "CANARY-SWAP-EXTERNAL.rs"), "w", encoding="utf-8") as output:
            output.write("external")
        os.rename(race_safe, race_link)
        swapping = True

        def swap_race_path():
            for _ in range(200):
                if not swapping:
                    break
                os.rename(race_link, race_backup)
                os.symlink(race_external, race_link)
                os.unlink(race_link)
                os.rename(race_backup, race_link)

        swap_thread = threading.Thread(target=swap_race_path)
        swap_thread.start()
        try:
            for _ in range(8):
                _, payload = tool_call_raw(url, {"pattern": "**/CANARY-SWAP-EXTERNAL.rs"})
                if payload is not None:
                    assert all("CANARY-SWAP-EXTERNAL" not in match for match in payload["matches"])
        finally:
            swapping = False
            swap_thread.join(timeout=5)
        assert os.path.isdir(race_link)

        expect_tool_error(url, {"pattern": "../*.rs"}, "parent-segment pattern")
        expect_tool_error(url, {"pattern": "/tmp/*.rs"}, "absolute pattern")
        expect_tool_error(url, {"pattern": "*.rs", "cwd": "../"}, "cwd traversal")
        expect_tool_error(url, {"pattern": "a" * 4097}, "oversized direct pattern")
        expect_tool_error(url, {"pattern": "*.rs", "cwd": "x" * 4097}, "oversized direct cwd")
        expect_tool_error(url, {"pattern": "/".join(["x"] * 129)}, "pathological pattern depth")
        expect_tool_error(url, {"pattern": "x" * 256}, "pathological pattern segment")
        expect_tool_error(url, {"pattern": "*.dat", "cwd": os.path.relpath(deep_root, workspace)}, "pathological path/depth")
        expect_tool_error(url, {"pattern": "**/*.dat", "cwd": os.path.relpath(depth_root, workspace)}, "pathological traversal depth")
        expect_tool_error(url, {"pattern": "*.dat", "cwd": "target/scan-cap"}, "per-directory scan cap")
        total_error = expect_tool_error(url, {"pattern": "*.never", "cwd": "target/total-cap"}, "total traversal cap")
        assert "traversal exceeds maximum" in json.dumps(total_error.get("content", []))

        invalid = mcp("tools/call", {
            "name": "file_search",
            "arguments": {"pattern": "*.rs", "extra": True},
        }, 90)
        status, body = request(url, headers("tools/call", "file_search"), invalid)
        assert status == 400 and body["error"]["code"] == -32602

        print("file_search MCP acceptance: PASS")
    finally:
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=5)
PY
