use super::*;
use crate::core::workspace_path::{resolve_write_target, EntryKind};
use serde::Serialize;
use serde_json::Value;
use std::path::Path;
const MAX_MUTATION_PATHS: usize = 64;
const MAX_COMMIT_MESSAGE_BYTES: usize = 4096;
const MAX_MUTATION_CONFIG_BYTES: usize = 16 * 1024;
pub(super) fn validate_mutation_config(root: &Path) -> Result<(), McpError> {
    let output = git_command(root)
        .args([
            "config",
            "--local",
            "--no-includes",
            "--name-only",
            "--list",
        ])
        .output()
        .map_err(|_| McpError::Internal("failed to inspect git mutation configuration".into()))?;
    if !output.status.success() || output.stdout.len() > MAX_MUTATION_CONFIG_BYTES {
        return Err(McpError::InvalidRequest(
            "repository Git mutation configuration could not be verified".into(),
        ));
    }
    let text = std::str::from_utf8(&output.stdout).map_err(|_| invalid_git_output())?;
    for raw in text.lines() {
        let key = raw.trim().to_ascii_lowercase();
        let dangerous = key.starts_with("include.")
            || key.starts_with("includeif.")
            || key.starts_with("filter.")
            || (key.starts_with("merge.") && key.ends_with(".driver"))
            || key == "merge.default"
            || key == "core.attributesfile";
        if dangerous {
            return Err(McpError::InvalidRequest(
                "repository Git configuration contains executable mutation hooks or drivers".into(),
            ));
        }
    }
    Ok(())
}
#[derive(Debug, Serialize)]
pub(super) struct GitPathsMutationResult {
    repository_root: String,
    operation: &'static str,
    paths: Vec<String>,
}
#[derive(Debug, Serialize)]
pub(super) struct GitCommitMutationResult {
    repository_root: String,
    operation: &'static str,
    branch: String,
    head: String,
}
#[derive(Debug, Serialize)]
pub(super) struct GitOperationState {
    repository_root: String,
    operation: Option<&'static str>,
    branch: Option<String>,
    head: String,
    conflicts: Vec<String>,
    next_actions: Vec<&'static str>,
}
#[derive(Debug, Serialize)]
pub(super) struct GitBranchDeleteResult {
    repository_root: String,
    operation: &'static str,
    branch: String,
}
pub(super) fn git_stage(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<GitPathsMutationResult, McpError> {
    let repo = resolve_repo(arguments, config)?;
    validate_mutation_config(&repo.root)?;
    let paths = mutation_paths(arguments, &repo)?;
    let mut args = vec!["add", "--"];
    args.extend(paths.iter().map(String::as_str));
    run_git(&repo.root, &args, MAX_GIT_OUTPUT_BYTES)?;
    reject_protected_diff_changes(&repo.root, "staged", &[])?;
    Ok(GitPathsMutationResult {
        repository_root: repo.relative_root,
        operation: "stage",
        paths,
    })
}
pub(super) fn git_unstage(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<GitPathsMutationResult, McpError> {
    let repo = resolve_repo(arguments, config)?;
    validate_mutation_config(&repo.root)?;
    let paths = mutation_paths(arguments, &repo)?;
    let mut args = vec!["restore", "--staged", "--"];
    args.extend(paths.iter().map(String::as_str));
    run_git(&repo.root, &args, MAX_GIT_OUTPUT_BYTES)?;
    Ok(GitPathsMutationResult {
        repository_root: repo.relative_root,
        operation: "unstage",
        paths,
    })
}
pub(super) fn git_commit(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<GitCommitMutationResult, McpError> {
    let repo = resolve_repo(arguments, config)?;
    validate_mutation_config(&repo.root)?;
    ensure_no_operation(&repo.root)?;
    reject_protected_diff_changes(&repo.root, "staged", &[])?;
    let message = arguments
        .get("message")
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::InvalidRequest("commit message is required".into()))?;
    if message.trim().is_empty()
        || message.len() > MAX_COMMIT_MESSAGE_BYTES
        || message.contains('\0')
    {
        return Err(McpError::InvalidRequest("commit message is invalid".into()));
    }
    require_repo_identity(&repo.root)?;
    let staged_names = run_git(
        &repo.root,
        &["diff", "--cached", "--name-only", "-z"],
        MAX_GIT_OUTPUT_BYTES,
    )?;
    if staged_names.is_empty() {
        return Err(McpError::InvalidRequest(
            "nothing is staged for commit".into(),
        ));
    }
    run_git(
        &repo.root,
        &["commit", "--no-gpg-sign", "-m", message],
        MAX_GIT_OUTPUT_BYTES,
    )?;
    mutation_head_result(repo, "commit")
}
pub(super) fn git_commit_amend(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<GitCommitMutationResult, McpError> {
    let repo = resolve_repo(arguments, config)?;
    validate_mutation_config(&repo.root)?;
    ensure_no_operation(&repo.root)?;
    reject_protected_diff_changes(&repo.root, "staged", &[])?;
    require_repo_identity(&repo.root)?;
    let message = arguments.get("message").and_then(Value::as_str);
    if message.is_some_and(|value| {
        value.trim().is_empty() || value.len() > MAX_COMMIT_MESSAGE_BYTES || value.contains('\0')
    }) {
        return Err(McpError::InvalidRequest("commit message is invalid".into()));
    }
    let staged = run_git(
        &repo.root,
        &["diff", "--cached", "--name-only", "-z"],
        MAX_GIT_OUTPUT_BYTES,
    )?;
    if staged.is_empty() && message.is_none() {
        return Err(McpError::InvalidRequest(
            "amend requires staged changes or a replacement message".into(),
        ));
    }
    let mut args = vec!["commit", "--amend", "--no-gpg-sign"];
    if let Some(message) = message {
        args.extend(["-m", message]);
    } else {
        args.push("--no-edit");
    }
    run_git(&repo.root, &args, MAX_GIT_OUTPUT_BYTES)?;
    mutation_head_result(repo, "amend")
}
pub(super) fn git_merge_start(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<GitOperationState, McpError> {
    let repo = resolve_repo(arguments, config)?;
    validate_mutation_config(&repo.root)?;
    ensure_clean_worktree(&repo.root)?;
    ensure_no_operation(&repo.root)?;
    let target = resolve_commit_ref(&repo.root, &validated_ref(arguments, "ref")?)?;
    let _ = run_git(
        &repo.root,
        &["merge", "--no-edit", "--no-ff", &target],
        MAX_GIT_OUTPUT_BYTES,
    );
    operation_state(repo)
}
pub(super) fn git_merge_continue(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<GitOperationState, McpError> {
    let repo = resolve_repo(arguments, config)?;
    validate_mutation_config(&repo.root)?;
    require_operation(&repo.root, "merge")?;
    let state = status_conflicts(&repo.root)?;
    if !state.is_empty() {
        return Err(McpError::InvalidRequest(
            "merge still has unresolved conflicts".into(),
        ));
    }
    require_repo_identity(&repo.root)?;
    run_git(
        &repo.root,
        &["commit", "--no-gpg-sign", "--no-edit"],
        MAX_GIT_OUTPUT_BYTES,
    )?;
    operation_state(repo)
}
pub(super) fn git_merge_abort(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<GitOperationState, McpError> {
    let repo = resolve_repo(arguments, config)?;
    validate_mutation_config(&repo.root)?;
    require_operation(&repo.root, "merge")?;
    run_git(&repo.root, &["merge", "--abort"], MAX_GIT_OUTPUT_BYTES)?;
    operation_state(repo)
}
pub(super) fn git_rebase_start(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<GitOperationState, McpError> {
    let repo = resolve_repo(arguments, config)?;
    validate_mutation_config(&repo.root)?;
    ensure_clean_worktree(&repo.root)?;
    ensure_no_operation(&repo.root)?;
    let target = resolve_commit_ref(&repo.root, &validated_ref(arguments, "ref")?)?;
    let _ = run_git(&repo.root, &["rebase", &target], MAX_GIT_OUTPUT_BYTES);
    operation_state(repo)
}

pub(super) fn git_rebase_continue(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<GitOperationState, McpError> {
    let repo = resolve_repo(arguments, config)?;
    validate_mutation_config(&repo.root)?;
    require_operation(&repo.root, "rebase")?;
    if !status_conflicts(&repo.root)?.is_empty() {
        return Err(McpError::InvalidRequest(
            "rebase still has unresolved conflicts".into(),
        ));
    }
    run_git(
        &repo.root,
        &["-c", "core.editor=true", "rebase", "--continue"],
        MAX_GIT_OUTPUT_BYTES,
    )?;
    operation_state(repo)
}

pub(super) fn git_rebase_abort(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<GitOperationState, McpError> {
    let repo = resolve_repo(arguments, config)?;
    validate_mutation_config(&repo.root)?;
    require_operation(&repo.root, "rebase")?;
    run_git(&repo.root, &["rebase", "--abort"], MAX_GIT_OUTPUT_BYTES)?;
    operation_state(repo)
}

pub(super) fn git_operation_status(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<GitOperationState, McpError> {
    operation_state(resolve_repo(arguments, config)?)
}

pub(super) fn git_branch_delete(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<GitBranchDeleteResult, McpError> {
    let repo = resolve_repo(arguments, config)?;
    validate_mutation_config(&repo.root)?;
    ensure_clean_worktree(&repo.root)?;
    ensure_no_operation(&repo.root)?;
    let branch = branch::validate_branch_name(
        arguments
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| McpError::InvalidRequest("branch name is required".into()))?,
    )?;
    let current = current_branch(&repo.root)?;
    if current.as_deref() == Some(branch.as_str()) {
        return Err(McpError::InvalidRequest(
            "cannot delete the current branch".into(),
        ));
    }
    run_git(
        &repo.root,
        &["branch", "-d", "--", &branch],
        MAX_GIT_OUTPUT_BYTES,
    )?;
    Ok(GitBranchDeleteResult {
        repository_root: repo.relative_root,
        operation: "delete",
        branch,
    })
}

fn mutation_paths(arguments: &Value, repo: &RepoContext) -> Result<Vec<String>, McpError> {
    let values = arguments
        .get("paths")
        .and_then(Value::as_array)
        .ok_or_else(|| McpError::InvalidRequest("paths is required".into()))?;
    if values.is_empty() || values.len() > MAX_MUTATION_PATHS {
        return Err(McpError::InvalidRequest(
            "git mutation paths exceed allowed bounds".into(),
        ));
    }
    let root = repo.root.to_string_lossy();
    let mut out = Vec::with_capacity(values.len());
    for value in values {
        let raw = value
            .as_str()
            .ok_or_else(|| McpError::InvalidRequest("git mutation path is invalid".into()))?;
        if raw.is_empty() || raw.len() > MAX_GIT_PATH_BYTES {
            return Err(McpError::InvalidRequest(
                "git mutation path is invalid".into(),
            ));
        }
        let resolved = resolve_write_target(
            &repo.execution_root,
            Some(root.as_ref()),
            raw,
            EntryKind::File,
        )?;
        reject_protected_target(&repo.execution_root, &resolved)?;
        let relative = resolved
            .strip_prefix(&repo.root)
            .map_err(|_| McpError::InvalidRequest("git path is outside repository".into()))?;
        let relative = relative
            .to_str()
            .ok_or_else(|| McpError::InvalidRequest("git path is not valid UTF-8".into()))?
            .to_owned();
        if !resolved.exists() {
            run_git(
                &repo.root,
                &["ls-files", "--error-unmatch", "--", &relative],
                8192,
            )?;
        }
        out.push(relative);
    }
    out.sort();
    out.dedup();
    Ok(out)
}

fn require_repo_identity(root: &Path) -> Result<(), McpError> {
    for key in ["user.name", "user.email"] {
        let output = run_git(root, &["config", "--local", "--get", key], 1024)?;
        let value = std::str::from_utf8(&output)
            .map_err(|_| invalid_git_output())?
            .trim();
        if value.is_empty() || value.len() > 320 || value.contains(['\0', '\n', '\r']) {
            return Err(McpError::InvalidRequest(
                "repository commit identity is invalid".into(),
            ));
        }
    }
    Ok(())
}

fn mutation_head_result(
    repo: RepoContext,
    operation: &'static str,
) -> Result<GitCommitMutationResult, McpError> {
    let head = head_sha(&repo.root)?;
    let branch = current_branch(&repo.root)?
        .ok_or_else(|| McpError::InvalidRequest("git repository is detached".into()))?;
    Ok(GitCommitMutationResult {
        repository_root: repo.relative_root,
        operation,
        branch,
        head,
    })
}

fn head_sha(root: &Path) -> Result<String, McpError> {
    let out = run_git(root, &["rev-parse", "HEAD"], 128)?;
    let value = std::str::from_utf8(&out)
        .map_err(|_| invalid_git_output())?
        .trim();
    if value.len() != 40 || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(invalid_git_output());
    }
    Ok(value.to_owned())
}

fn current_branch(root: &Path) -> Result<Option<String>, McpError> {
    let out = run_git(root, &["branch", "--show-current"], MAX_GIT_REF_BYTES + 8)?;
    let value = std::str::from_utf8(&out)
        .map_err(|_| invalid_git_output())?
        .trim();
    if value.is_empty() {
        Ok(None)
    } else {
        Ok(Some(branch::validate_branch_name(value)?))
    }
}

fn ensure_clean_worktree(root: &Path) -> Result<(), McpError> {
    let out = run_git(
        root,
        &["status", "--porcelain=v2", "-z"],
        MAX_GIT_OUTPUT_BYTES,
    )?;
    if out.is_empty() {
        Ok(())
    } else {
        Err(McpError::InvalidRequest(
            "git worktree must be clean".into(),
        ))
    }
}

fn operation_kind(root: &Path) -> Result<Option<&'static str>, McpError> {
    let git_dir = run_git(root, &["rev-parse", "--absolute-git-dir"], 4096)?;
    let git_dir = std::str::from_utf8(&git_dir)
        .map_err(|_| invalid_git_output())?
        .trim();
    let git_dir = std::fs::canonicalize(git_dir)
        .map_err(|_| McpError::InvalidRequest("git metadata is inaccessible".into()))?;
    if git_dir.join("MERGE_HEAD").is_file() {
        return Ok(Some("merge"));
    }
    if git_dir.join("rebase-merge").is_dir() || git_dir.join("rebase-apply").is_dir() {
        return Ok(Some("rebase"));
    }
    Ok(None)
}

fn ensure_no_operation(root: &Path) -> Result<(), McpError> {
    if operation_kind(root)?.is_some() {
        Err(McpError::InvalidRequest(
            "another git operation is active".into(),
        ))
    } else {
        Ok(())
    }
}
fn require_operation(root: &Path, expected: &str) -> Result<(), McpError> {
    if operation_kind(root)? == Some(expected) {
        Ok(())
    } else {
        Err(McpError::InvalidRequest(format!(
            "no active {expected} operation"
        )))
    }
}

fn status_conflicts(root: &Path) -> Result<Vec<String>, McpError> {
    let output = run_git(
        root,
        &["diff", "--name-only", "--diff-filter=U", "-z"],
        MAX_GIT_OUTPUT_BYTES,
    )?;
    let mut paths = Vec::new();
    for field in output.split(|b| *b == 0).filter(|v| !v.is_empty()) {
        let path = std::str::from_utf8(field).map_err(|_| invalid_git_output())?;
        if is_protected_git_path(root, path) {
            return Err(McpError::InvalidRequest(
                "git conflict references a protected path".into(),
            ));
        }
        paths.push(path.to_owned());
        if paths.len() > MAX_GIT_RESULTS {
            return Err(McpError::InvalidRequest(
                "git conflict count exceeds maximum".into(),
            ));
        }
    }
    Ok(paths)
}

fn operation_state(repo: RepoContext) -> Result<GitOperationState, McpError> {
    let operation = operation_kind(&repo.root)?;
    let conflicts = status_conflicts(&repo.root)?;
    let next_actions = match operation {
        Some("merge") if conflicts.is_empty() => vec!["continue", "abort"],
        Some("merge") => vec!["inspect", "edit", "stage", "continue", "abort"],
        Some("rebase") if conflicts.is_empty() => vec!["continue", "abort"],
        Some("rebase") => vec!["inspect", "edit", "stage", "continue", "abort"],
        _ => vec![],
    };
    Ok(GitOperationState {
        repository_root: repo.relative_root,
        operation,
        branch: current_branch(&repo.root)?,
        head: head_sha(&repo.root)?,
        conflicts,
        next_actions,
    })
}
