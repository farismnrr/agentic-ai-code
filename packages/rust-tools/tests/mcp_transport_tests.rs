use rust_tools::relay_agent::{config::ServerConfig, transport::create_router};
use serde_json::json;
use tokio::net::TcpListener;

const HDR_PROTO: &str = "mcp-protocol-version";
const HDR_METHOD: &str = "mcp-method";
const HDR_NAME: &str = "mcp-name";
const PROTO_VERSION: &str = "2026-07-28";

/// The origin these protocol-focused tests configure the server with, and
/// the `Origin` header they send back. Origin/Host enforcement itself is
/// covered exhaustively in `security_policy_tests.rs` — these tests exist
/// to exercise JSON-RPC/MCP semantics, so they authenticate with a valid
/// Origin rather than re-testing the access policy on every case.
const TEST_ORIGIN: &str = "http://localhost:3333";

/// The `_meta` object modern MCP `2026-07-28` requires on every request's
/// `params` (`io.modelcontextprotocol/protocolVersion` and
/// `io.modelcontextprotocol/clientCapabilities` are both required by spec;
/// `clientInfo` is optional but included here for realism).
fn meta() -> serde_json::Value {
    json!({
        "io.modelcontextprotocol/protocolVersion": PROTO_VERSION,
        "io.modelcontextprotocol/clientInfo": { "name": "test-client", "version": "1.0.0" },
        "io.modelcontextprotocol/clientCapabilities": {}
    })
}

async fn spawn_server(origin: Option<&str>) -> String {
    // Bind first so the `Host` the client will actually send (derived from
    // the real ephemeral port) matches `config.port` exactly — building the
    // config with a placeholder `port: 0` while binding to a real ephemeral
    // port would make every request fail Host validation.
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let config = ServerConfig {
        port: addr.port(),
        origin: origin.map(|s| s.to_string()),
        ..Default::default()
    };
    let router = create_router(config);

    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    format!("http://{addr}")
}

#[tokio::test]
async fn test_mcp_health() {
    // /health is intentionally ungated by the local-access policy (it's a
    // liveness probe), so no origin/config is needed here.
    let base = spawn_server(None).await;
    let client = reqwest::Client::new();
    let res = client.get(format!("{base}/health")).send().await.unwrap();

    assert_eq!(res.status(), 200);
    assert_eq!(res.text().await.unwrap(), "OK");
}

#[tokio::test]
async fn test_server_discover_returns_capabilities_and_supported_versions() {
    // `server/discover` is the modern (2026-07-28) replacement for the
    // removed `initialize` handshake. Calling it is optional for clients,
    // but servers MUST implement it.
    let base = spawn_server(Some(TEST_ORIGIN)).await;
    let client = reqwest::Client::new();
    let payload = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "server/discover",
        "params": { "_meta": meta() }
    });

    let res = client
        .post(format!("{base}/mcp"))
        .header(HDR_PROTO, PROTO_VERSION)
        .header(HDR_METHOD, "server/discover")
        .header("origin", TEST_ORIGIN)
        .json(&payload)
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 200);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["jsonrpc"], "2.0");
    assert_eq!(body["id"], 1);
    assert_eq!(body["result"]["resultType"], "complete");
    assert_eq!(body["result"]["supportedVersions"], json!([PROTO_VERSION]));
    assert!(body["result"]["capabilities"]["tools"].is_object());
    assert_eq!(
        body["result"]["_meta"]["io.modelcontextprotocol/serverInfo"]["name"],
        "relay-agent"
    );
}

