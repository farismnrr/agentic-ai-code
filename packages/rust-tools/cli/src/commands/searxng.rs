use relay_infrastructure::observability::classify_reqwest_error;
use reqwest::Url;
use serde::Deserialize;

#[derive(clap::Args, Debug)]
pub struct Args {
    /// The search query
    query: Option<String>,

    /// SearxNG base URL
    #[arg(long, default_value = "http://127.0.0.1:8888")]
    base_url: String,
}

#[derive(Deserialize, Debug)]
struct SearxngResultItem {
    title: Option<String>,
    url: Option<String>,
    content: Option<String>,
    snippet: Option<String>,
}

#[derive(Deserialize, Debug)]
struct SearxngResponse {
    results: Option<Vec<SearxngResultItem>>,
}

async fn run_search(query: &str, base_url: &str) -> String {
    let mut url = match Url::parse(base_url) {
        Ok(u) => match u.join("/search") {
            Ok(joined) => joined,
            Err(e) => return format!("Error: {e}"),
        },
        Err(e) => return format!("Error: {e}"),
    };

    url.query_pairs_mut()
        .append_pair("q", query)
        .append_pair("format", "json");

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
    {
        Ok(c) => c,
        Err(e) => return format!("Error: Failed to build HTTP client: {e}"),
    };
    let res = match client.get(url).send().await {
        Ok(r) => r,
        Err(e) => return format!("Error: {}", classify_reqwest_error(&e)),
    };

    if !res.status().is_success() {
        return format!("Search failed with status: {}", res.status().as_u16());
    }

    let data: SearxngResponse = match res.json().await {
        Ok(d) => d,
        Err(e) => return format!("Error: {}", classify_reqwest_error(&e)),
    };

    let items = data.results.unwrap_or_default();
    if items.is_empty() {
        return "No results found.".to_string();
    }

    let formatted: Vec<String> = items
        .into_iter()
        .take(10)
        .map(|r| {
            let title = r.title.unwrap_or_default();
            let url_str = r.url.unwrap_or_default();
            let snippet = r.content.or(r.snippet).unwrap_or_default();
            format!("Title: {title}\nURL: {url_str}\nSnippet: {snippet}")
        })
        .collect();

    if formatted.is_empty() {
        "No results found.".to_string()
    } else {
        formatted.join("\n\n")
    }
}

pub async fn run(args: Args) {
    let query = match args.query {
        Some(q) if !q.trim().is_empty() => q,
        _ => {
            eprintln!("Usage: ai-tools searxng <query> [--base-url <url>]");
            std::process::exit(1);
        }
    };

    let output = run_search(&query, &args.base_url).await;
    println!("{output}");
}
