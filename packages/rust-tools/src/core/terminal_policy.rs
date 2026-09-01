use crate::core::error::McpError;
use std::path::{Path, PathBuf};

pub fn resolve_contained_cwd(
    execution_root: &Path,
    cwd: Option<&str>,
) -> Result<PathBuf, McpError> {
    crate::core::workspace_path::resolve_contained_cwd(execution_root, cwd).map_err(|error| {
        match error {
            McpError::InvalidRequest(message)
                if message == "path is outside the execution root" =>
            {
                McpError::InvalidRequest(
                    "path traversal outside execution root is forbidden".into(),
                )
            }
            error => error,
        }
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
    if ["ssh", "scp", "sftp"].contains(&binary_name) {
        return Err(McpError::InvalidRequest(format!(
            "execution of '{}' is unavailable through terminal_exec; use ssh_readonly_exec for remote diagnostics",
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
