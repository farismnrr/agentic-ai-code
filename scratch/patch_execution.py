import sys

content = open("packages/rust-tools/src/relay_agent/execution.rs").read()

content = content.replace("pub async fn dispatch_tool_call(\n    tool: &Tool,\n    arguments: &Value,\n) -> Result<ToolCallResult, McpError> {", "pub async fn dispatch_tool_call(\n    tool: &Tool,\n    arguments: &Value,\n    config: &crate::relay_agent::config::ServerConfig,\n) -> Result<ToolCallResult, McpError> {")

# In terminal_exec handling, add boundary check for cwd
cwd_block = """            if let Some(cwd) = arguments.get("cwd").and_then(|v| v.as_str()) {
                args.push("--cwd".to_string());
                args.push(cwd.to_string());
            }"""

replacement = """            let execution_root = config
                .resolved_execution_root()
                .map_err(|e| McpError::Internal(e.to_string()))?;

            if let Some(cwd) = arguments.get("cwd").and_then(|v| v.as_str()) {
                let requested_cwd = std::path::Path::new(cwd);
                let canonical_cwd = std::fs::canonicalize(requested_cwd)
                    .unwrap_or_else(|_| execution_root.join(cwd)); // Fallback if doesn't exist yet, wait, let's just canonicalize or fail
                
                // Let's do it better
            }
"""

with open("packages/rust-tools/src/relay_agent/execution.rs", "w") as f:
    f.write(content)
