use ai_tools::core::config::{ActivityConfig, ServerConfig, ToolProfile};
use ai_tools::infrastructure::transport::create_router;
use ai_tools::interfaces::mcp::{
    retained_tool_catalog, PRIMARY_TOOL_NAMES, TOOL_DESCRIPTION_REPORTING_SUFFIX,
};
use serde_json::{json, Value};
use std::{fs, path::PathBuf};
use uuid::Uuid;

#[path = "terminal_discovery/text_search.rs"]
mod text_search;

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

    // 1. tools/list returns the retained Full base plus always-discoverable creative status.
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
        retained_tool_catalog().len() + 1,
        "Full runtime must expose the retained base plus creative_status"
    );

    // Verify key capabilities are listed
    let tool_names: Vec<&str> = tools
        .iter()
        .filter_map(|t| t.get("name").and_then(Value::as_str))
        .collect();
    assert!(tool_names.contains(&"terminal_exec"));
    assert!(tool_names.contains(&"workspace_list"));
    assert!(tool_names.contains(&"ssh_readonly_exec"));
    assert!(tool_names.contains(&"telegram_send_message"));
    assert!(tool_names.contains(&"creative_status"));
    for tool in tools {
        let description = tool
            .get("description")
            .and_then(Value::as_str)
            .expect("wire tool description");
        assert!(
            description.ends_with(TOOL_DESCRIPTION_REPORTING_SUFFIX),
            "{} must expose the reporting suffix on the MCP wire",
            tool.get("name")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
        );
    }

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
                    "args": ["ps"]
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
async fn wire_client_primary_profile_exposes_runtime_core_and_denies_full_tools() {
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

    // 0. initialize handshake with protocol 2026-07-28
    let init_res = post_mcp(
        &client,
        port,
        "initialize",
        None,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2026-07-28",
                "capabilities": {},
                "clientInfo": { "name": "e2e-wire-client", "version": "1.0" },
                "_meta": meta()
            }
        }),
    )
    .await;
    assert_eq!(init_res.status(), reqwest::StatusCode::OK);
    let init_json: Value = init_res.json().await.expect("initialize JSON");
    assert_eq!(
        init_json
            .pointer("/result/protocolVersion")
            .and_then(Value::as_str),
        Some("2026-07-28"),
        "protocolVersion must negotiate 2026-07-28"
    );

    // 1. tools/list returns the retained Primary core plus creative_status.
    let list_res = post_mcp(
        &client,
        port,
        "tools/list",
        None,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
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
        PRIMARY_TOOL_NAMES.len() + 1,
        "Primary runtime must expose the retained core plus creative_status"
    );

    let tool_names: Vec<&str> = tools
        .iter()
        .filter_map(|t| t.get("name").and_then(Value::as_str))
        .collect();

    assert!(tool_names.contains(&"creative_status"));
    for structured in [
        "directory_list",
        "file_search",
        "file_write",
        "file_edit",
        "file_read",
        "file_read_multiple",
        "text_search",
        "apply_patch",
    ] {
        let tool = tools
            .iter()
            .find(|tool| tool.get("name").and_then(Value::as_str) == Some(structured))
            .unwrap_or_else(|| panic!("missing tool: {structured}"));
        assert!(
            tool.get("outputSchema").is_some_and(Value::is_object),
            "{structured} must expose outputSchema"
        );
    }

    // Required Primary tools
    for required in [
        "terminal_exec",
        "directory_list",
        "file_search",
        "text_search",
        "file_read",
        "file_read_multiple",
        "file_write",
        "file_edit",
        "apply_patch",
        "workspace_add",
        "workspace_list",
        "workspace_get",
        "workspace_remove",
    ] {
        assert!(
            tool_names.contains(&required),
            "Primary profile must include {required}"
        );
    }

    // Full tools that MUST NOT appear in Primary
    for absent in [
        "ssh_readonly_exec",
        "http_fetch",
        "web_search",
        "git_remote_list",
        "git_remote_branch_get",
        "git_fetch",
        "git_push",
        "change_request_list",
        "change_request_get",
        "change_request_create",
        "change_request_update",
        "change_request_checks",
        "change_request_merge",
        "issue_list",
        "issue_get",
        "issue_create",
        "issue_update",
        "issue_comment",
        "issue_close",
        "issue_reopen",
        "workflow_list",
        "workflow_get",
        "workflow_run_list",
        "workflow_run_get",
        "workflow_run_jobs",
        "workflow_job_log_preview",
        "workflow_dispatch",
        "workflow_run_rerun",
        "workflow_run_cancel",
        "dependabot_alert_list",
        "dependabot_alert_get",
        "code_scanning_alert_list",
        "code_scanning_alert_get",
        "secret_scanning_alert_list",
        "secret_scanning_alert_get",
        "secret_scanning_alert_locations",
        "telegram_send_message",
    ] {
        assert!(
            !tool_names.contains(&absent),
            "Primary profile must NOT include {absent}"
        );
    }

    // 2. tools/call to tool not in Primary returns 404
    let missing_call = post_mcp(
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
                    "command": "docker"
                },
                "_meta": meta()
            }
        }),
    )
    .await;
    assert_eq!(missing_call.status(), reqwest::StatusCode::NOT_FOUND);

    // 3. tools/call to valid Primary tool succeeds
    let valid_call = post_mcp(
        &client,
        port,
        "tools/call",
        Some("workspace_list"),
        json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {
                "name": "workspace_list",
                "arguments": {},
                "_meta": meta()
            }
        }),
    )
    .await;
    assert_eq!(valid_call.status(), reqwest::StatusCode::OK);
    let valid_json: Value = valid_call.json().await.expect("workspace_list JSON");
    assert!(valid_json.pointer("/result").is_some());

    let read_call = post_mcp(
        &client,
        port,
        "tools/call",
        Some("file_read"),
        json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "tools/call",
            "params": {
                "name": "file_read",
                "arguments": {
                    "path": "Cargo.toml",
                    "limit_lines": 5
                },
                "_meta": meta()
            }
        }),
    )
    .await;
    assert_eq!(read_call.status(), reqwest::StatusCode::OK);
    let read_json: Value = read_call.json().await.expect("file_read JSON");
    assert_eq!(
        read_json
            .pointer("/result/isError")
            .and_then(Value::as_bool),
        Some(false)
    );
    let structured = read_json
        .pointer("/result/structuredContent")
        .expect("file_read structuredContent");
    assert!(structured.is_object());
    assert!(read_json
        .pointer("/result/content")
        .and_then(Value::as_array)
        .is_some_and(Vec::is_empty));

    server.abort();
    let _ = server.await;
    fs::remove_dir_all(activity_state_dir).expect("remove activity state directory");
}
