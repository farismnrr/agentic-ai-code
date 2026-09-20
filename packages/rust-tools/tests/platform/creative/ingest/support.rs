use ai_tools::application::creative::dispatch_tool;
use ai_tools::core::config::ServerConfig;
use serde_json::Value;
use std::time::Duration;

pub(super) fn call_for_owner(
    config: &ServerConfig,
    name: &str,
    arguments: Value,
    owner: &str,
) -> Value {
    let result = tokio::runtime::Runtime::new()
        .expect("creative owner test runtime")
        .block_on(dispatch_tool(name, &arguments, config, owner))
        .expect("creative owner dispatch")
        .expect("creative owner tool result");
    assert!(!result.is_error, "creative owner tool returned an error");
    serde_json::from_str(&result.content[0].text).expect("creative owner JSON result")
}

pub(super) fn http_client(label: &str) -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap_or_else(|_| panic!("{label}"))
}

pub(super) fn abort_server(runtime: &tokio::runtime::Runtime, server: tokio::task::JoinHandle<()>) {
    server.abort();
    runtime.block_on(async {
        let _ = server.await;
    });
}
