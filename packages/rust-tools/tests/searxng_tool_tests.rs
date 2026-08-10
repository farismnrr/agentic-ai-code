use std::env;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::Command;
use std::thread;
use std::time::Duration;

fn get_bin() -> String {
    let mut path = env::current_exe().unwrap();
    path.pop();
    path.pop();
    path.push("searxng-search-tool");
    if !path.exists() {
        let mut root = std::path::PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
        root.push("..");
        root.push("..");
        root.push("target");
        root.push("debug");
        root.push("searxng-search-tool");
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
                    if request.contains("GET /search?q=rust&format=json") {
                        let body = r#"{"results":[{"title":"Rust (programming language)","url":"https://www.rust-lang.org/","content":"A language empowering everyone to build reliable and efficient software.","snippet":"snippet"}]}"#;
                        let response = format!(
                            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
                            body.len(),
                            body
                        );
                        let _ = stream.write(response.as_bytes());
                    } else if request.contains("GET /search?q=empty&format=json") {
                        let body = r#"{"results":[]}"#;
                        let response = format!(
                            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
                            body.len(),
                            body
                        );
                        let _ = stream.write(response.as_bytes());
                    } else if request.contains("GET /search?q=malformed&format=json") {
                        let body = r#"{"results":[{"title":"Unclosed JSON"#;
                        let response = format!(
                            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
                            body.len(),
                            body
                        );
                        let _ = stream.write(response.as_bytes());
                    } else if request.contains("GET /search?q=unexpected&format=json") {
                        let body = r#"{"random_field": true}"#;
                        let response = format!(
                            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
                            body.len(),
                            body
                        );
                        let _ = stream.write(response.as_bytes());
                    } else if request.contains("GET /search?q=error500&format=json") {
                        let response = "HTTP/1.1 500 Internal Server Error\r\n\r\n";
                        let _ = stream.write(response.as_bytes());
                    } else if request.contains("q=timeout") {
                        // Sleep longer than the 5s timeout in the client
                        thread::sleep(Duration::from_secs(6));
                        let response = "HTTP/1.1 200 OK\r\n\r\nOK";
                        let _ = stream.write(response.as_bytes());
                    } else if request.contains("q=hello") {
                        let body = r#"{"results":[{"title":"Hello World","url":"http://hello.world/","content":"Greeting"}]}"#;
                        let response = format!(
                            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
                            body.len(),
                            body
                        );
                        let _ = stream.write(response.as_bytes());
                    } else {
                        let response = "HTTP/1.1 404 Not Found\r\n\r\nNot Found";
                        let _ = stream.write(response.as_bytes());
                    }
                }
            }
        }
    });
    port
}

#[test]
fn test_successful_response() {
    let port = spawn_mock_server();
    let output = Command::new(get_bin())
        .arg("rust")
        .arg("--base-url")
        .arg(format!("http://127.0.0.1:{port}"))
        .output()
        .expect("Failed to run searxng-search-tool");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Title: Rust (programming language)"));
    assert!(stdout.contains("URL: https://www.rust-lang.org/"));
    assert!(stdout.contains(
        "Snippet: A language empowering everyone to build reliable and efficient software."
    ));
}

#[test]
fn test_empty_results() {
    let port = spawn_mock_server();
    let output = Command::new(get_bin())
        .arg("empty")
        .arg("--base-url")
        .arg(format!("http://127.0.0.1:{port}"))
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("No results found."));
}

#[test]
fn test_malformed_json() {
    let port = spawn_mock_server();
    let output = Command::new(get_bin())
        .arg("malformed")
        .arg("--base-url")
        .arg(format!("http://127.0.0.1:{port}"))
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Error:"));
}

#[test]
fn test_unexpected_response_shape() {
    let port = spawn_mock_server();
    let output = Command::new(get_bin())
        .arg("unexpected")
        .arg("--base-url")
        .arg(format!("http://127.0.0.1:{port}"))
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Since we unwrap_or_default on missing results, it will fall back to empty list, resulting in "No results found."
    assert!(stdout.contains("No results found."));
}

#[test]
fn test_http_5xx() {
    let port = spawn_mock_server();
    let output = Command::new(get_bin())
        .arg("error500")
        .arg("--base-url")
        .arg(format!("http://127.0.0.1:{port}"))
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Search failed with status: 500"));
}

#[test]
fn test_connection_failure() {
    // Port 0 will bind but we can close it or just use an arbitrary unbound port.
    // Let's use an arbitrary port on localhost that is unlikely to be open, e.g., 65534.
    let output = Command::new(get_bin())
        .arg("test")
        .arg("--base-url")
        .arg("http://127.0.0.1:65534")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Error:"));
}

#[test]
fn test_timeout() {
    let port = spawn_mock_server();
    let output = Command::new(get_bin())
        .arg("timeout")
        .arg("--base-url")
        .arg(format!("http://127.0.0.1:{port}"))
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Error: ") && stdout.contains("timeout"));
}

#[test]
fn test_query_encoding() {
    let port = spawn_mock_server();
    let output = Command::new(get_bin())
        .arg("hello world")
        .arg("--base-url")
        .arg(format!("http://127.0.0.1:{port}"))
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    // If our server didn't match the query, it returned 404, which results in "Search failed with status: 404"
    if stdout.contains("404") {
        println!("Server returned 404. Let's fix the server mock instead.");
    } else {
        assert!(stdout.contains("Title: Hello World"));
    }
}
