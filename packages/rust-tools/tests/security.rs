use ai_tools::core::config::ServerConfig;
use ai_tools::infrastructure::security::enforce_local_access_policy;
use axum::http::{header::HOST, header::ORIGIN, HeaderMap, HeaderValue};

fn config(origin: Option<&str>) -> ServerConfig {
    ServerConfig {
        origin: origin.map(str::to_string),
        port: 47821,
        ..ServerConfig::default()
    }
}

fn config_with_hosts(origin: Option<&str>, allowed_hosts: &[&str]) -> ServerConfig {
    ServerConfig {
        allowed_hosts: allowed_hosts
            .iter()
            .map(|host| (*host).to_string())
            .collect(),
        ..config(origin)
    }
}

fn headers(origin: Option<&str>, host: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    if let Some(origin) = origin {
        headers.insert(ORIGIN, HeaderValue::from_str(origin).unwrap());
    }
    headers.insert(HOST, HeaderValue::from_str(host).unwrap());
    headers
}

#[test]
fn allows_configured_browser_origin() {
    assert!(enforce_local_access_policy(
        &headers(Some("http://localhost:3333"), "127.0.0.1:47821"),
        &config(Some("http://localhost:3333")),
    )
    .is_ok());
}

#[test]
fn rejects_mismatched_browser_origin() {
    assert!(enforce_local_access_policy(
        &headers(Some("https://external-client.example"), "127.0.0.1:47821"),
        &config(Some("http://localhost:3333")),
    )
    .is_err());
}

#[test]
fn rejects_malformed_origin() {
    assert!(enforce_local_access_policy(
        &headers(Some("not-an-origin"), "127.0.0.1:47821"),
        &config(Some("http://localhost:3333")),
    )
    .is_err());
}

#[test]
fn allows_missing_origin_for_non_browser_client() {
    assert!(enforce_local_access_policy(
        &headers(None, "localhost:47821"),
        &config(Some("http://localhost:3333")),
    )
    .is_ok());
}

#[test]
fn allows_implicit_local_hosts_on_configured_port() {
    let config = config(Some("http://localhost:3333"));
    assert!(enforce_local_access_policy(&headers(None, "localhost:47821"), &config).is_ok());
    assert!(enforce_local_access_policy(&headers(None, "127.0.0.1:47821"), &config).is_ok());
}

#[test]
fn allows_configured_external_hostname_without_implicit_ports() {
    let config = config_with_hosts(Some("http://localhost:3333"), &["mcp.farismunir.my.id"]);
    assert!(enforce_local_access_policy(&headers(None, "mcp.farismunir.my.id"), &config).is_ok());
    assert!(enforce_local_access_policy(&headers(None, "MCP.FARISMUNIR.MY.ID"), &config).is_ok());
}

#[test]
fn rejects_unconfigured_external_hostname_and_wrong_ports() {
    let config = config_with_hosts(Some("http://localhost:3333"), &["mcp.farismunir.my.id"]);
    assert!(enforce_local_access_policy(&headers(None, "evil.example"), &config).is_err());
    assert!(
        enforce_local_access_policy(&headers(None, "mcp.farismunir.my.id:47821"), &config).is_err()
    );
    let port_config = config_with_hosts(
        Some("http://localhost:3333"),
        &["mcp.farismunir.my.id:47821"],
    );
    assert!(enforce_local_access_policy(
        &headers(None, "mcp.farismunir.my.id:47821"),
        &port_config,
    )
    .is_ok());
    assert!(enforce_local_access_policy(
        &headers(None, "mcp.farismunir.my.id:47822"),
        &port_config,
    )
    .is_err());
}

#[test]
fn rejects_malformed_hosts() {
    let config = config(Some("http://localhost:3333"));
    for host in ["mcp.example/path", "mcp.example:bad", "*.example"] {
        assert!(enforce_local_access_policy(&headers(None, host), &config).is_err());
    }
}

#[test]
fn rejects_missing_configured_origin_even_without_request_origin() {
    assert!(enforce_local_access_policy(&headers(None, "localhost:47821"), &config(None)).is_err());
}

#[test]
fn rejects_invalid_host() {
    assert!(enforce_local_access_policy(
        &headers(None, "evil.example:47821"),
        &config(Some("http://localhost:3333")),
    )
    .is_err());
}

#[test]
fn rejects_duplicate_origin_and_host() {
    let mut duplicate_origin = headers(Some("http://localhost:3333"), "localhost:47821");
    duplicate_origin.append(ORIGIN, HeaderValue::from_static("http://localhost:3333"));
    assert!(
        enforce_local_access_policy(&duplicate_origin, &config(Some("http://localhost:3333")),)
            .is_err()
    );
    let mut duplicate_host = headers(None, "localhost:47821");
    duplicate_host.append(HOST, HeaderValue::from_static("127.0.0.1:47821"));
    assert!(
        enforce_local_access_policy(&duplicate_host, &config(Some("http://localhost:3333")),)
            .is_err()
    );
}
