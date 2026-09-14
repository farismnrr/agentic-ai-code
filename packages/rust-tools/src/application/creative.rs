//! Creative production contracts and contained execution state.
//!
//! The MCP layer remains agent/model/provider neutral. Upper layers author
//! creative intent, prompts, graphs, and execution-binding choices; this module
//! validates, stores, and executes only the reviewed semantic contract.

mod contracts;
mod graph;
mod registry;
mod store;

pub use contracts::*;
pub use graph::*;
pub use registry::*;

use crate::core::config::ServerConfig;
use crate::core::error::McpError;
use crate::interfaces::mcp::{ToolCallResult, ToolResultContent};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};

pub fn dispatch_tool(
    tool_name: &str,
    arguments: &Value,
    config: &ServerConfig,
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
        "creative_catalog" => catalog(arguments)?,
        "creative_project" => project(arguments, config)?,
        "creative_element" => element(arguments, config)?,
        "creative_asset" => asset(arguments, config)?,
        "creative_graph" => graph_tool(arguments, config)?,
        "creative_job" => job(arguments, config)?,
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
        "execution_bindings_registered": registry::execution_bindings().len(),
        "activation": if config.enable_creative {
            Value::Null
        } else {
            Value::String("start relay-agent with --enable-creative or RELAY_ENABLE_CREATIVE=true".into())
        },
        "running_process_restart_required_for_activation_change": true
    })
}

