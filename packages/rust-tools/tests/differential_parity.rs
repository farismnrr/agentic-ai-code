use std::collections::HashMap;
use std::env;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::thread;

fn get_project_root() -> PathBuf {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    PathBuf::from(manifest_dir)
}

fn run_js_cli(cli_file: &str, args: &[&str], envs: &HashMap<&str, &str>) -> Output {
    let mut root = get_project_root();
    root.push("tests");
    root.push(cli_file);

    let mut cmd = Command::new("node");
    cmd.arg("--experimental-strip-types").arg(root).args(args);

    for (k, v) in envs {
        cmd.env(k, v);
    }

    cmd.output()
        .unwrap_or_else(|e| panic!("Failed to execute node for {cli_file}: {e}"))
}

fn run_rust_cli(bin_name: &str, args: &[&str], envs: &HashMap<&str, &str>) -> Output {
    // Determine target debug directory
    let mut root = get_project_root();
    root.push("..");
    root.push("..");
    root.push("target");
    root.push("debug");
    root.push(bin_name);

    // If not found, fallback to cargo run
    let mut cmd = if root.exists() {
        Command::new(root)
    } else {
        let mut c = Command::new(env::var("CARGO").unwrap_or_else(|_| "cargo".to_string()));
        c.arg("run").arg("--bin").arg(bin_name).arg("--");
        c
    };

    cmd.args(args);

    for (k, v) in envs {
        cmd.env(k, v);
    }

    cmd.output()
        .unwrap_or_else(|e| panic!("Failed to execute rust bin {bin_name}: {e}"))
}

fn assert_differential_parity(js_out: &Output, rs_out: &Output, allow_stderr_mismatch: bool) {
    let js_stdout = String::from_utf8_lossy(&js_out.stdout);
    let rs_stdout = String::from_utf8_lossy(&rs_out.stdout);
    let js_stderr = String::from_utf8_lossy(&js_out.stderr).to_string();
    let rs_stderr = String::from_utf8_lossy(&rs_out.stderr).to_string();

    // Normalization rules for structured errors
    let mut js_normalized = js_stdout.to_string();
    let mut rs_normalized = rs_stdout.to_string();

    // Curl URL malformed
    if js_normalized.contains("Error: Invalid URL")
        && rs_normalized.contains("Error: relative URL without a base")
    {
        js_normalized = "URL Error".to_string();
        rs_normalized = "URL Error".to_string();
    }
    if js_normalized.contains("Error: fetch failed")
        && rs_normalized.contains("Error: error sending request for url")
    {
        js_normalized = "Fetch Error".to_string();
        rs_normalized = "Fetch Error".to_string();
    }
    if js_normalized.contains("Error: SSRF guard blocked request.")
        && rs_normalized.contains("Error: SSRF guard blocked request")
    {
        js_normalized = "SSRF Error".to_string();
        rs_normalized = "SSRF Error".to_string();
    }
    if js_normalized.contains("ENOENT") && rs_normalized.contains("No such file or directory") {
        js_normalized = "ENOENT Error".to_string();
        rs_normalized = "ENOENT Error".to_string();
    }

    js_normalized = js_normalized.replace("Snippet: undefined", "Snippet: ");

    // Some timeout normalization just in case
    if js_normalized.contains("timed out") && rs_normalized.contains("timed out") {
        js_normalized = "Timeout Error".to_string();
        rs_normalized = "Timeout Error".to_string();
    }

    if js_out.status.code() != rs_out.status.code() {
        println!("JS Exit: {:?}", js_out.status.code());
        println!("RS Exit: {:?}", rs_out.status.code());
        println!("JS Stdout: {js_stdout}");
        println!("RS Stdout: {rs_stdout}");
        println!("JS Stderr: {js_stderr}");
        println!("RS Stderr: {rs_stderr}");
        panic!("Exit code mismatch");
    }

    // Exact stdout matching with normalization
    if js_normalized.trim() != rs_normalized.trim() {
        println!("JS Stdout:\n{js_stdout}");
        println!("RS Stdout:\n{rs_stdout}");
        panic!("Stdout mismatch");
    }

    if !allow_stderr_mismatch && js_stderr.trim() != rs_stderr.trim() {
        println!("JS Stderr:\n{js_stderr}");
        println!("RS Stderr:\n{rs_stderr}");
        panic!("Stderr mismatch");
    }
}

