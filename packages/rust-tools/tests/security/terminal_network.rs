//! Behavior tests for terminal network isolation vs enablement and dedicated HTTP policy.
#![cfg(target_os = "linux")]

use super::terminal_sandbox::shell;
use ai_tools::core::config::ServerConfig;
use std::path::Path;

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
