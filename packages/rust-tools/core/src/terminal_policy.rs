use crate::error::McpError;
use std::path::{Path, PathBuf};

pub fn resolve_contained_cwd(
    execution_root: &Path,
    cwd: Option<&str>,
) -> Result<PathBuf, McpError> {
    let target = cwd.map_or_else(
        || execution_root.to_path_buf(),
        |value| {
            let path = Path::new(value);
            if path.is_absolute() {
                path.to_path_buf()
            } else {
                execution_root.join(path)
            }
        },
    );
    let canonical = std::fs::canonicalize(&target).map_err(|_| {
        McpError::InvalidRequest("cwd path does not exist or is inaccessible".into())
    })?;
    if !canonical.starts_with(execution_root) {
        return Err(McpError::InvalidRequest(
            "path traversal outside execution root is forbidden".into(),
        ));
    }
    Ok(canonical)
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
            "path traversal or absolute paths in executable name are forbidden".into(),
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