fn spawn_mock_server() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = stream.unwrap();
            let mut buffer = [0; 1024];
            stream.read(&mut buffer).unwrap();

            let request = String::from_utf8_lossy(&buffer);

            if request.contains("GET /search?q=test&format=json") {
                let response = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"results\": [{\"title\": \"Mock Result\", \"url\": \"http://mock.local\"}]}";
                stream.write(response.as_bytes()).unwrap();
            } else if request.contains("GET /timeout") {
                thread::sleep(std::time::Duration::from_millis(2000));
                let response = "HTTP/1.1 200 OK\r\n\r\nTimeout";
                stream.write(response.as_bytes()).unwrap();
            } else {
                let response = "HTTP/1.1 200 OK\r\n\r\nOK";
                stream.write(response.as_bytes()).unwrap();
            }
            stream.flush().unwrap();
        }
    });
    port
}

// ---------------------------------------------------------
// Terminal Tool Parity Tests
// ---------------------------------------------------------
#[test]
fn test_terminal_basic_echo() {
    let args = vec!["--no-guard", "echo", "hello world"];
    let js_out = run_js_cli("terminal_cli.js", &args, &HashMap::new());
    let rs_out = run_rust_cli("terminal-tool", &args, &HashMap::new());
    assert_differential_parity(&js_out, &rs_out, true);
}

#[test]
fn test_terminal_missing_command() {
    let args = vec!["--no-guard"];
    let js_out = run_js_cli("terminal_cli.js", &args, &HashMap::new());
    let rs_out = run_rust_cli("terminal-tool", &args, &HashMap::new());
    assert_differential_parity(&js_out, &rs_out, true);
}

#[test]
fn test_terminal_guard_blocked() {
    // Missing --no-guard should trigger block
    let args = vec!["echo", "hello"];
    let js_out = run_js_cli("terminal_cli.js", &args, &HashMap::new());
    let rs_out = run_rust_cli("terminal-tool", &args, &HashMap::new());
    assert!(
        !rs_out.status.success(),
        "Rust should exit non-zero for SSRF block"
    );
    let rs_stdout = String::from_utf8_lossy(&rs_out.stdout);
    let js_stdout = String::from_utf8_lossy(&js_out.stdout);
    assert!(
        rs_stdout.to_lowercase().contains("blocked") || rs_stdout.to_lowercase().contains("error"),
        "Expected block message in Rust, got: {rs_stdout}"
    );
    assert!(
        js_stdout.to_lowercase().contains("blocked") || js_stdout.to_lowercase().contains("error"),
        "Expected block message in JS, got: {js_stdout}"
    );
}

#[test]
fn test_terminal_timeout() {
    // Test timeout handling with a sleep command - only for Rust, as JS hardcodes 30s
    let args = vec!["--no-guard", "--timeout", "100", "sleep", "2"];
    let rs_out = run_rust_cli("terminal-tool", &args, &HashMap::new());

    let rs_stdout = String::from_utf8_lossy(&rs_out.stdout);
    assert!(
        rs_stdout.contains("timed out"),
        "Expected timeout message in Rust stdout"
    );
}

#[test]
fn test_terminal_dependency_failure() {
    let args = vec!["--no-guard", "nonexistent_command_12345"];
    let js_out = run_js_cli("terminal_cli.js", &args, &HashMap::new());
    let rs_out = run_rust_cli("terminal-tool", &args, &HashMap::new());
    
    assert!(
        !rs_out.status.success(),
        "Rust should exit non-zero for missing binary"
    );
    
    let rs_stdout = String::from_utf8_lossy(&rs_out.stdout);
    let js_stdout = String::from_utf8_lossy(&js_out.stdout);
    assert!(
        rs_stdout.contains("No such file or directory"),
        "Expected ENOENT message in Rust, got: {rs_stdout}"
    );
    assert!(
        js_stdout.contains("ENOENT") || js_stdout.contains("No such file or directory"),
        "Expected ENOENT message in JS, got: {js_stdout}"
    );
}

