use ai_tools::core::config::ServerConfig;
use ai_tools::infrastructure::transport::create_router;
use serde_json::{json, Value};
use std::fs;
use std::time::{Duration, Instant};

async fn call_tool_without_tasks(
    client: &reqwest::Client,
    port: u16,
    id: u64,
    name: &str,
    arguments: Value,
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
                    "io.modelcontextprotocol/clientCapabilities": {}
                }
            }
        }))
        .send()
        .await
        .expect("tool-call response");
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    response.json().await.expect("tool-call JSON")
}

fn completed_tool_record(response: &Value) -> Value {
    assert_eq!(
        response
            .pointer("/result/resultType")
            .and_then(Value::as_str),
        Some("complete"),
        "fallback tools must return an ordinary MCP tool result: {response}"
    );
    assert_eq!(
        response.pointer("/result/isError").and_then(Value::as_bool),
        Some(false),
        "fallback tool call failed: {response}"
    );
    let text = response
        .pointer("/result/content/0/text")
        .and_then(Value::as_str)
        .expect("fallback result text");
    serde_json::from_str(text).expect("fallback job JSON")
}

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
async fn terminal_job_fallback_is_pollable_without_tasks_negotiation() {
    let root = std::env::temp_dir().join(format!(
        "terminal-job-fallback-wire-{}",
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
        .timeout(Duration::from_secs(8))
        .build()
        .expect("test HTTP client");

    let async_request = call_tool_without_tasks(
        &client,
        port,
        1,
        "terminal_exec",
        json!({
            "command":"printf",
            "args":["must-not-run"],
            "cwd":root,
            "timeout_ms":5000,
            "execution_mode":"async"
        }),
    )
    .await;
    assert_eq!(
        async_request
            .pointer("/result/resultType")
            .and_then(Value::as_str),
        Some("complete")
    );
    assert_eq!(
        async_request
            .pointer("/result/isError")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert!(async_request
        .pointer("/result/content/0/text")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .contains("async execution requires MCP Tasks capability"));

    let start_arguments = json!({
        "command":"printf",
        "args":["fallback-ready"],
        "cwd":root,
        "timeout_ms":5000,
        "idempotency_key":"fallback-no-tasks"
    });
    let started = call_tool_without_tasks(
        &client,
        port,
        2,
        "terminal_job_start",
        start_arguments.clone(),
    )
    .await;
    let started_job = completed_tool_record(&started);
    let task_id = started_job["taskId"]
        .as_str()
        .expect("fallback job task ID")
        .to_owned();

    let replayed =
        call_tool_without_tasks(&client, port, 3, "terminal_job_start", start_arguments).await;
    assert_eq!(completed_tool_record(&replayed)["taskId"], task_id);

    let completed = tokio::time::timeout(Duration::from_secs(5), async {
        let mut id = 4;
        loop {
            let response = call_tool_without_tasks(
                &client,
                port,
                id,
                "terminal_job_get",
                json!({"taskId":task_id}),
            )
            .await;
            let record = completed_tool_record(&response);
            if record["status"] == "completed" {
                break record;
            }
            assert!(
                record["status"] == "queued" || record["status"] == "working",
                "unexpected terminal job state: {record}"
            );
            id += 1;
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("terminal job did not complete within the bounded poll window");
    assert_eq!(completed["output"]["stdout"], "fallback-ready");

    let long_job = call_tool_without_tasks(
        &client,
        port,
        20,
        "terminal_job_start",
        json!({
            "command":"sleep",
            "args":["30"],
            "cwd":root,
            "timeout_ms":30000
        }),
    )
    .await;
    let long_job = completed_tool_record(&long_job);
    let long_task_id = long_job["taskId"]
        .as_str()
        .expect("long-running fallback task ID")
        .to_owned();
    let cancelled = call_tool_without_tasks(
        &client,
        port,
        21,
        "terminal_job_cancel",
        json!({"taskId":long_task_id}),
    )
    .await;
    assert_eq!(completed_tool_record(&cancelled)["status"], "cancelled");

    let settled = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let response = call_tool_without_tasks(
                &client,
                port,
                22,
                "terminal_job_get",
                json!({"taskId":long_task_id}),
            )
            .await;
            let record = completed_tool_record(&response);
            if record.get("executionDurationMs").is_some() {
                break record;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("cancelled process did not settle within the bounded poll window");
    assert_eq!(settled["status"], "cancelled");

    server.abort();
    let _ = server.await;
    fs::remove_dir_all(&root).expect("remove temporary workspace");
}
