//! Structured Git worktree operations.

use super::context::resolve_repo;
use super::process::{run_git, validate_ref};
use crate::core::config::ServerConfig;
use crate::core::error::McpError;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WorktreeEntry {
    pub path: String,
    pub head_sha: String,
    pub branch: Option<String>,
    pub is_bare: bool,
    pub is_locked: bool,
    pub lock_reason: Option<String>,
    pub is_prunable: bool,
    pub prune_reason: Option<String>,
    pub is_main: bool,
}

pub fn git_worktree_list(arguments: &Value, config: &ServerConfig) -> Result<Value, McpError> {
    let repo = resolve_repo(arguments, config)?;
    let out = run_git(&repo.root, &["worktree", "list", "--porcelain"], 65536)?;
    let text = std::str::from_utf8(&out)
        .map_err(|_| McpError::Internal("invalid git worktree output".into()))?;

    let worktrees = parse_worktree_porcelain(text, &repo.root)
        .into_iter()
        .filter(|worktree| {
            let path = Path::new(&worktree.path);
            fs::canonicalize(path)
                .map(|canonical| config.is_path_contained(&canonical))
                .unwrap_or_else(|_| config.is_path_contained(path))
        })
        .collect::<Vec<_>>();
    Ok(json!({
        "worktrees": worktrees,
        "total": worktrees.len()
    }))
}

pub fn git_worktree_get(arguments: &Value, config: &ServerConfig) -> Result<Value, McpError> {
    let path_arg = arguments
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::InvalidRequest("path is required".into()))?;

    let list = git_worktree_list(arguments, config)?;
    let worktrees: Vec<WorktreeEntry> =
        serde_json::from_value(list.get("worktrees").cloned().unwrap_or_else(|| json!([])))
            .map_err(|_| McpError::Internal("failed to deserialize worktrees".into()))?;

    let canonical_target = fs::canonicalize(path_arg).unwrap_or_else(|_| PathBuf::from(path_arg));
    let target_str = canonical_target.to_string_lossy();

    let found = worktrees.into_iter().find(|wt| {
        wt.path == path_arg || wt.path == target_str || Path::new(&wt.path) == canonical_target
    });

    match found {
        Some(wt) => Ok(json!({ "worktree": wt })),
        None => Err(McpError::InvalidRequest("worktree not found".into())),
    }
}

pub fn git_worktree_add(arguments: &Value, config: &ServerConfig) -> Result<Value, McpError> {
    let repo = resolve_repo(arguments, config)?;
    super::mutation::validate_mutation_config(&repo.root)?;
    let path_arg = arguments
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::InvalidRequest("path is required".into()))?;

    let dest = if Path::new(path_arg).is_absolute() {
        PathBuf::from(path_arg)
    } else {
        repo.root.join(path_arg)
    };

    let parent = dest
        .parent()
        .ok_or_else(|| McpError::InvalidRequest("invalid destination path".into()))?;
    let canonical_parent = fs::canonicalize(parent)
        .map_err(|_| McpError::InvalidRequest("parent directory does not exist".into()))?;

    if !config.is_path_contained(&canonical_parent) {
        return Err(McpError::InvalidRequest(
            "worktree destination parent is outside authorized workspace roots".into(),
        ));
    }

    if dest.exists() {
        return Err(McpError::InvalidRequest(
            "worktree destination path already exists".into(),
        ));
    }

    let mut args: Vec<&str> = vec!["worktree", "add"];
    if arguments.get("force").and_then(Value::as_bool) == Some(true) {
        args.push("--force");
    }

    let branch = arguments.get("branch").and_then(Value::as_str);
    let commit = arguments.get("commit").and_then(Value::as_str);
    if branch.is_some() && commit.is_some() {
        return Err(McpError::InvalidRequest(
            "branch and commit are mutually exclusive worktree start points".into(),
        ));
    }
    let create_branch = arguments.get("create_branch").and_then(Value::as_str);
    if let Some(branch_name) = create_branch {
        validate_ref(branch_name)?;
        args.extend(["-b", branch_name]);
    }

    let dest_str = dest.to_string_lossy();
    args.push(&dest_str);

    if let Some(commit_ref) = commit {
        validate_ref(commit_ref)?;
        args.push(commit_ref);
    } else if let Some(branch_ref) = branch {
        validate_ref(branch_ref)?;
        args.push(branch_ref);
    }

    run_git(&repo.root, &args, 8192)?;

    // Automatically authorize the newly created worktree root in the workspace allowlist
    if let Ok(canonical_dest) = fs::canonicalize(&dest) {
        let _ = config.ensure_workspaces_initialized();
        if let Ok(mut guard) = config.workspaces.write() {
            let _ = guard.add(&canonical_dest);
        }
    }

    Ok(json!({
        "path": dest.to_string_lossy().into_owned(),
        "created": true,
        "branch": create_branch.or(branch),
    }))
}

