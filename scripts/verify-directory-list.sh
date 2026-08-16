#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
manifest="$root/Cargo.toml"
command -v cargo >/dev/null
command -v python3 >/dev/null
command -v bwrap >/dev/null

RUSTFLAGS='-D warnings' cargo build --manifest-path "$manifest" --locked --bin ai-tools

exec python3 - "$root/target/debug/ai-tools" <<'PY'
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

RELAY = sys.argv[1]
PROTOCOL = "2026-07-28"
ORIGIN = "http://localhost:3333"


def free_port():
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


def request(url, headers, body):
    req = urllib.request.Request(
        url,
        data=json.dumps(body).encode(),
        headers=headers,
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=5) as response:
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
    if name is not None:
        result["Mcp-Name"] = name
    return result


def meta():
    return {
        "io.modelcontextprotocol/protocolVersion": PROTOCOL,
        "io.modelcontextprotocol/clientCapabilities": {},
    }


def mcp(method, params=None, request_id=1):
    return {
        "jsonrpc": "2.0",
        "id": request_id,
        "method": method,
        "params": params if params is not None else {"_meta": meta()},
    }


def wait_for_health(port, process):
    url = f"http://127.0.0.1:{port}/health"
    for _ in range(50):
        if process.poll() is not None:
            raise AssertionError(f"relay exited early: {process.stderr.read().strip()}")
        try:
            with urllib.request.urlopen(url, timeout=1) as response:
                if response.status == 200:
                    return
        except (urllib.error.URLError, ConnectionError):
            pass
        time.sleep(0.1)
    raise AssertionError("relay did not become healthy")


def tool_call(url, arguments, request_id=10):
    body = mcp(
        "tools/call",
        {"name": "directory_list", "arguments": arguments, "_meta": meta()},
        request_id,
    )
    status, response = request(
        url,
        headers("tools/call", "directory_list"),
        body,
    )
    return status, response


def parse_listing(response):
    result = response["result"]
    assert result["resultType"] == "complete"
    assert result["isError"] is False, result
    assert "io.modelcontextprotocol/serverInfo" in result["_meta"]
    text = next(item["text"] for item in result["content"] if item.get("type") == "text")
    return json.loads(text)


def expect_tool_error(url, arguments, label):
    status, response = tool_call(url, arguments)
    assert status == 200, (label, status, response)
    result = response["result"]
    assert result["isError"] is True, (label, result)
    serialized = json.dumps(result)
    assert "/tmp/" not in serialized, (label, serialized)
    return result


