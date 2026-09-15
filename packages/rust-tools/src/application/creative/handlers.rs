use super::graph::CreativeGraph;
use super::support::*;
use super::{contracts::*, graph, jobs, store};

mod asset;
mod project;
use crate::core::config::ServerConfig;
use crate::core::error::McpError;
use crate::interfaces::mcp::ToolCallResult;
pub(in crate::application::creative) use asset::asset;
pub(super) use project::project;
use serde_json::{json, Value};

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
        "reject" => {
            let project = store::reject_element_revision(
                cwd,
                config,
                project_id,
                required_str(arguments, "element_id")?,
                required_str(arguments, "revision_id")?,
            )?;
            complete(json!({
                "element_id": required_str(arguments, "element_id")?,
                "revision_id": required_str(arguments, "revision_id")?,
                "state": "rejected",
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

pub(super) async fn graph_tool(
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
            let approved = arguments
                .get("approved")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let job =
                jobs::execute_graph_direct(cwd, config, owner, &graph, &project, approved).await?;
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
        "partial_rerun" | "rerun_selected" | "rerun_subgraph" | "rerun_all_dirty" => {
            let project_id = required_str(arguments, "project_id")?;
            let graph_id = required_str(arguments, "graph_id")?;
            let previous_job_id = required_str(arguments, "previous_job_id")?;
            let changed_node_ids: Vec<String> = if action == "rerun_selected" {
                vec![required_str(arguments, "node_id")?.to_owned()]
            } else {
                parse_required(arguments, "changed_node_ids")?
            };
            let graph = store::load_graph(cwd, config, project_id, graph_id)?;
            let project = store::load_project(cwd, config, project_id)?;
            let previous = jobs::get(cwd, config, owner, project_id, previous_job_id)?;
            let invalidated = graph::dirty_descendants(&graph, &changed_node_ids)?;
            let approved = arguments
                .get("approved")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let job = jobs::execute_graph_partial_direct(
                cwd,
                config,
                owner,
                &graph,
                &project,
                jobs::PartialGraphExecution {
                    previous: &previous,
                    changed_node_ids: &changed_node_ids,
                    approved,
                },
            )
            .await?;
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
                "execution_scope": action,
                "invalidated_node_ids": invalidated_node_ids,
                "reused_node_ids": reused_node_ids
            }))
        }
        "template_instantiate" => {
            let project_id = required_str(arguments, "project_id")?;
            let template_id = required_str(arguments, "template_id")?;
            let graph_id = required_str(arguments, "graph_id")?;
            validate_id(graph_id, "graph_id")?;
            let template = store::load_template(cwd, config, project_id, template_id)?;
            let project = store::load_project(cwd, config, project_id)?;
            let replacements = arguments
                .get("replacements")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            let binding_overrides = arguments
                .get("binding_overrides")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            let mut graph = template.graph.clone();
            graph.graph_id = graph_id.to_owned();
            graph.project_id = project_id.to_owned();
            graph.revision = 1;
            for node in &mut graph.nodes {
                substitute_graph_value(&mut node.inputs, &replacements)?;
                if let Some(binding_id) =
                    binding_overrides.get(&node.node_id).and_then(Value::as_str)
                {
                    validate_id(binding_id, "execution_binding_id")?;
                    node.execution_binding_id = Some(binding_id.to_owned());
                }
            }
            let validation = graph::validate_graph(&graph, &project, config)?;
            if !validation.valid {
                return error_result(
                    "graph_template_instantiation_failed",
                    "Instantiated Creative Graph did not pass validation",
                    json!({"validation": validation}),
                );
            }
            store::store_graph(cwd, config, &graph)?;
            complete(json!({
                "graph": graph,
                "template_id": template_id,
                "template_version": template.version,
                "validation": validation
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

pub(super) async fn job(
    arguments: &Value,
    config: &ServerConfig,
    owner: &str,
) -> Result<ToolCallResult, McpError> {
    let action = required_str(arguments, "action")?;
    let cwd = arguments.get("cwd").and_then(Value::as_str);
    let project_id = required_str(arguments, "project_id")?;
    match action {
        "compile_spec" => {
            let capability_id = required_str(arguments, "capability_id")?;
            let binding_id = required_str(arguments, "execution_binding_id")?;
            let semantic_spec = arguments
                .get("semantic_spec")
                .ok_or_else(|| McpError::InvalidRequest("semantic_spec is required".into()))?;
            let changed_fields = arguments
                .get("changed_fields")
                .and_then(Value::as_array)
                .map(|values| {
                    values
                        .iter()
                        .map(|value| {
                            value.as_str().map(str::to_owned).ok_or_else(|| {
                                McpError::InvalidRequest("changed_fields is invalid".into())
                            })
                        })
                        .collect::<Result<Vec<_>, _>>()
                })
                .transpose()?
                .unwrap_or_default();
            let compiled = super::compiler::compile(
                config,
                capability_id,
                binding_id,
                semantic_spec,
                &changed_fields,
            )?;
            complete(json!({"compiled": compiled}))
        }
        "cost_estimate" => {
            let mut request = parse_job_submit_request(arguments)?;
            let _ = jobs::prepare_request(config, &mut request)?;
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
            ).await?
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

fn substitute_graph_value(
    value: &mut Value,
    replacements: &serde_json::Map<String, Value>,
) -> Result<(), McpError> {
    match value {
        Value::String(current) => {
            if let Some(replacement) = replacements.get(current) {
                if !replacement.is_string() {
                    return Err(McpError::InvalidRequest(
                        "graph template replacements must map string identities to strings".into(),
                    ));
                }
                *value = replacement.clone();
            }
        }
        Value::Array(items) => {
            for item in items {
                substitute_graph_value(item, replacements)?;
            }
        }
        Value::Object(object) => {
            for item in object.values_mut() {
                substitute_graph_value(item, replacements)?;
            }
        }
        _ => {}
    }
    Ok(())
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
        semantic_spec: arguments.get("semantic_spec").cloned(),
        changed_fields: arguments
            .get("changed_fields")
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default(),
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
