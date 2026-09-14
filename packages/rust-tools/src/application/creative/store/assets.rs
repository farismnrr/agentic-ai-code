use super::io::{resolve_asset_path, save_project, sha256_file};
use super::support::{new_id, validate_media_type};
use super::{load_project, now_ms, MAX_REGISTER_ASSET_BYTES, STATE_PREFIX};
use crate::application::creative::contracts::{
    validate_id, AssetMetadata, AssetRecord, AssetSource, AssetState, AssetSurface,
    CreativeProject, MAX_PROJECT_ASSETS,
};
use crate::core::config::ServerConfig;
use crate::core::error::McpError;

pub struct AssetRegistrationInput {
    pub path: String,
    pub media_type: String,
    pub role: String,
    pub source: AssetSource,
    pub source_surface: AssetSurface,
    pub state: AssetState,
    pub job_id: Option<String>,
    pub parent_asset_id: Option<String>,
    pub element_id: Option<String>,
    pub metadata: AssetMetadata,
}

#[derive(Default)]
pub struct AssetSearch {
    pub media_type: Option<String>,
    pub role: Option<String>,
    pub source: Option<AssetSource>,
    pub source_surface: Option<AssetSurface>,
    pub state: Option<AssetState>,
    pub job_id: Option<String>,
    pub element_id: Option<String>,
    pub parent_asset_id: Option<String>,
    pub created_after_ms: Option<u128>,
    pub created_before_ms: Option<u128>,
    pub limit: usize,
}

pub fn promote_asset(
    cwd: Option<&str>,
    config: &ServerConfig,
    project_id: &str,
    asset_id: &str,
) -> Result<CreativeProject, McpError> {
    validate_id(asset_id, "asset_id")?;
    let mut project = load_project(cwd, config, project_id)?;
    let asset = project
        .assets
        .iter_mut()
        .find(|value| value.asset_id == asset_id)
        .ok_or_else(|| McpError::InvalidRequest("unknown creative asset".into()))?;
    asset.state = AssetState::Accepted;
    project.updated_at_ms = now_ms();
    save_project(cwd, config, &project)?;
    Ok(project)
}

pub fn register_asset(
    cwd: Option<&str>,
    config: &ServerConfig,
    project_id: &str,
    input: AssetRegistrationInput,
) -> Result<(CreativeProject, String), McpError> {
    validate_media_type(&input.media_type)?;
    let mut project = load_project(cwd, config, project_id)?;
    if project.assets.len() >= MAX_PROJECT_ASSETS {
        return Err(McpError::InvalidRequest(
            "creative asset capacity reached".into(),
        ));
    }
    if let Some(job_id) = input.job_id.as_deref() {
        validate_id(job_id, "asset job id")?;
        if !project.job_ids.iter().any(|value| value == job_id) {
            return Err(McpError::InvalidRequest("unknown asset job lineage".into()));
        }
    }
    if let Some(parent) = input.parent_asset_id.as_deref() {
        if project.asset(parent).is_none() {
            return Err(McpError::InvalidRequest("unknown parent asset".into()));
        }
    }
    if let Some(element) = input.element_id.as_deref() {
        if project.element(element).is_none() {
            return Err(McpError::InvalidRequest("unknown asset element".into()));
        }
    }

    let resolved = resolve_asset_path(cwd, config, &input.path)?;
    if resolved.relative == STATE_PREFIX
        || resolved.relative.starts_with(&format!("{STATE_PREFIX}/"))
    {
        return Err(McpError::InvalidRequest(
            "creative state files cannot be registered as media assets".into(),
        ));
    }
    let metadata = std::fs::metadata(&resolved.absolute)
        .map_err(|_| McpError::InvalidRequest("asset file is inaccessible".into()))?;
    if metadata.len() > MAX_REGISTER_ASSET_BYTES {
        return Err(McpError::InvalidRequest(
            "asset file exceeds registration maximum".into(),
        ));
    }
    let checksum = sha256_file(&resolved.absolute)?;
    let asset_id = new_id("asset");
    project.assets.push(AssetRecord {
        asset_id: asset_id.clone(),
        media_type: input.media_type,
        role: input.role,
        relative_path: resolved.relative,
        checksum_sha256: checksum,
        bytes: metadata.len(),
        source: input.source,
        source_surface: input.source_surface,
        state: input.state,
        job_id: input.job_id,
        parent_asset_id: input.parent_asset_id,
        element_id: input.element_id,
        metadata: input.metadata,
        created_at_ms: now_ms(),
    });
    project.updated_at_ms = now_ms();
    save_project(cwd, config, &project)?;
    Ok((project, asset_id))
}

pub fn search_assets(
    cwd: Option<&str>,
    config: &ServerConfig,
    project_id: &str,
    query: AssetSearch,
) -> Result<Vec<AssetRecord>, McpError> {
    if let Some(media_type) = query.media_type.as_deref() {
        validate_media_type(media_type)?;
    }
    if let Some(role) = query.role.as_deref() {
        if role.is_empty() || role.len() > 128 || role.chars().any(char::is_control) {
            return Err(McpError::InvalidRequest(
                "creative asset role filter exceeds allowed bounds".into(),
            ));
        }
    }
    for (value, field) in [
        (query.job_id.as_deref(), "asset job id"),
        (query.element_id.as_deref(), "element_id"),
        (query.parent_asset_id.as_deref(), "parent_asset_id"),
    ] {
        if let Some(value) = value {
            validate_id(value, field)?;
        }
    }
    if query
        .created_after_ms
        .zip(query.created_before_ms)
        .is_some_and(|(after, before)| after > before)
    {
        return Err(McpError::InvalidRequest(
            "creative asset time range is invalid".into(),
        ));
    }
    let project = load_project(cwd, config, project_id)?;
    let limit = query.limit.clamp(1, 200);
    let mut assets = project
        .assets
        .into_iter()
        .filter(|asset| {
            query
                .media_type
                .as_deref()
                .is_none_or(|value| asset.media_type == value)
                && query
                    .role
                    .as_deref()
                    .is_none_or(|value| asset.role == value)
                && query
                    .source
                    .as_ref()
                    .is_none_or(|value| &asset.source == value)
                && query
                    .source_surface
                    .as_ref()
                    .is_none_or(|value| &asset.source_surface == value)
                && query
                    .state
                    .as_ref()
                    .is_none_or(|value| &asset.state == value)
                && query
                    .job_id
                    .as_deref()
                    .is_none_or(|value| asset.job_id.as_deref() == Some(value))
                && query
                    .element_id
                    .as_deref()
                    .is_none_or(|value| asset.element_id.as_deref() == Some(value))
                && query
                    .parent_asset_id
                    .as_deref()
                    .is_none_or(|value| asset.parent_asset_id.as_deref() == Some(value))
                && query
                    .created_after_ms
                    .is_none_or(|value| asset.created_at_ms >= value)
                && query
                    .created_before_ms
                    .is_none_or(|value| asset.created_at_ms <= value)
        })
        .collect::<Vec<_>>();
    assets.sort_by_key(|asset| std::cmp::Reverse(asset.created_at_ms));
    assets.truncate(limit);
    Ok(assets)
}
