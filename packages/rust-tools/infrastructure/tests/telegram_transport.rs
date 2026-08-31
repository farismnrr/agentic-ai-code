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
        telegram_enabled: false,
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

fn tool_call(id: u64, name: &str, arguments: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "tools/call",
        "params": {
            "name": name,
            "arguments": arguments,
            "_meta": meta()
        }
    })
}

#[tokio::test]
async fn telegram_tool_is_discoverable_and_routes_through_mcp_without_network_delivery() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("loopback listener");
    let port = listener.local_addr().expect("listener address").port();
    let activity_state_dir =
        std::env::temp_dir().join(format!("ai-tools-telegram-transport-{}", Uuid::new_v4()));
    fs::create_dir_all(&activity_state_dir).expect("activity state directory");
    let config = fixture_config(port, &activity_state_dir);
    let working_directory = fs::canonicalize(env!("CARGO_MANIFEST_DIR"))
        .expect("canonical authorized working directory");
    let unauthorized_directory = std::env::temp_dir();
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
    let telegram_tool = tools
        .iter()
        .find(|tool| tool.get("name").and_then(Value::as_str) == Some("telegram_send_message"))
        .expect("full profile exposes Telegram messaging");
    let required = telegram_tool
        .pointer("/inputSchema/required")
        .and_then(Value::as_array)
        .expect("Telegram required fields");
    assert_eq!(
        required,
        &vec![json!("working_directory"), json!("message")]
    );
    assert!(!tools
        .iter()
        .any(|tool| tool.get("name").and_then(Value::as_str) == Some("task_completed")));

    let valid = post_mcp(
        port,
        "tools/call",
        Some("telegram_send_message"),
        tool_call(
            2,
            "telegram_send_message",
            json!({
                "working_directory": working_directory.to_string_lossy(),
                "message": "transport fixture"
            }),
        ),
    )
    .await;
    assert_eq!(valid.status(), reqwest::StatusCode::OK);
    let valid: Value = valid.json().await.expect("valid tools/call JSON");
    assert_eq!(
        valid.pointer("/result/isError").and_then(Value::as_bool),
        Some(false)
    );
    let status = valid
        .pointer("/result/content/0/text")
        .and_then(Value::as_str)
        .and_then(|text| serde_json::from_str::<Value>(text).ok())
        .and_then(|value| {
            value
                .get("status")
                .and_then(Value::as_str)
                .map(str::to_owned)
        });
    assert_eq!(status.as_deref(), Some("disabled"));

    let missing_directory = post_mcp(
        port,
        "tools/call",
        Some("telegram_send_message"),
        tool_call(3, "telegram_send_message", json!({ "message": "missing" })),
    )
    .await;
    assert_eq!(missing_directory.status(), reqwest::StatusCode::BAD_REQUEST);

    for (id, directory) in [
        (4, Value::String("relative/path".into())),
        (
            5,
            Value::String(unauthorized_directory.to_string_lossy().into_owned()),
        ),
    ] {
        let rejected = post_mcp(
            port,
            "tools/call",
            Some("telegram_send_message"),
            tool_call(
                id,
                "telegram_send_message",
                json!({ "working_directory": directory, "message": "rejected" }),
            ),
        )
        .await;
        assert_eq!(rejected.status(), reqwest::StatusCode::OK);
        let rejected: Value = rejected.json().await.expect("rejected tools/call JSON");
        assert_eq!(
            rejected.pointer("/result/isError").and_then(Value::as_bool),
            Some(true)
        );
    }

    let legacy_tool = post_mcp(
        port,
        "tools/call",
        Some("task_completed"),
        tool_call(6, "task_completed", json!({})),
    )
    .await;
    assert_eq!(legacy_tool.status(), reqwest::StatusCode::NOT_FOUND);

    let legacy_extension = post_mcp(
        port,
        "server/task_completed",
        None,
        json!({
            "jsonrpc": "2.0",
            "id": 7,
            "method": "server/task_completed",
            "params": { "_meta": meta() }
        }),
    )
    .await;
    assert_eq!(legacy_extension.status(), reqwest::StatusCode::NOT_FOUND);

    server.abort();
    let _ = server.await;
    fs::remove_dir_all(activity_state_dir).expect("remove activity state directory");
}
