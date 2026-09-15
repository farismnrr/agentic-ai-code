use crate::core::error::McpError;
use reqwest::Url;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;
use url::Host;

pub fn is_safe_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(ipv4) => {
            !ipv4.is_private()
                && !ipv4.is_loopback()
                && !ipv4.is_link_local()
                && !ipv4.is_multicast()
                && !ipv4.is_broadcast()
                && !ipv4.is_documentation()
                && !ipv4.is_unspecified()
        }
        IpAddr::V6(ipv6) => {
            if let Some(ipv4) = ipv6.to_ipv4_mapped() {
                return is_safe_ip(&IpAddr::V4(ipv4));
            }
            !ipv6.is_loopback()
                && !ipv6.is_multicast()
                && !ipv6.is_unspecified()
                && (ipv6.segments()[0] & 0xfe00) != 0xfc00
                && (ipv6.segments()[0] & 0xffc0) != 0xfe80
        }
    }
}

pub fn is_safe_http_scheme(scheme: &str) -> bool {
    matches!(scheme.to_ascii_lowercase().as_str(), "http" | "https")
}

pub fn validate_public_http_url(value: &str) -> Result<Url, McpError> {
    let url =
        Url::parse(value).map_err(|_| McpError::InvalidRequest("invalid HTTP(S) URL".into()))?;
    if !is_safe_http_scheme(url.scheme()) {
        return Err(McpError::InvalidRequest(
            "only HTTP(S) URLs are allowed".into(),
        ));
    }
    let host = url
        .host()
        .ok_or_else(|| McpError::InvalidRequest("URL host is required".into()))?;
    let unsafe_literal = match host {
        Host::Ipv4(ip) => !is_safe_ip(&IpAddr::V4(ip)),
        Host::Ipv6(ip) => !is_safe_ip(&IpAddr::V6(ip)),
        Host::Domain(_) => false,
    };
    if unsafe_literal {
        return Err(McpError::InvalidRequest(
            "private/local URL targets are not allowed".into(),
        ));
    }
    Ok(url)
}

#[derive(Debug)]
struct SafeResolver;

impl reqwest::dns::Resolve for SafeResolver {
    fn resolve(&self, name: reqwest::dns::Name) -> reqwest::dns::Resolving {
        Box::pin(async move {
            let host = name.as_str();
            let addrs = tokio::net::lookup_host(format!("{host}:80")).await?;
            let mut safe = Vec::new();
            for addr in addrs {
                if !is_safe_ip(&addr.ip()) {
                    return Err(Box::new(std::io::Error::new(
                        std::io::ErrorKind::PermissionDenied,
                        "SSRF guard blocked private/local DNS resolution",
                    ))
                        as Box<dyn std::error::Error + Send + Sync>);
                }
                safe.push(addr);
            }
            if safe.is_empty() {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "DNS resolution returned no public addresses",
                ))
                    as Box<dyn std::error::Error + Send + Sync>);
            }
            let iter: reqwest::dns::Addrs = Box::new(safe.into_iter());
            Ok(iter)
        })
    }
}

pub fn safe_http_client(
    timeout: Option<Duration>,
    max_redirects: usize,
) -> Result<reqwest::Client, McpError> {
    let policy = reqwest::redirect::Policy::custom(move |attempt| {
        if attempt.previous().len() >= max_redirects {
            return attempt.error("too many redirects");
        }
        let url = attempt.url();
        if !is_safe_http_scheme(url.scheme()) {
            return attempt.error("redirect scheme not allowed");
        }
        if let Some(host) = url.host() {
            let unsafe_literal = match host {
                Host::Ipv4(ip) => !is_safe_ip(&IpAddr::V4(ip)),
                Host::Ipv6(ip) => !is_safe_ip(&IpAddr::V6(ip)),
                Host::Domain(_) => false,
            };
            if unsafe_literal {
                return attempt.error("redirect to private/local IP blocked");
            }
        }
        attempt.follow()
    });
    let mut builder = reqwest::Client::builder()
        .redirect(policy)
        .dns_resolver(Arc::new(SafeResolver));
    if let Some(timeout) = timeout {
        builder = builder.timeout(timeout);
    }
    builder
        .build()
        .map_err(|_| McpError::Internal("safe HTTP client could not be constructed".into()))
}
