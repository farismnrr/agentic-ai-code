#!/usr/bin/env bash
set -euo pipefail

# Phase 4 deterministic black-box conformance harness.  The assertions below
# exercise the built relay over HTTP; source inspection belongs only in the
# structural zero-bypass gate.
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
manifest="$root/packages/rust-tools/Cargo.toml"

command -v cargo >/dev/null
command -v python3 >/dev/null
command -v openssl >/dev/null
command -v bwrap >/dev/null

RUSTFLAGS='-D warnings' cargo build --manifest-path "$manifest" --locked \
  --bin relay-agent --bin terminal-tool

exec python3 - "$root/target/debug/relay-agent" <<'PY'
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

    def do_GET(self):
        if self.path != "/.well-known/jwks.json":
            self.send_response(404)
            self.end_headers()
            return
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(self.jwks)))
        self.end_headers()
        self.wfile.write(self.jwks)

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


def start_relay(temp_dir, port, issuer=None, trusted_proxy=False):
    args = [RELAY, "--port", str(port), "--dir", temp_dir, "--execution-root", temp_dir,
            "--origin", ORIGIN]
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


def make_token(key_path, issuer, scope, subject="owner", expires_in=600):
    now = int(time.time())
    header = b64(json.dumps({"alg": "RS256", "kid": "fixture-key", "typ": "JWT"}, separators=(",", ":")).encode())
    payload = b64(json.dumps({"iss": issuer, "aud": AUDIENCE, "sub": subject,
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
        local = remote_untrusted = remote = None
        try:
            local_port = free_port()
            local = start_relay(temp_dir, local_port)
            local_url = f"http://127.0.0.1:{local_port}/mcp"
            valid_discover = mcp("server/discover")
            status, response_headers, body = request(local_url, headers=headers(), body=valid_discover)
            assert_status(status, 200, "local server/discover")
            assert body["result"]["supportedVersions"] == [PROTOCOL]
            assert response_headers.get("x-correlation-id")

            status, _, body = request(local_url, headers=headers(origin=None), body=valid_discover)
            assert_status(status, 403, "missing Origin")
            assert body["error"]["code"] == -32600

            status, _, _ = request(local_url, headers=headers(host="evil.example:9999"), body=valid_discover)
            assert_status(status, 403, "untrusted Host")

            status, _, body = request(local_url, headers=headers(method="server/discover"),
                                      body=mcp("server/discover", {"_meta": {}}))
            assert_status(status, 400, "missing MCP metadata")
            assert body["error"]["code"] == -32020

            invalid_call = mcp("tools/call", {"name": "terminal_exec", "arguments": {"command": "true", "extra": True},
                                               "_meta": {"io.modelcontextprotocol/protocolVersion": PROTOCOL,
                                                         "io.modelcontextprotocol/clientCapabilities": {}}})
            status, _, body = request(local_url, headers=headers(method="tools/call", name="terminal_exec"), body=invalid_call)
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

            dispatch_marker = os.path.join(temp_dir, "rejected-dispatch-marker")

            untrusted_port = free_port()
            remote_untrusted = start_relay(temp_dir, untrusted_port, issuer=issuer)
            untrusted_url = f"http://127.0.0.1:{untrusted_port}/mcp"
            status, _, body = request(untrusted_url,
                                      headers=headers(method="server/discover", forwarded="https"), body=valid_discover)
            assert_status(status, 403, "spoofed forwarded HTTPS without trust")
            assert "requires HTTPS" in body["error"]["message"]

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

            invalid_dispatch = {"name": "terminal_exec", "arguments": {"command": "touch", "args": [dispatch_marker], "extra": True},
                                "_meta": call_params["_meta"]}
            status, _, body = request(remote_url,
                headers=headers(method="tools/call", name="terminal_exec", forwarded="https", auth=f"Bearer {make_token(key_path, issuer, 'relay.coding')}"),
                body=mcp("tools/call", invalid_dispatch))
            assert_status(status, 400, "invalid tool arguments before dispatch")
            assert body["error"]["code"] == -32602
            assert not os.path.exists(dispatch_marker), "invalid tool arguments reached execution"

            wrong_owner = make_token(key_path, issuer, "relay.coding", subject="other")
            status, challenge_headers, body = request(remote_url,
                headers=headers(method="tools/call", name="terminal_exec", forwarded="https", auth=f"Bearer {wrong_owner}"),
                body=auth_call(dispatch_marker))
            assert_status(status, 403, "wrong owner")
            assert "Authenticated subject is not authorized" in body["error"]["message"]
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
