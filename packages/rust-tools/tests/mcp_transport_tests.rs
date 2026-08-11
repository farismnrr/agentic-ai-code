use rust_tools::relay_agent::{config::ServerConfig, transport::create_router};
use serde_json::json;
use tokio::net::TcpListener;

const PROTO_HEADER: &str = "mcp-protocol-version";
const PROTO_VERSION: &str = "2026-07-28";

async fn spawn_server(origin: Option<&str>) -> String {
    let config = ServerConfig {
        port: 0,
        origin: origin.map(|s| s.to_string()),
        ..Default::default()
    };
    let router = create_router(config);

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    format!("http://{addr}")
}

#[tokio::test]
async fn test_mcp_health() {
    let base = spawn_server(None).await;
    let client = reqwest::Client::new();
    let res = client.get(format!("{base}/health")).send().await.unwrap();

    assert_eq!(res.status(), 200);
    assert_eq!(res.text().await.unwrap(), "OK");
}

#[tokio::test]
async fn test_mcp_initialize() {
    let base = spawn_server(None).await;
    let client = reqwest::Client::new();
    let payload = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": PROTO_VERSION,
            "capabilities": {},
            "clientInfo": { "name": "test-client", "version": "1.0.0" }
        }
    });

    let res = client
        .post(format!("{base}/mcp"))
        .header(PROTO_HEADER, PROTO_VERSION)
        .json(&payload)
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 200);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["jsonrpc"], "2.0");
    assert_eq!(body["id"], 1);
    assert_eq!(body["result"]["protocolVersion"], PROTO_VERSION);
}

#[tokio::test]
async fn test_mcp_invalid_protocol_version_in_initialize_params() {
    let base = spawn_server(None).await;
    let client = reqwest::Client::new();
    let payload = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-01-01",
            "capabilities": {},
            "clientInfo": { "name": "test-client", "version": "1.0.0" }
        }
    });

    let res = client
        .post(format!("{base}/mcp"))
        .header(PROTO_HEADER, PROTO_VERSION)
        .json(&payload)
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 400);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["error"]["code"], -32602);
}

#[tokio::test]
async fn test_missing_protocol_version_header_is_rejected() {
    let base = spawn_server(None).await;
    let client = reqwest::Client::new();
    let payload = json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list" });

    let res = client
        .post(format!("{base}/mcp"))
        .json(&payload)
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 400);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["error"]["code"], -32600);
}

#[tokio::test]
async fn test_wrong_protocol_version_header_is_rejected() {
    let base = spawn_server(None).await;
    let client = reqwest::Client::new();
    let payload = json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list" });

    let res = client
        .post(format!("{base}/mcp"))
        .header(PROTO_HEADER, "2024-01-01")
        .json(&payload)
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 400);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["error"]["code"], -32600);
}

#[tokio::test]
async fn test_tools_list_returns_full_catalog_with_schemas() {
    let base = spawn_server(None).await;
    let client = reqwest::Client::new();
    let payload = json!({ "jsonrpc": "2.0", "id": 42, "method": "tools/list" });

    let res = client
        .post(format!("{base}/mcp"))
        .header(PROTO_HEADER, PROTO_VERSION)
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
async fn test_tools_call_unknown_tool_is_rejected() {
    let base = spawn_server(None).await;
    let client = reqwest::Client::new();
    let payload = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": { "name": "does_not_exist", "arguments": {} }
    });

    let res = client
        .post(format!("{base}/mcp"))
        .header(PROTO_HEADER, PROTO_VERSION)
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
    let base = spawn_server(None).await;
    let client = reqwest::Client::new();
    let payload = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": { "name": "terminal_exec", "arguments": { "command": "echo hi" } }
    });

    let res = client
        .post(format!("{base}/mcp"))
        .header(PROTO_HEADER, PROTO_VERSION)
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
    let base = spawn_server(None).await;
    let client = reqwest::Client::new();
    let payload = json!({ "jsonrpc": "2.0", "id": 4, "method": "tools/call" });

    let res = client
        .post(format!("{base}/mcp"))
        .header(PROTO_HEADER, PROTO_VERSION)
        .json(&payload)
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 400);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["error"]["code"], -32602);
}

#[tokio::test]
async fn test_unknown_method_returns_method_not_found() {
    let base = spawn_server(None).await;
    let client = reqwest::Client::new();
    let payload = json!({ "jsonrpc": "2.0", "id": 5, "method": "resources/list" });

    let res = client
        .post(format!("{base}/mcp"))
        .header(PROTO_HEADER, PROTO_VERSION)
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
    let base = spawn_server(None).await;
    let client = reqwest::Client::new();

    let res = client
        .post(format!("{base}/mcp"))
        .header(PROTO_HEADER, PROTO_VERSION)
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
    let base = spawn_server(None).await;
    let client = reqwest::Client::new();
    let payload = json!({ "jsonrpc": "1.0", "id": 6, "method": "tools/list" });

    let res = client
        .post(format!("{base}/mcp"))
        .header(PROTO_HEADER, PROTO_VERSION)
        .json(&payload)
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 400);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["error"]["code"], -32600);
}

#[tokio::test]
async fn test_notification_without_id_gets_no_error_envelope() {
    let base = spawn_server(None).await;
    let client = reqwest::Client::new();
    let payload = json!({ "jsonrpc": "2.0", "method": "notifications/initialized" });

    let res = client
        .post(format!("{base}/mcp"))
        .header(PROTO_HEADER, PROTO_VERSION)
        .json(&payload)
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 200);
    let body: serde_json::Value = res.json().await.unwrap();
    assert!(body.get("error").is_none());
}

#[tokio::test]
async fn test_oversized_body_is_rejected() {
    let base = spawn_server(None).await;
    let client = reqwest::Client::new();

    // MAX_BODY_BYTES is 1 MiB; send comfortably over that.
    let huge_arg = "a".repeat(2 * 1024 * 1024);
    let payload = json!({
        "jsonrpc": "2.0",
        "id": 7,
        "method": "tools/call",
        "params": { "name": "terminal_exec", "arguments": { "command": huge_arg } }
    });

    let res = client
        .post(format!("{base}/mcp"))
        .header(PROTO_HEADER, PROTO_VERSION)
        .json(&payload)
        .send()
        .await
        .unwrap();

    // axum's DefaultBodyLimit rejects with 413 before the handler runs.
    assert_eq!(res.status(), 413);
}

#[tokio::test]
async fn test_wrong_content_type_is_rejected() {
    let base = spawn_server(None).await;
    let client = reqwest::Client::new();
    let payload = json!({ "jsonrpc": "2.0", "id": 8, "method": "tools/list" });

    let res = client
        .post(format!("{base}/mcp"))
        .header(PROTO_HEADER, PROTO_VERSION)
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
    // will not reflect any Origin, matching the fail-closed invariant.
    let base = spawn_server(None).await;
    let client = reqwest::Client::new();
    let payload = json!({ "jsonrpc": "2.0", "id": 9, "method": "tools/list" });

    let res = client
        .post(format!("{base}/mcp"))
        .header(PROTO_HEADER, PROTO_VERSION)
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
