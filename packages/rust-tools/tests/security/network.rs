use ai_tools::core::network::{is_safe_ip, safe_http_client, validate_public_http_url};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::time::Duration;

#[test]
fn public_http_policy_rejects_private_local_and_non_http_targets() {
    for value in [
        "http://127.0.0.1/x",
        "http://10.1.2.3/x",
        "http://169.254.1.1/x",
        "http://[::1]/x",
        "file:///tmp/x",
    ] {
        assert!(validate_public_http_url(value).is_err(), "accepted {value}");
    }
    assert!(validate_public_http_url("https://example.com/x").is_ok());
}

#[test]
fn public_http_policy_rejects_reserved_ip_classes() {
    for ip in [
        IpAddr::V4(Ipv4Addr::LOCALHOST),
        IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)),
        IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
        IpAddr::V6(Ipv6Addr::LOCALHOST),
        "fc00::1".parse().unwrap(),
        "fe80::1".parse().unwrap(),
    ] {
        assert!(!is_safe_ip(&ip), "accepted reserved address {ip}");
    }
    assert!(is_safe_ip(&"8.8.8.8".parse().unwrap()));
}

#[tokio::test]
#[ignore = "requires public DNS and outbound HTTPS"]
async fn safe_http_client_uses_the_https_scheme_default_port() {
    let client = safe_http_client(Some(Duration::from_secs(5)), 3).expect("safe HTTP client");
    let response = tokio::time::timeout(
        Duration::from_secs(6),
        client.get("https://example.com/").send(),
    )
    .await
    .expect("HTTPS request exceeded its bound")
    .expect("HTTPS request failed");

    assert_eq!(response.status(), reqwest::StatusCode::OK);
}
