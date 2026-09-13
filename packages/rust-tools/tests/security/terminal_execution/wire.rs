use ai_tools::core::config::ServerConfig;
use ai_tools::infrastructure::transport::create_router;
use serde_json::{json, Value};
use std::fs;
use std::time::{Duration, Instant};

#[tokio::test]
async fn sync_terminal_exec_runs_minimal_commands_over_mcp_http() {
    let root =
        std::env::temp_dir().join(format!("terminal-exec-sync-wire-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&root).expect("temporary authorized workspace");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("loopback listener");
    let port = listener.local_addr().expect("listener address").port();
    let config = ServerConfig {
        port,
        origin: Some("http://localhost:3333".into()),
        dir: Some(root.to_string_lossy().into_owned()),
        execution_root: Some(root.to_string_lossy().into_owned()),
        default_terminal_timeout_ms: 5_000,
        ..ServerConfig::default()
    };
    let router = create_router(config);
    let server = tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("MCP test server");
    });
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(12))
        .build()
        .expect("test HTTP client");

    for (id, command) in [(1, "true"), (2, "printf ok"), (3, "rm -f nonexistent")] {
        let started = Instant::now();
        let response = client
            .post(format!("http://127.0.0.1:{port}/mcp"))
            .header("content-type", "application/json")
            .header("origin", "http://localhost:3333")
            .header("mcp-protocol-version", "2026-07-28")
            .header("mcp-method", "tools/call")
            .header("mcp-name", "terminal_exec")
            .json(&json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "tools/call",
                "params": {
                    "name": "terminal_exec",
                    "arguments": {
                        "command": command,
                        "cwd": root,
                        "timeout_ms": 5_000,
                        "execution_mode": "sync"
                    },
                    "_meta": {
                        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
                        "io.modelcontextprotocol/clientCapabilities": {}
                    }
                }
            }))
            .send()
            .await
            .expect("terminal_exec response");
        let status = response.status();
        let result: Value = response.json().await.expect("MCP response JSON");
        let elapsed = started.elapsed();
        assert_eq!(status, reqwest::StatusCode::OK);
        assert_eq!(
            result.pointer("/result/isError").and_then(Value::as_bool),
            Some(false),
            "sync terminal_exec failed for {command:?}: {result}"
        );
        assert!(
            elapsed < Duration::from_secs(8),
            "timeout_ms=5000 should bound sync terminal_exec for {command:?}, took {elapsed:?}"
        );
    }

    server.abort();
    let _ = server.await;
    fs::remove_dir_all(&root).expect("remove temporary workspace");
}
