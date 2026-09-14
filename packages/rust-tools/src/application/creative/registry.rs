use crate::core::error::McpError;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

mod workflows;
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

pub fn capabilities() -> Vec<CapabilityDescriptor> {
    vec![
        capability_descriptor(
            "image.generate",
            "image",
            true,
            &[],
            &["image"],
            &["network_or_compute", "workspace_write"],
        ),
        capability_descriptor(
            "image.reference_generate",
            "image",
            true,
            &["reference_image"],
            &["image"],
            &["network_or_compute", "workspace_write"],
        ),
        capability_descriptor(
            "image.edit",
            "image",
            true,
            &["image"],
            &["image"],
            &["network_or_compute", "workspace_write"],
        ),
        capability_descriptor(
            "image.inpaint",
            "image",
            true,
            &["image", "mask"],
            &["image"],
            &["network_or_compute", "workspace_write"],
        ),
        capability_descriptor(
            "image.upscale",
            "image",
            true,
            &["image"],
            &["image"],
            &["network_or_compute", "workspace_write"],
        ),
        capability_descriptor(
            "image.remove_background",
            "image",
            true,
            &["image"],
            &["image_alpha"],
            &["network_or_compute", "workspace_write"],
        ),
        capability_descriptor(
            "image.outpaint",
            "image",
            true,
            &["image"],
            &["image"],
            &["network_or_compute", "workspace_write"],
        ),
        capability_descriptor(
            "video.generate",
            "video",
            true,
            &[],
            &["video"],
            &["network_or_compute", "workspace_write"],
        ),
        capability_descriptor(
            "video.image_to_video",
            "video",
            true,
            &["reference_image"],
            &["video"],
            &["network_or_compute", "workspace_write"],
        ),
        capability_descriptor(
            "video.reference_generate",
            "video",
            true,
            &["reference_image"],
            &["video"],
            &["network_or_compute", "workspace_write"],
        ),
        capability_descriptor(
            "video.extend",
            "video",
            true,
            &["video"],
            &["video"],
            &["network_or_compute", "workspace_write"],
        ),
        capability_descriptor(
            "video.reframe",
            "video",
            true,
            &["video"],
            &["video"],
            &["network_or_compute", "workspace_write"],
        ),
        capability_descriptor(
            "video.upscale",
            "video",
            true,
            &["video"],
            &["video"],
            &["network_or_compute", "workspace_write"],
        ),
        capability_descriptor(
            "video.remove_background",
            "video",
            true,
            &["video"],
            &["video_alpha"],
            &["network_or_compute", "workspace_write"],
        ),
        capability_descriptor(
            "video.motion_control",
            "video",
            true,
            &["reference_image", "motion_reference"],
            &["video"],
            &["network_or_compute", "workspace_write"],
        ),
        capability_descriptor(
            "video.clip_extract",
            "video",
            true,
            &["video"],
            &["video_clip"],
            &["compute", "workspace_write"],
        ),
        capability_descriptor(
            "audio.speech",
            "audio",
            true,
            &[],
            &["audio"],
            &["network_or_compute", "workspace_write"],
        ),
        capability_descriptor(
            "audio.voice",
            "audio",
            true,
            &[],
            &["audio"],
            &["network_or_compute", "workspace_write"],
        ),
        capability_descriptor(
            "audio.music",
            "audio",
            true,
            &[],
            &["audio"],
            &["network_or_compute", "workspace_write"],
        ),
        capability_descriptor(
            "audio.sfx",
            "audio",
            true,
            &[],
            &["audio"],
            &["network_or_compute", "workspace_write"],
        ),
        capability_descriptor(
            "audio.voice_clone",
            "audio",
            true,
            &["voice_reference"],
            &["voice_artifact"],
            &["network_or_compute", "workspace_write"],
        ),
        capability_descriptor(
            "audio.voice_change",
            "audio",
            true,
            &["audio"],
            &["audio"],
            &["network_or_compute", "workspace_write"],
        ),
        capability_descriptor(
            "audio.video_dub",
            "audio",
            true,
            &["video", "voice_reference"],
            &["video"],
            &["network_or_compute", "workspace_write"],
        ),
        capability_descriptor(
            "3d.image_to_mesh",
            "3d",
            true,
            &["reference_image"],
            &["mesh"],
            &["network_or_compute", "workspace_write"],
        ),
        capability_descriptor(
            "3d.text_to_mesh",
            "3d",
            true,
            &[],
            &["mesh"],
            &["network_or_compute", "workspace_write"],
        ),
        capability_descriptor(
            "3d.texture",
            "3d",
            true,
            &["mesh", "reference_image"],
            &["mesh"],
            &["network_or_compute", "workspace_write"],
        ),
        capability_descriptor(
            "3d.rig_bootstrap",
            "3d",
            true,
            &["mesh"],
            &["rigged_mesh"],
            &["network_or_compute", "workspace_write"],
        ),
        capability_descriptor(
            "storyboard.store",
            "scene",
            false,
            &["element_reference"],
            &["storyboard"],
            &["workspace_write"],
        ),
        capability_descriptor(
            "scene.manifest_validate",
            "scene",
            false,
            &["element_reference"],
            &["scene_manifest"],
            &["workspace_read"],
        ),
        capability_descriptor(
            "graph.validate",
            "graph",
            false,
            &[],
            &["graph_validation"],
            &["workspace_read"],
        ),
        capability_descriptor(
            "graph.execute",
            "graph",
            false,
            &[],
            &["graph_run"],
            &["workspace_write"],
        ),
        capability_descriptor(
            "dcc.execute",
            "dcc",
            true,
            &["asset"],
            &["asset"],
            &["privileged_bridge", "workspace_write"],
        ),
        capability_descriptor(
            "dcc.render",
            "dcc",
            true,
            &["scene"],
            &["image", "video"],
            &["privileged_bridge", "compute", "workspace_write"],
        ),
        capability_descriptor(
            "game.build",
            "game",
            true,
            &["game_source", "asset"],
            &["game_build"],
            &["process_exec", "workspace_write"],
        ),
        capability_descriptor(
            "game.playtest",
            "game",
            true,
            &["game_build"],
            &["playtest_evidence"],
            &["process_exec", "workspace_read"],
        ),
        capability_descriptor(
            "game.deploy",
            "game",
            true,
            &["game_build"],
            &["deployment"],
            &["network_write", "external_mutation"],
        ),
    ]
}

