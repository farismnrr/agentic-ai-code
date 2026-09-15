//! Creative production contracts and contained execution state.
//!
//! The MCP layer remains agent/model/provider neutral. Upper layers author
//! creative intent, prompts, graphs, and execution-binding choices; this module
//! validates, stores, and executes only the reviewed semantic contract.

mod compiler;
mod contracts;
mod graph;
mod handlers;
pub(crate) mod ingest;
mod jobs;
mod media;
mod registry;
mod store;
mod support;

pub use contracts::*;
pub use graph::*;
pub use registry::*;

use crate::core::config::ServerConfig;
use crate::core::error::McpError;
use crate::interfaces::mcp::ToolCallResult;
use serde_json::{json, Value};
use support::{complete, error_result, required_str};

pub async fn dispatch_tool(
    tool_name: &str,
    arguments: &Value,
    config: &ServerConfig,
    owner: &str,
) -> Result<Option<ToolCallResult>, McpError> {
    if tool_name == "creative_status" {
        return Ok(Some(complete(status(config))?));
    }
    if !tool_name.starts_with("creative_") {
        return Ok(None);
    }
    if !config.enable_creative {
        return Ok(Some(error_result(
            "creative_capability_disabled",
            "Creative production tools are disabled by operator configuration",
            json!({
                "activation": "start relay-agent with --enable-creative or RELAY_ENABLE_CREATIVE=true",
                "restart_required_for_running_process": true
            }),
        )?));
    }
    let result = match tool_name {
        "creative_catalog" => catalog(arguments, config)?,
        "creative_project" => handlers::project(arguments, config)?,
        "creative_element" => handlers::element(arguments, config)?,
        "creative_asset" => handlers::asset(arguments, config, owner).await?,
        "creative_graph" => handlers::graph_tool(arguments, config, owner)?,
        "creative_job" => handlers::job(arguments, config, owner)?,
        _ => return Ok(None),
    };
    Ok(Some(result))
}

fn status(config: &ServerConfig) -> Value {
    json!({
        "enabled": config.enable_creative,
        "schema_version": CREATIVE_SCHEMA_VERSION,
        "agent_agnostic": true,
        "model_provider_agnostic": true,
        "state_namespace": ".masihawam/creative",
        "execution_bindings_registered": registry::execution_bindings(config).map(|items| items.len()).unwrap_or(0),
        "activation": if config.enable_creative {
            Value::Null
        } else {
            Value::String("start relay-agent with --enable-creative or RELAY_ENABLE_CREATIVE=true".into())
        },
        "running_process_restart_required_for_activation_change": true
    })
}

fn catalog(arguments: &Value, config: &ServerConfig) -> Result<ToolCallResult, McpError> {
    let catalog = required_str(arguments, "catalog")?;
    let id = arguments.get("id").and_then(Value::as_str);
    let value = match (catalog, id) {
        ("capabilities", None) => {
            json!({"catalog":"capabilities","items":registry::capabilities()})
        }
        ("capabilities", Some(id)) => {
            json!({"catalog":"capabilities","item":registry::capability(id)})
        }
        ("execution_bindings", None) => {
            json!({"catalog":"execution_bindings","items":registry::execution_bindings(config)?})
        }
        ("execution_bindings", Some(id)) => {
            json!({"catalog":"execution_bindings","item":registry::binding(config, id)?})
        }
        ("workflows", None) => json!({"catalog":"workflows","items":registry::workflows()}),
        ("workflows", Some(id)) => json!({"catalog":"workflows","item":registry::workflow(id)}),
        _ => {
            return Err(McpError::InvalidRequest(
                "creative catalog must be capabilities, execution_bindings, or workflows".into(),
            ))
        }
    };
    complete(value)
}
