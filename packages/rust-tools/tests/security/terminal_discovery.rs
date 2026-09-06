use ai_tools::core::config::{ActivityConfig, ServerConfig, ToolProfile};
use ai_tools::infrastructure::transport::create_router;
use serde_json::{json, Value};
use std::{fs, path::PathBuf};
use uuid::Uuid;

fn fixture_config(
    port: u16,
    activity_state_dir: &std::path::Path,
    profile: ToolProfile,
) -> ServerConfig {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    ServerConfig {
        port,
        origin: Some("http://localhost:3333".into()),
        dir: Some(root.to_string_lossy().into_owned()),
        execution_root: Some(root.to_string_lossy().into_owned()),
        tool_profile: profile,
        activity: ActivityConfig {
            state_dir: Some(activity_state_dir.to_string_lossy().into_owned()),
            ..ActivityConfig::default()
        },
        ..ServerConfig::default()
    }
}

async fn post_mcp(
    client: &reqwest::Client,
    port: u16,
    method: &str,
    name: Option<&str>,
    body: Value,
) -> reqwest::Response {
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
async fn wire_client_discovery_and_invocation_are_consistent() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("loopback listener");
    let port = listener.local_addr().expect("listener address").port();
    let activity_state_dir =
        std::env::temp_dir().join(format!("ai-tools-discovery-test-{}", Uuid::new_v4()));
    fs::create_dir_all(&activity_state_dir).expect("activity state directory");
    let config = fixture_config(port, &activity_state_dir, ToolProfile::Full);
    let router = create_router(config);
    let server = tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("MCP test server");
    });

    let client = reqwest::Client::new();

    // 1. tools/list returns Full catalog of 52 tools
    let list_res = post_mcp(
        &client,
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
    assert_eq!(list_res.status(), reqwest::StatusCode::OK);
    let list_json: Value = list_res.json().await.expect("tools/list JSON");
    let tools = list_json
        .pointer("/result/tools")
        .and_then(Value::as_array)
        .expect("tools array");
    assert_eq!(tools.len(), 52, "Full profile must have 52 tools");

    // Verify key capabilities are listed
    let tool_names: Vec<&str> = tools
        .iter()
        .filter_map(|t| t.get("name").and_then(Value::as_str))
        .collect();
    assert!(tool_names.contains(&"terminal_exec"));
    assert!(tool_names.contains(&"terminal_job_start"));
    assert!(tool_names.contains(&"workspace_list"));
    assert!(tool_names.contains(&"ssh_readonly_exec"));
    assert!(tool_names.contains(&"telegram_send_message"));

    // 2. tools/call on the same connection is consistent
    let call_res = post_mcp(
        &client,
        port,
        "tools/call",
        Some("workspace_list"),
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "workspace_list",
                "arguments": {},
                "_meta": meta()
            }
        }),
    )
    .await;
    assert_eq!(call_res.status(), reqwest::StatusCode::OK);
    let call_json: Value = call_res.json().await.expect("tools/call JSON");
    assert_eq!(
        call_json
            .pointer("/result/isError")
            .and_then(Value::as_bool),
        Some(false)
    );

    // 3. Structured capability-revoked error for disabled capability
    let ssh_call = post_mcp(
        &client,
        port,
        "tools/call",
        Some("ssh_readonly_exec"),
        json!({
            "jsonrpc": "2.0",
            "id": 3,
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
    assert_eq!(ssh_call.status(), reqwest::StatusCode::OK);
    let ssh_json: Value = ssh_call.json().await.expect("tools/call JSON");
    assert_eq!(
        ssh_json.pointer("/result/isError").and_then(Value::as_bool),
        Some(true)
    );
    let err_text = ssh_json
        .pointer("/result/content/0/text")
        .and_then(Value::as_str)
        .unwrap_or_default();
    assert!(err_text.contains("SSH diagnostics are disabled"));

    // 4. Calling an unknown / unadvertised tool returns structured 404 error
    let unknown_call = post_mcp(
        &client,
        port,
        "tools/call",
        Some("nonexistent_tool"),
        json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {
                "name": "nonexistent_tool",
                "arguments": {},
                "_meta": meta()
            }
        }),
    )
    .await;
    assert_eq!(unknown_call.status(), reqwest::StatusCode::NOT_FOUND);
    let unknown_json: Value = unknown_call.json().await.expect("JSON-RPC error");
    assert_eq!(
        unknown_json.pointer("/error/code").and_then(Value::as_i64),
        Some(-32602)
    );

    server.abort();
    let _ = server.await;
    fs::remove_dir_all(activity_state_dir).expect("remove activity state directory");
}

#[tokio::test]
async fn wire_client_primary_profile_exposes_fifteen_tools_and_denies_full_tools() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("loopback listener");
    let port = listener.local_addr().expect("listener address").port();
    let activity_state_dir =
        std::env::temp_dir().join(format!("ai-tools-primary-test-{}", Uuid::new_v4()));
    fs::create_dir_all(&activity_state_dir).expect("activity state directory");
    let config = fixture_config(port, &activity_state_dir, ToolProfile::Primary);
    let router = create_router(config);
    let server = tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("MCP test server");
    });

    let client = reqwest::Client::new();

    let list_res = post_mcp(
        &client,
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
    assert_eq!(list_res.status(), reqwest::StatusCode::OK);
    let list_json: Value = list_res.json().await.expect("tools/list JSON");
    let tools = list_json
        .pointer("/result/tools")
        .and_then(Value::as_array)
        .expect("tools array");
    assert_eq!(
        tools.len(),
        15,
        "Primary profile must have exactly 15 tools"
    );

    // ssh_readonly_exec is NOT in Primary profile
    let ssh_call = post_mcp(
        &client,
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
                    "command": "docker"
                },
                "_meta": meta()
            }
        }),
    )
    .await;
    assert_eq!(ssh_call.status(), reqwest::StatusCode::NOT_FOUND);

    server.abort();
    let _ = server.await;
    fs::remove_dir_all(activity_state_dir).expect("remove activity state directory");
}
