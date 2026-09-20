#![cfg(all(target_os = "linux", feature = "test-protected-index"))]

use ai_tools::application::execution::protected_index_test_support as index_test;
use ai_tools::application::execution::{start_terminal_job, JobManager, JobState};
use ai_tools::core::config::{ActivityConfig, ServerConfig};
use ai_tools::infrastructure::transport::create_router_with_jobs;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use uuid::Uuid;

fn temporary_root(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("ai-tools-{label}-{}", Uuid::new_v4()));
    fs::create_dir_all(&root).expect("temporary protected-index root");
    root
}

fn config_for(root: &Path, port: u16) -> ServerConfig {
    let activity_state_dir = root.join("activity-state");
    fs::create_dir_all(&activity_state_dir).expect("activity state directory");
    ServerConfig {
        port,
        origin: Some("http://localhost:3333".into()),
        dir: Some(root.to_string_lossy().into_owned()),
        execution_root: Some(root.to_string_lossy().into_owned()),
        default_terminal_timeout_ms: 5_000,
        max_terminal_timeout_ms: 5_000,
        activity: ActivityConfig {
            state_dir: Some(activity_state_dir.to_string_lossy().into_owned()),
            ..ActivityConfig::default()
        },
        ..ServerConfig::default()
    }
}

fn meta() -> Value {
    json!({
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientCapabilities": {}
    })
}

async fn post_mcp(
    client: &reqwest::Client,
    port: u16,
    method: &str,
    body: Value,
) -> reqwest::Response {
    client
        .post(format!("http://127.0.0.1:{port}/mcp"))
        .header("content-type", "application/json")
        .header("mcp-protocol-version", "2026-07-28")
        .header("mcp-method", method)
        .json(&body)
        .send()
        .await
        .expect("MCP request")
}

#[tokio::test]
async fn protected_index_permanent_failures_are_sticky_and_serving_stays_responsive() {
    let root = temporary_root("protected-index-regression");
    fs::write(root.join("ordinary.txt"), "ordinary").expect("ordinary entry");

    let first = index_test::prime_with_entry_limit(&root, 0)
        .expect_err("a zero-entry budget must fail before indexing the entry");
    assert_eq!(first.kind(), std::io::ErrorKind::InvalidData);
    assert!(index_test::is_permanent_error(
        std::io::ErrorKind::InvalidData
    ));
    assert!(index_test::is_permanent_error(
        std::io::ErrorKind::InvalidInput
    ));
    for transient in [
        std::io::ErrorKind::TimedOut,
        std::io::ErrorKind::Interrupted,
        std::io::ErrorKind::WouldBlock,
        std::io::ErrorKind::PermissionDenied,
        std::io::ErrorKind::Other,
    ] {
        assert!(
            !index_test::is_permanent_error(transient),
            "{transient:?} must remain retryable"
        );
    }

    let retry_started = std::time::Instant::now();
    let retry = index_test::prime_with_entry_limit(&root, 10_000)
        .expect_err("a permanently failed root must fail without rescanning");
    assert_eq!(retry.kind(), std::io::ErrorKind::InvalidData);
    assert!(
        retry_started.elapsed() < Duration::from_millis(250),
        "sticky permanent failure unexpectedly rescanned the root: {:?}",
        retry_started.elapsed()
    );
    index_test::schedule_initialization(&root)
        .expect("permanently failed roots must not be automatically requeued");
    let discovered = index_test::discover(&root)
        .expect_err("discovery must fail closed with the stored permanent error");
    assert_eq!(discovered.kind(), std::io::ErrorKind::InvalidData);
    assert_ne!(discovered.kind(), std::io::ErrorKind::WouldBlock);

    let config = config_for(&root, 0);
    let manager = JobManager::new(config.clone());
    let execution_started = std::time::Instant::now();
    let id = start_terminal_job(
        &json!({
            "command": "true",
            "cwd": root.to_string_lossy(),
            "timeout_ms": 5000
        }),
        &config,
        &manager,
    )
    .await
    .expect("terminal job should return a terminal snapshot id");
    let execution = manager.wait(&id).await.expect("terminal job wait");
    assert_eq!(execution.state, JobState::Failed);
    let diagnostic = execution
        .result
        .as_ref()
        .and_then(|result| result.content.first())
        .map(|content| content.text.as_str())
        .unwrap_or_default();
    assert!(diagnostic.contains("protected_path_discovery"));
    assert!(!diagnostic.contains("WouldBlock"));
    assert!(
        execution_started.elapsed() < Duration::from_secs(2),
        "execution spun instead of failing closed: {:?}",
        execution_started.elapsed()
    );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("loopback listener");
    let port = listener.local_addr().expect("listener address").port();
    let mut relay_config = config.clone();
    relay_config.port = port;
    let router = create_router_with_jobs(relay_config, manager.clone());
    let server = tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("MCP test server");
    });

    tokio::time::timeout(Duration::from_millis(250), manager.prepare_for_serving())
        .await
        .expect("prepare_for_serving must not perform protected-path traversal");

    let client = reqwest::Client::new();
    let initialize = tokio::time::timeout(
        Duration::from_secs(2),
        post_mcp(
            &client,
            port,
            "initialize",
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2026-07-28",
                    "capabilities": {},
                    "clientInfo": { "name": "protected-index-regression", "version": "1" },
                    "_meta": meta()
                }
            }),
        ),
    )
    .await
    .expect("initialize must remain responsive during priming");
    assert_eq!(initialize.status(), reqwest::StatusCode::OK);
    let initialize_json: Value = initialize.json().await.expect("initialize JSON");
    assert_eq!(
        initialize_json
            .pointer("/result/protocolVersion")
            .and_then(Value::as_str),
        Some("2026-07-28")
    );

    let discover = tokio::time::timeout(
        Duration::from_secs(2),
        post_mcp(
            &client,
            port,
            "server/discover",
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "server/discover",
                "params": { "_meta": meta() }
            }),
        ),
    )
    .await
    .expect("server/discover must remain responsive during priming");
    assert_eq!(discover.status(), reqwest::StatusCode::OK);
    let discover_json: Value = discover.json().await.expect("server/discover JSON");
    assert!(discover_json.pointer("/result").is_some());

    let tools_list = tokio::time::timeout(
        Duration::from_secs(2),
        post_mcp(
            &client,
            port,
            "tools/list",
            json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "tools/list",
                "params": { "_meta": meta() }
            }),
        ),
    )
    .await
    .expect("tools/list must remain responsive without startup priming");
    assert_eq!(tools_list.status(), reqwest::StatusCode::OK);
    let tools_json: Value = tools_list.json().await.expect("tools/list JSON");
    assert!(tools_json.pointer("/result/tools").is_some());

    server.abort();
    let _ = server.await;
    manager.shutdown().await;
    fs::remove_dir_all(root).expect("remove protected-index regression root");
}

