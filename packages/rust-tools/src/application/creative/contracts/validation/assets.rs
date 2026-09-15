use super::validate_text;
use crate::application::creative::contracts::{
    AssetMetadata, AssetRecord, AssetSource, AssetSurface, CreativeProject,
    MAX_ELEMENT_DEPENDENCIES,
};
use crate::core::error::McpError;
use std::collections::HashSet;
use std::path::{Component, Path};

pub(super) fn validate_asset(
    asset: &AssetRecord,
    project: &CreativeProject,
) -> Result<(), McpError> {
    super::validate_id(&asset.asset_id, "asset_id")?;
    validate_asset_media_type(&asset.media_type)?;
    validate_text(&asset.role, 1, 128, "asset role")?;
    validate_text(&asset.relative_path, 1, 4_096, "asset relative path")?;
    let relative_path = Path::new(&asset.relative_path);
    if relative_path.is_absolute()
        || relative_path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(McpError::InvalidRequest(
            "asset relative path must contain only normal relative components".into(),
        ));
    }
    if asset.checksum_sha256.len() != 64
        || !asset
            .checksum_sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(McpError::InvalidRequest(
            "asset checksum must be a SHA-256 hex digest".into(),
        ));
    }
    validate_asset_metadata(&asset.metadata)?;
    if asset.source == AssetSource::ManualImport
        && asset.source_surface != AssetSurface::ManualImport
    {
        return Err(McpError::InvalidRequest(
            "manual-import asset source and source surface must agree".into(),
        ));
    }
    if let Some(job_id) = asset.job_id.as_deref() {
        super::validate_id(job_id, "asset job id")?;
        if !project.job_ids.iter().any(|value| value == job_id) {
            return Err(McpError::InvalidRequest(
                "asset job lineage references an unknown project job".into(),
            ));
        }
    }
    if let Some(parent_id) = asset.parent_asset_id.as_deref() {
        if parent_id == asset.asset_id || project.asset(parent_id).is_none() {
            return Err(McpError::InvalidRequest(
                "asset parent must reference another project asset".into(),
            ));
        }
    }
    if let Some(element_id) = asset.element_id.as_deref() {
        if project.element(element_id).is_none() {
            return Err(McpError::InvalidRequest(
                "asset element binding references an unknown element".into(),
            ));
        }
    }
    if asset.dependency_element_ids.len() > MAX_ELEMENT_DEPENDENCIES {
        return Err(McpError::InvalidRequest(
            "asset element dependency count exceeds maximum".into(),
        ));
    }
    let mut dependencies = HashSet::new();
    for dependency_id in &asset.dependency_element_ids {
        super::validate_id(dependency_id, "asset dependency element id")?;
        if !dependencies.insert(dependency_id.as_str()) || project.element(dependency_id).is_none()
        {
            return Err(McpError::InvalidRequest(
                "asset element dependency is unknown or duplicated".into(),
            ));
        }
    }
    Ok(())
}

pub(super) fn validate_asset_lineage(project: &CreativeProject) -> Result<(), McpError> {
    for asset in &project.assets {
        let mut seen = HashSet::new();
        let mut current = asset.parent_asset_id.as_deref();
        while let Some(parent_id) = current {
            if !seen.insert(parent_id) {
                return Err(McpError::InvalidRequest(
                    "asset parent lineage must be acyclic".into(),
                ));
            }
            current = project
                .asset(parent_id)
                .and_then(|parent| parent.parent_asset_id.as_deref());
        }
    }
    Ok(())
}

fn validate_asset_media_type(value: &str) -> Result<(), McpError> {
    let allowed = value.starts_with("image/")
        || value.starts_with("video/")
        || value.starts_with("audio/")
        || value.starts_with("model/")
        || matches!(value, "application/octet-stream" | "application/x-blender");
    if value.is_empty() || value.len() > 128 || value.chars().any(char::is_control) || !allowed {
        return Err(McpError::InvalidRequest(
            "creative asset media type is unsupported".into(),
        ));
    }
    Ok(())
}

fn validate_asset_metadata(metadata: &AssetMetadata) -> Result<(), McpError> {
    if metadata
        .width
        .is_some_and(|value| value == 0 || value > 16_384)
        || metadata
            .height
            .is_some_and(|value| value == 0 || value > 16_384)
        || metadata
            .duration_ms
            .is_some_and(|value| value == 0 || value > 86_400_000)
        || metadata
            .frame_rate
            .is_some_and(|value| !value.is_finite() || !(1.0..=240.0).contains(&value))
        || metadata
            .sample_rate_hz
            .is_some_and(|value| !(8_000..=384_000).contains(&value))
        || metadata
            .channels
            .is_some_and(|value| value == 0 || value > 32)
    {
        return Err(McpError::InvalidRequest(
            "creative asset media metadata is outside allowed bounds".into(),
        ));
    }
    if let Some(language) = metadata.language.as_deref() {
        validate_text(language, 1, 32, "asset language")?;
    }
    if let Some(kind) = metadata.artifact_kind.as_deref() {
        validate_text(kind, 1, 64, "asset artifact kind")?;
    }
    if let Some(version) = metadata.artifact_version.as_deref() {
        validate_text(version, 1, 128, "asset artifact version")?;
    }
    if let Some(notes) = metadata.license_notes.as_deref() {
        validate_text(notes, 1, 1024, "asset license notes")?;
    }
    Ok(())
}