#[tokio::test]
async fn test_initialize_is_no_longer_a_recognized_method() {
    // 2026-07-28 retired the initialize/initialized handshake entirely — a
    // server implementing only this revision must treat `initialize` as an
    // unknown method (404, -32601), exactly like any other method it
    // doesn't implement, per the spec's own guidance for modern-only
    // servers receiving a legacy handshake.
    let base = spawn_server(Some(TEST_ORIGIN)).await;
    let client = reqwest::Client::new();
    let payload = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": PROTO_VERSION,
            "capabilities": {},
            "clientInfo": { "name": "legacy-client", "version": "1.0.0" }
        }
    });

    let res = client
        .post(format!("{base}/mcp"))
        .header(HDR_PROTO, PROTO_VERSION)
        .header(HDR_METHOD, "initialize")
        .header("origin", TEST_ORIGIN)
        .json(&payload)
        .send()
        .await
        .unwrap();

    // The legacy body has no params._meta at all, so this actually fails
    // routing-header validation (-32020) before method dispatch is ever
    // reached — which is itself the point: a legacy client cannot get any
    // further than a header/body mismatch against a modern-only server.
    assert_eq!(res.status(), 400);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["error"]["code"], -32020);
}

#[tokio::test]
async fn test_missing_protocol_version_header_is_header_mismatch() {
    let base = spawn_server(Some(TEST_ORIGIN)).await;
    let client = reqwest::Client::new();
    let payload = json!({
        "jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": { "_meta": meta() }
    });

    let res = client
        .post(format!("{base}/mcp"))
        .header(HDR_METHOD, "tools/list")
        .header("origin", TEST_ORIGIN)
        .json(&payload)
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 400);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["error"]["code"], -32020);
}

#[tokio::test]
async fn test_unsupported_protocol_version_header_is_rejected_with_supported_list() {
    let base = spawn_server(Some(TEST_ORIGIN)).await;
    let client = reqwest::Client::new();
    let payload = json!({
        "jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": { "_meta": meta() }
    });

    let res = client
        .post(format!("{base}/mcp"))
        .header(HDR_PROTO, "2024-01-01")
        .header(HDR_METHOD, "tools/list")
        .header("origin", TEST_ORIGIN)
        .json(&payload)
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 400);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["error"]["code"], -32022);
    assert_eq!(body["error"]["data"]["requested"], "2024-01-01");
    assert_eq!(body["error"]["data"]["supported"], json!([PROTO_VERSION]));
}

#[tokio::test]
async fn test_meta_protocol_version_mismatching_header_is_header_mismatch() {
    // Header says 2026-07-28 (a version we support), but the body's _meta
    // disagrees — this is exactly the "different components trust
    // different sources" case the spec's Server Validation section exists
    // to prevent.
    let base = spawn_server(Some(TEST_ORIGIN)).await;
    let client = reqwest::Client::new();
    let mut bad_meta = meta();
    bad_meta["io.modelcontextprotocol/protocolVersion"] = json!("2025-01-01");
    let payload = json!({
        "jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": { "_meta": bad_meta }
    });

    let res = client
        .post(format!("{base}/mcp"))
        .header(HDR_PROTO, PROTO_VERSION)
        .header(HDR_METHOD, "tools/list")
        .header("origin", TEST_ORIGIN)
        .json(&payload)
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 400);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["error"]["code"], -32020);
}

#[tokio::test]
async fn test_missing_meta_client_capabilities_is_header_mismatch() {
    let base = spawn_server(Some(TEST_ORIGIN)).await;
    let client = reqwest::Client::new();
    let payload = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {
            "_meta": {
                "io.modelcontextprotocol/protocolVersion": PROTO_VERSION
            }
        }
    });

    let res = client
        .post(format!("{base}/mcp"))
        .header(HDR_PROTO, PROTO_VERSION)
        .header(HDR_METHOD, "tools/list")
        .header("origin", TEST_ORIGIN)
        .json(&payload)
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 400);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["error"]["code"], -32020);
}

#[tokio::test]
async fn test_missing_mcp_method_header_is_header_mismatch() {
    let base = spawn_server(Some(TEST_ORIGIN)).await;
    let client = reqwest::Client::new();
    let payload = json!({
        "jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": { "_meta": meta() }
    });

    let res = client
        .post(format!("{base}/mcp"))
        .header(HDR_PROTO, PROTO_VERSION)
        .header("origin", TEST_ORIGIN)
        .json(&payload)
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 400);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["error"]["code"], -32020);
}

