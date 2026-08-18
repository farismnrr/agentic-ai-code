//! Bounded, read-only MCP resources for the verified repository.

use relay_core::{config::ServerConfig, error::McpError};
use relay_interfaces::mcp::resources::{Resource, ResourceContent};
use serde_json::json;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

const MIME: &str = "text/plain; charset=utf-8";
const MAX_RESOURCE_BYTES: usize = 64 * 1024;
const MAX_STATUS_BYTES: usize = 16 * 1024;
pub const RESOURCE_NAMES: [&str; 4] = ["manifest", "agent-guidance", "status", "head"];

pub fn list(config: &ServerConfig) -> Result<Vec<Resource>, McpError> {
    let (_, id) = repository(config)?;
    Ok(RESOURCE_NAMES
        .iter()
        .map(|name| Resource {
            uri: uri(&id, name),
            name: (*name).to_owned(),
            description: match *name {
                "manifest" => "Bounded repository identity and capability metadata.",
                "agent-guidance" => "Approved AGENTS.md and resource-index guidance.",
                "status" => "Bounded non-mutating Git workspace status.",
                "head" => "Current verified Git HEAD and ref metadata.",
                _ => "Repository resource.",
            }
            .to_owned(),
            mime_type: MIME,
        })
        .collect())
}

pub fn read(config: &ServerConfig, requested: &str) -> Result<ResourceContent, McpError> {
    let (root, id) = repository(config)?;
    let Some((resource_id, name)) = parse_uri(requested) else {
        return Err(unknown());
    };
    if resource_id != id || !RESOURCE_NAMES.contains(&name) {
        return Err(unknown());
    }
    let text = match name {
        "manifest" => json!({ "repository": id, "root": "verified-execution-root", "markers": ["Cargo.toml", "package.json"], "resources": RESOURCE_NAMES, "capabilities": ["workspace-read", "workspace-write", "git-read", "lsp", "mcp-tools"] }).to_string(),
        "agent-guidance" => guidance(&root)?,
        "status" => git_text(&root, &["status", "--short", "--branch"], MAX_STATUS_BYTES)?,
        "head" => git_text(&root, &["rev-parse", "--verify", "HEAD"], MAX_STATUS_BYTES)?,
        _ => unreachable!(),
    };
    Ok(ResourceContent {
        uri: requested.to_owned(),
        text: bounded(text, MAX_RESOURCE_BYTES)?,
        mime_type: MIME,
    })
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
    if !root.join(".git").exists() {
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
        if !canonical.starts_with(root) || canonical.is_dir() {
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
    bounded(
        String::from_utf8(output.stdout).map_err(|_| unknown())?,
        limit,
    )
}

fn bounded(text: String, limit: usize) -> Result<String, McpError> {
    if text.len() > limit {
        return Err(McpError::InvalidRequest(
            "resource exceeds maximum size".into(),
        ));
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use relay_core::config::ServerConfig;

    #[test]
    fn rejects_non_server_owned_uris() {
        let config = ServerConfig::default();
        assert!(read(&config, "workspace://ai-code/../../.env").is_err());
        assert!(read(&config, "workspace://other/status").is_err());
        assert!(read(&config, "file:///home/.env").is_err());
    }

    #[test]
    fn resource_names_are_deterministic_and_bounded() {
        assert_eq!(
            RESOURCE_NAMES,
            ["manifest", "agent-guidance", "status", "head"]
        );
        assert!(bounded("x".repeat(MAX_RESOURCE_BYTES + 1), MAX_RESOURCE_BYTES).is_err());
    }
}
