use super::graph::CreativeGraph;
use super::support::*;
use super::{contracts::*, graph, ingest, jobs, store};
use crate::core::config::ServerConfig;
use crate::core::error::McpError;
use crate::interfaces::mcp::ToolCallResult;
use serde_json::{json, Value};

pub(super) fn project(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<ToolCallResult, McpError> {
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
            complete(json!({
                "layout": store::project_layout(&created.project_id)?,
                "project": created
            }))
        }
        "get" => {
            let project = store::load_project(cwd, config, required_str(arguments, "project_id")?)?;
            complete(json!({
                "layout": store::project_layout(&project.project_id)?,
                "project": project
            }))
        }
        "list" => complete(json!({"project_ids": store::list_projects(cwd, config)?})),
        _ => Err(McpError::InvalidRequest(
            "unsupported creative project action".into(),
        )),
    }
}

pub(super) fn element(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<ToolCallResult, McpError> {
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

pub(super) async fn asset(
    arguments: &Value,
    config: &ServerConfig,
    owner: &str,
) -> Result<ToolCallResult, McpError> {
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
        "upload_request" => {
            let request = ingest::request_upload(
                cwd,
                config,
                owner,
                project_id,
                ingest::UploadRequest {
                    source: parse_optional(arguments, "source")?.unwrap_or(AssetSource::McpUpload),
                    media_type: required_str(arguments, "media_type")?.to_owned(),
                    role: required_str(arguments, "role")?.to_owned(),
                    filename: required_str(arguments, "filename")?.to_owned(),
                    max_bytes: arguments.get("max_bytes").and_then(Value::as_u64),
                    ttl_ms: arguments.get("ttl_ms").and_then(Value::as_u64),
                },
            )?;
            complete(json!({"upload": request}))
        }
        "upload_complete" => {
            let (asset_id, asset) = ingest::complete_upload(
                cwd,
                config,
                owner,
                project_id,
                required_str(arguments, "ticket_id")?,
            )?;
            complete(json!({
                "asset_id": asset_id,
                "asset": asset,
                "preview": ingest::asset_preview(&asset)
            }))
        }
        "upload_list" => complete(json!({
            "uploads": ingest::list_uploads(cwd, config, owner, project_id)?
        })),
        "import_url" => {
            let (asset_id, asset) = ingest::import_url(
                cwd,
                config,
                project_id,
                ingest::UrlImportRequest {
                    url: required_str(arguments, "url")?.to_owned(),
                    media_type: optional_string(arguments, "media_type"),
                    role: required_str(arguments, "role")?.to_owned(),
                    filename: optional_string(arguments, "filename"),
                    max_bytes: arguments.get("max_bytes").and_then(Value::as_u64),
                    parent_asset_id: optional_string(arguments, "parent_asset_id"),
                    element_id: optional_string(arguments, "element_id"),
                },
            )
            .await?;
            complete(json!({
                "asset_id": asset_id,
                "asset": asset,
                "preview": ingest::asset_preview(&asset)
            }))
        }
        "preview" => {
            let project = store::load_project(cwd, config, project_id)?;
            let asset = project
                .asset(required_str(arguments, "asset_id")?)
                .ok_or_else(|| McpError::InvalidRequest("unknown creative asset".into()))?;
            complete(json!({"preview": ingest::asset_preview(asset)}))
        }
        _ => Err(McpError::InvalidRequest(
            "unsupported creative asset action".into(),
        )),
    }
}

pub(super) fn graph_tool(
    arguments: &Value,
    config: &ServerConfig,
    owner: &str,
) -> Result<ToolCallResult, McpError> {
    let action = required_str(arguments, "action")?;
    let cwd = arguments.get("cwd").and_then(Value::as_str);
    match action {
        "validate" => {
            let graph: CreativeGraph = parse_required(arguments, "graph")?;
            let project = store::load_project(cwd, config, &graph.project_id)?;
            let validation = graph::validate_graph(&graph, &project, config)?;
            complete(json!({"validation": validation}))
        }
        "execute" => {
            let graph: CreativeGraph = parse_required(arguments, "graph")?;
            let project = store::load_project(cwd, config, &graph.project_id)?;
            let validation = graph::validate_graph(&graph, &project, config)?;
            if !validation.valid {
                return error_result(
                    "graph_validation_failed",
                    "Creative graph did not pass validation",
                    json!({"validation": validation}),
                );
            }
            store::store_graph(cwd, config, &graph)?;
            let job = graph::execute_graph(&graph, &project, config, owner, store::now_ms())?;
            store::store_job(cwd, config, &job)?;
            complete(json!({"job": job}))
        }
        "template_save" => {
            let graph: CreativeGraph = parse_required(arguments, "graph")?;
            let project = store::load_project(cwd, config, &graph.project_id)?;
            let template = graph::CreativeGraphTemplate {
                schema_version: CREATIVE_SCHEMA_VERSION,
                template_id: required_str(arguments, "template_id")?.to_owned(),
                project_id: project.project_id.clone(),
                version: arguments
                    .get("version")
                    .and_then(Value::as_u64)
                    .and_then(|value| u32::try_from(value).ok())
                    .unwrap_or(1),
                description: arguments
                    .get("description")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
                graph,
                input_node_ids: parse_optional(arguments, "input_node_ids")?.unwrap_or_default(),
                output_node_ids: parse_required(arguments, "output_node_ids")?,
                created_at_ms: store::now_ms(),
            };
            graph::validate_graph_template(&template, &project, config)?;
            store::store_template(cwd, config, &template)?;
            complete(json!({"template": template}))
        }
        "template_get" => {
            let project_id = required_str(arguments, "project_id")?;
            let template_id = required_str(arguments, "template_id")?;
            complete(json!({
                "template": store::load_template(cwd, config, project_id, template_id)?
            }))
        }
        "template_list" => {
            let project_id = required_str(arguments, "project_id")?;
            complete(json!({
                "templates": store::list_templates(cwd, config, project_id)?
            }))
        }
        "partial_rerun" => {
            let project_id = required_str(arguments, "project_id")?;
            let graph_id = required_str(arguments, "graph_id")?;
            let previous_job_id = required_str(arguments, "previous_job_id")?;
            let changed_node_ids: Vec<String> = parse_required(arguments, "changed_node_ids")?;
            let graph = store::load_graph(cwd, config, project_id, graph_id)?;
            let project = store::load_project(cwd, config, project_id)?;
            let previous = jobs::get(cwd, config, owner, project_id, previous_job_id)?;
            let invalidated = graph::dirty_descendants(&graph, &changed_node_ids)?;
            let job = graph::execute_graph_partial(
                &graph,
                &project,
                config,
                owner,
                &previous,
                &changed_node_ids,
                store::now_ms(),
            )?;
            store::store_job(cwd, config, &job)?;
            let mut invalidated_node_ids = invalidated.into_iter().collect::<Vec<_>>();
            invalidated_node_ids.sort();
            let reused_node_ids = job
                .node_runs
                .iter()
                .filter(|run| run.reused)
                .map(|run| run.node_id.clone())
                .collect::<Vec<_>>();
            complete(json!({
                "job": job,
                "invalidated_node_ids": invalidated_node_ids,
                "reused_node_ids": reused_node_ids
            }))
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

pub(super) fn job(
    arguments: &Value,
    config: &ServerConfig,
    owner: &str,
) -> Result<ToolCallResult, McpError> {
    let action = required_str(arguments, "action")?;
    let cwd = arguments.get("cwd").and_then(Value::as_str);
    let project_id = required_str(arguments, "project_id")?;
    match action {
        "cost_estimate" => {
            let request = parse_job_submit_request(arguments)?;
            let estimate = jobs::estimate_request(cwd, config, project_id, &request)?;
            complete(json!({"estimate": estimate}))
        }
        "budget_status" => complete(json!({
            "budget": jobs::budget_status(cwd, config, owner, project_id)?
        })),
        "submit" => {
            let request = parse_job_submit_request(arguments)?;
            let (decision, job, estimate) = jobs::submit(cwd, config, owner, project_id, request)?;
            match decision {
                jobs::AdmissionDecision::Allowed => complete(json!({
                    "job": job,
                    "estimate": estimate
                })),
                jobs::AdmissionDecision::ApprovalRequired => error_result(
                    "creative_job_approval_required",
                    "Creative job estimate exceeds the configured approval threshold",
                    json!({"estimate": estimate, "budget": jobs::budget_status(cwd, config, owner, project_id)?}),
                ),
                jobs::AdmissionDecision::JobHardLimitExceeded => error_result(
                    "creative_job_hard_limit_exceeded",
                    "Creative job estimate exceeds the operator hard compute maximum",
                    json!({"estimate": estimate}),
                ),
                jobs::AdmissionDecision::ProjectHardLimitExceeded => error_result(
                    "creative_project_hard_limit_exceeded",
                    "Creative project admitted compute would exceed the operator hard maximum",
                    json!({"estimate": estimate, "budget": jobs::budget_status(cwd, config, owner, project_id)?}),
                ),
                jobs::AdmissionDecision::OutputHardLimitExceeded => error_result(
                    "creative_job_output_limit_exceeded",
                    "Creative job estimate exceeds the operator output-byte maximum",
                    json!({"estimate": estimate}),
                ),
                jobs::AdmissionDecision::ConcurrencyLimitExceeded => error_result(
                    "creative_job_concurrency_limit",
                    "Creative project has reached its running-job limit",
                    json!({"budget": jobs::budget_status(cwd, config, owner, project_id)?}),
                ),
            }
        }
        "get" => complete(json!({
            "job": jobs::get(cwd, config, owner, project_id, required_str(arguments, "job_id")?)?
        })),
        "wait" => complete(json!({
            "job": jobs::wait(
                cwd,
                config,
                owner,
                project_id,
                required_str(arguments, "job_id")?,
                arguments.get("retry").and_then(Value::as_bool).unwrap_or(false),
            )?
        })),
        "list" => complete(json!({"jobs": jobs::list(cwd, config, owner, project_id)?})),
        "cancel" => complete(json!({
            "job": jobs::cancel(cwd, config, owner, project_id, required_str(arguments, "job_id")?)?
        })),
        _ => Err(McpError::InvalidRequest(
            "unsupported creative job action".into(),
        )),
    }
}

fn parse_job_submit_request(arguments: &Value) -> Result<jobs::SubmitRequest, McpError> {
    Ok(jobs::SubmitRequest {
        graph_id: optional_string(arguments, "graph_id"),
        capability_id: optional_string(arguments, "capability_id"),
        workflow_id: optional_string(arguments, "workflow_id"),
        execution_binding_id: optional_string(arguments, "execution_binding_id"),
        parameters: arguments
            .get("parameters")
            .cloned()
            .unwrap_or_else(|| json!({})),
        approved: arguments
            .get("approved")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        max_retries: arguments
            .get("max_retries")
            .and_then(Value::as_u64)
            .and_then(|value| u32::try_from(value).ok()),
        timeout_ms: arguments.get("timeout_ms").and_then(Value::as_u64),
    })
}
