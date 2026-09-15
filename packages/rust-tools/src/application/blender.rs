use crate::core::config::ServerConfig;
use crate::core::error::McpError;
use serde::{Deserialize, Serialize};
use std::net::{Ipv4Addr, SocketAddrV4};
use std::path::{Component, Path};

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