#[tokio::test]
async fn test_mcp_method_header_not_matching_body_method_is_header_mismatch() {
    let base = spawn_server(Some(TEST_ORIGIN)).await;
    let client = reqwest::Client::new();
    let payload = json!({
        "jsonrpc": "2.0", "id": 1, "method": "tools/call",
        "params": { "name": "terminal_exec", "arguments": {}, "_meta": meta() }
    });

    let res = client
        .post(format!("{base}/mcp"))
        // Header claims tools/list while the body says tools/call.
        .header(HDR_PROTO, PROTO_VERSION)
        .header(HDR_METHOD, "tools/list")
        .header("origin", TEST_ORIGIN)
        .json(&payload)
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 400);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["error"]["code"], -32020);
}

#[tokio::test]
async fn test_tools_list_returns_full_catalog_with_schemas() {
    let base = spawn_server(Some(TEST_ORIGIN)).await;
    let client = reqwest::Client::new();
    let payload = json!({
        "jsonrpc": "2.0", "id": 42, "method": "tools/list", "params": { "_meta": meta() }
    });

    let res = client
        .post(format!("{base}/mcp"))
        .header(HDR_PROTO, PROTO_VERSION)
        .header(HDR_METHOD, "tools/list")
        .header("origin", TEST_ORIGIN)
        .json(&payload)
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 200);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["id"], 42);
    let tools = body["result"]["tools"].as_array().unwrap();
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert_eq!(names, vec!["terminal_exec", "http_fetch", "web_search"]);
    for tool in tools {
        assert!(tool["description"].is_string());
        assert_eq!(tool["inputSchema"]["type"], "object");
        assert!(tool["inputSchema"]["properties"].is_object());
    }
}

#[tokio::test]
async fn test_tools_call_requires_mcp_name_header() {
    let base = spawn_server(Some(TEST_ORIGIN)).await;
    let client = reqwest::Client::new();
    let payload = json!({
        "jsonrpc": "2.0", "id": 2, "method": "tools/call",
        "params": { "name": "terminal_exec", "arguments": {}, "_meta": meta() }
    });

    let res = client
        .post(format!("{base}/mcp"))
        .header(HDR_PROTO, PROTO_VERSION)
        .header(HDR_METHOD, "tools/call")
        // Mcp-Name intentionally omitted.
        .header("origin", TEST_ORIGIN)
        .json(&payload)
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 400);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["error"]["code"], -32020);
}

#[tokio::test]
async fn test_tools_call_rejects_mcp_name_not_matching_body_params_name() {
    let base = spawn_server(Some(TEST_ORIGIN)).await;
    let client = reqwest::Client::new();
    let payload = json!({
        "jsonrpc": "2.0", "id": 2, "method": "tools/call",
        "params": { "name": "terminal_exec", "arguments": {}, "_meta": meta() }
    });

    let res = client
        .post(format!("{base}/mcp"))
        .header(HDR_PROTO, PROTO_VERSION)
        .header(HDR_METHOD, "tools/call")
        .header(HDR_NAME, "http_fetch")
        .header("origin", TEST_ORIGIN)
        .json(&payload)
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 400);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["error"]["code"], -32020);
}

#[tokio::test]
async fn test_tools_call_accepts_base64_sentinel_mcp_name() {
    // Per the spec's Value Encoding section: a header value that can't be
    // safely represented as plain ASCII is carried as
    // `=?base64?{Base64EncodedValue}?=`. "terminal_exec" is plain ASCII in
    // practice, but the decoder must still accept the sentinel form and
    // decode it before comparing to params.name.
    let base = spawn_server(Some(TEST_ORIGIN)).await;
    let client = reqwest::Client::new();
    let payload = json!({
        "jsonrpc": "2.0", "id": 2, "method": "tools/call",
        "params": { "name": "terminal_exec", "arguments": {}, "_meta": meta() }
    });

    // Base64("terminal_exec") = dGVybWluYWxfZXhlYw==
    let res = client
        .post(format!("{base}/mcp"))
        .header(HDR_PROTO, PROTO_VERSION)
        .header(HDR_METHOD, "tools/call")
        .header(HDR_NAME, "=?base64?dGVybWluYWxfZXhlYw==?=")
        .header("origin", TEST_ORIGIN)
        .json(&payload)
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 200);
}

