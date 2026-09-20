use super::{validate_submit_shape, SubmitRequest};
use crate::application::creative::contracts::{validate_spec, CreativeProject};
use crate::application::creative::graph::{CreativeEstimate, CreativeGraph};
use crate::application::creative::{graph, registry, store};
use crate::core::config::ServerConfig;
use crate::core::error::McpError;
use serde_json::Value;

const CONTROL_NODE_COMPUTE_UNITS: u64 = 2;
const CONTROL_NODE_OUTPUT_BYTES: u64 = 4 * 1024;
const REVIEWED_CAPABILITY_COMPUTE_UNITS: u64 = 10;
const REVIEWED_CAPABILITY_OUTPUT_BYTES: u64 = 16 * 1024;

pub fn estimate_capability(
    config: &ServerConfig,
    capability_id: &str,
    execution_binding_id: Option<&str>,
    parameters: &Value,
) -> Result<CreativeEstimate, McpError> {
    validate_spec(parameters)?;
    let capability = registry::validate_capability_parameters(capability_id, parameters)?;
    if !capability.requires_execution_binding {
        return Ok(CreativeEstimate {
            compute_units: REVIEWED_CAPABILITY_COMPUTE_UNITS,
            output_bytes: REVIEWED_CAPABILITY_OUTPUT_BYTES,
            estimated_cost_micros: None,
            source: "reviewed_local_contract".into(),
        });
    }
    let binding =
        registry::validate_binding_selection(config, capability_id, execution_binding_id)?
            .ok_or_else(|| {
                McpError::Internal("required creative binding was not returned".into())
            })?;
    if !binding.estimate_available {
        return Err(McpError::InvalidRequest(
            "creative cost estimate is unavailable for selected execution binding".into(),
        ));
    }
    estimate_from_binding_constraints(&binding.binding_id, &binding.constraints, parameters)
}

pub fn estimate_graph(
    config: &ServerConfig,
    graph: &CreativeGraph,
    project: &CreativeProject,
) -> Result<CreativeEstimate, McpError> {
    let validation = graph::validate_graph(graph, project, config)?;
    if !validation.valid {
        return Err(McpError::InvalidRequest(
            "creative graph cannot be estimated before validation succeeds".into(),
        ));
    }
    let mut compute_units = 0u64;
    let mut output_bytes = 0u64;
    let mut estimated_cost_micros = Some(0u64);
    for node in &graph.nodes {
        if let Some(capability_id) = graph::capability_for_node(&node.kind) {
            let estimate = estimate_capability(
                config,
                capability_id,
                node.execution_binding_id.as_deref(),
                &node.inputs,
            )?;
            compute_units = checked_add(compute_units, estimate.compute_units)?;
            output_bytes = checked_add(output_bytes, estimate.output_bytes)?;
            estimated_cost_micros = match (estimated_cost_micros, estimate.estimated_cost_micros) {
                (Some(total), Some(value)) => Some(checked_add(total, value)?),
                _ => None,
            };
        } else {
            compute_units = checked_add(compute_units, CONTROL_NODE_COMPUTE_UNITS)?;
            output_bytes = checked_add(output_bytes, CONTROL_NODE_OUTPUT_BYTES)?;
        }
    }
    Ok(CreativeEstimate {
        compute_units: compute_units.max(1),
        output_bytes: output_bytes.max(1),
        estimated_cost_micros,
        source: "creative_graph".into(),
    })
}

