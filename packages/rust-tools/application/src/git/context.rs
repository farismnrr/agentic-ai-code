use super::*;

pub(super) struct RepoContext {
    pub(super) root: PathBuf,
    pub(super) relative_root: String,
    pub(super) execution_root: PathBuf,
}

pub(super) fn resolve_repo(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<RepoContext, McpError> {
    let execution_root = config
        .resolved_execution_root()
        .map_err(|_| McpError::Internal("failed to resolve execution root".into()))?;
    let cwd_arg = arguments.get("cwd").and_then(Value::as_str);
    if cwd_arg.is_some_and(|value| value.len() > MAX_GIT_PATH_BYTES) {
        return Err(McpError::InvalidRequest("git cwd exceeds maximum".into()));
    }
    let cwd = resolve_contained_cwd(&execution_root, cwd_arg)?;
    validate_git_metadata_paths(&cwd, &execution_root)?;
    let out = run_git(&cwd, &["rev-parse", "--show-toplevel"], 8192)?;
    let root_text = std::str::from_utf8(&out)
        .map_err(|_| invalid_git_output())?
        .trim();
    let root = std::fs::canonicalize(root_text)
        .map_err(|_| McpError::InvalidRequest("git repository root is inaccessible".into()))?;
    if !root.starts_with(&execution_root) {
        return Err(McpError::InvalidRequest(
            "git repository is outside execution root".into(),
        ));
    }
    let relative_root = root
        .strip_prefix(&execution_root)
        .ok()
        .and_then(Path::to_str)
        .unwrap_or("")
        .trim_start_matches('/')
        .to_owned();
    Ok(RepoContext {
        root,
        relative_root,
        execution_root,
    })
}
