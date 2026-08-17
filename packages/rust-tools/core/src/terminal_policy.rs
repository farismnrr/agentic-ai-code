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

#[cfg(test)]
mod tests {
    use super::validate_executable;

    #[test]
    fn privilege_escalation_commands_remain_forbidden() {
        for command in ["sudo", "su", "doas", "pkexec", "runas"] {
            assert!(validate_executable(command, true).is_err(), "{command}");
        }
    }

    #[test]
    fn docker_requires_explicit_opt_in() {
        assert!(validate_executable("docker", false).is_err());
        assert!(validate_executable("docker", true).is_ok());
    }

    #[test]
    fn executable_paths_remain_forbidden() {
        assert!(validate_executable("/usr/bin/docker", true).is_err());
        assert!(validate_executable("../docker", true).is_err());
    }
}