pub fn estimate_request(
    cwd: Option<&str>,
    config: &ServerConfig,
    project_id: &str,
    request: &SubmitRequest,
) -> Result<CreativeEstimate, McpError> {
    let project = store::load_project(cwd, config, project_id)?;
    validate_submit_shape(request)?;
    if let Some(graph_id) = request.graph_id.as_deref() {
        let graph = store::load_graph(cwd, config, project_id, graph_id)?;
        return estimate_graph(config, &graph, &project);
    }
    if let Some(capability_id) = request.capability_id.as_deref() {
        return estimate_capability(
            config,
            capability_id,
            request.execution_binding_id.as_deref(),
            &request.parameters,
        );
    }
    let workflow_id = request
        .workflow_id
        .as_deref()
        .ok_or_else(|| McpError::InvalidRequest("creative job target is required".into()))?;
    let workflow = registry::validate_workflow_parameters(workflow_id, &request.parameters)?;
    if workflow_id == "image_to_3d_bootstrap" {
        let route = request
            .parameters
            .get("route")
            .and_then(Value::as_str)
            .ok_or_else(|| McpError::InvalidRequest("bootstrap route is required".into()))?;
        return match route {
            "manual" => {
                if request.execution_binding_id.is_some() {
                    return Err(McpError::InvalidRequest(
                        "manual Blender bootstrap must not select an execution binding".into(),
                    ));
                }
                Ok(CreativeEstimate {
                    compute_units: 25,
                    output_bytes: 256 * 1024,
                    estimated_cost_micros: None,
                    source: "reviewed_blender_bootstrap".into(),
                })
            }
            "binding" | "hybrid" => estimate_capability(
                config,
                "3d.image_to_mesh",
                request.execution_binding_id.as_deref(),
                &request.parameters,
            ),
            _ => Err(McpError::InvalidRequest(
                "bootstrap route is unsupported".into(),
            )),
        };
    }
    if matches!(
        workflow_id,
        "export_profile"
            | "project_handoff"
            | "promo_pack"
            | "key_visual_bundle"
            | "portfolio_delivery"
            | "game_source_scaffold"
            | "game_build_playtest"
            | "game_iteration"
            | "scene_continuity_review"
            | "visual_qa_evidence"
            | "temporal_qa_evidence"
            | "scoped_revision"
    ) {
        if request.execution_binding_id.is_some() {
            return Err(McpError::InvalidRequest(
                "reviewed local workflows do not accept an execution binding".into(),
            ));
        }
        return Ok(CreativeEstimate {
            compute_units: 5,
            output_bytes: 4 * 1024 * 1024,
            estimated_cost_micros: None,
            source: "reviewed_local_delivery".into(),
        });
    }
    if matches!(
        workflow_id,
        "character_mesh_production"
            | "character_rig_production"
            | "character_action"
            | "character_secondary_motion"
    ) {
        if request.execution_binding_id.is_some() {
            return Err(McpError::InvalidRequest(
                "Blender character production workflows do not accept an execution binding".into(),
            ));
        }
        return Ok(CreativeEstimate {
            compute_units: 75,
            output_bytes: 64 * 1024 * 1024,
            estimated_cost_micros: None,
            source: "reviewed_blender_character_workflow".into(),
        });
    }
    if workflow.required_capabilities.len() != 1 {
        return Err(McpError::InvalidRequest(
            "multi-capability workflow jobs must execute through a Creative Graph".into(),
        ));
    }
    estimate_capability(
        config,
        &workflow.required_capabilities[0],
        request.execution_binding_id.as_deref(),
        &request.parameters,
    )
}

