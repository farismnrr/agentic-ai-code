use std::env;
use std::process::Command;

fn get_bin(name: &str) -> String {
    let mut path = env::current_exe().unwrap();
    path.pop(); // remove test exe name
    path.pop(); // remove deps dir
    path.push(name);
    path.to_str().unwrap().to_string()
}

#[test]
fn test_terminal_tool_echo() {
    let output = Command::new(get_bin("terminal-tool"))
        .arg("--no-guard")
        .arg("echo")
        .arg("hello world")
        .output()
        .expect("Failed to execute terminal-tool");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("hello world"));
}

#[test]
fn test_terminal_tool_no_args() {
    let output = Command::new(get_bin("terminal-tool"))
        .output()
        .expect("Failed to execute terminal-tool");

    assert!(!output.status.success());
}

#[test]
fn test_curl_tool_no_args() {
    let output = Command::new(get_bin("curl-tool"))
        .output()
        .expect("Failed to execute curl-tool");

    assert!(!output.status.success());
}

#[test]
fn test_curl_tool_localhost_blocked() {
    let output = Command::new(get_bin("curl-tool"))
        .arg("http://127.0.0.1")
        .output()
        .expect("Failed to execute curl-tool");

    // SSRF guard blocks the request, so exit code must be non-zero
    assert!(!output.status.success(), "Expected non-zero exit for SSRF-blocked request");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("SSRF guard blocked request"), "Expected SSRF block message, got: {stdout}");
}

#[test]
fn test_searxng_no_args() {
    let output = Command::new(get_bin("searxng-search-tool"))
        .output()
        .expect("Failed to execute searxng-search-tool");

    assert!(!output.status.success());
}
