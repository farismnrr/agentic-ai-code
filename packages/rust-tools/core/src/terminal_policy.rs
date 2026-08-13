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

pub fn validate_executable(binary: &str) -> Result<(), McpError> {
    let binary_name = Path::new(binary)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    if ["sudo", "su", "doas", "pkexec", "runas", "docker"].contains(&binary_name) {
        return Err(McpError::InvalidRequest(format!(
            "execution of '{}' is forbidden: privilege escalation is not allowed",
            binary_name
        )));
    }
    if binary.contains('/') || binary.contains('\\') || binary == ".." {
        return Err(McpError::InvalidRequest(
            "path traversal or absolute paths in executable name are forbidden".into(),
        ));
    }
    Ok(())
}
