use relay_core::error::McpError;
use std::path::{Path, PathBuf};

pub fn effect_classes(
    tool_id: &str,
    destructive_hint: bool,
    open_world_hint: bool,
) -> Vec<&'static str> {
    match tool_id {
        "terminal_exec" | "terminal_job_start" => vec![
            "process_exec",
            "workspace_write",
            "network_read",
            "external_mutation",
        ],
        "http_fetch" => vec!["network_read", "network_write", "external_mutation"],
        "web_search" => vec!["network_read"],
        "file_write" | "file_edit" | "apply_patch" => vec!["workspace_write"],
        "workspace_add" | "workspace_remove" => vec!["workspace_write"],
        "git_branch_create" | "git_branch_switch" | "git_stage" | "git_unstage" | "git_commit"
        | "git_merge_start" | "git_merge_continue" | "git_worktree_add" | "git_stash_push"
        | "git_stash_apply" | "git_tag_create" | "git_branch_rename" | "git_cherry_pick"
        | "git_revert" | "git_remote_add" => vec!["workspace_write"],
        "git_commit_amend"
        | "git_merge_abort"
        | "git_rebase_start"
        | "git_rebase_continue"
        | "git_rebase_abort"
        | "git_branch_delete"
        | "git_worktree_remove"
        | "git_worktree_prune"
        | "git_stash_pop"
        | "git_stash_drop"
        | "git_tag_delete"
        | "git_restore"
        | "git_clean"
        | "git_reset"
        | "git_remote_remove"
        | "git_remote_set_url" => vec!["workspace_write", "workspace_delete"],
        "git_remote_branch_get" => vec!["git_read", "network_read"],
        "git_fetch" => vec!["git_read", "workspace_write", "network_read"],
        "git_push" | "git_remote_branch_delete" => vec![
            "git_read",
            "network_read",
            "network_write",
            "external_mutation",
            "privileged_bridge",
        ],
        "change_request_list" | "change_request_get" | "change_request_checks" => {
            vec!["network_read", "privileged_bridge"]
        }
        "change_request_create" | "change_request_update" | "change_request_merge" => vec![
            "network_read",
            "network_write",
            "external_mutation",
            "privileged_bridge",
        ],
        name if name.starts_with("git_") => vec!["git_read"],
        _ if destructive_hint => vec!["external_mutation"],
        _ if open_world_hint => vec!["network_read", "external_mutation"],
        _ => vec!["workspace_read"],
    }
}

pub fn repository_identity(root: &Path) -> Result<String, McpError> {
    let git_identity = std::fs::canonicalize(root.join(".git"))
        .map_err(|_| McpError::InvalidRequest("repository identity is unavailable".into()))?;
    Ok(format!("{}|{}", root.display(), git_identity.display()))
}

pub fn canonical_repository_root(
    candidate: &Path,
    execution_root: &Path,
) -> Result<PathBuf, McpError> {
    let candidate = std::fs::canonicalize(candidate)
        .map_err(|_| McpError::InvalidRequest("hook repository is unavailable".into()))?;
    if !candidate.starts_with(execution_root) {
        return Err(McpError::InvalidRequest(
            "hook repository is outside execution root".into(),
        ));
    }
    let mut current = Some(candidate.as_path());
    while let Some(path) = current {
        if path.join(".git").exists() && path.join(".agents").is_dir() {
            return Ok(path.to_path_buf());
        }
        if path == execution_root {
            break;
        }
        current = path.parent();
    }
    Err(McpError::InvalidRequest(
        "hook repository metadata is unavailable".into(),
    ))
}

pub fn contained_config_path(root: &Path, relative: &str) -> Result<PathBuf, McpError> {
    let relative_path = Path::new(relative);
    if relative_path.is_absolute()
        || relative_path
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err(McpError::InvalidRequest(
            "hook configuration must be a direct .agents path".into(),
        ));
    }
    let agents = root.join(".agents");
    let path = root.join(relative_path);
    if !path.starts_with(&agents) {
        return Err(McpError::InvalidRequest(
            "hook configuration must be beneath .agents".into(),
        ));
    }
    let canonical = std::fs::canonicalize(&path)
        .map_err(|_| McpError::InvalidRequest("hook configuration is unavailable".into()))?;
    let canonical_agents = std::fs::canonicalize(&agents)
        .map_err(|_| McpError::InvalidRequest("hook metadata is unavailable".into()))?;
    if !canonical.starts_with(canonical_agents) {
        return Err(McpError::InvalidRequest(
            "hook configuration escapes .agents".into(),
        ));
    }
    Ok(canonical)
}
