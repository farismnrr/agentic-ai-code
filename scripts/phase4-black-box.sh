#!/usr/bin/env bash
set -euo pipefail

# Phase 4 deterministic black-box conformance harness.  The assertions below
# exercise the built relay over HTTP; source inspection belongs only in the
# structural zero-bypass gate.
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
manifest="$root/Cargo.toml"

command -v cargo >/dev/null
command -v python3 >/dev/null
command -v openssl >/dev/null
command -v bwrap >/dev/null

RUSTFLAGS='-D warnings' cargo build --manifest-path "$manifest" --locked --bin ai-tools

exec python3 - "$root/target/debug/ai-tools" <<'PY'
import base64
import json
import os
import socket
import subprocess
import sys
import tempfile
import threading
import time
import urllib.error
import urllib.request
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer


RELAY = sys.argv[1]
PROTOCOL = "2026-07-28"
AUDIENCE = "https://relay.example/mcp"
ORIGIN = "http://localhost:3333"


def b64(value):
    return base64.urlsafe_b64encode(value).rstrip(b"=").decode("ascii")


def der_tlv(data, offset=0):
    tag = data[offset]
    offset += 1
    length = data[offset]
    offset += 1
    if length & 0x80:
        count = length & 0x7F
        length = int.from_bytes(data[offset:offset + count], "big")
        offset += count
    end = offset + length
    return tag, data[offset:end], end


def rsa_jwk(public_der):
    _, spki, _ = der_tlv(public_der)
    _, _, offset = der_tlv(spki)  # algorithm identifier
    _, bit_string, _ = der_tlv(spki, offset)
    _, rsa_body, _ = der_tlv(bit_string, 1)  # first byte is the unused-bit count
    _, modulus, offset = der_tlv(rsa_body)
    _, exponent, _ = der_tlv(rsa_body, offset)
    return {"kty": "RSA", "kid": "fixture-key", "use": "sig", "alg": "RS256",
            "n": b64(modulus.lstrip(b"\x00")), "e": b64(exponent.lstrip(b"\x00"))}


class JwksHandler(BaseHTTPRequestHandler):
    jwks = b""
    discovery = b""

    def do_GET(self):
        if self.path == "/.well-known/jwks.json":
            payload = self.jwks
        elif self.path == "/.well-known/openid-configuration":
            payload = self.discovery
        else:
            self.send_response(404)
            self.end_headers()
            return
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(payload)))
        self.end_headers()
        self.wfile.write(payload)

    def log_message(self, *_args):
        pass


def free_port():
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


def request(url, headers=None, body=None):
    data = None if body is None else json.dumps(body).encode()
    req = urllib.request.Request(url, data=data, headers=headers or {}, method="GET" if body is None else "POST")
    try:
        with urllib.request.urlopen(req, timeout=5) as response:
            raw = response.read()
            status = response.status
            response_headers = dict(response.headers.items())
    except urllib.error.HTTPError as error:
        raw = error.read()
        status = error.code
        response_headers = dict(error.headers.items())
    try:
        parsed = json.loads(raw) if raw else None
    except json.JSONDecodeError as error:
        raise AssertionError(f"non-JSON response ({status}): {raw!r}") from error
    return status, response_headers, parsed


def request_after_admission(url, headers=None, body=None):
    deadline = time.monotonic() + 5
    while True:
        status, response_headers, parsed = request(url, headers=headers, body=body)
        if status != 429:
            return status, response_headers, parsed
        if time.monotonic() >= deadline:
            return status, response_headers, parsed
        time.sleep(min(float(response_headers.get("retry-after", "1")),
                       max(0, deadline - time.monotonic())))


def assert_status(actual, expected, label):
    if actual != expected:
        raise AssertionError(f"{label}: expected HTTP {expected}, got {actual}")


def mcp(method, params=None, request_id=1):
    return {"jsonrpc": "2.0", "id": request_id, "method": method,
            "params": params or {"_meta": {
                "io.modelcontextprotocol/protocolVersion": PROTOCOL,
                "io.modelcontextprotocol/clientCapabilities": {},
            }}}


