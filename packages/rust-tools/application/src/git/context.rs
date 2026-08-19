use super::*;

pub(super) struct RepoContext {
    pub(super) root: PathBuf,
    pub(super) relative_root: String,
    pub(super) execution_root: PathBuf,
}

/// Resolve a canonical Git workspace identity for non-MCP application
/// services such as LSP through the same hardened Git process path.
pub(crate) fn resolve_git_workspace(
    cwd_arg: Option<&str>,
    config: &ServerConfig,
) -> Result<PathBuf, McpError> {
    config
        .ensure_workspaces_initialized()
        .map_err(|e| McpError::Internal(e.to_string()))?;
    if cwd_arg.is_some_and(|value| value.len() > MAX_GIT_PATH_BYTES) {
        return Err(McpError::InvalidRequest(
            "workspace cwd exceeds maximum".into(),
        ));
    }
    let guard = config
        .workspaces
        .read()
        .map_err(|_| McpError::Internal("workspace lock poisoned".into()))?;
    let cwd = relay_core::workspace_path::resolve_contained_cwd_in_allowlist(&guard, cwd_arg)?;
    let containing_root = guard.containing_root(&cwd).ok_or_else(|| {
        McpError::InvalidRequest("workspace is outside authorized workspace roots".into())
    })?;
    validate_git_metadata_paths(&cwd, containing_root)?;
    let out = run_git(&cwd, &["rev-parse", "--show-toplevel"], 8192)?;
    let root_text = std::str::from_utf8(&out)
        .map_err(|_| invalid_git_output())?
        .trim();
    let root = std::fs::canonicalize(root_text)
        .map_err(|_| McpError::InvalidRequest("workspace root is inaccessible".into()))?;
    if !guard.is_contained(&root) {
        return Err(McpError::InvalidRequest(
            "workspace root is outside authorized workspace roots".into(),
        ));
    }
    Ok(root)
}

pub(super) fn resolve_repo(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<RepoContext, McpError> {
    config
        .ensure_workspaces_initialized()
        .map_err(|e| McpError::Internal(e.to_string()))?;
    let cwd_arg = arguments.get("cwd").and_then(Value::as_str);
    if cwd_arg.is_some_and(|value| value.len() > MAX_GIT_PATH_BYTES) {
        return Err(McpError::InvalidRequest("git cwd exceeds maximum".into()));
    }
    let guard = config
        .workspaces
        .read()
        .map_err(|_| McpError::Internal("workspace lock poisoned".into()))?;
    let cwd = relay_core::workspace_path::resolve_contained_cwd_in_allowlist(&guard, cwd_arg)?;
    let containing_root = guard
        .containing_root(&cwd)
        .ok_or_else(|| {
            McpError::InvalidRequest("git cwd is outside authorized workspace roots".into())
        })?
        .to_path_buf();
    validate_git_metadata_paths(&cwd, &containing_root)?;
    let out = run_git(&cwd, &["rev-parse", "--show-toplevel"], 8192)?;
    let root_text = std::str::from_utf8(&out)
        .map_err(|_| invalid_git_output())?
        .trim();
    let root = std::fs::canonicalize(root_text)
        .map_err(|_| McpError::InvalidRequest("git repository root is inaccessible".into()))?;
    if !guard.is_contained(&root) {
        return Err(McpError::InvalidRequest(
            "git repository is outside authorized workspace roots".into(),
        ));
    }
    let effective_root = guard.containing_root(&root).unwrap_or(&containing_root);
    let relative_root = root
        .strip_prefix(effective_root)
        .ok()
        .and_then(Path::to_str)
        .unwrap_or("")
        .trim_start_matches('/')
        .to_owned();
    Ok(RepoContext {
        root,
        relative_root,
        execution_root: effective_root.to_path_buf(),
    })
}
