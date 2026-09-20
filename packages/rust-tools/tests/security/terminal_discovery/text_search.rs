use super::{meta, post_mcp};
use ai_tools::core::config::{ActivityConfig, ServerConfig, ToolProfile};
use ai_tools::infrastructure::transport::create_router;
use serde_json::{json, Value};
use std::fs;
use uuid::Uuid;

#[tokio::test]
async fn text_search_exclude_globs_are_enforced_over_the_wire() {
    let root = std::env::temp_dir().join(format!("ai-tools-text-search-{}", Uuid::new_v4()));
    let activity_state_dir = root.join("activity");
    fs::create_dir_all(&activity_state_dir).expect("activity state directory");
    fs::write(root.join("include.txt"), "wire-search-marker\n").unwrap();
    fs::write(root.join("exclude.tmp"), "wire-search-marker\n").unwrap();

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
    let call = post_mcp(
        &client,
        port,
        "tools/call",
        Some("text_search"),
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "text_search",
                "arguments": {
                    "query": "wire-search-marker",
                    "exclude": ["*.tmp"],
                    "max_results": 10
                },
                "_meta": meta()
            }
        }),
    )
    .await;
    assert_eq!(call.status(), reqwest::StatusCode::OK);
    let body: Value = call.json().await.expect("text_search JSON");
    assert_eq!(
        body.pointer("/result/isError").and_then(Value::as_bool),
        Some(false)
    );
    assert!(body
        .pointer("/result/content")
        .and_then(Value::as_array)
        .is_some_and(Vec::is_empty));
    let result = body
        .pointer("/result/structuredContent")
        .expect("text_search structuredContent");
    let matches = result["matches"].as_array().expect("matches");
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0]["path"], "include.txt");

    server.abort();
    let _ = server.await;
    fs::remove_dir_all(root).expect("remove text search fixture");
}