def headers(origin=ORIGIN, protocol=PROTOCOL, method=None, name=None, auth=None, host=None, forwarded=None):
    result = {"Content-Type": "application/json",
              "MCP-Protocol-Version": protocol, "Mcp-Method": method or "server/discover"}
    if origin is not None:
        result["Origin"] = origin
    if name is not None:
        result["Mcp-Name"] = name
    if auth is not None:
        result["Authorization"] = auth
    if host is not None:
        result["Host"] = host
    if forwarded is not None:
        result["X-Forwarded-Proto"] = forwarded
    return result


def wait_for_health(port, process):
    url = f"http://127.0.0.1:{port}/health"
    for _ in range(50):
        if process.poll() is not None:
            stderr = process.stderr.read() if process.stderr is not None else ""
            raise AssertionError(f"relay exited during startup with status {process.returncode}: {stderr.strip()}")
        try:
            status, _, _ = request(url)
            if status == 200:
                return
        except (urllib.error.URLError, ConnectionError):
            pass
        time.sleep(0.1)
    raise AssertionError("relay did not become healthy")


def start_relay(temp_dir, port, issuer=None, trusted_proxy=False, extra_args=()):
    args = [RELAY, "relay", "--port", str(port), "--dir", temp_dir, "--execution-root", temp_dir,
            "--origin", ORIGIN]
    args += list(extra_args)
    if issuer is None:
        args += ["--mode", "local"]
    else:
        args += ["--mode", "remote", "--oauth-issuer", issuer, "--oauth-audience", AUDIENCE,
                 "--oauth-owner-subject", "owner"]
        if trusted_proxy:
            args += ["--trusted-proxy", "--trusted-proxy-cidr", "127.0.0.1/32"]
    fixture_env = os.environ.copy()
    fixture_env["RELAY_AGENT_ALLOW_INSECURE_OAUTH_ISSUER_FIXTURE"] = "1"
    process = subprocess.Popen(args, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
                               env=fixture_env)
    wait_for_health(port, process)
    return process


def stop_relay(process):
    process.terminate()
    try:
        process.wait(timeout=5)
    except subprocess.TimeoutExpired:
        process.kill()
        process.wait(timeout=5)


