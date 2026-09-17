//! Behavior tests for terminal network isolation vs enablement and dedicated HTTP policy.
#![cfg(target_os = "linux")]

use super::terminal_sandbox::shell;
use ai_tools::core::config::ServerConfig;
use serde_json::{json, Value};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

struct RelayProcess(Child);

impl RelayProcess {
    fn try_wait(&mut self) -> std::io::Result<Option<std::process::ExitStatus>> {
        self.0.try_wait()
    }

    fn stop(&mut self) {
        if self.0.try_wait().ok().flatten().is_some() {
            return;
        }
        unsafe {
            libc::kill(self.0.id() as i32, libc::SIGTERM);
        }
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            if self.0.try_wait().ok().flatten().is_some() {
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

impl Drop for RelayProcess {
    fn drop(&mut self) {
        self.stop();
    }
}

pub(super) async fn test_network_boundaries(config: &mut ServerConfig, root: &Path) {
    // 1. Terminal network disabled (default): outbound connections fail at the
    // sandbox execution layer (via --unshare-net), not by name-matching 'curl'.
    assert!(!config.allow_terminal_network);

    let loopback_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("host loopback listener");
    let port = loopback_listener
        .local_addr()
        .expect("listener addr")
        .port();

    // Raw socket connection via Python fails because loopback namespace is unshared
    let py_socket = shell(
        config,
        root,
        &format!(
            "python3 -c \"import socket; s = socket.socket(); s.settimeout(0.5); s.connect(('127.0.0.1', {port}))\" 2>&1"
        ),
    )
    .await;
    assert_ne!(
        py_socket.exit_code,
        Some(0),
        "raw socket connect must fail when terminal network is disabled: {}",
        py_socket.stdout
    );

    // curl to host loopback port fails
    let curl_call = shell(
        config,
        root,
        &format!("curl -s --connect-timeout 1 http://127.0.0.1:{port} 2>&1"),
    )
    .await;
    assert_ne!(
        curl_call.exit_code,
        Some(0),
        "curl must fail when terminal network is disabled"
    );

    // Outbound connection to public IP fails
    let ext_socket = shell(
        config,
        root,
        "python3 -c \"import socket; s = socket.socket(); s.settimeout(0.5); s.connect(('1.1.1.1', 80))\" 2>&1",
    )
    .await;
    assert_ne!(
        ext_socket.exit_code,
        Some(0),
        "external connect must fail when terminal network is disabled"
    );

    // 2. Terminal network enabled: ordinary network commands are permitted
    config.allow_terminal_network = true;

    let net_enabled = shell(
        config,
        root,
        &format!(
            "python3 -c \"import socket; s = socket.socket(); s.settimeout(2.0); s.connect(('127.0.0.1', {port})); print('network-enabled-ok')\""
        ),
    )
    .await;
    assert_eq!(
        net_enabled.exit_code,
        Some(0),
        "ordinary network command must succeed when terminal network is enabled: {}",
        net_enabled.stderr
    );
    assert!(net_enabled.stdout.contains("network-enabled-ok"));

    // Reset to default disabled
    config.allow_terminal_network = false;

    // 3. Dedicated HTTP tool policy remains independent of terminal network flag
    assert!(!config.allow_terminal_network);
    let http_tool = ai_tools::interfaces::mcp::find_tool("http_fetch")
        .expect("http_fetch tool must exist in catalog");
    assert!(
        ai_tools::application::execution::tool_call_supports_tasks(
            &http_tool,
            &serde_json::json!({"url": "http://127.0.0.1:80", "method": "GET"})
        ),
        "http_fetch retains its independent task support"
    );
}

#[tokio::test]
async fn http_fetch_reaches_its_policy_without_scanning_the_workspace() {
    use std::fs;
    use std::os::unix::fs::symlink;

    let fixture =
        std::env::temp_dir().join(format!("http-fetch-network-only-{}", uuid::Uuid::new_v4()));
    let workspace = fixture.join("workspace");
    let home = fixture.join("home");
    let activity_state = fixture.join("activity-state");
    fs::create_dir_all(&workspace).expect("create workspace");
    fs::create_dir_all(&home).expect("create isolated host home");
    symlink("/etc/shadow", workspace.join(".env.local"))
        .expect("create protected-shaped workspace symlink");

    let reservation = std::net::TcpListener::bind("127.0.0.1:0").expect("reserve relay port");
    let port = reservation.local_addr().expect("relay address").port();
    drop(reservation);
    let mut relay = RelayProcess(
        Command::new(env!("CARGO_BIN_EXE_ai-tools"))
            .args([
                "relay",
                "--mode",
                "local",
                "--bind-host",
                "127.0.0.1",
                "--origin",
                "http://localhost:3333",
                "--workspace-root",
            ])
            .arg(&workspace)
            .arg("--execution-root")
            .arg(&workspace)
            .arg("--activity-mode")
            .arg("off")
            .arg("--activity-state-dir")
            .arg(&activity_state)
            .arg("--port")
            .arg(port.to_string())
            .env("HOME", &home)
            .env("XDG_STATE_HOME", home.join(".local/state"))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("start local relay child"),
    );

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .expect("test HTTP client");
    let health_url = format!("http://127.0.0.1:{port}/health");
    let startup_deadline = Instant::now() + Duration::from_secs(8);
    loop {
        if let Some(status) = relay.try_wait().expect("check relay process") {
            panic!("relay exited before readiness with {status}");
        }
        if client
            .get(&health_url)
            .send()
            .await
            .is_ok_and(|response| response.status().is_success())
        {
            break;
        }
        assert!(
            Instant::now() < startup_deadline,
            "relay did not become ready within its startup bound"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    let response = client
        .post(format!("http://127.0.0.1:{port}/mcp"))
        .header("content-type", "application/json")
        .header("origin", "http://localhost:3333")
        .header("mcp-protocol-version", "2026-07-28")
        .header("mcp-method", "tools/call")
        .header("mcp-name", "http_fetch")
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "http_fetch",
                "arguments": {
                    "url": "http://127.0.0.1:9/blocked",
                    "timeout_ms": 1500,
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
        .expect("HTTP fetch MCP response");
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let body: Value = response.json().await.expect("MCP response JSON");
    let text = body
        .pointer("/result/content/0/text")
        .and_then(Value::as_str)
        .expect("HTTP fetch result text");
    assert!(
        text.contains("SSRF Error"),
        "HTTP fetch did not reach its URL policy: {body}"
    );
    assert!(
        !text.contains("protected_path_discovery"),
        "HTTP fetch still scanned the workspace: {body}"
    );

    relay.stop();
    fs::remove_dir_all(&fixture).expect("remove temporary relay fixture");
}