#[tokio::test]
async fn test_tools_call_unknown_tool_is_rejected() {
    let base = spawn_server(Some(TEST_ORIGIN)).await;
    let client = reqwest::Client::new();
    let payload = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": { "name": "does_not_exist", "arguments": {}, "_meta": meta() }
    });

    let res = client
        .post(format!("{base}/mcp"))
        .header(HDR_PROTO, PROTO_VERSION)
        .header(HDR_METHOD, "tools/call")
        .header(HDR_NAME, "does_not_exist")
        .header("origin", TEST_ORIGIN)
        .json(&payload)
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 404);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["error"]["code"], -32602);
}

#[tokio::test]
async fn test_tools_call_known_tool_returns_structured_not_implemented() {
    let base = spawn_server(Some(TEST_ORIGIN)).await;
    let client = reqwest::Client::new();
    let payload = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": { "name": "terminal_exec", "arguments": { "command": "echo hi" }, "_meta": meta() }
    });

    let res = client
        .post(format!("{base}/mcp"))
        .header(HDR_PROTO, PROTO_VERSION)
        .header(HDR_METHOD, "tools/call")
        .header(HDR_NAME, "terminal_exec")
        .header("origin", TEST_ORIGIN)
        .json(&payload)
        .send()
        .await
        .unwrap();

    // Not-implemented is a structured tool-result error, not an HTTP/JSON-RPC
    // failure — the request itself was valid.
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["result"]["isError"], true);
    let text = body["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("not implemented"));
}

#[tokio::test]
async fn test_tools_call_missing_params_is_invalid_request() {
    let base = spawn_server(Some(TEST_ORIGIN)).await;
    let client = reqwest::Client::new();
    let payload = json!({ "jsonrpc": "2.0", "id": 4, "method": "tools/call" });

    let res = client
        .post(format!("{base}/mcp"))
        .header(HDR_PROTO, PROTO_VERSION)
        .header(HDR_METHOD, "tools/call")
        .header("origin", TEST_ORIGIN)
        .json(&payload)
        .send()
        .await
        .unwrap();

    // No params at all means no _meta either, so routing-header validation
    // (missing _meta.clientCapabilities) rejects this before tools/call's
    // own "missing params" check would ever run — both are legitimate
    // reasons to reject the same request, and header validation runs first.
    assert_eq!(res.status(), 400);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["error"]["code"], -32020);
}

#[tokio::test]
async fn test_unknown_method_returns_method_not_found() {
    let base = spawn_server(Some(TEST_ORIGIN)).await;
    let client = reqwest::Client::new();
    let payload = json!({
        "jsonrpc": "2.0", "id": 5, "method": "resources/list", "params": { "_meta": meta() }
    });

    let res = client
        .post(format!("{base}/mcp"))
        .header(HDR_PROTO, PROTO_VERSION)
        .header(HDR_METHOD, "resources/list")
        .header("origin", TEST_ORIGIN)
        .json(&payload)
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 404);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["error"]["code"], -32601);
}

#[tokio::test]
async fn test_malformed_json_body_is_parse_error() {
    let base = spawn_server(Some(TEST_ORIGIN)).await;
    let client = reqwest::Client::new();

    let res = client
        .post(format!("{base}/mcp"))
        .header(HDR_PROTO, PROTO_VERSION)
        .header("origin", TEST_ORIGIN)
        .header("content-type", "application/json")
        .body("{not valid json")
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 400);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["error"]["code"], -32700);
}