def make_token(key_path, issuer, scope, subject="owner", expires_in=600, include_typ=True, kid="fixture-key", audience=AUDIENCE):
    now = int(time.time())
    header_fields = {"alg": "RS256", "kid": kid}
    if include_typ:
        header_fields["typ"] = "JWT"
    header = b64(json.dumps(header_fields, separators=(",", ":")).encode())
    payload = b64(json.dumps({"iss": issuer, "aud": audience, "sub": subject,
                              "scope": scope, "iat": now - 120, "exp": now + expires_in}, separators=(",", ":")).encode())
    signing_input = f"{header}.{payload}".encode()
    with tempfile.NamedTemporaryFile() as input_file, tempfile.NamedTemporaryFile() as signature_file:
        input_file.write(signing_input)
        input_file.flush()
        subprocess.run(["openssl", "dgst", "-sha256", "-sign", key_path, "-out", signature_file.name,
                        input_file.name], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        signature_file.seek(0)
        return f"{header}.{payload}.{b64(signature_file.read())}"


def run():
    with tempfile.TemporaryDirectory(prefix="relay-phase4-") as temp_dir:
        key_path = os.path.join(temp_dir, "fixture-key.pem")
        public_path = os.path.join(temp_dir, "fixture-key.der")
        subprocess.run(["openssl", "genrsa", "-out", key_path, "2048"], check=True,
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        with open(public_path, "wb") as output:
            subprocess.run(["openssl", "rsa", "-in", key_path, "-pubout", "-outform", "DER"],
                           check=True, stdout=output, stderr=subprocess.DEVNULL)
        JwksHandler.jwks = json.dumps({"keys": [rsa_jwk(open(public_path, "rb").read())]}).encode()
        idp = ThreadingHTTPServer(("127.0.0.1", free_port()), JwksHandler)
        idp_thread = threading.Thread(target=idp.serve_forever, daemon=True)
        idp_thread.start()
        issuer = f"http://127.0.0.1:{idp.server_port}"
        JwksHandler.discovery = json.dumps({
            "issuer": issuer,
            "jwks_uri": f"{issuer}/.well-known/jwks.json",
        }).encode()
        local = remote_untrusted = remote = None
        try:
            local_port = free_port()
            local = start_relay(temp_dir, local_port, extra_args=("--max-running-jobs", "1"))
            local_url = f"http://127.0.0.1:{local_port}/mcp"
            valid_discover = mcp("server/discover")
            initialize_request = {"jsonrpc": "2.0", "id": 1, "method": "initialize",
                                   "params": {"protocolVersion": "2025-03-26", "capabilities": {},
                                              "clientInfo": {"name": "external MCP client", "version": "test"}}}
            status, _, body = request(local_url, headers=headers(method="initialize"), body=initialize_request)
            assert_status(status, 200, "legacy initialize")
            assert body["result"]["protocolVersion"] == "2025-03-26"

            legacy_tools_list_request = {"jsonrpc": "2.0", "id": 2, "method": "tools/list"}
            legacy_headers = {"Content-Type": "application/json", "Origin": ORIGIN,
                              "MCP-Protocol-Version": "2025-03-26", "Mcp-Method": "tools/list"}
            status, _, body = request(local_url, headers=legacy_headers, body=legacy_tools_list_request)
            assert_status(status, 200, "legacy tools/list without modern headers")
            legacy_tools = body["result"]["tools"]
            assert isinstance(legacy_tools, list) and legacy_tools
            assert any(tool["name"] == "terminal_exec" for tool in legacy_tools)

            status, response_headers, body = request(local_url, headers=headers(), body=valid_discover)
            assert_status(status, 200, "local server/discover")
            assert body["result"]["supportedVersions"] == [PROTOCOL]
            instructions = body["result"]["instructions"]
            assert "Local" not in instructions
            assert "Plan 0" not in instructions
            assert response_headers.get("x-request-id")

            status, _, body = request(local_url, headers=headers(origin=None), body=valid_discover)
            assert_status(status, 200, "missing Origin for non-browser MCP client")
            assert body["result"]["supportedVersions"] == [PROTOCOL]

            status, _, _ = request(local_url, headers=headers(host="evil.example:9999"), body=valid_discover)
            assert_status(status, 403, "untrusted Host")

            status, _, body = request(local_url, headers=headers(method="server/discover"),
                                      body=mcp("server/discover", {"_meta": {}}))
            assert_status(status, 400, "missing MCP metadata")
            assert body["error"]["code"] == -32020

            invalid_call = mcp("tools/call", {"name": "terminal_exec", "arguments": {"command": "true", "extra": True},
                                               "_meta": {"io.modelcontextprotocol/protocolVersion": PROTOCOL,
                                                         "io.modelcontextprotocol/clientCapabilities": {}}})
            status, _, body = request_after_admission(local_url, headers=headers(method="tools/call", name="terminal_exec"), body=invalid_call)
            assert_status(status, 400, "invalid tool schema")
            assert body["error"]["code"] == -32602

            call_meta = {"io.modelcontextprotocol/protocolVersion": PROTOCOL,
                         "io.modelcontextprotocol/clientCapabilities": {}}
            for command, expected_error in (("true", False), ("false", True)):
                tool_call = mcp("tools/call", {
                    "name": "terminal_exec",
                    "arguments": {"command": command},
                    "_meta": call_meta,
                })
                status, _, body = request(
                    local_url,
                    headers=headers(method="tools/call", name="terminal_exec"),
                    body=tool_call,
                )
                assert_status(status, 200, f"terminal_exec {command}")
                assert "error" not in body, f"terminal_exec {command} returned a JSON-RPC error"
                result = body["result"]
                assert result["resultType"] == "complete"
                assert result["isError"] is expected_error
                assert isinstance(result["content"], list) and result["content"]
                assert "io.modelcontextprotocol/serverInfo" in result["_meta"]

            def fallback_job(name, arguments, request_id):
                body = mcp("tools/call", {
                    "name": name,
                    "arguments": arguments,
                    "_meta": call_meta,
                }, request_id=request_id)
                status, _, response = request_after_admission(
                    local_url,
                    headers=headers(method="tools/call", name=name),
                    body=body,
                )
                assert_status(status, 200, name)
                result = response["result"]
                assert result["resultType"] == "complete" and result["isError"] is False
                text = next(item["text"] for item in result["content"] if item.get("type") == "text")
                return json.loads(text)

            first_job = fallback_job(
                "terminal_job_start",
                {"command": "sh", "args": ["-c", "sleep 5"], "timeout_ms": 0},
                20,
            )
            first_id = first_job["taskId"]
            deadline = time.monotonic() + 2
            while True:
                first_snapshot = fallback_job("terminal_job_get", {"taskId": first_id}, 21)
                if first_snapshot["status"] == "working":
                    break
                if time.monotonic() >= deadline:
                    raise AssertionError(f"first fallback job did not start: {first_snapshot}")
                time.sleep(0.05)

            queued_job = fallback_job("terminal_job_start", {"command": "true"}, 22)
            queued_id = queued_job["taskId"]
            queued_snapshot = fallback_job("terminal_job_get", {"taskId": queued_id}, 23)
            assert queued_snapshot["status"] == "queued", queued_snapshot
            fallback_job("terminal_job_cancel", {"taskId": queued_id}, 24)

            deadline = time.monotonic() + 2
            while True:
                cancelled_snapshot = fallback_job("terminal_job_get", {"taskId": queued_id}, 25)
                if cancelled_snapshot["status"] == "cancelled":
                    break
                if time.monotonic() >= deadline:
                    raise AssertionError(
                        f"queued job cancellation waited for a semaphore permit: {cancelled_snapshot}"
                    )
                time.sleep(0.05)

            first_snapshot = fallback_job("terminal_job_get", {"taskId": first_id}, 26)
            assert first_snapshot["status"] == "working", first_snapshot
            fallback_job("terminal_job_cancel", {"taskId": first_id}, 27)

            dispatch_marker = os.path.join(temp_dir, "rejected-dispatch-marker")

            forbidden_call = mcp("tools/call", {
                "name": "terminal_exec",
                "arguments": {"command": "sudo"},
                "_meta": call_meta,
            })
            status, _, body = request(
                local_url,
                headers=headers(method="tools/call", name="terminal_exec"),
                body=forbidden_call,
            )
            assert_status(status, 200, "forbidden executable tools/call")
            result = body["result"]
            assert result["resultType"] == "complete" and result["isError"] is True
            assert "privilege escalation" in json.dumps(result["content"])

            path_binary_call = mcp("tools/call", {
                "name": "terminal_exec",
                "arguments": {"command": "bin/tool"},
                "_meta": call_meta,
            })
            status, _, body = request(
                local_url,
                headers=headers(method="tools/call", name="terminal_exec"),
                body=path_binary_call,
            )
            assert_status(status, 200, "path-qualified executable tools/call")
            result = body["result"]
            assert result["resultType"] == "complete" and result["isError"] is True
            assert "path traversal" in json.dumps(result["content"])

            cwd_escape_call = mcp("tools/call", {
                "name": "terminal_exec",
                "arguments": {"command": "true", "cwd": "../"},
                "_meta": call_meta,
            })
            status, _, body = request(
                local_url,
                headers=headers(method="tools/call", name="terminal_exec"),
                body=cwd_escape_call,
            )
            assert_status(status, 200, "cwd traversal tools/call")
            result = body["result"]
            assert result["resultType"] == "complete" and result["isError"] is True
            assert "path traversal" in json.dumps(result["content"]) or "does not exist" in json.dumps(result["content"])

            untrusted_port = free_port()
            remote_untrusted = start_relay(temp_dir, untrusted_port, issuer=issuer)
            untrusted_url = f"http://127.0.0.1:{untrusted_port}/mcp"
            status, _, body = request(untrusted_url,
                                      headers=headers(method="server/discover", forwarded="https"), body=valid_discover)
            assert_status(status, 403, "spoofed forwarded HTTPS without trust")
            assert body["error"]["message"] == "Invalid request"

            remote_port = free_port()
            remote = start_relay(temp_dir, remote_port, issuer=issuer, trusted_proxy=True)
            remote_url = f"http://127.0.0.1:{remote_port}/mcp"
            metadata_status, _, metadata = request(f"http://127.0.0.1:{remote_port}/.well-known/oauth-protected-resource")
            assert_status(metadata_status, 200, "protected resource metadata")
            assert metadata["resource"] == AUDIENCE
            assert metadata["scopes_supported"] == ["relay.coding"]
            path_metadata_status, _, path_metadata = request(
                f"http://127.0.0.1:{remote_port}/.well-known/oauth-protected-resource/mcp")
            assert_status(path_metadata_status, 200, "path-derived protected resource metadata")
            assert path_metadata == metadata

            status, challenge_headers, body = request(remote_url,
                headers=headers(method="server/discover", forwarded="https"), body=valid_discover)
            assert_status(status, 401, "missing bearer token")
            challenge = challenge_headers.get("www-authenticate", "")
            assert challenge.startswith("Bearer ") and 'resource_metadata="https://relay.example/.well-known/oauth-protected-resource/mcp"' in challenge
            assert "offline_access" not in challenge
            assert body["error"]["code"] == -32600

            auth_call = lambda marker: mcp("tools/call", {
                "name": "terminal_exec",
                "arguments": {"command": "touch", "args": [marker]},
                "_meta": {"io.modelcontextprotocol/protocolVersion": PROTOCOL,
                          "io.modelcontextprotocol/clientCapabilities": {}},
            })
            status, _, body = request(remote_url,
                headers=headers(method="tools/call", name="terminal_exec", forwarded="https"),
                body=auth_call(dispatch_marker))
            assert_status(status, 200, "missing bearer tools/call challenge")
            result = body["result"]
            assert result["resultType"] == "complete" and result["isError"] is True
            assert isinstance(result["content"], list) and result["content"]
            assert "io.modelcontextprotocol/serverInfo" in result["_meta"]
            missing_challenges = result["_meta"]["mcp/www_authenticate"]
            assert isinstance(missing_challenges, list) and len(missing_challenges) == 1
            missing_challenge = missing_challenges[0]
            assert 'error="invalid_token"' in missing_challenge
            assert 'error_description="Authentication is required"' not in missing_challenge
            assert "error_description=" in missing_challenge
            assert 'resource_metadata="https://relay.example/.well-known/oauth-protected-resource/mcp"' in missing_challenge
            assert not os.path.exists(dispatch_marker), "missing auth reached tool execution"

            workspace_auth_edit = os.path.join(temp_dir, "workspace-auth-edit.txt")
            workspace_auth_read = os.path.join(temp_dir, "workspace-auth-read.txt")
            with open(workspace_auth_edit, "w", encoding="utf-8") as output:
                output.write("before")
            with open(workspace_auth_read, "w", encoding="utf-8") as output:
                output.write("read-canary")
            workspace_call_meta = {"io.modelcontextprotocol/protocolVersion": PROTOCOL,
                                   "io.modelcontextprotocol/clientCapabilities": {}}
            workspace_auth_calls = {
                "directory_list": {"path": "."},
                "file_search": {"pattern": "**/*"},
                "text_search": {"query": "read-canary"},
                "file_read": {"path": "workspace-auth-read.txt"},
                "file_edit": {"path": "workspace-auth-edit.txt", "old_text": "before", "new_text": "after"},
                "file_write": {"path": "workspace-auth-write.txt", "content": "written"},
            }
            for index, (workspace_name, workspace_arguments) in enumerate(workspace_auth_calls.items(), start=60):
                status, _, body = request_after_admission(
                    remote_url,
                    headers=headers(method="tools/call", name=workspace_name, forwarded="https"),
                    body=mcp("tools/call", {"name": workspace_name, "arguments": workspace_arguments,
                                             "_meta": workspace_call_meta}, request_id=index),
                )
                assert_status(status, 200, f"missing bearer workspace tool {workspace_name}")
                result = body["result"]
                assert result["resultType"] == "complete" and result["isError"] is True
                challenges = result["_meta"]["mcp/www_authenticate"]
                assert isinstance(challenges, list) and len(challenges) == 1
                assert 'error="invalid_token"' in challenges[0]
            assert open(workspace_auth_edit, encoding="utf-8").read() == "before"
            assert not os.path.exists(os.path.join(temp_dir, "workspace-auth-write.txt"))

            status, challenge_headers, body = request(remote_url,
                headers=headers(method="server/discover", forwarded="https", auth="Bearer malformed"), body=valid_discover)
            assert_status(status, 401, "invalid bearer token")
            assert 'error="invalid_token"' in challenge_headers.get("www-authenticate", "")
            assert body["error"]["code"] == -32600

            malformed_http_challenge = challenge_headers.get("www-authenticate", "")
            assert missing_challenge == malformed_http_challenge
            status, _, body = request(remote_url,
                headers=headers(method="tools/call", name="terminal_exec", forwarded="https", auth="Bearer malformed"),
                body=auth_call(dispatch_marker))
            assert_status(status, 401, "malformed bearer tools/call")
            assert body["error"]["code"] == -32600
            assert not os.path.exists(dispatch_marker), "malformed bearer reached tool execution"

            for malformed, label in (("a.b.c", "malformed base64 bearer"),
                                     (f"{b64(b'{not-json}')}.e30.signature", "malformed header bearer")):
                status, _, body = request(remote_url,
                    headers=headers(method="server/discover", forwarded="https", auth=f"Bearer {malformed}"), body=valid_discover)
                assert_status(status, 401, label)
                assert body["error"]["code"] == -32600

            for include_typ, label in ((True, "valid bearer with typ"), (False, "valid bearer without typ")):
                token = make_token(key_path, issuer, "relay.coding", include_typ=include_typ)
                status, _, body = request(remote_url,
                    headers=headers(method="server/discover", forwarded="https", auth=f"Bearer {token}"), body=valid_discover)
                assert_status(status, 200, label)

            bad_signature = make_token(key_path, issuer, "relay.coding")[:-1] + ("A" if make_token(key_path, issuer, "relay.coding")[-1] != "A" else "B")
            status, _, body = request(remote_url,
                headers=headers(method="server/discover", forwarded="https", auth=f"Bearer {bad_signature}"), body=valid_discover)
            assert_status(status, 401, "bad signature bearer")

            for token, label in ((make_token(key_path, issuer + "/wrong", "relay.coding"), "wrong issuer"),
                                 (make_token(key_path, issuer, "relay.coding", audience="https://wrong.example/mcp"), "wrong audience")):
                status, _, body = request(remote_url,
                    headers=headers(method="server/discover", forwarded="https", auth=f"Bearer {token}"), body=valid_discover)
                assert_status(status, 401, label)

            expired = make_token(key_path, issuer, "relay.coding", expires_in=-120)
            status, challenge_headers, body = request(remote_url,
                headers=headers(method="server/discover", forwarded="https", auth=f"Bearer {expired}"), body=valid_discover)
            assert_status(status, 401, "expired bearer token")
            assert 'error="invalid_token"' in challenge_headers.get("www-authenticate", "")
            assert body["error"]["code"] == -32600

            status, _, body = request(remote_url,
                headers=headers(method="tools/call", name="terminal_exec", forwarded="https", auth=f"Bearer {expired}"),
                body=auth_call(dispatch_marker))
            assert_status(status, 401, "expired bearer tools/call")
            assert body["error"]["code"] == -32600
            assert not os.path.exists(dispatch_marker), "expired bearer reached tool execution"

            no_scope = make_token(key_path, issuer, "openid")
            call_params = {"name": "terminal_exec", "arguments": {"command": "true"},
                           "_meta": {"io.modelcontextprotocol/protocolVersion": PROTOCOL,
                                     "io.modelcontextprotocol/clientCapabilities": {}}}
            status, challenge_headers, body = request(remote_url,
                headers=headers(method="server/discover", forwarded="https", auth=f"Bearer {no_scope}"),
                body=valid_discover)
            assert_status(status, 403, "missing relay.coding scope on protected resource")
            insufficient_scope_http_challenge = challenge_headers.get("www-authenticate", "")
            assert 'error="insufficient_scope"' in insufficient_scope_http_challenge
            assert 'scope="relay.coding"' in insufficient_scope_http_challenge

            status, challenge_headers, body = request(remote_url,
                headers=headers(method="tools/call", name="terminal_exec", forwarded="https", auth=f"Bearer {no_scope}"),
                body=mcp("tools/call", {"name": "terminal_exec", "arguments": {"command": "touch", "args": [dispatch_marker]},
                                         "_meta": call_params["_meta"]}))
            assert_status(status, 200, "missing relay.coding scope tools/call challenge")
            assert "www-authenticate" not in challenge_headers
            result = body["result"]
            assert result["resultType"] == "complete" and result["isError"] is True
            assert isinstance(result["content"], list) and result["content"]
            assert "io.modelcontextprotocol/serverInfo" in result["_meta"]
            scope_challenges = result["_meta"]["mcp/www_authenticate"]
            assert isinstance(scope_challenges, list) and len(scope_challenges) == 1
            assert scope_challenges[0] == insufficient_scope_http_challenge
            assert "error_description=" in scope_challenges[0]
            assert not os.path.exists(dispatch_marker), "missing scope reached tool execution"

            for index, (workspace_name, workspace_arguments) in enumerate(workspace_auth_calls.items(), start=70):
                status, _, body = request_after_admission(
                    remote_url,
                    headers=headers(method="tools/call", name=workspace_name, forwarded="https", auth=f"Bearer {no_scope}"),
                    body=mcp("tools/call", {"name": workspace_name, "arguments": workspace_arguments,
                                             "_meta": workspace_call_meta}, request_id=index),
                )
                assert_status(status, 200, f"missing scope workspace tool {workspace_name}")
                result = body["result"]
                assert result["resultType"] == "complete" and result["isError"] is True
                challenges = result["_meta"]["mcp/www_authenticate"]
                assert isinstance(challenges, list) and len(challenges) == 1
                assert 'error="insufficient_scope"' in challenges[0]
                assert 'scope="relay.coding"' in challenges[0]
            assert open(workspace_auth_edit, encoding="utf-8").read() == "before"
            assert not os.path.exists(os.path.join(temp_dir, "workspace-auth-write.txt"))

            invalid_dispatch = {"name": "terminal_exec", "arguments": {"command": "touch", "args": [dispatch_marker], "extra": True},
                                "_meta": call_params["_meta"]}
            status, _, body = request(remote_url,
                headers=headers(method="tools/call", name="terminal_exec", forwarded="https", auth=f"Bearer {make_token(key_path, issuer, 'relay.coding')}"),
                body=mcp("tools/call", invalid_dispatch))
            assert_status(status, 400, "invalid tool arguments before dispatch")
            assert body["error"]["code"] == -32602
            assert not os.path.exists(dispatch_marker), "invalid tool arguments reached execution"

            status, _, body = request(remote_url,
                headers=headers(method="server/discover", forwarded="http"), body=valid_discover)
            assert_status(status, 403, "trusted proxy rejects non-https forwarded scheme")
            assert body["error"]["message"] == "Invalid request"

            wrong_owner = make_token(key_path, issuer, "relay.coding", subject="other")
            status, challenge_headers, body = request(remote_url,
                headers=headers(method="tools/call", name="terminal_exec", forwarded="https", auth=f"Bearer {wrong_owner}"),
                body=auth_call(dispatch_marker))
            assert_status(status, 403, "wrong owner")
            assert body["error"]["message"] == "Invalid request"
            assert "other" not in body["error"]["message"]
            assert not os.path.exists(dispatch_marker), "wrong owner reached tool execution"
        finally:
            for process in (remote, remote_untrusted, local):
                if process is not None:
                    stop_relay(process)
            idp.shutdown()
            idp.server_close()


try:
    run()
except Exception as error:
    print(f"phase4 black-box conformance: FAIL: {error}", file=sys.stderr)
    raise
else:
    print("phase4 black-box conformance: pass")
PY