pub fn execution_bindings() -> Vec<ExecutionBindingDescriptor> {
    // Phase-03 intentionally ships no default executor. Bindings are operator-owned
    // implementation state and later phases may register reviewed implementations.
    // An empty list is a truthful availability state, not an invitation to auto-route.
    Vec::new()
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

pub fn binding(id: &str) -> Option<ExecutionBindingDescriptor> {
    execution_bindings()
        .into_iter()
        .find(|descriptor| descriptor.binding_id == id)
}

pub fn compatible_binding_ids(capability_id: &str) -> Vec<String> {
    execution_bindings()
        .into_iter()
        .filter(|descriptor| {
            descriptor.availability == BindingAvailability::Available
                && descriptor
                    .capabilities
                    .iter()
                    .any(|value| value == capability_id)
        })
        .map(|descriptor| descriptor.binding_id)
        .collect()
}

pub fn validate_binding_selection(
    capability_id: &str,
    selected: Option<&str>,
) -> Result<Option<ExecutionBindingDescriptor>, McpError> {
    let descriptor = capability(capability_id)
        .ok_or_else(|| McpError::InvalidRequest("unknown creative capability".into()))?;
    if !descriptor.requires_execution_binding {
        return Ok(None);
    }
    let compatible = compatible_binding_ids(capability_id);
    let Some(selected) = selected else {
        return Err(McpError::InvalidRequest(format!(
            "execution_binding_required:{capability_id}:{}",
            compatible.join(",")
        )));
    };
    let binding = binding(selected)
        .ok_or_else(|| McpError::InvalidRequest("execution binding is unavailable".into()))?;
    if binding.availability != BindingAvailability::Available {
        return Err(McpError::InvalidRequest(
            "execution binding is unavailable".into(),
        ));
    }
    if !binding
        .capabilities
        .iter()
        .any(|value| value == capability_id)
    {
        return Err(McpError::InvalidRequest(
            "execution binding is incompatible with capability".into(),
        ));
    }
    Ok(Some(binding))
}

fn capability_descriptor(
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
