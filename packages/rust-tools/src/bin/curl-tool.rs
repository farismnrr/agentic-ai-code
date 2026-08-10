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
    if !no_guard {
        eprintln!(
            "WARN: SSRF guard is enabled but no external validation is provided in CLI. Pass --no-guard if you want to bypass SSRF protection."
        );
        return "Error: SSRF guard blocked request. Use --no-guard to bypass.".to_string();
    }

    let parsed_url = match Url::parse(url_str) {
        Ok(u) => u,
        Err(e) => return format!("Error: {}", e),
    };

    let method = match Method::from_str(&method_str.to_uppercase()) {
        Ok(m) => m,
        Err(e) => return format!("Error: {}", e),
    };

    let client = reqwest::Client::new();
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
