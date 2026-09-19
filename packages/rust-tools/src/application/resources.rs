//! Bounded, read-only MCP resources for the verified repository.

use crate::core::{config::ServerConfig, error::McpError};
use crate::interfaces::mcp::resources::{Resource, ResourceContent};
use base64::{
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
    Engine as _,
};
use ring::digest::{Context, SHA256};
use serde_json::json;
use std::{
    fs,
    io::Cursor,
    path::{Path, PathBuf},
    process::Command,
};

const MIME: &str = "text/plain; charset=utf-8";
const MAX_RESOURCE_BYTES: usize = 64 * 1024;
const MAX_STATUS_BYTES: usize = 16 * 1024;
const MAX_MEDIA_RESOURCE_BYTES: usize = 16 * 1024 * 1024;
const MAX_MEDIA_RESOURCE_DIMENSION: u32 = 4096;
pub const RESOURCE_NAMES: [&str; 4] = ["manifest", "agent-guidance", "status", "head"];
const BLENDER_RESOURCE_NAME: &str = "blender-capability";

pub fn list(config: &ServerConfig) -> Result<Vec<Resource>, McpError> {
    let (_, id) = repository(config)?;
    Ok(resource_names(config)
        .into_iter()
        .map(|name| Resource {
            uri: uri(&id, name),
            name: name.to_owned(),
            description: match name {
                "manifest" => "Bounded repository identity and capability metadata.",
                "agent-guidance" => "Approved AGENTS.md and resource-index guidance.",
                "status" => "Bounded non-mutating Git workspace status.",
                "head" => "Current verified Git HEAD and ref metadata.",
                BLENDER_RESOURCE_NAME => "Enabled Blender production capability, contained project layout, and structured-first routing guidance.",
                _ => "Repository resource.",
            }
            .to_owned(),
            mime_type: MIME,
        })
        .collect())
}

pub fn read(config: &ServerConfig, requested: &str) -> Result<ResourceContent, McpError> {
    if requested.starts_with("creative://asset/") {
        return read_creative_asset(config, requested);
    }
    let (root, id) = repository(config)?;
    let Some((resource_id, name)) = parse_uri(requested) else {
        return Err(unknown());
    };
    let names = resource_names(config);
    if resource_id != id || !names.contains(&name) {
        return Err(unknown());
    }
    let text = match name {
        "manifest" => {
            let mut capabilities = vec![
                "workspace-read",
                "workspace-write",
                "git-read",
                "lsp",
                "mcp-tools",
            ];
            if blender_enabled(config) {
                capabilities.push("blender");
            }
            json!({ "repository": id, "root": "verified-execution-root", "markers": ["Cargo.toml", "package.json"], "resources": names, "capabilities": capabilities }).to_string()
        }
        "agent-guidance" => guidance(&root)?,
        "status" => git_text(&root, &["status", "--short", "--branch"], MAX_STATUS_BYTES)?,
        "head" => git_text(&root, &["rev-parse", "--verify", "HEAD"], MAX_STATUS_BYTES)?,
        BLENDER_RESOURCE_NAME => blender_capability(),
        _ => unreachable!(),
    };
    Ok(ResourceContent {
        uri: requested.to_owned(),
        text: Some(bounded(text, MAX_RESOURCE_BYTES)?),
        blob: None,
        mime_type: MIME.to_owned(),
    })
}

pub(crate) fn creative_asset_resource_uri(
    cwd: Option<&str>,
    config: &ServerConfig,
    project_id: &str,
    asset_id: &str,
) -> Result<String, McpError> {
    crate::application::creative::validate_id(project_id, "project_id")?;
    crate::application::creative::validate_id(asset_id, "asset_id")?;
    let fallback;
    let cwd = match cwd {
        Some(value) => value,
        None => {
            fallback = config
                .resolved_execution_root()
                .map_err(|_| {
                    McpError::InvalidRequest(
                        "creative resource project context is unavailable".into(),
                    )
                })?
                .to_string_lossy()
                .into_owned();
            &fallback
        }
    };
    if cwd.is_empty() || cwd.len() > 4096 || cwd.chars().any(char::is_control) {
        return Err(McpError::InvalidRequest(
            "creative resource project context is invalid".into(),
        ));
    }
    let context = URL_SAFE_NO_PAD.encode(cwd.as_bytes());
    Ok(format!(
        "creative://asset/{context}/{project_id}/{asset_id}"
    ))
}

