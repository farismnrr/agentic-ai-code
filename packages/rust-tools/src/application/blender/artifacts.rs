use super::{
    artifact_relative_path, resolve_project_root, session, validate_blender_relative_path,
    BlenderArtifactScope,
};
use crate::application::creative::{
    self, AssetMetadata, AssetRecord, AssetRegistrationInput, AssetSource, AssetState, AssetSurface,
};
use crate::application::workspace::{write_contained_bytes, MAX_INTERNAL_BINARY_WRITE_BYTES};
use crate::core::config::ServerConfig;
use crate::core::error::McpError;
use crate::core::workspace_path::{resolve_existing_path, EntryKind};
use ring::digest::{Context, SHA256};
use std::fs;
use std::path::{Path, PathBuf};

pub struct MaterializedAsset {
    pub source: AssetRecord,
    pub asset_id: String,
    pub relative_path: String,
    pub absolute_path: PathBuf,
}

pub fn preflight_output(
    cwd: Option<&str>,
    config: &ServerConfig,
    relative_path: &str,
) -> Result<PathBuf, McpError> {
    validate_blender_relative_path(relative_path)?;
    session::ensure_project_layout(cwd, config)?;
    let project_root = resolve_project_root(cwd, config)?;
    let blender_root = fs::canonicalize(project_root.join("blender"))
        .map_err(|_| McpError::InvalidRequest("Blender project layout is unavailable".into()))?;
    let absolute = project_root.join(relative_path);
    let parent = absolute
        .parent()
        .ok_or_else(|| McpError::InvalidRequest("Blender artifact target has no parent".into()))?;
    let canonical_parent = fs::canonicalize(parent)
        .map_err(|_| McpError::InvalidRequest("Blender artifact parent is unavailable".into()))?;
    if !canonical_parent.starts_with(&blender_root) {
        return Err(McpError::InvalidRequest(
            "Blender artifact target escapes the project Blender subtree".into(),
        ));
    }
    match fs::symlink_metadata(&absolute) {
        Ok(_) => Err(McpError::InvalidRequest(
            "Blender artifact target already exists".into(),
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(absolute),
        Err(_) => Err(McpError::InvalidRequest(
            "Blender artifact target cannot be inspected".into(),
        )),
    }
}

pub fn verify_output(
    cwd: Option<&str>,
    config: &ServerConfig,
    relative_path: &str,
) -> Result<PathBuf, McpError> {
    validate_blender_relative_path(relative_path)?;
    let project_root = resolve_project_root(cwd, config)?;
    let blender_root = fs::canonicalize(project_root.join("blender"))
        .map_err(|_| McpError::InvalidRequest("Blender project layout is unavailable".into()))?;
    let absolute = project_root.join(relative_path);
    let metadata = fs::symlink_metadata(&absolute)
        .map_err(|_| McpError::InvalidRequest("Blender artifact output is missing".into()))?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.len() as usize > MAX_INTERNAL_BINARY_WRITE_BYTES
    {
        return Err(McpError::InvalidRequest(
            "Blender artifact output is not a bounded regular file".into(),
        ));
    }
    let canonical = fs::canonicalize(&absolute)
        .map_err(|_| McpError::InvalidRequest("Blender artifact output is inaccessible".into()))?;
    if !canonical.starts_with(&blender_root) {
        return Err(McpError::InvalidRequest(
            "Blender artifact output escaped the project Blender subtree".into(),
        ));
    }
    Ok(canonical)
}

pub fn materialize_asset(
    cwd: Option<&str>,
    config: &ServerConfig,
    project_id: &str,
    asset_id: &str,
    purpose: &str,
    target_name: Option<&str>,
) -> Result<MaterializedAsset, McpError> {
    let source = creative::require_asset(cwd, config, project_id, asset_id)?;
    let execution_root = config
        .resolved_execution_root()
        .map_err(|_| McpError::InvalidRequest("execution root is unavailable".into()))?;
    let source_path =
        resolve_existing_path(&execution_root, cwd, &source.relative_path, EntryKind::File)?;
    let source_metadata = fs::metadata(&source_path)
        .map_err(|_| McpError::InvalidRequest("creative asset source is inaccessible".into()))?;
    if source_metadata.len() as usize > MAX_INTERNAL_BINARY_WRITE_BYTES {
        return Err(McpError::InvalidRequest(
            "creative asset exceeds Blender materialization maximum".into(),
        ));
    }
    let default_name = source_path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| McpError::InvalidRequest("creative asset name is not UTF-8".into()))?;
    let file_name = target_name.unwrap_or(default_name);
    let scope = match purpose {
        "reference" => BlenderArtifactScope::Reference,
        "asset" => BlenderArtifactScope::Asset,
        _ => {
            return Err(McpError::InvalidRequest(
                "Blender asset purpose must be reference or asset".into(),
            ))
        }
    };
    let relative_path = artifact_relative_path(scope, file_name)?;
    let bytes = fs::read(&source_path)
        .map_err(|_| McpError::InvalidRequest("creative asset source cannot be read".into()))?;
    if bytes.len() as u64 != source.bytes || sha256_bytes(&bytes) != source.checksum_sha256 {
        return Err(McpError::InvalidRequest(
            "creative asset bytes no longer match durable provenance".into(),
        ));
    }
    let absolute_path = match preflight_output(cwd, config, &relative_path) {
        Ok(_) => {
            write_contained_bytes(&relative_path, cwd, &bytes, true, false, config)?;
            verify_output(cwd, config, &relative_path)?
        }
        Err(McpError::InvalidRequest(message))
            if message == "Blender artifact target already exists" =>
        {
            let existing_path = verify_output(cwd, config, &relative_path)?;
            let existing_bytes = fs::read(&existing_path).map_err(|_| {
                McpError::InvalidRequest("Blender materialized asset is inaccessible".into())
            })?;
            if existing_bytes.len() as u64 != source.bytes
                || sha256_bytes(&existing_bytes) != source.checksum_sha256
            {
                return Err(McpError::InvalidRequest(
                    "Blender artifact target already exists with different bytes".into(),
                ));
            }
            if let Some(existing_asset) = creative::find_blender_materialized_asset(
                cwd,
                config,
                project_id,
                &source.asset_id,
                &relative_path,
            )? {
                return Ok(MaterializedAsset {
                    source,
                    asset_id: existing_asset.asset_id,
                    relative_path,
                    absolute_path: existing_path,
                });
            }
            existing_path
        }
        Err(error) => return Err(error),
    };

    let mut metadata = source.metadata.clone();
    metadata.artifact_kind = Some("blender_materialized".into());
    let materialized_id = creative::register_internal_asset(
        cwd,
        config,
        project_id,
        AssetRegistrationInput {
            path: relative_path.clone(),
            media_type: source.media_type.clone(),
            role: format!("blender_{purpose}"),
            source: AssetSource::BlenderMaterialized,
            source_surface: AssetSurface::Blender,
            state: AssetState::Candidate,
            job_id: None,
            parent_asset_id: Some(source.asset_id.clone()),
            element_id: source.element_id.clone(),
            dependency_element_ids: source.dependency_element_ids.clone(),
            metadata,
        },
    )?;
    Ok(MaterializedAsset {
        source,
        asset_id: materialized_id,
        relative_path,
        absolute_path,
    })
}

pub struct GeneratedOutputRegistration<'a> {
    pub relative_path: &'a str,
    pub media_type: &'a str,
    pub role: &'a str,
    pub artifact_kind: &'a str,
    pub parent_asset_id: Option<String>,
}