#[test]
fn ordinary_directory_churn_reconciles_without_user_visible_interruption() {
    let root = temporary_root("protected-index-directory-churn");
    fs::write(root.join("ordinary.txt"), "ordinary").expect("ordinary entry");
    index_test::prime_with_entry_limit(&root, 100_000).expect("initial protected index");

    let generated = root.join("target/incremental/unit");
    fs::create_dir_all(&generated).expect("generated directory churn");
    fs::write(generated.join("artifact.o"), "ordinary").expect("generated artifact");

    index_test::discover(&root)
        .expect("ordinary generated directory churn should reconcile inline without interruption");

    fs::remove_dir_all(root).expect("remove protected-index churn root");
}

#[test]
fn snapshot_only_freshness_rejects_a_new_protected_path_before_spawn() {
    let root = temporary_root("protected-index-freshness");
    fs::write(root.join("ordinary.txt"), "ordinary").expect("ordinary entry");

    let error = index_test::snapshot_rejects_new_protected_path(
        &root,
        Path::new("deep/generated/.env.test"),
    )
    .expect_err("new protected paths must invalidate the pre-spawn snapshot");
    assert_eq!(error.kind(), std::io::ErrorKind::Interrupted);

    fs::remove_dir_all(root).expect("remove protected-index freshness root");
}

#[test]
fn protected_index_scans_a_ten_thousand_entry_workspace_within_a_terminal_warmup_window() {
    let root = temporary_root("protected-index-scan-budget");
    for index in 0..10_000 {
        fs::write(root.join(format!("entry-{index}")), "ordinary").expect("workspace entry");
    }

    let started = std::time::Instant::now();
    let scanned = index_test::prime_with_entry_limit(&root, 500_000)
        .expect("ten thousand ordinary entries should index successfully");
    assert_eq!(scanned, 10_000);
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "protected index warmup exceeded the terminal window: {:?}",
        started.elapsed()
    );

    fs::remove_dir_all(root).expect("remove protected-index scan root");
}
