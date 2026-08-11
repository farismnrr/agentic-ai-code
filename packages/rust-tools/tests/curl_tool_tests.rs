use std::env;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::Command;
use std::thread;

fn get_bin() -> String {
    let mut path = env::current_exe().unwrap();
    path.pop();
    path.pop();
    path.push("curl-tool");
    if !path.exists() {
        let mut root = std::path::PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
        root.push("..");
        root.push("..");
        root.push("target");
        root.push("debug");
        root.push("curl-tool");
        if root.exists() {
            return root.to_str().unwrap().to_string();
        }
    }
    path.to_str().unwrap().to_string()
}

fn spawn_mock_server() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    thread::spawn(move || {
        for stream in listener.incoming() {
            if let Ok(mut stream) = stream {
                let mut buffer = [0; 1024];
                if stream.read(&mut buffer).is_ok() {
                    let request = String::from_utf8_lossy(&buffer);
                    if request.contains("redirect-to-private") {
                        let response =
                            "HTTP/1.1 302 Found\r\nLocation: http://192.168.1.1/\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
                        let _ = stream.write(response.as_bytes());
                    } else if request.contains("redirect-to-loopback") {
                        let response = "HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1/\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
                        let _ = stream.write(response.as_bytes());
                    } else if request.contains("redirect-to-link-local") {
                        let response =
                            "HTTP/1.1 302 Found\r\nLocation: http://169.254.169.254/\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
                        let _ = stream.write(response.as_bytes());
                    } else if request.contains("redirect-to-localtest") {
                        let response =
                            "HTTP/1.1 302 Found\r\nLocation: http://localtest.me/\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
                        let _ = stream.write(response.as_bytes());
                    } else if request.contains("redirect-to-mockserver") {
                        let response =
                            format!("HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:{port}/\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                        let _ = stream.write(response.as_bytes());
                        let _ = stream.flush();
                        std::thread::sleep(std::time::Duration::from_millis(50));
                    } else if request.contains("GET /timeout") {
                        thread::sleep(std::time::Duration::from_millis(1500));
                        let response = "HTTP/1.1 200 OK\r\nContent-Length: 7\r\nConnection: close\r\n\r\nTimeout";
                        let _ = stream.write(response.as_bytes());
                        let _ = stream.flush();
                    } else {
                        let response = "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nOK";
                        let _ = stream.write(response.as_bytes());
                        let _ = stream.flush();
                        std::thread::sleep(std::time::Duration::from_millis(50));
                    }
                }
            }
        }
    });
    port
}

#[test]
fn test_ipv4_mapped_ipv6_blocked() {
    let output = Command::new(get_bin())
        .arg("http://[::ffff:127.0.0.1]")
        .output()
        .expect("Failed to run curl-tool");

    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("STDOUT: {stdout}");
    assert_eq!(
        output.status.code(),
        Some(1),
        "Expected exit code 1 (Runtime Error) for SSRF block"
    );
    assert!(stdout.contains("SSRF Error") || stdout.contains("SSRF guard blocked"));
}

#[test]
fn test_hostname_resolves_to_private() {
    // localtest.me resolves to 127.0.0.1
    let output = Command::new(get_bin())
        .arg("http://localtest.me")
        .output()
        .expect("Failed to run curl-tool");

    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("STDOUT: {stdout}");
    assert!(stdout.contains("SSRF Error") && stdout.contains("resolves to private/local IP"));
}

#[test]
fn test_redirect_to_private_blocked() {
    let port = spawn_mock_server();
    let output = Command::new(get_bin())
        .arg(format!("http://8.8.8.8/redirect-to-private"))
        .env("HTTP_PROXY", format!("http://127.0.0.1:{}", port))
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("STDOUT: {stdout}");
    assert!(stdout.contains("SSRF Error") && stdout.contains("SSRF guard blocked redirect"));
}

#[test]
fn test_redirect_to_loopback_blocked() {
    let port = spawn_mock_server();
    let output = Command::new(get_bin())
        .arg(format!("http://8.8.8.8/redirect-to-loopback"))
        .env("HTTP_PROXY", format!("http://127.0.0.1:{}", port))
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("STDOUT: {stdout}");
    assert!(stdout.contains("SSRF Error") && stdout.contains("SSRF guard blocked redirect"));
}

