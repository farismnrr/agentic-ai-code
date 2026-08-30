use crate::error::McpError;
use std::path::{Path, PathBuf};

pub fn resolve_contained_cwd(
    execution_root: &Path,
    cwd: Option<&str>,
) -> Result<PathBuf, McpError> {
    crate::workspace_path::resolve_contained_cwd(execution_root, cwd).map_err(|error| match error {
        McpError::InvalidRequest(message) if message == "path is outside the execution root" => {
            McpError::InvalidRequest("path traversal outside execution root is forbidden".into())
        }
        error => error,
    })
}

pub fn validate_executable(binary: &str, allow_docker: bool) -> Result<(), McpError> {
    let binary_name = Path::new(binary)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    if ["sudo", "su", "doas", "pkexec", "runas"].contains(&binary_name) {
        return Err(McpError::InvalidRequest(format!(
            "execution of '{}' is forbidden: privilege escalation is not allowed",
            binary_name
        )));
    }
    if binary_name == "docker" && !allow_docker {
        return Err(McpError::InvalidRequest(
            "execution of 'docker' is forbidden unless RELAY_ALLOW_DOCKER=true".into(),
        ));
    }
    if binary.contains('/') || binary.contains('\\') || binary == ".." {
        return Err(McpError::InvalidRequest(
            "path traversal or absolute paths in executable name are forbidden; use an executable from the relay safe PATH and pass a repository script path as an argument to an approved interpreter (for example command=bash with args=[\"scripts/check.sh\"])".into(),
        ));
    }
    Ok(())
}

/// Detect the dedicated SSH terminal shape without granting any authority.
/// Full alias/remote-command validation remains owned by `ssh_policy`.
pub fn is_ssh_request(arguments: &serde_json::Value) -> bool {
    let Some(command) = arguments.get("command").and_then(serde_json::Value::as_str) else {
        return false;
    };
    shell_words::split(command)
        .ok()
        .and_then(|parts| parts.into_iter().next())
        .as_deref()
        == Some("ssh")
}
