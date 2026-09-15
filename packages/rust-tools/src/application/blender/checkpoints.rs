use super::{artifact_relative_path, artifacts, bridge, BlenderArtifactScope};
use crate::application::creative;
use crate::application::workspace::write_contained_bytes;
use crate::core::config::ServerConfig;
use crate::core::error::McpError;
use ring::digest::{Context, SHA256};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;
use uuid::Uuid;

const CHECKPOINT_MANIFEST_VERSION: u32 = 1;
const MAX_CHECKPOINT_LABEL_BYTES: usize = 128;

#[derive(Debug, Serialize, Deserialize)]
struct CheckpointManifest {
    schema_version: u32,
    checkpoint_id: String,
    project_id: String,
    owner: String,
    asset_id: String,
    scene_asset_id: String,
    relative_path: String,
    checksum_sha256: String,
    label: Option<String>,
}

pub async fn create(
    cwd: Option<&str>,
    config: &ServerConfig,
    owner: &str,
    project_id: &str,
    label: Option<&str>,
) -> Result<Value, McpError> {
    if let Some(label) = label {
        if label.is_empty()
            || label.len() > MAX_CHECKPOINT_LABEL_BYTES
            || label.chars().any(char::is_control)
        {
            return Err(McpError::InvalidRequest(
                "Blender checkpoint label exceeds allowed bounds".into(),
            ));
        }
    }
    let checkpoint_id = format!("checkpoint_{}", Uuid::new_v4().simple());
    let file_name = format!("{checkpoint_id}.blend");
    let scene_relative_path = artifact_relative_path(BlenderArtifactScope::Scene, &file_name)?;
    let scene_absolute_path = artifacts::preflight_output(cwd, config, &scene_relative_path)?;
    let relative_path = artifact_relative_path(BlenderArtifactScope::Checkpoint, &file_name)?;
    let _checkpoint_absolute_path = artifacts::preflight_output(cwd, config, &relative_path)?;
    let path_literal = artifacts::path_literal(&scene_absolute_path)?;
    let code = format!(
        r#"# MASIHAWAM_CHECKPOINT_CREATE
import bpy
_path = {path_literal}
bpy.ops.wm.save_as_mainfile(filepath=_path, check_existing=False, copy=True)
result = {{"saved": True}}
"#
    );
    let reply = bridge::execute(config, &code, true).await?;
    let scene_path = artifacts::verify_output(cwd, config, &scene_relative_path)?;
    let scene_asset_id = artifacts::register_generated_output(
        cwd,
        config,
        project_id,
        artifacts::GeneratedOutputRegistration {
            relative_path: &scene_relative_path,
            media_type: "application/x-blender",
            role: "blender_scene",
            artifact_kind: "blender_scene_snapshot",
            parent_asset_id: None,
        },
    )?;
    let checkpoint_bytes = fs::read(&scene_path)
        .map_err(|_| McpError::InvalidRequest("Blender scene snapshot cannot be read".into()))?;
    write_contained_bytes(&relative_path, cwd, &checkpoint_bytes, true, false, config)?;
    let checkpoint_asset_id = artifacts::register_generated_output(
        cwd,
        config,
        project_id,
        artifacts::GeneratedOutputRegistration {
            relative_path: &relative_path,
            media_type: "application/x-blender",
            role: "blender_checkpoint",
            artifact_kind: "blender_checkpoint",
            parent_asset_id: Some(scene_asset_id.clone()),
        },
    )?;
    let asset = creative::require_asset(cwd, config, project_id, &checkpoint_asset_id)?;
    let manifest = CheckpointManifest {
        schema_version: CHECKPOINT_MANIFEST_VERSION,
        checkpoint_id: checkpoint_id.clone(),
        project_id: project_id.to_owned(),
        owner: owner.to_owned(),
        asset_id: checkpoint_asset_id.clone(),
        scene_asset_id: scene_asset_id.clone(),
        relative_path: relative_path.clone(),
        checksum_sha256: asset.checksum_sha256.clone(),
        label: label.map(str::to_owned),
    };
    let manifest_value = serde_json::to_value(&manifest)
        .map_err(|_| McpError::Internal("Blender checkpoint state encoding failed".into()))?;
    creative::store_blender_checkpoint_state(
        cwd,
        config,
        project_id,
        &checkpoint_id,
        &manifest_value,
    )?;
    Ok(json!({
        "checkpoint_id":checkpoint_id,
        "asset_id":checkpoint_asset_id,
        "scene_asset_id":scene_asset_id,
        "scene_relative_path":scene_relative_path,
        "relative_path":relative_path,
        "label":label,
        "bridge_result":reply.result
    }))
}