fn estimate_from_binding_constraints(
    binding_id: &str,
    constraints: &Value,
    parameters: &Value,
) -> Result<CreativeEstimate, McpError> {
    let estimate = constraints
        .get("estimate")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            McpError::InvalidRequest(
                "selected execution binding does not expose bounded estimate data".into(),
            )
        })?;
    let base_compute = required_positive_u64(estimate, "base_compute_units")?;
    let base_output = required_positive_u64(estimate, "base_output_bytes")?;
    let width = bounded_u64(parameters, "width", 1, 16_384)?.unwrap_or(0);
    let height = bounded_u64(parameters, "height", 1, 16_384)?.unwrap_or(0);
    let duration_ms = bounded_u64(parameters, "duration_ms", 1, 86_400_000)?.unwrap_or(0);
    let batch_count = bounded_u64(parameters, "batch_count", 1, 128)?.unwrap_or(1);
    let megapixels_milli = if width > 0 && height > 0 {
        width
            .checked_mul(height)
            .and_then(|pixels| pixels.checked_mul(1_000))
            .ok_or_else(estimate_overflow)?
            / 1_000_000
    } else {
        0
    };
    let seconds_milli = duration_ms;
    let compute = scaled_estimate(
        base_compute,
        optional_u64(estimate, "compute_units_per_megapixel")?,
        megapixels_milli,
        optional_u64(estimate, "compute_units_per_second")?,
        seconds_milli,
        optional_u64(estimate, "compute_units_per_item")?,
        batch_count,
    )?;
    let output = scaled_estimate(
        base_output,
        optional_u64(estimate, "output_bytes_per_megapixel")?,
        megapixels_milli,
        optional_u64(estimate, "output_bytes_per_second")?,
        seconds_milli,
        optional_u64(estimate, "output_bytes_per_item")?,
        batch_count,
    )?;
    let cost_per_compute = optional_u64(estimate, "cost_micros_per_compute_unit")?;
    let estimated_cost_micros = cost_per_compute
        .map(|unit| compute.checked_mul(unit).ok_or_else(estimate_overflow))
        .transpose()?;
    Ok(CreativeEstimate {
        compute_units: compute.max(1),
        output_bytes: output.max(1),
        estimated_cost_micros,
        source: format!("binding:{binding_id}"),
    })
}

fn scaled_estimate(
    base: u64,
    per_megapixel: Option<u64>,
    megapixels_milli: u64,
    per_second: Option<u64>,
    seconds_milli: u64,
    per_item: Option<u64>,
    batch_count: u64,
) -> Result<u64, McpError> {
    let mut value = base;
    if let Some(unit) = per_megapixel {
        value = checked_add(value, checked_mul_div(unit, megapixels_milli, 1_000)?)?;
    }
    if let Some(unit) = per_second {
        value = checked_add(value, checked_mul_div(unit, seconds_milli, 1_000)?)?;
    }
    if let Some(unit) = per_item {
        value = checked_add(
            value,
            unit.checked_mul(batch_count)
                .ok_or_else(estimate_overflow)?,
        )?;
    }
    Ok(value)
}

fn checked_mul_div(left: u64, right: u64, divisor: u64) -> Result<u64, McpError> {
    left.checked_mul(right)
        .map(|value| value / divisor)
        .ok_or_else(estimate_overflow)
}

fn checked_add(left: u64, right: u64) -> Result<u64, McpError> {
    left.checked_add(right).ok_or_else(estimate_overflow)
}

fn estimate_overflow() -> McpError {
    McpError::InvalidRequest("creative estimate exceeds numeric bounds".into())
}

fn required_positive_u64(
    object: &serde_json::Map<String, Value>,
    field: &str,
) -> Result<u64, McpError> {
    let value = object.get(field).and_then(Value::as_u64).ok_or_else(|| {
        McpError::InvalidRequest("creative binding estimate profile is incomplete".into())
    })?;
    if value == 0 {
        return Err(McpError::InvalidRequest(
            "creative binding estimate profile contains zero bounds".into(),
        ));
    }
    Ok(value)
}

fn optional_u64(
    object: &serde_json::Map<String, Value>,
    field: &str,
) -> Result<Option<u64>, McpError> {
    match object.get(field) {
        None => Ok(None),
        Some(value) => value.as_u64().map(Some).ok_or_else(|| {
            McpError::InvalidRequest("creative binding estimate profile is invalid".into())
        }),
    }
}

fn bounded_u64(
    parameters: &Value,
    field: &str,
    minimum: u64,
    maximum: u64,
) -> Result<Option<u64>, McpError> {
    match parameters.get(field) {
        None => Ok(None),
        Some(value) => {
            let value = value.as_u64().ok_or_else(|| {
                McpError::InvalidRequest(format!("creative {field} parameter is invalid"))
            })?;
            if value < minimum || value > maximum {
                return Err(McpError::InvalidRequest(format!(
                    "creative {field} parameter exceeds allowed bounds"
                )));
            }
            Ok(Some(value))
        }
    }
}