pub fn register_generated_output(
    cwd: Option<&str>,
    config: &ServerConfig,
    project_id: &str,
    registration: GeneratedOutputRegistration<'_>,
) -> Result<String, McpError> {
    verify_output(cwd, config, registration.relative_path)?;
    creative::register_internal_asset(
        cwd,
        config,
        project_id,
        AssetRegistrationInput {
            path: registration.relative_path.to_owned(),
            media_type: registration.media_type.to_owned(),
            role: registration.role.to_owned(),
            source: AssetSource::GeneratedAsset,
            source_surface: AssetSurface::Blender,
            state: AssetState::Candidate,
            job_id: None,
            parent_asset_id: registration.parent_asset_id,
            element_id: None,
            dependency_element_ids: Vec::new(),
            metadata: AssetMetadata {
                artifact_kind: Some(registration.artifact_kind.to_owned()),
                ..AssetMetadata::default()
            },
        },
    )
}

fn sha256_bytes(bytes: &[u8]) -> String {
    let mut context = Context::new(&SHA256);
    context.update(bytes);
    context
        .finish()
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub fn path_literal(path: &Path) -> Result<String, McpError> {
    let value = path
        .to_str()
        .ok_or_else(|| McpError::InvalidRequest("Blender artifact path is not UTF-8".into()))?;
    serde_json::to_string(value)
        .map_err(|_| McpError::Internal("Blender artifact path could not be encoded".into()))
}
