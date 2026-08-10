use clap::Parser;
use reqwest::header::{HeaderName, HeaderValue};
use reqwest::Method;
use std::str::FromStr;
use url::Url;

#[derive(Parser, Debug)]
#[command(name = "curl-tool")]
#[command(about = "Fetch a URL and return its response", long_about = None)]
struct Args {
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

    /// Bypass SSRF guard protection
    #[arg(long = "no-guard")]
    no_guard: bool,
}

async fn run_curl(
    url_str: &str,
    method_str: &str,
    headers_raw: &[String],
    body_data: Option<&str>,
    no_guard: bool,
) -> String {
    let parsed_url = match Url::parse(url_str) {
        Ok(u) => u,
        Err(e) => return format!("Error: {}", e),
    };

    if !no_guard {
        if let Some(host) = parsed_url.host_str() {
            // Check if it's already an IP string
            if let Ok(ip) = host.parse::<std::net::IpAddr>() {
                if !is_safe_ip(&ip) {
                    return "Error: SSRF guard blocked request to private/local IP. Use --no-guard to bypass.".to_string();
                }
            } else {
                // Resolve hostname to IP using trust-dns-resolver
                use trust_dns_resolver::config::*;
                use trust_dns_resolver::TokioAsyncResolver;
                let resolver =
                    TokioAsyncResolver::tokio(ResolverConfig::default(), ResolverOpts::default());

                let response = match resolver.lookup_ip(host).await {
                    Ok(r) => r,
                    Err(e) => return format!("Error: DNS lookup failed: {}", e),
                };

                for ip in response.iter() {
                    if !is_safe_ip(&ip) {
                        return format!("Error: SSRF guard blocked request because {} resolves to private/local IP {}. Use --no-guard to bypass.", host, ip);
                    }
                }
            }
        }
    }

    fn is_safe_ip(ip: &std::net::IpAddr) -> bool {
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
                !ipv6.is_loopback()
                && !ipv6.is_multicast()
                && !ipv6.is_unspecified()
                && (ipv6.segments()[0] & 0xfe00) != 0xfc00 // Unique Local Address
                && (ipv6.segments()[0] & 0xffc0) != 0xfe80 // Link-Local
            }
        }
    }

    let method = match Method::from_str(&method_str.to_uppercase()) {
        Ok(m) => m,
        Err(e) => return format!("Error: {}", e),
    };

    let client = if !no_guard {
        match reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
        {
            Ok(c) => c,
            Err(_) => reqwest::Client::new(),
        }
    } else {
        reqwest::Client::new()
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
        Err(e) => return format!("Error: {}", e),
    };

    let status = res.status().as_u16();
    let text = match res.text().await {
        Ok(t) => t,
        Err(e) => return format!("Error: {}", e),
    };

    let truncated_text = if text.len() > 10000 {
        &text[..10000]
    } else {
        &text
    };

    format!("Status: {}\nBody: {}", status, truncated_text)
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let url = match args.url {
        Some(u) if !u.trim().is_empty() => u,
        _ => {
            eprintln!("Usage: curl-tool <url> [--request <method>] [--header <header>...] [--data <body>] [--no-guard]");
            std::process::exit(1);
        }
    };

    let output = run_curl(
        &url,
        &args.method,
        &args.headers,
        args.data.as_deref(),
        args.no_guard,
    )
    .await;

    println!("{}", output);
}
