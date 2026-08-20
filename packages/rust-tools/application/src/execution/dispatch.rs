use relay_interfaces::mcp::Tool;
use serde_json::Value;

pub(super) fn supports_tasks(tool: &Tool, arguments: &Value) -> bool {
    let catalog_support = tool
        .execution
        .as_ref()
        .and_then(|execution| execution.get("taskSupport"))
        .and_then(Value::as_str)
        .is_some_and(|support| matches!(support, "optional" | "required"));
    if !catalog_support || tool.name != "http_fetch" {
        return catalog_support;
    }
    matches!(
        arguments
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or("GET")
            .to_ascii_uppercase()
            .as_str(),
        "GET" | "HEAD" | "OPTIONS"
    )
}