pub fn git_worktree_remove(arguments: &Value, config: &ServerConfig) -> Result<Value, McpError> {
    let repo = resolve_repo(arguments, config)?;
    super::mutation::validate_mutation_config(&repo.root)?;
    let path_arg = arguments
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::InvalidRequest("path is required".into()))?;

    let dest = if Path::new(path_arg).is_absolute() {
        PathBuf::from(path_arg)
    } else {
        repo.root.join(path_arg)
    };

    let canonical_dest = fs::canonicalize(&dest)
        .map_err(|_| McpError::InvalidRequest("worktree path is inaccessible".into()))?;
    if !config.is_path_contained(&canonical_dest) {
        return Err(McpError::InvalidRequest(
            "worktree path is outside authorized workspace roots".into(),
        ));
    }
    if canonical_dest == repo.root {
        return Err(McpError::InvalidRequest(
            "cannot remove the main worktree".into(),
        ));
    }

    let mut args: Vec<&str> = vec!["worktree", "remove"];
    if arguments.get("force").and_then(Value::as_bool) == Some(true) {
        args.push("--force");
    }
    let dest_str = dest.to_string_lossy();
    args.push(&dest_str);

    run_git(&repo.root, &args, 8192)?;

    // Remove from workspace allowlist if present
    let _ = config.ensure_workspaces_initialized();
    if let Ok(mut guard) = config.workspaces.write() {
        let _ = guard.remove(&canonical_dest);
    }

    Ok(json!({
        "path": path_arg,
        "removed": true,
    }))
}

pub fn git_worktree_prune(arguments: &Value, config: &ServerConfig) -> Result<Value, McpError> {
    let repo = resolve_repo(arguments, config)?;
    super::mutation::validate_mutation_config(&repo.root)?;
    let dry_run = arguments
        .get("dry_run")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let mut args: Vec<&str> = vec!["worktree", "prune"];
    if dry_run {
        args.push("--dry-run");
    }
    let expire = arguments.get("expire").and_then(Value::as_str);
    if let Some(exp) = expire {
        args.extend(["--expire", exp]);
    }

    let out = run_git(&repo.root, &args, 8192)?;
    let text = std::str::from_utf8(&out).unwrap_or("").trim().to_owned();

    Ok(json!({
        "pruned": true,
        "dry_run": dry_run,
        "output": text,
    }))
}

fn parse_worktree_porcelain(text: &str, main_root: &Path) -> Vec<WorktreeEntry> {
    let mut entries = Vec::new();
    let mut current_path: Option<String> = None;
    let mut current_head: Option<String> = None;
    let mut current_branch: Option<String> = None;
    let mut is_bare = false;
    let mut is_locked = false;
    let mut lock_reason = None;
    let mut is_prunable = false;
    let mut prune_reason = None;

    let flush_entry = |entries: &mut Vec<WorktreeEntry>,
                       path: &mut Option<String>,
                       head: &mut Option<String>,
                       branch: &mut Option<String>,
                       bare: &mut bool,
                       locked: &mut bool,
                       l_reason: &mut Option<String>,
                       prunable: &mut bool,
                       p_reason: &mut Option<String>| {
        if let Some(p) = path.take() {
            let is_main = Path::new(&p) == main_root;
            entries.push(WorktreeEntry {
                path: p,
                head_sha: head.take().unwrap_or_default(),
                branch: branch.take(),
                is_bare: *bare,
                is_locked: *locked,
                lock_reason: l_reason.take(),
                is_prunable: *prunable,
                prune_reason: p_reason.take(),
                is_main,
            });
            *bare = false;
            *locked = false;
            *prunable = false;
        }
    };

    for line in text.lines() {
        if line.is_empty() {
            flush_entry(
                &mut entries,
                &mut current_path,
                &mut current_head,
                &mut current_branch,
                &mut is_bare,
                &mut is_locked,
                &mut lock_reason,
                &mut is_prunable,
                &mut prune_reason,
            );
            continue;
        }
        if let Some(p) = line.strip_prefix("worktree ") {
            flush_entry(
                &mut entries,
                &mut current_path,
                &mut current_head,
                &mut current_branch,
                &mut is_bare,
                &mut is_locked,
                &mut lock_reason,
                &mut is_prunable,
                &mut prune_reason,
            );
            current_path = Some(p.to_string());
        } else if let Some(h) = line.strip_prefix("HEAD ") {
            current_head = Some(h.to_string());
        } else if let Some(b) = line.strip_prefix("branch ") {
            current_branch = Some(b.trim_start_matches("refs/heads/").to_string());
        } else if line == "bare" {
            is_bare = true;
        } else if let Some(reason) = line.strip_prefix("locked ") {
            is_locked = true;
            lock_reason = Some(reason.to_string());
        } else if line == "locked" {
            is_locked = true;
        } else if let Some(reason) = line.strip_prefix("prunable ") {
            is_prunable = true;
            prune_reason = Some(reason.to_string());
        } else if line == "prunable" {
            is_prunable = true;
        }
    }

    flush_entry(
        &mut entries,
        &mut current_path,
        &mut current_head,
        &mut current_branch,
        &mut is_bare,
        &mut is_locked,
        &mut lock_reason,
        &mut is_prunable,
        &mut prune_reason,
    );

    entries
}
