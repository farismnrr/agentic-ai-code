use ai_tools::core::network::{safe_http_client, validate_public_http_url};
use ai_tools::infrastructure::observability::classify_reqwest_error;
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

async fn run_curl(
    url_str: &str,
    method_str: &str,
    headers_raw: &[String],
    body_data: Option<&str>,
    timeout_ms: u64,
    no_guard: bool,
) -> String {
    let parsed_url = if no_guard {
        match Url::parse(url_str) {
            Ok(url) => url,
            Err(_) => return "Error: invalid URL".to_string(),
        }
    } else {
        match validate_public_http_url(url_str) {
            Ok(url) => url,
            Err(_) => {
                return "Error: SSRF Error: request target is not a public HTTP(S) URL".to_string()
            }
        }
    };

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
        match safe_http_client(timeout, 10) {
            Ok(client) => client,
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