// ---------------------------------------------------------
// Curl Tool Parity Tests
// ---------------------------------------------------------
#[test]
fn test_curl_guard_blocked() {
    // Missing --no-guard should trigger block on localhost.
    // NOTE: JS oracle exits 0 even when SSRF-blocked (known JS bug).
    // Rust correctly exits 1. We only compare stdout message, not exit code.
    let args = vec!["http://127.0.0.1:80"];
    let js_out = run_js_cli("curl_cli.js", &args, &HashMap::new());
    let rs_out = run_rust_cli("curl-tool", &args, &HashMap::new());
    // Rust must exit non-zero (SSRF block)
    assert!(
        !rs_out.status.success(),
        "Rust should exit non-zero for SSRF block"
    );
    // Both must mention SSRF/URL guard in output
    let rs_stdout = String::from_utf8_lossy(&rs_out.stdout);
    let js_stdout = String::from_utf8_lossy(&js_out.stdout);
    assert!(
        rs_stdout.to_lowercase().contains("ssrf") || rs_stdout.contains("blocked"),
        "Expected SSRF block message in Rust, got: {rs_stdout}"
    );
    assert!(
        js_stdout.to_lowercase().contains("error") || js_stdout.to_lowercase().contains("invalid"),
        "Expected error message in JS, got: {js_stdout}"
    );
}

#[test]
fn test_curl_malformed_url() {
    // NOTE: JS oracle exits 0 for malformed URLs (known JS bug).
    // Rust correctly exits 1. We only verify that both produce an error message.
    let args = vec!["--no-guard", "not_a_url"];
    let rs_out = run_rust_cli("curl-tool", &args, &HashMap::new());
    // Rust must exit non-zero for invalid URL
    assert!(
        !rs_out.status.success(),
        "Rust should exit non-zero for malformed URL"
    );
    let rs_stdout = String::from_utf8_lossy(&rs_out.stdout);
    assert!(
        rs_stdout.to_lowercase().contains("error"),
        "Expected error message in Rust for malformed URL, got: {rs_stdout}"
    );
}

#[test]
fn test_curl_timeout() {
    let port = spawn_mock_server();
    let url = format!("http://127.0.0.1:{port}/timeout");
    let args = vec!["--no-guard", "--timeout", "100", &url];
    let rs_out = run_rust_cli("curl-tool", &args, &HashMap::new());

    assert!(
        !rs_out.status.success(),
        "Rust should exit non-zero on timeout"
    );
    let rs_stdout = String::from_utf8_lossy(&rs_out.stdout);
    assert!(
        rs_stdout.to_lowercase().contains("timeout")
            || rs_stdout.to_lowercase().contains("timed out")
            || rs_stdout.to_lowercase().contains("deadline has elapsed"),
        "Expected timeout message in Rust stdout, got: {rs_stdout}"
    );
}

#[test]
fn test_curl_deterministic_fixture() {
    let port = spawn_mock_server();
    let url = format!("http://127.0.0.1:{port}/");
    let args = vec!["--no-guard", &url];
    let js_out = run_js_cli("curl_cli.js", &args, &HashMap::new());
    let rs_out = run_rust_cli("curl-tool", &args, &HashMap::new());
    assert_differential_parity(&js_out, &rs_out, true);
}

// ---------------------------------------------------------
// SearXNG Tool Parity Tests
// ---------------------------------------------------------
#[test]
fn test_searxng_malformed_query() {
    let args = vec!["query", "--base-url", "http://invalid-domain.local"];
    let js_out = run_js_cli("searxng_cli.js", &args, &HashMap::new());
    let rs_out = run_rust_cli("searxng-search-tool", &args, &HashMap::new());
    assert_differential_parity(&js_out, &rs_out, true);
}

#[test]
fn test_searxng_deterministic_success() {
    let port = spawn_mock_server();
    let base_url = format!("http://127.0.0.1:{port}");
    let args = vec!["test", "--base-url", &base_url];
    let js_out = run_js_cli("searxng_cli.js", &args, &HashMap::new());
    let rs_out = run_rust_cli("searxng-search-tool", &args, &HashMap::new());
    assert_differential_parity(&js_out, &rs_out, true);
}