fn catalog(arguments: &Value) -> Result<ToolCallResult, McpError> {
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
            json!({"catalog":"execution_bindings","items":registry::execution_bindings()})
        }
        ("execution_bindings", Some(id)) => {
            json!({"catalog":"execution_bindings","item":registry::binding(id)})
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

fn project(arguments: &Value, config: &ServerConfig) -> Result<ToolCallResult, McpError> {
    let action = required_str(arguments, "action")?;
    let cwd = arguments.get("cwd").and_then(Value::as_str);
    match action {
        "create" => {
            let tracks: Vec<CreativeTrack> = parse_required(arguments, "tracks")?;
            let target = parse_optional(arguments, "target")?;
            let created = store::create_project(
                cwd,
                config,
                store::NewProject {
                    project_id: optional_string(arguments, "project_id"),
                    title: required_str(arguments, "title")?.to_owned(),
                    intent: required_str(arguments, "intent")?.to_owned(),
                    tracks,
                    target,
                },
            )?;
            complete(json!({"project": created}))
        }
        "get" => {
            let project = store::load_project(cwd, config, required_str(arguments, "project_id")?)?;
            complete(json!({"project": project}))
        }
        "list" => complete(json!({"project_ids": store::list_projects(cwd, config)?})),
        _ => Err(McpError::InvalidRequest(
            "unsupported creative project action".into(),
        )),
    }
}

fn element(arguments: &Value, config: &ServerConfig) -> Result<ToolCallResult, McpError> {
    let action = required_str(arguments, "action")?;
    let cwd = arguments.get("cwd").and_then(Value::as_str);
    let project_id = required_str(arguments, "project_id")?;
    match action {
        "create_revision" => {
            let authority: ReferenceAuthority = parse_required(arguments, "authority")?;
            let kind = parse_optional(arguments, "kind")?;
            let reference_asset_ids = arguments
                .get("reference_asset_ids")
                .cloned()
                .map(serde_json::from_value)
                .transpose()
                .map_err(|_| McpError::InvalidRequest("reference_asset_ids are invalid".into()))?
                .unwrap_or_default();
            let (project, element_id, revision_id) = store::add_element_revision(
                cwd,
                config,
                project_id,
                store::ElementRevisionInput {
                    element_id: optional_string(arguments, "element_id"),
                    kind,
                    name: optional_string(arguments, "name"),
                    authority,
                    reference_asset_ids,
                    spec: arguments.get("spec").cloned().unwrap_or_else(|| json!({})),
                },
            )?;
            complete(json!({
                "element_id": element_id,
                "revision_id": revision_id,
                "project_updated_at_ms": project.updated_at_ms
            }))
        }
        "promote" => {
            let project = store::promote_element_revision(
                cwd,
                config,
                project_id,
                required_str(arguments, "element_id")?,
                required_str(arguments, "revision_id")?,
            )?;
            complete(json!({
                "element_id": required_str(arguments, "element_id")?,
                "selected_revision_id": required_str(arguments, "revision_id")?,
                "project_updated_at_ms": project.updated_at_ms
            }))
        }
        "get" => {
            let project = store::load_project(cwd, config, project_id)?;
            let element_id = required_str(arguments, "element_id")?;
            let element = project
                .element(element_id)
                .ok_or_else(|| McpError::InvalidRequest("unknown creative element".into()))?;
            complete(json!({"element": element}))
        }
        "list" => {
            let project = store::load_project(cwd, config, project_id)?;
            complete(json!({"elements": project.elements}))
        }
        _ => Err(McpError::InvalidRequest(
            "unsupported creative element action".into(),
        )),
    }
}

fn asset(arguments: &Value, config: &ServerConfig) -> Result<ToolCallResult, McpError> {
    let action = required_str(arguments, "action")?;
    let cwd = arguments.get("cwd").and_then(Value::as_str);
    let project_id = required_str(arguments, "project_id")?;
    match action {
        "register" => {
            for field in [
                "source",
                "source_surface",
                "state",
                "job_id",
                "created_after_ms",
                "created_before_ms",
                "limit",
            ] {
                if arguments.get(field).is_some() {
                    return Err(McpError::InvalidRequest(format!(
                        "creative asset register does not accept {field}"
                    )));
                }
            }
            let (project, asset_id) = store::register_asset(
                cwd,
                config,
                project_id,
                store::AssetRegistrationInput {
                    path: required_str(arguments, "path")?.to_owned(),
                    media_type: required_str(arguments, "media_type")?.to_owned(),
                    role: required_str(arguments, "role")?.to_owned(),
                    source: AssetSource::ManualImport,
                    source_surface: AssetSurface::ManualImport,
                    state: AssetState::Candidate,
                    job_id: None,
                    parent_asset_id: optional_string(arguments, "parent_asset_id"),
                    element_id: optional_string(arguments, "element_id"),
                    metadata: parse_optional(arguments, "metadata")?.unwrap_or_default(),
                },
            )?;
            complete(json!({
                "asset_id": asset_id,
                "source": "manual_import",
                "source_surface": "manual_import",
                "state": "candidate",
                "project_updated_at_ms": project.updated_at_ms
            }))
        }
        "promote" => {
            let asset_id = required_str(arguments, "asset_id")?;
            let project = store::promote_asset(cwd, config, project_id, asset_id)?;
            complete(json!({
                "asset_id": asset_id,
                "state": "accepted",
                "project_updated_at_ms": project.updated_at_ms
            }))
        }
        "get" => {
            let project = store::load_project(cwd, config, project_id)?;
            let asset_id = required_str(arguments, "asset_id")?;
            let asset = project
                .asset(asset_id)
                .ok_or_else(|| McpError::InvalidRequest("unknown creative asset".into()))?;
            complete(json!({"asset": asset}))
        }
        "list" => {
            let project = store::load_project(cwd, config, project_id)?;
            complete(json!({"assets": project.assets}))
        }
        "search" => {
            let assets = store::search_assets(
                cwd,
                config,
                project_id,
                store::AssetSearch {
                    media_type: optional_string(arguments, "media_type"),
                    role: optional_string(arguments, "role"),
                    source: parse_optional(arguments, "source")?,
                    source_surface: parse_optional(arguments, "source_surface")?,
                    state: parse_optional(arguments, "state")?,
                    job_id: optional_string(arguments, "job_id"),
                    element_id: optional_string(arguments, "element_id"),
                    parent_asset_id: optional_string(arguments, "parent_asset_id"),
                    created_after_ms: arguments
                        .get("created_after_ms")
                        .and_then(Value::as_u64)
                        .map(u128::from),
                    created_before_ms: arguments
                        .get("created_before_ms")
                        .and_then(Value::as_u64)
                        .map(u128::from),
                    limit: arguments
                        .get("limit")
                        .and_then(Value::as_u64)
                        .and_then(|value| usize::try_from(value).ok())
                        .unwrap_or(50),
                },
            )?;
            complete(json!({"assets": assets}))
        }
        _ => Err(McpError::InvalidRequest(
            "unsupported creative asset action".into(),
        )),
    }
}

fn graph_tool(arguments: &Value, config: &ServerConfig) -> Result<ToolCallResult, McpError> {
    let action = required_str(arguments, "action")?;
    let cwd = arguments.get("cwd").and_then(Value::as_str);
    match action {
        "validate" => {
            let graph: CreativeGraph = parse_required(arguments, "graph")?;
            let project = store::load_project(cwd, config, &graph.project_id)?;
            let validation = graph::validate_graph(&graph, &project)?;
            complete(json!({"validation": validation}))
        }
        "execute" => {
            let graph: CreativeGraph = parse_required(arguments, "graph")?;
            let project = store::load_project(cwd, config, &graph.project_id)?;
            let validation = graph::validate_graph(&graph, &project)?;
            if !validation.valid {
                return error_result(
                    "graph_validation_failed",
                    "Creative graph did not pass validation",
                    json!({"validation": validation}),
                );
            }
            store::store_graph(cwd, config, &graph)?;
            let job = graph::execute_graph(&graph, &project, store::now_ms())?;
            store::store_job(cwd, config, &job)?;
            complete(json!({"job": job}))
        }
        "get" => {
            let project_id = required_str(arguments, "project_id")?;
            let graph_id = required_str(arguments, "graph_id")?;
            complete(json!({
                "graph": store::load_graph(cwd, config, project_id, graph_id)?
            }))
        }
        _ => Err(McpError::InvalidRequest(
            "unsupported creative graph action".into(),
        )),
    }
}

fn job(arguments: &Value, config: &ServerConfig) -> Result<ToolCallResult, McpError> {
    let action = required_str(arguments, "action")?;
    let cwd = arguments.get("cwd").and_then(Value::as_str);
    let project_id = required_str(arguments, "project_id")?;
    match action {
        "get" => complete(json!({
            "job": store::load_job(cwd, config, project_id, required_str(arguments, "job_id")?)?
        })),
        "list" => complete(json!({"jobs": store::list_jobs(cwd, config, project_id)?})),
        "cancel" => complete(json!({
            "job": store::cancel_job(cwd, config, project_id, required_str(arguments, "job_id")?)?
        })),
        _ => Err(McpError::InvalidRequest(
            "unsupported creative job action".into(),
        )),
    }
}

fn required_str<'a>(arguments: &'a Value, field: &str) -> Result<&'a str, McpError> {
    arguments
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| McpError::InvalidRequest(format!("creative {field} is required")))
}

