use relay_infrastructure::observability::classify_reqwest_error;
use reqwest::header::{HeaderName, HeaderValue};
use reqwest::Method;
use std::str::FromStr;
use url::Url;

#[derive(clap::Args, Debug)]
pub struct Args {
    /// Target URL
    url: Option<String>,

    /// HTTP request method (e.g. GET, POST)
    #[arg(short = 'X', long = "request", default_value = "GET")]
    method: String,

    /// HTTP headers (can be repeated)
    #[arg(short = 'H', long = "header")]
    headers: Vec<String>,

    /// HTTP request body
    #[arg(short = 'd', long = "data")]
    data: Option<String>,

    /// Request timeout in milliseconds (0 = no timeout)
    #[arg(long = "timeout", default_value = "30000")]
    timeout_ms: u64,

    /// Bypass SSRF guard protection
    #[arg(long = "no-guard")]
    no_guard: bool,
}

pub fn is_safe_ip(ip: &std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(ipv4) => {
            !ipv4.is_private()
                && !ipv4.is_loopback()
                && !ipv4.is_link_local()
                && !ipv4.is_multicast()
                && !ipv4.is_broadcast()
                && !ipv4.is_documentation()
                && !ipv4.is_unspecified()
        }
        std::net::IpAddr::V6(ipv6) => {
            if let Some(ipv4) = ipv6.to_ipv4_mapped() {
                return is_safe_ip(&std::net::IpAddr::V4(ipv4));
            }

            !ipv6.is_loopback()
            && !ipv6.is_multicast()
            && !ipv6.is_unspecified()
            && (ipv6.segments()[0] & 0xfe00) != 0xfc00 // Unique Local Address
            && (ipv6.segments()[0] & 0xffc0) != 0xfe80 // Link-Local
        }
    }
}

pub fn is_safe_scheme(scheme: &str) -> bool {
    let s = scheme.to_lowercase();
    s == "http" || s == "https"
}

struct SafeResolver;

impl reqwest::dns::Resolve for SafeResolver {
    fn resolve(&self, name: reqwest::dns::Name) -> reqwest::dns::Resolving {
        Box::pin(async move {
            let host = name.as_str();
            let addrs = tokio::net::lookup_host(format!("{}:80", host)).await?;
            let mut safe_addrs = vec![];
            for addr in addrs {
                let ip = addr.ip();
                if !is_safe_ip(&ip) {
                    return Err(Box::new(std::io::Error::new(
                        std::io::ErrorKind::PermissionDenied,
                        format!(
                            "SSRF guard blocked request because {} resolves to private/local IP {}",
                            host, ip
                        ),
                    ))
                        as Box<dyn std::error::Error + Send + Sync>);
                }
                safe_addrs.push(addr);
            }
            if safe_addrs.is_empty() {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("No IP addresses found for {}", host),
                ))
                    as Box<dyn std::error::Error + Send + Sync>);
            }
            let addrs_iter: reqwest::dns::Addrs = Box::new(safe_addrs.into_iter());
            Ok(addrs_iter)
        })
    }
}

async fn run_curl(
    url_str: &str,
    method_str: &str,
    headers_raw: &[String],
    body_data: Option<&str>,
    timeout_ms: u64,
    no_guard: bool,
) -> String {
    let parsed_url = match Url::parse(url_str) {
        Ok(u) => u,
        Err(_) => return "Error: invalid URL".to_string(),
    };

    if !no_guard {
        if !is_safe_scheme(parsed_url.scheme()) {
            return format!(
                "Error: SSRF Error: Scheme '{}' is not allowed",
                parsed_url.scheme()
            );
        }

        if let Some(host) = parsed_url.host_str() {
            if let Ok(ip) = host.parse::<std::net::IpAddr>() {
                if !is_safe_ip(&ip) {
                    return "Error: SSRF Error: SSRF guard blocked request to private/local IP. Use --no-guard to bypass.".to_string();
                }
            }
        }
    }

    let method = match Method::from_str(&method_str.to_uppercase()) {
        Ok(m) => m,
        Err(_) => return "Error: invalid HTTP method".to_string(),
    };

    let timeout = if timeout_ms == 0 {
        None
    } else {
        Some(std::time::Duration::from_millis(timeout_ms))
    };

    let client = if !no_guard {
        let policy = reqwest::redirect::Policy::custom(move |attempt| {
            if attempt.previous().len() > 10 {
                return attempt.error("too many redirects");
            }

            let url = attempt.url();
            if !is_safe_scheme(url.scheme()) {
                return attempt.error("redirect scheme not allowed");
            }

            if let Some(host) = url.host_str() {
                if let Ok(ip) = host.parse::<std::net::IpAddr>() {
                    if !is_safe_ip(&ip) {
                        return attempt
                            .error("redirect to private IP literal blocked by SSRF guard");
                    }
                }
            }
            attempt.follow()
        });

        let mut builder = reqwest::Client::builder()
            .redirect(policy)
            .dns_resolver(std::sync::Arc::new(SafeResolver));

        if let Some(t) = timeout {
            builder = builder.timeout(t);
        }
        match builder.build() {
            Ok(c) => c,
            Err(_) => return "Error: failed to build HTTP client".to_string(),
        }
    } else {
        let mut builder =
            reqwest::Client::builder().redirect(reqwest::redirect::Policy::limited(10));
        if let Some(t) = timeout {
            builder = builder.timeout(t);
        }
        match builder.build() {
            Ok(c) => c,
            Err(_) => return "Error: failed to build HTTP client".to_string(),
        }
    };

    let mut req_builder = client.request(method, parsed_url.clone());

    for h in headers_raw {
        let mut parts = h.splitn(2, ':');
        if let (Some(key), Some(val)) = (parts.next(), parts.next()) {
            let key_trimmed = key.trim();
            let val_trimmed = val.trim();
            if let (Ok(name), Ok(value)) = (
                HeaderName::from_str(key_trimmed),
                HeaderValue::from_str(val_trimmed),
            ) {
                req_builder = req_builder.header(name, value);
            }
        }
    }

    if let Some(body) = body_data {
        req_builder = req_builder.body(body.to_string());
    }

    let res = match req_builder.send().await {
        Ok(r) => r,
        Err(e) => {
            let label = classify_reqwest_error(&e);
            if e.is_redirect() || e.is_builder() || e.is_request() {
                return format!("Error: SSRF Error: SSRF guard blocked request/redirect: {label}");
            }
            return format!("Error: Fetch Error: {label}");
        }
    };

    let status = res.status().as_u16();
    let text = match res.text().await {
        Ok(t) => t,
        Err(e) => return format!("Error: {}", classify_reqwest_error(&e)),
    };

    let truncated_text = if text.len() > 10000 {
        &text[..10000]
    } else {
        &text
    };

    format!("Status: {status}\nBody: {truncated_text}")
}

pub async fn run(args: Args) {
    let url = match args.url {
        Some(u) if !u.trim().is_empty() => u,
        _ => {
            eprintln!("Usage: ai-tools curl <url> [--request <method>] [--header <header>...] [--data <body>] [--no-guard]");
            std::process::exit(1);
        }
    };

    let output = run_curl(
        &url,
        &args.method,
        &args.headers,
        args.data.as_deref(),
        args.timeout_ms,
        args.no_guard,
    )
    .await;

    let is_error = output.starts_with("Error:");
    println!("{output}");

    if is_error {
        std::process::exit(1);
    }
}
