use relay_core::config::{ActivityConfig, ServerConfig};
use relay_infrastructure::transport::create_router;
use serde_json::{json, Value};
use std::{fs, path::PathBuf};
use uuid::Uuid;

fn fixture_config(port: u16, activity_state_dir: &std::path::Path) -> ServerConfig {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    ServerConfig {
        port,
        origin: Some("http://localhost:3333".into()),
        dir: Some(root.to_string_lossy().into_owned()),
        execution_root: Some(root.to_string_lossy().into_owned()),
        activity: ActivityConfig {
            state_dir: Some(activity_state_dir.to_string_lossy().into_owned()),
            ..ActivityConfig::default()
        },
        ..ServerConfig::default()
    }
}

async fn post_mcp(port: u16, method: &str, name: Option<&str>, body: Value) -> reqwest::Response {
    let client = reqwest::Client::new();
    let mut request = client
        .post(format!("http://127.0.0.1:{port}/mcp"))
        .header("content-type", "application/json")
        .header("mcp-protocol-version", "2026-07-28")
        .header("mcp-method", method)
        .json(&body);
    if let Some(name) = name {
        request = request.header("mcp-name", name);
    }
    request.send().await.expect("MCP request")
}

fn meta() -> Value {
    json!({
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientCapabilities": {}
    })
}

#[tokio::test]
async fn dedicated_ssh_tool_is_discoverable_and_reaches_application_path() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("loopback listener");
    let port = listener.local_addr().expect("listener address").port();
    let activity_state_dir =
        std::env::temp_dir().join(format!("ai-tools-ssh-transport-{}", Uuid::new_v4()));
    fs::create_dir_all(&activity_state_dir).expect("activity state directory");
    let config = fixture_config(port, &activity_state_dir);
    let router = create_router(config);
    let server = tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("MCP test server");
    });

    let list = post_mcp(
        port,
        "tools/list",
        None,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/list",
            "params": { "_meta": meta() }
        }),
    )
    .await;
    assert_eq!(list.status(), reqwest::StatusCode::OK);
    let list: Value = list.json().await.expect("tools/list JSON");
    let tools = list
        .pointer("/result/tools")
        .and_then(Value::as_array)
        .expect("tools array");
    assert!(tools
        .iter()
        .any(|tool| { tool.get("name").and_then(Value::as_str) == Some("ssh_readonly_exec") }));

    let call = post_mcp(
        port,
        "tools/call",
        Some("ssh_readonly_exec"),
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "ssh_readonly_exec",
                "arguments": {
                    "alias": "fixture",
                    "command": "docker",
                    "args": ["ps"],
                    "execution_mode": "sync"
                },
                "_meta": meta()
            }
        }),
    )
    .await;
    assert_eq!(call.status(), reqwest::StatusCode::OK);
    let call: Value = call.json().await.expect("tools/call JSON");
    assert_eq!(
        call.pointer("/result/isError").and_then(Value::as_bool),
        Some(true)
    );
    let text = call
        .pointer("/result/content/0/text")
        .and_then(Value::as_str)
        .unwrap_or_default();
    assert!(text.contains("SSH diagnostics are disabled"));

    server.abort();
    let _ = server.await;
    fs::remove_dir_all(activity_state_dir).expect("remove activity state directory");
}