pub async fn restore(
    cwd: Option<&str>,
    config: &ServerConfig,
    owner: &str,
    project_id: &str,
    checkpoint_id: &str,
) -> Result<Value, McpError> {
    creative::validate_id(checkpoint_id, "checkpoint_id")?;
    let manifest_value =
        creative::load_blender_checkpoint_state(cwd, config, project_id, checkpoint_id)?;
    let manifest: CheckpointManifest = serde_json::from_value(manifest_value)
        .map_err(|_| McpError::InvalidRequest("Blender checkpoint state is invalid".into()))?;
    if manifest.schema_version != CHECKPOINT_MANIFEST_VERSION
        || manifest.checkpoint_id != checkpoint_id
        || manifest.project_id != project_id
        || manifest.owner != owner
    {
        return Err(McpError::InvalidRequest(
            "Blender checkpoint identity does not match this owner/project".into(),
        ));
    }
    let expected_path = artifact_relative_path(
        BlenderArtifactScope::Checkpoint,
        &format!("{checkpoint_id}.blend"),
    )?;
    if manifest.relative_path != expected_path {
        return Err(McpError::InvalidRequest(
            "Blender checkpoint manifest path is invalid".into(),
        ));
    }
    let asset = creative::require_asset(cwd, config, project_id, &manifest.asset_id)?;
    if asset.relative_path != manifest.relative_path
        || asset.checksum_sha256 != manifest.checksum_sha256
        || asset.role != "blender_checkpoint"
        || asset.parent_asset_id.as_deref() != Some(manifest.scene_asset_id.as_str())
    {
        return Err(McpError::InvalidRequest(
            "Blender checkpoint asset lineage is invalid".into(),
        ));
    }
    let absolute_path = artifacts::verify_output(cwd, config, &manifest.relative_path)?;
    let actual_checksum = sha256_file(&absolute_path)?;
    if actual_checksum != manifest.checksum_sha256 {
        return Err(McpError::InvalidRequest(
            "Blender checkpoint file checksum does not match its relay manifest".into(),
        ));
    }
    let path_literal = artifacts::path_literal(&absolute_path)?;
    let code = format!(
        r#"# MASIHAWAM_CHECKPOINT_RESTORE
import bpy
_path = {path_literal}
bpy.ops.wm.open_mainfile(filepath=_path, load_ui=False, use_scripts=False)
result = {{"restored": True}}
"#
    );
    let reply = bridge::execute(config, &code, true).await?;
    Ok(json!({
        "checkpoint_id":checkpoint_id,
        "asset_id":manifest.asset_id,
        "relative_path":manifest.relative_path,
        "bridge_result":reply.result
    }))
}

fn sha256_file(path: &Path) -> Result<String, McpError> {
    let mut file = File::open(path)
        .map_err(|_| McpError::InvalidRequest("Blender checkpoint is inaccessible".into()))?;
    let mut context = Context::new(&SHA256);
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|_| McpError::InvalidRequest("Blender checkpoint cannot be read".into()))?;
        if read == 0 {
            break;
        }
        context.update(&buffer[..read]);
    }
    Ok(context
        .finish()
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}
