use super::{CreativeProject, MAX_PROJECTS_PER_WORKSPACE, MAX_REGISTER_ASSET_BYTES, STATE_PREFIX};
use crate::application::creative::contracts::{validate_id, CREATIVE_SCHEMA_VERSION};
use crate::core::config::ServerConfig;
use crate::core::error::McpError;
use crate::core::workspace_path::EntryKind;
use ring::digest::{Context, SHA256};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::json;
use std::io::Read;
use std::path::{Path, PathBuf};

const MAX_STATE_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct ProjectIndex {
    pub(super) schema_version: u32,
    pub(super) project_ids: Vec<String>,
}

pub(super) fn save_project(
    cwd: Option<&str>,
    config: &ServerConfig,
    project: &CreativeProject,
) -> Result<(), McpError> {
    project.validate()?;
    write_json(
        cwd,
        config,
        &project_path(&project.project_id),
        project,
        true,
    )
}

pub(super) fn read_project_index(
    cwd: Option<&str>,
    config: &ServerConfig,
) -> Result<ProjectIndex, McpError> {
    match read_json::<ProjectIndex>(cwd, config, index_path()) {
        Ok(index) => {
            if index.schema_version != CREATIVE_SCHEMA_VERSION
                || index.project_ids.len() > MAX_PROJECTS_PER_WORKSPACE
            {
                return Err(McpError::InvalidRequest(
                    "creative project index is invalid".into(),
                ));
            }
            for project_id in &index.project_ids {
                validate_id(project_id, "project_id")?;
            }
            Ok(index)
        }
        Err(McpError::InvalidRequest(message))
            if message == "path does not exist or is inaccessible" =>
        {
            Ok(ProjectIndex {
                schema_version: CREATIVE_SCHEMA_VERSION,
                project_ids: Vec::new(),
            })
        }
        Err(error) => Err(error),
    }
}

pub(super) fn state_exists(
    cwd: Option<&str>,
    config: &ServerConfig,
    path: &str,
) -> Result<bool, McpError> {
    match crate::application::workspace::read_contained_text(path, cwd, config, 1) {
        Ok(_) => Ok(true),
        Err(McpError::InvalidRequest(message))
            if message == "path does not exist or is inaccessible" =>
        {
            Ok(false)
        }
        Err(McpError::InvalidRequest(message))
            if message == "contained text file exceeds maximum" =>
        {
            Ok(true)
        }
        Err(error) => Err(error),
    }
}

pub(super) fn read_json<T: DeserializeOwned>(
    cwd: Option<&str>,
    config: &ServerConfig,
    path: &str,
) -> Result<T, McpError> {
    let text =
        crate::application::workspace::read_contained_text(path, cwd, config, MAX_STATE_BYTES)?;
    serde_json::from_str(&text)
        .map_err(|_| McpError::InvalidRequest("creative state file is invalid".into()))
}

pub(super) fn write_json<T: Serialize>(
    cwd: Option<&str>,
    config: &ServerConfig,
    path: &str,
    value: &T,
    overwrite: bool,
) -> Result<(), McpError> {
    let content = serde_json::to_string_pretty(value)
        .map_err(|_| McpError::Internal("creative state could not be serialized".into()))?;
    if content.len() > MAX_STATE_BYTES {
        return Err(McpError::InvalidRequest(
            "creative state exceeds maximum".into(),
        ));
    }
    let arguments = json!({
        "path": path,
        "cwd": cwd,
        "content": content,
        "create_parents": true,
        "overwrite": overwrite,
    });
    crate::application::workspace::file_write(&arguments, config)?;
    Ok(())
}

pub(super) struct ResolvedAssetPath {
    pub(super) absolute: PathBuf,
    pub(super) relative: String,
}

pub(super) fn resolve_asset_path(
    cwd: Option<&str>,
    config: &ServerConfig,
    path: &str,
) -> Result<ResolvedAssetPath, McpError> {
    config
        .ensure_workspaces_initialized()
        .map_err(|error| McpError::Internal(error.to_string()))?;
    let guard = config
        .workspaces
        .read()
        .map_err(|_| McpError::Internal("workspace lock poisoned".into()))?;
    let cwd_path = crate::core::workspace_path::resolve_contained_cwd_in_allowlist(&guard, cwd)?;
    let target = crate::core::workspace_path::resolve_existing_path_in_allowlist(
        &guard,
        cwd,
        path,
        EntryKind::File,
    )?;
    if !target.starts_with(&cwd_path) {
        return Err(McpError::InvalidRequest(
            "creative asset must remain beneath the selected project cwd".into(),
        ));
    }
    let root = guard.containing_root(&target).ok_or_else(|| {
        McpError::InvalidRequest("creative asset is outside authorized workspace roots".into())
    })?;
    crate::application::workspace::reject_protected_target(root, &target)?;
    let relative = target
        .strip_prefix(&cwd_path)
        .map_err(|_| McpError::InvalidRequest("creative asset path is invalid".into()))?
        .to_string_lossy()
        .into_owned();
    if relative.is_empty() || Path::new(&relative).is_absolute() {
        return Err(McpError::InvalidRequest(
            "creative asset path is invalid".into(),
        ));
    }
    Ok(ResolvedAssetPath {
        absolute: target,
        relative,
    })
}

pub(super) fn sha256_file(path: &Path) -> Result<String, McpError> {
    let mut file = std::fs::File::open(path)
        .map_err(|_| McpError::InvalidRequest("asset file is inaccessible".into()))?;
    let mut context = Context::new(&SHA256);
    let mut total = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|_| McpError::InvalidRequest("asset file is inaccessible".into()))?;
        if read == 0 {
            break;
        }
        total = total.saturating_add(read as u64);
        if total > MAX_REGISTER_ASSET_BYTES {
            return Err(McpError::InvalidRequest(
                "asset file exceeds registration maximum".into(),
            ));
        }
        context.update(&buffer[..read]);
    }
    let digest = context.finish();
    let mut output = String::with_capacity(64);
    for byte in digest.as_ref() {
        output.push_str(&format!("{byte:02x}"));
    }
    Ok(output)
}

pub(super) fn index_path() -> &'static str {
    ".masihawam/creative/index.json"
}

pub(super) fn project_path(project_id: &str) -> String {
    format!("{STATE_PREFIX}/projects/{project_id}/project.json")
}

pub(super) fn graph_path(project_id: &str, graph_id: &str) -> String {
    format!("{STATE_PREFIX}/projects/{project_id}/graphs/{graph_id}.json")
}

pub(super) fn job_path(project_id: &str, job_id: &str) -> String {
    format!("{STATE_PREFIX}/projects/{project_id}/jobs/{job_id}.json")
}