with tempfile.TemporaryDirectory(prefix="relay-directory-list-") as base:
    workspace = os.path.join(base, "workspace")
    external = os.path.join(base, "external")
    os.makedirs(os.path.join(workspace, "tree", "a-dir"))
    os.makedirs(os.path.join(workspace, "tree", "c-dir"))
    os.makedirs(os.path.join(workspace, "empty"))
    os.makedirs(os.path.join(workspace, "huge"))
    os.makedirs(os.path.join(workspace, "scan-cap"))
    long_output = os.path.join(workspace, "long-output")
    long_level = long_output
    for index in range(3):
        long_level = os.path.join(long_level, f"level-{index}-" + "l" * 220)
        os.makedirs(long_level)
    for index in range(100):
        with open(os.path.join(long_level, f"entry-{index:03}-" + "x" * 220), "w", encoding="utf-8") as output:
            output.write("x")
    os.makedirs(os.path.join(external, "secret-dir"))
    with open(os.path.join(workspace, "tree", "a-dir", "a.txt"), "w", encoding="utf-8") as f:
        f.write("a\n")
    with open(os.path.join(workspace, "tree", "b.txt"), "w", encoding="utf-8") as f:
        f.write("b\n")
    with open(os.path.join(workspace, "tree", "c-dir", "c.txt"), "w", encoding="utf-8") as f:
        f.write("c\n")
    with open(os.path.join(external, "secret-dir", "EXTERNAL-CANARY-038"), "w", encoding="utf-8") as f:
        f.write("must never be listed\n")
    with open(os.path.join(external, "secret.txt"), "w", encoding="utf-8") as f:
        f.write("external\n")
    for index in range(105):
        with open(os.path.join(workspace, "huge", f"entry-{index:03}.txt"), "w", encoding="utf-8") as f:
            f.write("x")
    for index in range(4097):
        with open(os.path.join(workspace, "scan-cap", f"entry-{index:04}.txt"), "w", encoding="utf-8") as f:
            f.write("x")
    os.symlink(os.path.join(external, "secret-dir"), os.path.join(workspace, "tree", "x-external-dir"))
    os.symlink(os.path.join(external, "secret.txt"), os.path.join(workspace, "tree", "y-external-file"))
    os.symlink("loop-b", os.path.join(workspace, "tree", "loop-a"))
    os.symlink("loop-a", os.path.join(workspace, "tree", "loop-b"))
    invalid_utf8 = os.path.join(os.fsencode(workspace), b"tree/invalid-\xff")
    with open(invalid_utf8, "wb") as output:
        output.write(b"invalid")

    port = free_port()
    process = subprocess.Popen(
        [
            RELAY,
            "relay",
            "--port",
            str(port),
            "--dir",
            workspace,
            "--execution-root",
            workspace,
            "--origin",
            ORIGIN,
            "--mode",
            "local",
        ],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    try:
        wait_for_health(port, process)
        url = f"http://127.0.0.1:{port}/mcp"

        status, response = request(url, headers("tools/list"), mcp("tools/list"))
        assert status == 200, response
        tool = next(item for item in response["result"]["tools"] if item["name"] == "directory_list")
        assert tool["annotations"] == {
            "readOnlyHint": True,
            "destructiveHint": False,
            "idempotentHint": True,
            "openWorldHint": False,
        }
        assert tool["securitySchemes"] == [{"type": "oauth2", "scopes": ["relay.coding"]}]
        schema = tool["inputSchema"]
        assert schema["additionalProperties"] is False
        assert schema["properties"]["path"]["default"] == "."
        assert schema["properties"]["depth"] == {
            "type": "integer", "minimum": 0, "maximum": 4, "default": 2
        }
        assert schema["properties"]["max_entries"] == {
            "type": "integer", "minimum": 1, "maximum": 100, "default": 100
        }

        status, response = tool_call(url, {"path": "tree", "depth": 1})
        assert status == 200
        listing = parse_listing(response)
        expected_direct = [
            "a-dir", "b.txt", "c-dir", "loop-a", "loop-b", "x-external-dir", "y-external-file"
        ]
        assert [entry["path"] for entry in listing["entries"]] == expected_direct, listing
        types = {entry["path"]: entry["type"] for entry in listing["entries"]}
        assert types["a-dir"] == "directory"
        assert types["b.txt"] == "file"
        assert types["loop-a"] == "symlink"
        assert types["x-external-dir"] == "symlink"
        assert listing["truncated"] is False
        assert "invalid-" not in json.dumps(listing), "non-UTF-8 names must be omitted, not lossily renamed"

        status, response = tool_call(url, {"path": "tree", "depth": 2})
        assert status == 200
        listing = parse_listing(response)
        expected_nested = [
            "a-dir", "a-dir/a.txt", "b.txt", "c-dir", "c-dir/c.txt",
            "loop-a", "loop-b", "x-external-dir", "y-external-file"
        ]
        assert [entry["path"] for entry in listing["entries"]] == expected_nested, listing
        assert "EXTERNAL-CANARY-038" not in json.dumps(listing)

        # Stress a directory-to-external-symlink swap while recursive listing runs.
        # Stable no-follow directory descriptors may make the call succeed or fail,
        # but the external sentinel must never become visible.
        race_safe = os.path.join(workspace, "race-dir")
        race_backup = os.path.join(workspace, "race-dir-backup")
        race_external = os.path.join(external, "race-external")
        os.makedirs(os.path.join(race_safe, "nested"))
        os.makedirs(race_external)
        with open(os.path.join(race_external, "CANARY-RACE-DIR-038"), "w", encoding="utf-8") as output:
            output.write("external")
        swapping = True

        def swap_race_path():
            for _ in range(200):
                if not swapping:
                    break
                try:
                    os.rename(race_safe, race_backup)
                    os.symlink(race_external, race_safe)
                    os.unlink(race_safe)
                    os.rename(race_backup, race_safe)
                except FileNotFoundError:
                    pass

        swap_thread = threading.Thread(target=swap_race_path)
        swap_thread.start()
        try:
            for request_id in range(30, 38):
                status, response = tool_call(url, {"path": ".", "depth": 3}, request_id)
                assert status == 200
                result = response["result"]
                serialized = json.dumps(result)
                assert "CANARY-RACE-DIR-038" not in serialized, serialized
        finally:
            swapping = False
            swap_thread.join(timeout=5)
        assert os.path.isdir(race_safe)

        status, response = tool_call(url, {"cwd": "tree", "path": "a-dir", "depth": 1})
        assert status == 200
        listing = parse_listing(response)
        assert [entry["path"] for entry in listing["entries"]] == ["a.txt"]

        status, response = tool_call(url, {"path": os.path.join(workspace, "tree"), "depth": 0})
        assert status == 200
        listing = parse_listing(response)
        assert listing["entries"] == [] and listing["truncated"] is False

        status, response = tool_call(url, {"path": "empty"})
        assert status == 200
        listing = parse_listing(response)
        assert listing["entries"] == [] and listing["truncated"] is False

        status, response = tool_call(url, {"path": "huge", "depth": 1, "max_entries": 3})
        assert status == 200
        listing = parse_listing(response)
        assert [entry["path"] for entry in listing["entries"]] == [
            "entry-000.txt", "entry-001.txt", "entry-002.txt"
        ]
        assert listing["truncated"] is True

        status, response = tool_call(url, {"path": "huge", "depth": 1})
        assert status == 200
        listing = parse_listing(response)
        assert len(listing["entries"]) == 100 and listing["truncated"] is True

        status, response = tool_call(url, {"path": "long-output", "depth": 4})
        assert status == 200
        listing = parse_listing(response)
        assert len(listing["entries"]) == 100 and listing["truncated"] is True
        assert len(json.dumps(listing, separators=(",", ":")).encode()) <= 256 * 1024

        scan_cap_error = expect_tool_error(
            url, {"path": "scan-cap", "depth": 1}, "directory scan hard cap"
        )
        assert "directory scan exceeds maximum" in json.dumps(scan_cap_error["content"])
        assert "entry-4096" not in json.dumps(scan_cap_error)

        expect_tool_error(url, {"path": "missing"}, "missing path")
        expect_tool_error(url, {"path": "tree/b.txt"}, "file as directory")
        expect_tool_error(url, {"path": "tree/x-external-dir"}, "external symlink directory")
        expect_tool_error(url, {"path": "../external"}, "relative root escape")
        expect_tool_error(url, {"path": external}, "absolute root escape")

        for arguments, label in [
            ({"path": "tree", "extra": True}, "unknown property"),
            ({"path": "tree", "depth": 5}, "depth above schema maximum"),
            ({"path": "tree", "max_entries": 101}, "entry count above schema maximum"),
            ({"path": "tree", "depth": "2"}, "wrong depth type"),
        ]:
            status, response = tool_call(url, arguments)
            assert status == 400, (label, status, response)
            assert response["error"]["code"] == -32602, (label, response)

        print("directory_list MCP acceptance: PASS")
    finally:
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=5)
PY