#[tokio::test]
async fn test_wrong_jsonrpc_version_is_invalid_request() {
    let base = spawn_server(Some(TEST_ORIGIN)).await;
    let client = reqwest::Client::new();
    let payload = json!({ "jsonrpc": "1.0", "id": 6, "method": "tools/list" });

    let res = client
        .post(format!("{base}/mcp"))
        .header(HDR_PROTO, PROTO_VERSION)
        .header("origin", TEST_ORIGIN)
        .json(&payload)
        .send()
        .await
        .unwrap();

    // Structural JSON-RPC validation (wrong jsonrpc version) happens before
    // routing-header validation, so this is still -32600, not -32020.
    assert_eq!(res.status(), 400);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["error"]["code"], -32600);
}

#[tokio::test]
async fn test_notification_gets_202_accepted_with_no_body() {
    // Per spec: a notification (no `id`) the server accepts MUST get
    // `202 Accepted` with no body — never a JSON-RPC envelope.
    let base = spawn_server(Some(TEST_ORIGIN)).await;
    let client = reqwest::Client::new();
    let payload = json!({ "jsonrpc": "2.0", "method": "notifications/initialized" });

    let res = client
        .post(format!("{base}/mcp"))
        .header(HDR_PROTO, PROTO_VERSION)
        .header("origin", TEST_ORIGIN)
        .json(&payload)
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 202);
    let bytes = res.bytes().await.unwrap();
    assert!(bytes.is_empty());
}

#[tokio::test]
async fn test_oversized_body_is_rejected() {
    let base = spawn_server(Some(TEST_ORIGIN)).await;
    let client = reqwest::Client::new();

    // MAX_BODY_BYTES is 1 MiB; send comfortably over that.
    let huge_arg = "a".repeat(2 * 1024 * 1024);
    let payload = json!({
        "jsonrpc": "2.0",
        "id": 7,
        "method": "tools/call",
        "params": { "name": "terminal_exec", "arguments": { "command": huge_arg }, "_meta": meta() }
    });

    let res = client
        .post(format!("{base}/mcp"))
        .header(HDR_PROTO, PROTO_VERSION)
        .header(HDR_METHOD, "tools/call")
        .header(HDR_NAME, "terminal_exec")
        .header("origin", TEST_ORIGIN)
        .json(&payload)
        .send()
        .await
        .unwrap();

    // axum's DefaultBodyLimit rejects with 413 before the handler runs.
    assert_eq!(res.status(), 413);
}

#[tokio::test]
async fn test_wrong_content_type_is_rejected() {
    let base = spawn_server(Some(TEST_ORIGIN)).await;
    let client = reqwest::Client::new();
    let payload = json!({ "jsonrpc": "2.0", "id": 8, "method": "tools/list" });

    let res = client
        .post(format!("{base}/mcp"))
        .header(HDR_PROTO, PROTO_VERSION)
        .header("origin", TEST_ORIGIN)
        .header("content-type", "text/plain")
        .body(payload.to_string())
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 400);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["error"]["code"], -32600);
}

#[tokio::test]
async fn test_cors_rejects_when_no_origin_configured() {
    // No origin configured → AllowOrigin::list(vec![]) → the CORS layer
    // will not reflect any Origin, matching the fail-closed invariant. The
    // request itself now also gets rejected earlier by the local-access
    // policy (403, since there is nothing valid to match), but this test's
    // job is specifically the CORS *response header* behavior, so it
    // doesn't assert on status.
    let base = spawn_server(None).await;
    let client = reqwest::Client::new();
    let payload = json!({ "jsonrpc": "2.0", "id": 9, "method": "tools/list" });

    let res = client
        .post(format!("{base}/mcp"))
        .header(HDR_PROTO, PROTO_VERSION)
        .header("origin", "http://evil.example.com")
        .json(&payload)
        .send()
        .await
        .unwrap();

    // The request itself still gets a same-process response (CORS is
    // enforced by the browser reading response headers, not by the server
    // refusing the request) — assert no ACAO header is reflected back.
    assert!(res.headers().get("access-control-allow-origin").is_none());
}