#[test]
fn test_redirect_to_link_local_blocked() {
    let port = spawn_mock_server();
    let output = Command::new(get_bin())
        .arg(format!("http://8.8.8.8/redirect-to-link-local"))
        .env("HTTP_PROXY", format!("http://127.0.0.1:{}", port))
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("STDOUT: {stdout}");
    assert!(stdout.contains("SSRF Error") && stdout.contains("SSRF guard blocked redirect"));
}

#[test]
fn test_redirect_revalidation_dns() {
    let port = spawn_mock_server();
    // DNS rebinding simulation / redirect to private IP test
    // initial request OK, but redirect points to localtest.me
    // Since localtest.me resolves to 127.0.0.1, the redirect policy should block it.
    // We can just add /redirect-to-localtest to mock server!
    let output = Command::new(get_bin())
        .arg(format!("http://8.8.8.8/redirect-to-localtest"))
        .env("HTTP_PROXY", format!("http://127.0.0.1:{}", port))
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("STDOUT: {stdout}");
    assert!(stdout.contains("SSRF Error") && stdout.contains("SSRF guard blocked redirect"));
}

#[test]
fn test_hostname_resolving_to_private_ip_is_blocked() {
    let output = Command::new(get_bin())
        .arg("http://192.168.1.1")
        .output()
        .expect("Failed to run curl-tool");

    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("STDOUT: {stdout}");
    assert!(!output.status.success());
    assert!(stdout.contains("SSRF guard blocked"));
}

#[test]
fn test_malformed_url() {
    let output = Command::new(get_bin()).arg("not_a_url").output().unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("URL Error"));
}

#[test]
fn test_no_guard_bypasses_initial_check() {
    let port = spawn_mock_server();
    let output = Command::new(get_bin())
        .arg(format!("http://127.0.0.1:{port}/"))
        .arg("--no-guard")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("SSRF Error"),
        "Should not contain SSRF Error with --no-guard"
    );
    assert!(
        stdout.contains("Status: 200"),
        "Should fetch successfully with --no-guard"
    );
}

#[test]
fn test_no_guard_follows_redirect_to_private() {
    let port = spawn_mock_server();
    // Use --no-guard on our local server that redirects to another local endpoint on the same mock server.
    // It should follow the redirect successfully and print 200 OK.
    let output = Command::new(get_bin())
        .arg(format!("http://127.0.0.1:{port}/redirect-to-mockserver"))
        .arg("--no-guard")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("STDOUT: {stdout}");
    assert!(
        !stdout.contains("SSRF Error"),
        "Should not block redirect with SSRF guard"
    );
    assert!(
        stdout.contains("Status: 200") && stdout.contains("OK"),
        "Should successfully fetch the redirect target"
    );
}

#[test]
fn test_curl_timeout_enforced() {
    let port = spawn_mock_server();
    let output = Command::new(get_bin())
        .arg(format!("http://127.0.0.1:{port}/timeout"))
        .arg("--timeout")
        .arg("100") // 100ms timeout
        .arg("--no-guard")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!output.status.success());
    assert!(
        stdout.to_lowercase().contains("timeout")
            || stdout.to_lowercase().contains("timed out")
            || stdout.to_lowercase().contains("deadline has elapsed"),
        "Expected timeout message, got: {stdout}"
    );
}

#[test]
fn test_curl_timeout_zero_ignored() {
    let port = spawn_mock_server();
    // Server sleeps for 1.5s, curl with timeout 0 should wait and succeed
    let output = Command::new(get_bin())
        .arg(format!("http://127.0.0.1:{port}/timeout"))
        .arg("--timeout")
        .arg("0")
        .arg("--no-guard")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "Expected success when timeout=0, got exit code: {:?}, stdout: {stdout}",
        output.status.code()
    );
    assert!(stdout.contains("Status: 200"));
}

#[test]
fn test_curl_exit_code_contract_invalid_usage() {
    let output = std::process::Command::new(get_bin())
        .arg("--this-flag-does-not-exist")
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(2),
        "Expected exit code 2 (Invalid CLI Usage)"
    );
}
