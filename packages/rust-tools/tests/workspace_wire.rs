use ai_tools::core::config::{ActivityConfig, ServerConfig, ToolProfile};
use ai_tools::infrastructure::transport::create_router;
use serde_json::{json, Value};
use std::fs;
use uuid::Uuid;

fn meta() -> Value {
    json!({
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientCapabilities": {}
    })
}

async fn post_mcp(
    client: &reqwest::Client,
    port: u16,
    name: &str,
    id: u64,
    arguments: Value,
) -> Value {
    let response = client
        .post(format!("http://127.0.0.1:{port}/mcp"))
        .header("content-type", "application/json")
        .header("mcp-protocol-version", "2026-07-28")
        .header("mcp-method", "tools/call")
        .header("mcp-name", name)
        .json(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/call",
            "params": {
                "name": name,
                "arguments": arguments,
                "_meta": meta()
            }
        }))
        .send()
        .await
        .expect("MCP request");
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    response.json().await.expect("tools/call JSON")
}

#[tokio::test]
async fn structured_workspace_mutations_succeed_over_the_wire() {
    let root = std::env::temp_dir().join(format!("ai-tools-mutation-wire-{}", Uuid::new_v4()));
    let activity_state_dir = root.join("activity");
    fs::create_dir_all(&activity_state_dir).expect("activity state directory");
    fs::write(root.join("edit.txt"), "alpha\nbeta\ngamma\n").unwrap();
    fs::write(root.join("patch.txt"), "one\ntwo\n").unwrap();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("loopback listener");
    let port = listener.local_addr().expect("listener address").port();
    let config = ServerConfig {
        port,
        origin: Some("http://localhost:3333".into()),
        dir: Some(root.to_string_lossy().into_owned()),
        execution_root: Some(root.to_string_lossy().into_owned()),
        tool_profile: ToolProfile::Primary,
        activity: ActivityConfig {
            state_dir: Some(activity_state_dir.to_string_lossy().into_owned()),
            ..ActivityConfig::default()
        },
        ..ServerConfig::default()
    };
    let router = create_router(config);
    let server = tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("MCP test server");
    });
    let client = reqwest::Client::new();

    let read = post_mcp(
        &client,
        port,
        "file_read",
        1,
        json!({"path":"edit.txt","limit_lines":20}),
    )
    .await;
    assert_eq!(
        read.pointer("/result/isError").and_then(Value::as_bool),
        Some(false)
    );
    let hash = read
        .pointer("/result/structuredContent/sha256")
        .and_then(Value::as_str)
        .expect("file SHA")
        .to_owned();

    let edit_preview = post_mcp(
        &client,
        port,
        "file_edit",
        2,
        json!({
            "path":"edit.txt",
            "old_text":"beta",
            "new_text":"BETA",
            "dry_run":true,
            "expected_sha256":hash
        }),
    )
    .await;
    assert_eq!(
        edit_preview
            .pointer("/result/isError")
            .and_then(Value::as_bool),
        Some(false),
        "file_edit dry run must succeed: {edit_preview}"
    );

    let guarded_write = post_mcp(
        &client,
        port,
        "file_write",
        3,
        json!({
            "path":"edit.txt",
            "content":"alpha\nbeta\ngamma\n",
            "overwrite":true,
            "expected_sha256":hash
        }),
    )
    .await;
    assert_eq!(
        guarded_write
            .pointer("/result/isError")
            .and_then(Value::as_bool),
        Some(false),
        "guarded file_write must succeed: {guarded_write}"
    );

    let patch = post_mcp(
        &client,
        port,
        "apply_patch",
        4,
        json!({
            "patch":"--- patch.txt\n+++ patch.txt\n@@ -1,2 +1,2 @@\n one\n-two\n+three\n",
            "dry_run":true
        }),
    )
    .await;
    assert_eq!(
        patch.pointer("/result/isError").and_then(Value::as_bool),
        Some(false),
        "apply_patch dry run must succeed: {patch}"
    );

    let created = post_mcp(
        &client,
        port,
        "file_write",
        5,
        json!({"path":"ambiguous.txt","content":"same\nsame\n"}),
    )
    .await;
    assert_eq!(
        created.pointer("/result/isError").and_then(Value::as_bool),
        Some(false)
    );

    let ambiguous = post_mcp(
        &client,
        port,
        "file_edit",
        6,
        json!({"path":"ambiguous.txt","old_text":"same","new_text":"changed"}),
    )
    .await;
    assert_eq!(
        ambiguous
            .pointer("/result/isError")
            .and_then(Value::as_bool),
        Some(true)
    );

    let stale = post_mcp(
        &client,
        port,
        "file_edit",
        7,
        json!({
            "path":"edit.txt",
            "old_text":"beta",
            "new_text":"BETA",
            "expected_sha256":"0000000000000000000000000000000000000000000000000000000000000000"
        }),
    )
    .await;
    assert_eq!(
        stale.pointer("/result/isError").and_then(Value::as_bool),
        Some(true)
    );

    server.abort();
    let _ = server.await;
    fs::remove_dir_all(root).expect("remove mutation fixture");
}