fn read_creative_asset(
    config: &ServerConfig,
    requested: &str,
) -> Result<ResourceContent, McpError> {
    if !config.enable_creative {
        return Err(unknown());
    }
    let rest = requested
        .strip_prefix("creative://asset/")
        .ok_or_else(unknown)?;
    let mut parts = rest.split('/');
    let context = parts
        .next()
        .filter(|value| !value.is_empty())
        .ok_or_else(unknown)?;
    let project_id = parts
        .next()
        .filter(|value| !value.is_empty())
        .ok_or_else(unknown)?;
    let asset_id = parts
        .next()
        .filter(|value| !value.is_empty())
        .ok_or_else(unknown)?;
    if parts.next().is_some() {
        return Err(unknown());
    }
    crate::application::creative::validate_id(project_id, "project_id")?;
    crate::application::creative::validate_id(asset_id, "asset_id")?;
    let cwd = String::from_utf8(URL_SAFE_NO_PAD.decode(context).map_err(|_| unknown())?)
        .map_err(|_| unknown())?;
    if cwd.is_empty() || cwd.len() > 4096 || cwd.chars().any(char::is_control) {
        return Err(unknown());
    }

    let asset =
        crate::application::creative::require_asset(Some(&cwd), config, project_id, asset_id)?;
    let mime_type = match asset.media_type.as_str() {
        "image/png" => "image/png",
        "image/jpeg" | "image/jpg" => "image/jpeg",
        _ => {
            return Err(McpError::InvalidRequest(
                "creative media resource type is not reviewable".into(),
            ))
        }
    };
    let path = crate::application::creative::resolve_registered_asset_path(
        Some(&cwd),
        config,
        project_id,
        asset_id,
    )?;
    let metadata = fs::symlink_metadata(&path)
        .map_err(|_| McpError::InvalidRequest("creative media resource is unavailable".into()))?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.len() as usize > MAX_MEDIA_RESOURCE_BYTES
        || metadata.len() != asset.bytes
    {
        return Err(McpError::InvalidRequest(
            "creative media resource exceeds review bounds".into(),
        ));
    }
    let bytes = fs::read(&path)
        .map_err(|_| McpError::InvalidRequest("creative media resource is unavailable".into()))?;
    if sha256_bytes(&bytes) != asset.checksum_sha256 {
        return Err(McpError::InvalidRequest(
            "creative media resource no longer matches durable provenance".into(),
        ));
    }
    let format = match mime_type {
        "image/png" => image::ImageFormat::Png,
        "image/jpeg" => image::ImageFormat::Jpeg,
        _ => unreachable!(),
    };
    let (width, height) = image::ImageReader::with_format(Cursor::new(&bytes), format)
        .into_dimensions()
        .map_err(|_| McpError::InvalidRequest("creative media resource image is invalid".into()))?;
    if width == 0
        || height == 0
        || width > MAX_MEDIA_RESOURCE_DIMENSION
        || height > MAX_MEDIA_RESOURCE_DIMENSION
    {
        return Err(McpError::InvalidRequest(
            "creative media resource dimensions exceed review bounds".into(),
        ));
    }
    Ok(ResourceContent {
        uri: requested.to_owned(),
        text: None,
        blob: Some(STANDARD.encode(bytes)),
        mime_type: mime_type.to_owned(),
    })
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

fn blender_enabled(config: &ServerConfig) -> bool {
    config.enable_creative
}

fn resource_names(config: &ServerConfig) -> Vec<&'static str> {
    let mut names = RESOURCE_NAMES.to_vec();
    if blender_enabled(config) {
        names.push(BLENDER_RESOURCE_NAME);
    }
    names
}