fn optional_string(arguments: &Value, field: &str) -> Option<String> {
    arguments
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn parse_required<T: DeserializeOwned>(arguments: &Value, field: &str) -> Result<T, McpError> {
    let value = arguments
        .get(field)
        .cloned()
        .ok_or_else(|| McpError::InvalidRequest(format!("creative {field} is required")))?;
    serde_json::from_value(value)
        .map_err(|_| McpError::InvalidRequest(format!("creative {field} is invalid")))
}

fn parse_optional<T: DeserializeOwned>(
    arguments: &Value,
    field: &str,
) -> Result<Option<T>, McpError> {
    arguments
        .get(field)
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(|_| McpError::InvalidRequest(format!("creative {field} is invalid")))
}

fn complete(value: Value) -> Result<ToolCallResult, McpError> {
    let text = serde_json::to_string(&value)
        .map_err(|_| McpError::Internal("creative result could not be serialized".into()))?;
    Ok(ToolCallResult::complete(vec![ToolResultContent {
        kind: "text",
        text,
    }]))
}

fn error_result(code: &str, message: &str, detail: Value) -> Result<ToolCallResult, McpError> {
    let text = serde_json::to_string(&json!({
        "code": code,
        "message": message,
        "detail": detail
    }))
    .map_err(|_| McpError::Internal("creative error could not be serialized".into()))?;
    Ok(ToolCallResult::error(vec![ToolResultContent {
        kind: "text",
        text,
    }]))
}
