use crate::core::error::McpError;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

mod capabilities;
mod workflows;
pub use capabilities::capabilities;
pub use workflows::workflows;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CapabilityDescriptor {
    pub capability_id: String,
    pub family: String,
    pub requires_execution_binding: bool,
    #[serde(default)]
    pub input_roles: Vec<String>,
    #[serde(default)]
    pub output_roles: Vec<String>,
    #[serde(default)]
    pub effects: Vec<String>,
    pub parameter_schema: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecutionBindingDescriptor {
    pub binding_id: String,
    pub binding_version: String,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub media_roles: Vec<String>,
    pub extension_schema: Value,
    pub constraints: Value,
    #[serde(default)]
    pub license_notes: Option<String>,
    pub estimate_available: bool,
    pub availability: BindingAvailability,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BindingAvailability {
    Available,
    Unavailable,
    Degraded,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkflowDescriptor {
    pub workflow_id: String,
    pub version: u32,
    pub description: String,
    #[serde(default)]
    pub required_capabilities: Vec<String>,
    pub input_schema: Value,
    pub output_schema: Value,
}

pub fn capability(id: &str) -> Option<CapabilityDescriptor> {
    capabilities()
        .into_iter()
        .find(|descriptor| descriptor.capability_id == id)
}

pub fn workflow(id: &str) -> Option<WorkflowDescriptor> {
    workflows()
        .into_iter()
        .find(|descriptor| descriptor.workflow_id == id)
}

pub fn validate_capability_parameters(
    id: &str,
    parameters: &Value,
) -> Result<CapabilityDescriptor, McpError> {
    super::contracts::validate_spec(parameters)?;
    let descriptor = capability(id)
        .ok_or_else(|| McpError::InvalidRequest("unknown creative capability".into()))?;
    let validator = jsonschema::validator_for(&descriptor.parameter_schema)
        .map_err(|_| McpError::Internal("creative capability schema is invalid".into()))?;
    if validator.iter_errors(parameters).next().is_some() {
        return Err(McpError::InvalidRequest(
            "creative capability parameters do not match the capability schema".into(),
        ));
    }
    validate_capability_contract(id, parameters)?;
    Ok(descriptor)
}

pub fn validate_workflow_parameters(
    id: &str,
    parameters: &Value,
) -> Result<WorkflowDescriptor, McpError> {
    super::contracts::validate_spec(parameters)?;
    let descriptor =
        workflow(id).ok_or_else(|| McpError::InvalidRequest("unknown creative workflow".into()))?;
    let validator = jsonschema::validator_for(&descriptor.input_schema)
        .map_err(|_| McpError::Internal("creative workflow schema is invalid".into()))?;
    if validator.iter_errors(parameters).next().is_some() {
        return Err(McpError::InvalidRequest(
            "creative workflow parameters do not match the workflow schema".into(),
        ));
    }
    Ok(descriptor)
}

fn validate_capability_contract(id: &str, parameters: &Value) -> Result<(), McpError> {
    match id {
        "audio.voice_clone" => {
            if parameters.get("consent_asserted").and_then(Value::as_bool) != Some(true) {
                return Err(McpError::InvalidRequest(
                    "audio.voice_clone requires explicit consent_asserted=true".into(),
                ));
            }
            let references = parameters
                .get("reference_asset_ids")
                .and_then(Value::as_array)
                .ok_or_else(|| {
                    McpError::InvalidRequest(
                        "audio.voice_clone requires reference_asset_ids".into(),
                    )
                })?;
            if references.is_empty() || references.len() > 16 {
                return Err(McpError::InvalidRequest(
                    "audio.voice_clone reference count is outside allowed bounds".into(),
                ));
            }
        }
        "audio.voice_change" => {
            required_parameter_id(parameters, "asset_id")?;
            required_parameter_id(parameters, "voice_element_id")?;
        }
        "audio.video_dub" => {
            required_parameter_id(parameters, "video_asset_id")?;
            required_parameter_id(parameters, "voice_element_id")?;
            required_parameter_text(parameters, "language", 32)?;
        }
        "video.motion_control" => {
            required_parameter_id(parameters, "reference_asset_id")?;
            required_parameter_id(parameters, "motion_asset_id")?;
        }
        "video.clip_extract" => {
            required_parameter_id(parameters, "asset_id")?;
            let start = parameters
                .get("start_ms")
                .and_then(Value::as_u64)
                .ok_or_else(|| McpError::InvalidRequest("clip start_ms is required".into()))?;
            let end = parameters
                .get("end_ms")
                .and_then(Value::as_u64)
                .ok_or_else(|| McpError::InvalidRequest("clip end_ms is required".into()))?;
            if end <= start || end - start > 30 * 60 * 1000 {
                return Err(McpError::InvalidRequest(
                    "clip extraction range is outside allowed bounds".into(),
                ));
            }
        }
        _ => {}
    }
    Ok(())
}

fn required_parameter_id(parameters: &Value, field: &str) -> Result<(), McpError> {
    let value = parameters
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::InvalidRequest(format!("creative {field} is required")))?;
    super::contracts::validate_id(value, field)
}

fn required_parameter_text(
    parameters: &Value,
    field: &str,
    max_len: usize,
) -> Result<(), McpError> {
    let value = parameters
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::InvalidRequest(format!("creative {field} is required")))?;
    if value.is_empty() || value.len() > max_len || value.chars().any(char::is_control) {
        return Err(McpError::InvalidRequest(format!(
            "creative {field} is outside allowed bounds"
        )));
    }
    Ok(())
}

mod bindings;
pub use bindings::{
    binding, compatible_binding_ids, execution_bindings, validate_binding_selection,
};

pub(super) fn identity_prepare_descriptor() -> CapabilityDescriptor {
    CapabilityDescriptor {
        capability_id: "identity.prepare".into(),
        family: "identity".into(),
        requires_execution_binding: true,
        input_roles: vec!["reference_image".into()],
        output_roles: vec!["identity_artifact".into()],
        effects: vec!["network_or_compute".into(), "workspace_write".into()],
        parameter_schema: json!({
            "type":"object",
            "properties":{
                "element_id":{"type":"string","minLength":1,"maxLength":64},
                "reference_asset_ids":{"type":"array","minItems":1,"maxItems":32,"uniqueItems":true,"items":{"type":"string","minLength":1,"maxLength":64}},
                "subject_kind":{"type":"string","enum":["fictional","real_person"]},
                "authorization_attested":{"type":"boolean"},
                "artifact_kind":{"type":"string","minLength":1,"maxLength":64},
                "artifact_version":{"type":"string","minLength":1,"maxLength":128}
            },
            "required":["element_id","reference_asset_ids","subject_kind","artifact_kind","artifact_version"],
            "allOf":[{
                "if":{"properties":{"subject_kind":{"const":"real_person"}},"required":["subject_kind"]},
                "then":{"properties":{"authorization_attested":{"const":true}},"required":["authorization_attested"]}
            }],
            "additionalProperties":false
        }),
    }
}

pub(super) fn capability_descriptor(
    id: &str,
    family: &str,
    requires_binding: bool,
    input_roles: &[&str],
    output_roles: &[&str],
    effects: &[&str],
) -> CapabilityDescriptor {
    CapabilityDescriptor {
        capability_id: id.to_owned(),
        family: family.to_owned(),
        requires_execution_binding: requires_binding,
        input_roles: input_roles
            .iter()
            .map(|value| (*value).to_owned())
            .collect(),
        output_roles: output_roles
            .iter()
            .map(|value| (*value).to_owned())
            .collect(),
        effects: effects.iter().map(|value| (*value).to_owned()).collect(),
        parameter_schema: json!({
            "type": "object",
            "maxProperties": 64,
            "additionalProperties": true
        }),
    }
}