fn blender_capability() -> String {
    let tools = crate::interfaces::mcp::blender_tool_catalog()
        .into_iter()
        .map(|tool| tool.name)
        .collect::<Vec<_>>();
    json!({
        "capability": "blender",
        "protocol": crate::application::blender::BLENDER_LAB_PROTOCOL,
        "tools": tools,
        "project_layout": crate::application::blender::project_layout(),
        "routing": {
            "default": "bounded_mcp_control_plane",
            "heavy_execution": "foreground_operator_cli",
            "raw_python": "foreground_operator_cli_only",
            "session": "attach_external_or_explicit_relay_start",
            "network": "loopback_only"
        },
        "authority": {
            "caller_host_override": false,
            "caller_port_override": false,
            "caller_executable_override": false,
            "production_artifacts_project_contained": true
        }
    })
    .to_string()
}

fn unknown() -> McpError {
    McpError::InvalidParams("unknown resource URI".into())
}

fn repository(config: &ServerConfig) -> Result<(PathBuf, String), McpError> {
    let root = config
        .resolved_execution_root()
        .map_err(|_| McpError::InvalidRequest("repository is unavailable".into()))?;
    let root = fs::canonicalize(root)
        .map_err(|_| McpError::InvalidRequest("repository is unavailable".into()))?;
    let root_text = root.to_string_lossy();
    let verified = crate::application::git::resolve_git_workspace(Some(root_text.as_ref()), config)
        .map_err(|_| McpError::InvalidRequest("verified repository is unavailable".into()))?;
    if verified != root {
        return Err(McpError::InvalidRequest(
            "verified repository is unavailable".into(),
        ));
    }
    let name = root
        .file_name()
        .and_then(|v| v.to_str())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| McpError::InvalidRequest("repository identity is unavailable".into()))?;
    let name = name.to_owned();
    if !name
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(McpError::InvalidRequest(
            "repository identity is unavailable".into(),
        ));
    }
    Ok((root, name))
}

fn uri(id: &str, name: &str) -> String {
    format!("workspace://{id}/{name}")
}

fn parse_uri(value: &str) -> Option<(String, &str)> {
    let rest = value.strip_prefix("workspace://")?;
    let (id, name) = rest.split_once('/')?;
    if id.is_empty()
        || name.is_empty()
        || name.contains('/')
        || name.contains('%')
        || name.contains('.')
    {
        return None;
    }
    Some((id.to_owned(), name))
}

fn guidance(root: &Path) -> Result<String, McpError> {
    let mut parts = Vec::new();
    for relative in ["AGENTS.md", ".agents/knowledge/resources.md"] {
        let path = root.join(relative);
        if !path.exists() {
            continue;
        }
        let canonical = fs::canonicalize(&path).map_err(|_| unknown())?;
        if !canonical.starts_with(root)
            || canonical.is_dir()
            || crate::core::protected_paths::is_protected_path(root, &canonical)
        {
            return Err(unknown());
        }
        let bytes = fs::read(&canonical).map_err(|_| unknown())?;
        if bytes.len() > MAX_RESOURCE_BYTES {
            return Err(McpError::InvalidRequest(
                "approved guidance exceeds resource limit".into(),
            ));
        }
        parts.push(format!(
            "# {relative}\n{}",
            String::from_utf8(bytes).map_err(|_| unknown())?
        ));
    }
    Ok(parts.join("\n\n"))
}

fn git_text(root: &Path, args: &[&str], limit: usize) -> Result<String, McpError> {
    let output = Command::new("git")
        .args(["--no-pager", "-c", "core.fsmonitor=false"])
        .args(args)
        .current_dir(root)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .output()
        .map_err(|_| unknown())?;
    if !output.status.success() {
        return Err(unknown());
    }
    let text = String::from_utf8(output.stdout).map_err(|_| unknown())?;
    let text = if args.starts_with(&["status"]) {
        filter_status_text(&text)
    } else {
        text
    };
    bounded(text, limit)
}

fn filter_status_text(text: &str) -> String {
    let mut filtered = text
        .lines()
        .filter(|line| {
            line.starts_with("##")
                || !crate::core::protected_paths::contains_protected_path_reference(line)
        })
        .collect::<Vec<_>>()
        .join("\n");
    if text.ends_with('\n') && !filtered.is_empty() {
        filtered.push('\n');
    }
    filtered
}

fn bounded(text: String, limit: usize) -> Result<String, McpError> {
    if text.len() > limit {
        return Err(McpError::InvalidRequest(
            "resource exceeds maximum size".into(),
        ));
    }
    Ok(text)
}
