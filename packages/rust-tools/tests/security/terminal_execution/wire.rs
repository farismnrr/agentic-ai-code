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

#[tokio::test]
async fn auto_terminal_exec_keeps_fast_results_sync_and_hands_off_slow_jobs_once() {
    let root = std::env::temp_dir().join(format!(
        "terminal-exec-auto-handoff-wire-{}",
        uuid::Uuid::new_v4()
    ));
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

    // Warm the selected sandbox so the fast-command assertion measures command
    // routing, not its first protected-path index discovery.
    let warm = post_tool_call(
        &client,
        port,
        json!(1),
        "terminal_exec",
        json!({ "command": "true", "cwd": root, "execution_mode": "sync" }),
    )
    .await;
    assert_eq!(
        warm.pointer("/result/isError").and_then(Value::as_bool),
        Some(false)
    );

    let quick = post_tool_call(
        &client,
        port,
        json!(2),
        "terminal_exec",
        json!({
            "command": "printf",
            "args": ["quick"],
            "cwd": root,
            "timeout_ms": 5_000,
            "sync_wait_ms": 1_000,
            "execution_mode": "auto"
        }),
    )
    .await;
    assert_eq!(
        quick.pointer("/result/resultType").and_then(Value::as_str),
        Some("complete"),
        "quick auto command should return its normal result: {quick}"
    );
    assert_eq!(
        quick.pointer("/result/isError").and_then(Value::as_bool),
        Some(false)
    );
    assert!(
        quick
            .pointer("/result/content/0/text")
            .and_then(Value::as_str)
            .is_some_and(|text| text.contains("quick")),
        "quick auto command output missing: {quick}"
    );
    assert!(quick.pointer("/result/_meta/terminalJob/taskId").is_none());

    let long_arguments = json!({
        "command": "sh",
        "args": ["-c", "printf x >> auto-count; sleep 0.35; cat auto-count"],
        "cwd": root,
        "timeout_ms": 5_000,
        "sync_wait_ms": 50,
        "execution_mode": "auto"
    });
    let started = Instant::now();
    let handed_off = post_tool_call(
        &client,
        port,
        json!(3),
        "terminal_exec",
        long_arguments.clone(),
    )
    .await;
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "auto handoff exceeded its short wait window: {:?}",
        started.elapsed()
    );
    let task_id = handed_off
        .pointer("/result/_meta/terminalJob/taskId")
        .and_then(Value::as_str)
        .expect("legacy clients receive a terminal task id")
        .to_owned();
    assert_eq!(
        handed_off
            .pointer("/result/resultType")
            .and_then(Value::as_str),
        Some("complete"),
        "legacy client gets a normal tool result with a pollable handle: {handed_off}"
    );

    // Replaying the same JSON-RPC identity must return the accepted job, not
    // start a second write to the counter file.
    let retried = post_tool_call(&client, port, json!(3), "terminal_exec", long_arguments).await;
    assert_eq!(
        retried
            .pointer("/result/_meta/terminalJob/taskId")
            .and_then(Value::as_str),
        Some(task_id.as_str()),
        "retry must identify the same running job: {retried}"
    );

    let mut completed = None;
    for poll in 0..50 {
        let result = post_tool_call(
            &client,
            port,
            json!(100 + poll),
            "terminal_job_get",
            json!({ "taskId": task_id }),
        )
        .await;
        if result.pointer("/result/status").and_then(Value::as_str) == Some("completed") {
            completed = Some(result);
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    let completed = completed.expect("auto handoff task should complete and be pollable");
    assert_eq!(
        completed
            .pointer("/result/output/exitCode")
            .and_then(Value::as_i64),
        Some(0),
        "task should exit successfully: {completed}"
    );
    assert_eq!(
        completed
            .pointer("/result/output/stdout")
            .and_then(Value::as_str),
        Some("x"),
        "idempotent replay must not execute the command twice: {completed}"
    );

    let modern = post_tool_call_with_capabilities(
        &client,
        port,
        json!(4),
        "terminal_exec",
        json!({
            "command": "sh",
            "args": ["-c", "sleep 0.35; printf modern"],
            "cwd": root,
            "timeout_ms": 5_000,
            "sync_wait_ms": 50,
            "execution_mode": "auto"
        }),
        json!({ "extensions": { "io.modelcontextprotocol/tasks": {} } }),
    )
    .await;
    assert_eq!(
        modern.pointer("/result/resultType").and_then(Value::as_str),
        Some("task"),
        "Tasks-capable clients should receive the task envelope: {modern}"
    );
    let modern_task_id = modern
        .pointer("/result/taskId")
        .and_then(Value::as_str)
        .expect("MCP Tasks response contains taskId");
    let mut modern_completed = None;
    for poll in 0..50 {
        let result = post_task_get(&client, port, json!(200 + poll), modern_task_id).await;
        if result.pointer("/result/status").and_then(Value::as_str) == Some("completed") {
            modern_completed = Some(result);
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    let modern_completed = modern_completed.expect("modern task should remain pollable");
    assert_eq!(
        modern_completed
            .pointer("/result/output/stdout")
            .and_then(Value::as_str),
        Some("modern")
    );

    server.abort();
    let _ = server.await;
    fs::remove_dir_all(&root).expect("remove temporary workspace");
}

async fn post_tool_call(
    client: &reqwest::Client,
    port: u16,
    id: Value,
    name: &str,
    arguments: Value,
) -> Value {
    post_tool_call_with_capabilities(client, port, id, name, arguments, json!({})).await
}

async fn post_tool_call_with_capabilities(
    client: &reqwest::Client,
    port: u16,
    id: Value,
    name: &str,
    arguments: Value,
    client_capabilities: Value,
) -> Value {
    let response = client
        .post(format!("http://127.0.0.1:{port}/mcp"))
        .header("content-type", "application/json")
        .header("origin", "http://localhost:3333")
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
                "_meta": {
                    "io.modelcontextprotocol/protocolVersion": "2026-07-28",
                    "io.modelcontextprotocol/clientCapabilities": client_capabilities
                }
            }
        }))
        .send()
        .await
        .expect("MCP tool call response");
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    response.json().await.expect("MCP response JSON")
}

async fn post_task_get(client: &reqwest::Client, port: u16, id: Value, task_id: &str) -> Value {
    let response = client
        .post(format!("http://127.0.0.1:{port}/mcp"))
        .header("content-type", "application/json")
        .header("origin", "http://localhost:3333")
        .header("mcp-protocol-version", "2026-07-28")
        .header("mcp-method", "tasks/get")
        .header("mcp-name", task_id)
        .json(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tasks/get",
            "params": {
                "taskId": task_id,
                "_meta": {
                    "io.modelcontextprotocol/protocolVersion": "2026-07-28",
                    "io.modelcontextprotocol/clientCapabilities": {
                        "extensions": { "io.modelcontextprotocol/tasks": {} }
                    }
                }
            }
        }))
        .send()
        .await
        .expect("MCP task get response");
    let status = response.status();
    let body = response.text().await.expect("MCP task response body");
    assert_eq!(status, reqwest::StatusCode::OK, "MCP task response: {body}");
    serde_json::from_str(&body).expect("MCP task result JSON")
}
