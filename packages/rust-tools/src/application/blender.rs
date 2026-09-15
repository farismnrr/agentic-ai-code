use crate::core::config::ServerConfig;
use crate::core::error::McpError;
use crate::interfaces::mcp::{ToolCallResult, ToolResultContent};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::net::{Ipv4Addr, SocketAddrV4};
use std::path::{Component, Path, PathBuf};

mod bridge;
mod knowledge;
mod preview;
mod reads;
mod session;

pub const BLENDER_LAB_PROTOCOL: &str = "blender_lab_json_nul_v1";
pub const DEFAULT_BLENDER_LAB_PORT: u16 = 9876;
pub const MAX_BLENDER_REQUEST_BYTES: usize = 1024 * 1024;
pub const MAX_BLENDER_RESPONSE_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_BLENDER_PYTHON_BYTES: usize = 256 * 1024;
pub const REVIEWED_BLENDER_PATH_NAMES: &[&str] = &["blender", "blender.exe"];

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BlenderSessionOwnership {
    External,
    RelayOwned,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BlenderSessionState {
    Closed,
    Starting,
    Ready,
    Incompatible,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlenderSessionStatus {
    pub state: BlenderSessionState,
    #[serde(default)]
    pub ownership: Option<BlenderSessionOwnership>,
    pub protocol: String,
    pub loopback_port: u16,
    #[serde(default)]
    pub project_root: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct BlenderProjectLayout {
    pub root: String,
    pub scenes: String,
    pub assets: String,
    pub references: String,
    pub renders_preview: String,
    pub renders_final: String,
    pub animations: String,
    pub exports: String,
    pub checkpoints: String,
    pub tmp: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlenderArtifactScope {
    Scene,
    Asset,
    Reference,
    RenderPreview,
    RenderFinal,
    Animation,
    Export,
    Checkpoint,
    Temp,
}

pub fn project_layout() -> BlenderProjectLayout {
    BlenderProjectLayout {
        root: "blender".into(),
        scenes: "blender/scenes".into(),
        assets: "blender/assets".into(),
        references: "blender/references".into(),
        renders_preview: "blender/renders/preview".into(),
        renders_final: "blender/renders/final".into(),
        animations: "blender/animations".into(),
        exports: "blender/exports".into(),
        checkpoints: "blender/checkpoints".into(),
        tmp: "blender/tmp".into(),
    }
}

pub fn bridge_address(config: &ServerConfig) -> SocketAddrV4 {
    SocketAddrV4::new(Ipv4Addr::LOCALHOST, config.blender_bridge_port)
}

pub(super) fn resolve_project_root(
    cwd: Option<&str>,
    config: &ServerConfig,
) -> Result<PathBuf, McpError> {
    let root = config
        .resolved_execution_root()
        .map_err(|_| McpError::InvalidRequest("execution root is unavailable".into()))?;
    crate::core::workspace_path::resolve_contained_cwd(&root, cwd)
}

pub fn artifact_relative_path(
    scope: BlenderArtifactScope,
    file_name: &str,
) -> Result<String, McpError> {
    validate_leaf_name(file_name)?;
    let layout = project_layout();
    let base = match scope {
        BlenderArtifactScope::Scene => layout.scenes,
        BlenderArtifactScope::Asset => layout.assets,
        BlenderArtifactScope::Reference => layout.references,
        BlenderArtifactScope::RenderPreview => layout.renders_preview,
        BlenderArtifactScope::RenderFinal => layout.renders_final,
        BlenderArtifactScope::Animation => layout.animations,
        BlenderArtifactScope::Export => layout.exports,
        BlenderArtifactScope::Checkpoint => layout.checkpoints,
        BlenderArtifactScope::Temp => layout.tmp,
    };
    Ok(format!("{base}/{file_name}"))
}

pub fn validate_blender_relative_path(path: &str) -> Result<(), McpError> {
    if path.is_empty() || path.len() > 4_096 {
        return Err(McpError::InvalidRequest(
            "Blender project path exceeds allowed bounds".into(),
        ));
    }
    let path = Path::new(path);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
        || !path.starts_with("blender")
    {
        return Err(McpError::InvalidRequest(
            "Blender production path must remain beneath the project blender subtree".into(),
        ));
    }
    let allowed = [
        Path::new("blender/scenes"),
        Path::new("blender/assets"),
        Path::new("blender/references"),
        Path::new("blender/renders/preview"),
        Path::new("blender/renders/final"),
        Path::new("blender/animations"),
        Path::new("blender/exports"),
        Path::new("blender/checkpoints"),
        Path::new("blender/tmp"),
    ];
    if !allowed.iter().any(|root| path.starts_with(root)) {
        return Err(McpError::InvalidRequest(
            "Blender production path is outside the canonical project layout".into(),
        ));
    }
    Ok(())
}

fn validate_leaf_name(file_name: &str) -> Result<(), McpError> {
    if file_name.is_empty()
        || file_name.len() > 180
        || file_name.chars().any(char::is_control)
        || file_name.starts_with('.')
        || Path::new(file_name).components().count() != 1
        || file_name.contains(['/', '\\'])
    {
        return Err(McpError::InvalidRequest(
            "Blender artifact name must be one bounded project-local file name".into(),
        ));
    }
    Ok(())
}

pub async fn dispatch_tool(
    tool_name: &str,
    arguments: &Value,
    config: &ServerConfig,
    owner: &str,
) -> Result<Option<ToolCallResult>, McpError> {
    if !tool_name.starts_with("blender_") {
        return Ok(None);
    }
    let cwd = required_str(arguments, "cwd")?;
    let project_id = required_str(arguments, "project_id")?;
    crate::application::creative::require_project(Some(cwd), config, project_id)?;
    match tool_name {
        "blender_session" => {
            let action = required_str(arguments, "action")?;
            let status = match action {
                "status" => session::status(Some(cwd), config, owner, project_id).await?,
                "start" => session::start(Some(cwd), config, owner, project_id).await?,
                "stop" => session::stop(Some(cwd), config, owner, project_id).await?,
                _ => {
                    return Err(McpError::InvalidRequest(
                        "unsupported Blender session action".into(),
                    ))
                }
            };
            complete(json!({"session": status})).map(Some)
        }
        "blender_inspect" => {
            let scope = required_str(arguments, "scope")?;
            let target = arguments.get("target").and_then(Value::as_str);
            let detail = arguments
                .get("detail")
                .and_then(Value::as_str)
                .unwrap_or("standard");
            complete(json!({"inspection": reads::inspect(config, scope, target, detail).await?}))
                .map(Some)
        }
        "blender_python_api_docs" => {
            let query = required_str(arguments, "query")?;
            let module = arguments.get("module").and_then(Value::as_str);
            let limit = arguments.get("limit").and_then(Value::as_u64).unwrap_or(8) as usize;
            complete(json!({"docs": knowledge::lookup(config, query, module, limit).await?}))
                .map(Some)
        }
        "blender_screenshot" => {
            let source = arguments
                .get("source")
                .and_then(Value::as_str)
                .unwrap_or("viewport");
            let width = arguments
                .get("width")
                .and_then(Value::as_u64)
                .unwrap_or(1024) as u32;
            let height = arguments
                .get("height")
                .and_then(Value::as_u64)
                .unwrap_or(1024) as u32;
            let save_name = arguments.get("save_name").and_then(Value::as_str);
            complete(
                preview::screenshot(Some(cwd), config, source, width, height, save_name).await?,
            )
            .map(Some)
        }
        "blender_animation_preview" => {
            let start_frame = required_i64(arguments, "start_frame")? as i32;
            let end_frame = required_i64(arguments, "end_frame")? as i32;
            let request = preview::AnimationPreviewRequest {
                start_frame,
                end_frame,
                step: arguments.get("step").and_then(Value::as_u64).unwrap_or(1) as u32,
                max_frames: arguments
                    .get("max_frames")
                    .and_then(Value::as_u64)
                    .unwrap_or(48) as usize,
                width: arguments
                    .get("width")
                    .and_then(Value::as_u64)
                    .unwrap_or(640) as u32,
                height: arguments
                    .get("height")
                    .and_then(Value::as_u64)
                    .unwrap_or(360) as u32,
                save_name: arguments.get("save_name").and_then(Value::as_str),
            };
            complete(preview::animation_preview(Some(cwd), config, request).await?).map(Some)
        }
        _ => Ok(None),
    }
}

fn required_str<'a>(arguments: &'a Value, field: &str) -> Result<&'a str, McpError> {
    arguments
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| McpError::InvalidRequest(format!("Blender {field} is required")))
}

fn required_i64(arguments: &Value, field: &str) -> Result<i64, McpError> {
    arguments
        .get(field)
        .and_then(Value::as_i64)
        .ok_or_else(|| McpError::InvalidRequest(format!("Blender {field} is required")))
}

fn complete(value: Value) -> Result<ToolCallResult, McpError> {
    let text = serde_json::to_string(&value)
        .map_err(|_| McpError::Internal("Blender result could not be serialized".into()))?;
    Ok(ToolCallResult::complete(vec![ToolResultContent {
        kind: "text",
        text,
    }]))
}
