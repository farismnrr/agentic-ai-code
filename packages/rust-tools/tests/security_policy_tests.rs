//! Integration coverage for the local-access policy (`security.rs`):
//! Origin/Host enforcement over real HTTP, proving a rejected request never
//! reaches the MCP handler. Unit-level header parsing edge cases live in
//! `src/relay_agent/security.rs`'s own `#[cfg(test)]` module — this file
//! proves the middleware is actually wired into the router end to end.

use rust_tools::relay_agent::{config::ServerConfig, transport::create_router};
use serde_json::json;
use tokio::net::TcpListener;

const PROTO_HEADER: &str = "mcp-protocol-version";
const METHOD_HEADER: &str = "mcp-method";
const PROTO_VERSION: &str = "2026-07-28";
const TEST_ORIGIN: &str = "http://localhost:3333";

async fn spawn_server(origin: Option<&str>) -> String {
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

fn tools_list_payload() -> serde_json::Value {
    json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {
            "_meta": {
                "io.modelcontextprotocol/protocolVersion": PROTO_VERSION,
                "io.modelcontextprotocol/clientCapabilities": {}
            }
        }
    })
}

#[tokio::test]
async fn allowed_origin_and_host_reach_the_mcp_handler() {
    let base = spawn_server(Some(TEST_ORIGIN)).await;
    let client = reqwest::Client::new();

    let res = client
        .post(format!("{base}/mcp"))
        .header(PROTO_HEADER, PROTO_VERSION)
        .header(METHOD_HEADER, "tools/list")
        .header("origin", TEST_ORIGIN)
        .json(&tools_list_payload())
        .send()
        .await
        .unwrap();

    // reqwest sends the real Host header for the connection (127.0.0.1:<port>),
    // which must match config.port for this to succeed.
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = res.json().await.unwrap();
    assert!(body.get("error").is_none());
    assert!(body["result"]["tools"].is_array());
}

#[tokio::test]
async fn missing_origin_is_rejected_before_the_handler() {
    let base = spawn_server(Some(TEST_ORIGIN)).await;
    let client = reqwest::Client::new();

    let res = client
        .post(format!("{base}/mcp"))
        .header(PROTO_HEADER, PROTO_VERSION)
        .json(&tools_list_payload())
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 403);
    let body: serde_json::Value = res.json().await.unwrap();
    // Never reaches tools/list — no `result` field, only a JSON-RPC error.
    assert!(body.get("result").is_none());
    assert!(body["error"]["message"]
        .as_str()
        .unwrap()
        .contains("Origin"));
}

#[tokio::test]
async fn wrong_origin_is_rejected() {
    let base = spawn_server(Some(TEST_ORIGIN)).await;
    let client = reqwest::Client::new();

    let res = client
        .post(format!("{base}/mcp"))
        .header(PROTO_HEADER, PROTO_VERSION)
        .header("origin", "http://localhost:9999")
        .json(&tools_list_payload())
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 403);
}

#[tokio::test]
async fn wrong_scheme_is_rejected() {
    let base = spawn_server(Some(TEST_ORIGIN)).await;
    let client = reqwest::Client::new();

    let res = client
        .post(format!("{base}/mcp"))
        .header(PROTO_HEADER, PROTO_VERSION)
        .header("origin", "https://localhost:3333")
        .json(&tools_list_payload())
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 403);
}

#[tokio::test]
async fn wrong_port_in_origin_is_rejected() {
    let base = spawn_server(Some(TEST_ORIGIN)).await;
    let client = reqwest::Client::new();

    let res = client
        .post(format!("{base}/mcp"))
        .header(PROTO_HEADER, PROTO_VERSION)
        .header("origin", "http://localhost:4444")
        .json(&tools_list_payload())
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 403);
}

#[tokio::test]
async fn missing_host_is_rejected() {
    // reqwest always sends a Host header for HTTP/1.1, so exercise the
    // "missing Host" path directly against the router with a hand-built
    // request instead of going through a real TCP connection.
    use axum::body::Body;
    use axum::http::Request;
    use tower::util::ServiceExt;

    let router = create_router(ServerConfig {
        port: 47821,
        origin: Some(TEST_ORIGIN.to_string()),
        ..Default::default()
    });

    let req = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header(PROTO_HEADER, PROTO_VERSION)
        .header("origin", TEST_ORIGIN)
        .header("content-type", "application/json")
        .body(Body::from(tools_list_payload().to_string()))
        .unwrap();

    let res = router.oneshot(req).await.unwrap();
    assert_eq!(res.status(), 403);
}

#[tokio::test]
async fn wrong_host_is_rejected() {
    let base = spawn_server(Some(TEST_ORIGIN)).await;
    let client = reqwest::Client::new();

    // Force an explicit Host override distinct from the real connection
    // target — reqwest lets a caller set Host explicitly, which the server
    // must still validate against its own bind address.
    let res = client
        .post(format!("{base}/mcp"))
        .header(PROTO_HEADER, PROTO_VERSION)
        .header("origin", TEST_ORIGIN)
        .header("host", "evil.example.com")
        .json(&tools_list_payload())
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 403);
}

#[tokio::test]
async fn lookalike_host_is_rejected() {
    let base = spawn_server(Some(TEST_ORIGIN)).await;
    let client = reqwest::Client::new();

    let res = client
        .post(format!("{base}/mcp"))
        .header(PROTO_HEADER, PROTO_VERSION)
        .header("origin", TEST_ORIGIN)
        .header("host", "127.0.0.1.evil.example.com")
        .json(&tools_list_payload())
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 403);
}

#[tokio::test]
async fn proxy_header_does_not_bypass_host_validation() {
    let base = spawn_server(Some(TEST_ORIGIN)).await;
    let client = reqwest::Client::new();

    // A spoofed X-Forwarded-Host claiming a valid host must not rescue an
    // invalid real Host header.
    let res = client
        .post(format!("{base}/mcp"))
        .header(PROTO_HEADER, PROTO_VERSION)
        .header("origin", TEST_ORIGIN)
        .header("host", "evil.example.com")
        .header("x-forwarded-host", "127.0.0.1:47821")
        .json(&tools_list_payload())
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 403);
}

#[tokio::test]
async fn no_configured_origin_fails_closed_even_with_a_plausible_origin_header() {
    let base = spawn_server(None).await;
    let client = reqwest::Client::new();

    let res = client
        .post(format!("{base}/mcp"))
        .header(PROTO_HEADER, PROTO_VERSION)
        .header("origin", TEST_ORIGIN)
        .json(&tools_list_payload())
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 403);
}

#[tokio::test]
async fn rejected_request_never_reaches_tools_call_execution_path() {
    // Even a well-formed tools/call for a real tool must be blocked at the
    // access-policy layer before it gets anywhere near tool dispatch.
    let base = spawn_server(Some(TEST_ORIGIN)).await;
    let client = reqwest::Client::new();
    let payload = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": { "name": "terminal_exec", "arguments": { "command": "echo hi" } }
    });

    let res = client
        .post(format!("{base}/mcp"))
        .header(PROTO_HEADER, PROTO_VERSION)
        .header("origin", "http://not-the-configured-origin.example.com")
        .json(&payload)
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 403);
    let body: serde_json::Value = res.json().await.unwrap();
    assert!(body.get("result").is_none());
}
