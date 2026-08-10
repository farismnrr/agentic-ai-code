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
                    if request.contains("GET /redirect-to-private") {
                        let response =
                            "HTTP/1.1 302 Found\r\nLocation: http://192.168.1.1/\r\n\r\n";
                        let _ = stream.write(response.as_bytes());
                    } else if request.contains("GET /redirect-to-loopback") {
                        let response = "HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1/\r\n\r\n";
                        let _ = stream.write(response.as_bytes());
                    } else if request.contains("GET /redirect-to-link-local") {
                        let response =
                            "HTTP/1.1 302 Found\r\nLocation: http://169.254.169.254/\r\n\r\n";
                        let _ = stream.write(response.as_bytes());
                    } else if request.contains("GET /redirect-to-localtest") {
                        let response =
                            "HTTP/1.1 302 Found\r\nLocation: http://localtest.me/\r\n\r\n";
                        let _ = stream.write(response.as_bytes());
                    } else {
                        let response = "HTTP/1.1 200 OK\r\n\r\nOK";
                        let _ = stream.write(response.as_bytes());
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
        .arg(format!("http://127.0.0.1:{port}/redirect-to-private"))
        .env("CURL_TEST_ALLOW_INITIAL", "1")
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
        .arg(format!("http://127.0.0.1:{port}/redirect-to-loopback"))
        .env("CURL_TEST_ALLOW_INITIAL", "1")
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
        .arg(format!("http://127.0.0.1:{port}/redirect-to-link-local"))
        .env("CURL_TEST_ALLOW_INITIAL", "1")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("STDOUT: {stdout}");
    assert!(stdout.contains("SSRF Error") && stdout.contains("SSRF guard blocked redirect"));
}

#[test]
fn test_redirect_revalidation_dns() {
    let port = spawn_mock_server();
    // DNS rebinding simulation: initial request OK, but redirect points to localtest.me
    // Since localtest.me resolves to 127.0.0.1, the redirect policy should block it.
    // wait, our mock server doesn't have a /redirect-to-localtest route.
    // Let's just use http://localtest.me/ directly since we bypass initial. No, the initial is bypassed, so localtest.me isn't blocked.
    // But then it won't redirect. We need a redirect.
    // We can just add /redirect-to-localtest to mock server!
    let output = Command::new(get_bin())
        .arg(format!("http://127.0.0.1:{port}/redirect-to-localtest"))
        .env("CURL_TEST_ALLOW_INITIAL", "1")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("STDOUT: {stdout}");
    assert!(stdout.contains("SSRF Error") && stdout.contains("SSRF guard blocked redirect"));
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
    // Use --no-guard on our local server that redirects to 192.168.1.1
    // It should try to fetch 192.168.1.1 and fail with a network error, NOT an SSRF block
    let output = Command::new(get_bin())
        .arg(format!("http://127.0.0.1:{port}/redirect-to-private"))
        .arg("--no-guard")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("SSRF Error"),
        "Should not block redirect with SSRF guard"
    );
    // Depending on network, it might timeout or connection refused, but not SSRF blocked.
}
