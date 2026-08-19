//! Bounded Git intelligence and mutation primitives with a fail-closed process contract.

use crate::workspace::reject_protected_target;
use relay_core::config::ServerConfig;
use relay_core::error::McpError;
use relay_core::workspace_path::{resolve_existing_path, EntryKind};
use relay_interfaces::mcp::{ToolCallResult, ToolResultContent};
use serde::Serialize;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

const MAX_GIT_OUTPUT_BYTES: usize = 256 * 1024;
const MAX_GIT_PATH_BYTES: usize = 4096;
const MAX_GIT_REF_BYTES: usize = 512;
const MAX_GIT_RESULTS: usize = 100;
const DEFAULT_GIT_RESULTS: usize = 50;
const MAX_BLAME_LINES: usize = 500;
const MAX_DIFF_CONTEXT: u64 = 20;

#[derive(Debug, Serialize)]
struct GitStatusResult {
    repository_root: String,
    branch: Option<String>,
    detached: bool,
    upstream: Option<String>,
    ahead: u64,
    behind: u64,
    staged: Vec<String>,
    unstaged: Vec<String>,
    untracked: Vec<String>,
    conflicts: Vec<String>,
    truncated: bool,
}
#[derive(Debug, Serialize)]
struct GitTextResult {
    repository_root: String,
    text: String,
    truncated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    continuation: Option<String>,
}
#[derive(Debug, Serialize)]
struct GitLogResult {
    repository_root: String,
    commits: Vec<GitCommit>,
    truncated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    continuation: Option<String>,
}
#[derive(Debug, Serialize)]
struct GitCommit {
    sha: String,
    parents: Vec<String>,
    timestamp: i64,
    subject: String,
}

pub async fn dispatch_git_tool(
    name: &str,
    arguments: &Value,
    config: &ServerConfig,
) -> Result<Option<ToolCallResult>, McpError> {
    let value = match name {
        "git_status" => serde_json::to_value(git_status(arguments, config)?),
        "git_diff" => serde_json::to_value(git_diff(arguments, config)?),
        "git_log" => serde_json::to_value(git_log(arguments, config)?),
        "git_show" => serde_json::to_value(git_show(arguments, config)?),
        "git_blame" => serde_json::to_value(parse::git_blame(arguments, config)?),
        "git_branch_list" => serde_json::to_value(branch::git_branch_list(arguments, config)?),
        "git_branch_create" => serde_json::to_value(branch::git_branch_create(arguments, config)?),
        "git_branch_switch" => serde_json::to_value(branch::git_branch_switch(arguments, config)?),
        "git_stage" => serde_json::to_value(mutation::git_stage(arguments, config)?),
        "git_unstage" => serde_json::to_value(mutation::git_unstage(arguments, config)?),
        "git_commit" => serde_json::to_value(mutation::git_commit(arguments, config)?),
        "git_commit_amend" => serde_json::to_value(mutation::git_commit_amend(arguments, config)?),
        "git_operation_status" => {
            serde_json::to_value(mutation::git_operation_status(arguments, config)?)
        }
        "git_merge_start" => serde_json::to_value(mutation::git_merge_start(arguments, config)?),
        "git_merge_continue" => {
            serde_json::to_value(mutation::git_merge_continue(arguments, config)?)
        }
        "git_merge_abort" => serde_json::to_value(mutation::git_merge_abort(arguments, config)?),
        "git_rebase_start" => serde_json::to_value(mutation::git_rebase_start(arguments, config)?),
        "git_rebase_continue" => {
            serde_json::to_value(mutation::git_rebase_continue(arguments, config)?)
        }
        "git_rebase_abort" => serde_json::to_value(mutation::git_rebase_abort(arguments, config)?),
        "git_branch_delete" => {
            serde_json::to_value(mutation::git_branch_delete(arguments, config)?)
        }
        "git_remote_list" => serde_json::to_value(remote::git_remote_list(arguments, config)?),
        "git_remote_branch_get" => {
            serde_json::to_value(remote::git_remote_branch_get(arguments, config).await?)
        }
        "git_fetch" => serde_json::to_value(remote::git_fetch(arguments, config).await?),
        "git_push" => serde_json::to_value(remote::git_push(arguments, config).await?),
        "git_remote_branch_delete" => {
            serde_json::to_value(remote::git_remote_branch_delete(arguments, config).await?)
        }
        "git_remote_add" => serde_json::to_value(advanced::git_remote_add(arguments, config)?),
        "git_remote_remove" => {
            serde_json::to_value(advanced::git_remote_remove(arguments, config)?)
        }
        "git_remote_set_url" => {
            serde_json::to_value(advanced::git_remote_set_url(arguments, config)?)
        }
        "git_worktree_list" => {
            serde_json::to_value(worktree::git_worktree_list(arguments, config)?)
        }
        "git_worktree_get" => serde_json::to_value(worktree::git_worktree_get(arguments, config)?),
        "git_worktree_add" => serde_json::to_value(worktree::git_worktree_add(arguments, config)?),
        "git_worktree_remove" => {
            serde_json::to_value(worktree::git_worktree_remove(arguments, config)?)
        }
        "git_worktree_prune" => {
            serde_json::to_value(worktree::git_worktree_prune(arguments, config)?)
        }
        "git_stash_list" => serde_json::to_value(stash::git_stash_list(arguments, config)?),
        "git_stash_push" => serde_json::to_value(stash::git_stash_push(arguments, config)?),
        "git_stash_pop" => serde_json::to_value(stash::git_stash_pop(arguments, config)?),
        "git_stash_apply" => serde_json::to_value(stash::git_stash_apply(arguments, config)?),
        "git_stash_drop" => serde_json::to_value(stash::git_stash_drop(arguments, config)?),
        "git_tag_list" => serde_json::to_value(tag::git_tag_list(arguments, config)?),
        "git_tag_create" => serde_json::to_value(tag::git_tag_create(arguments, config)?),
        "git_tag_delete" => serde_json::to_value(tag::git_tag_delete(arguments, config)?),
        "git_branch_rename" => {
            serde_json::to_value(advanced::git_branch_rename(arguments, config)?)
        }
        "git_restore" => serde_json::to_value(advanced::git_restore(arguments, config)?),
        "git_clean" => serde_json::to_value(advanced::git_clean(arguments, config)?),
        "git_cherry_pick" => serde_json::to_value(advanced::git_cherry_pick(arguments, config)?),
        "git_revert" => serde_json::to_value(advanced::git_revert(arguments, config)?),
        "git_reset" => serde_json::to_value(advanced::git_reset(arguments, config)?),
        "change_request_list" => {
            serde_json::to_value(forge::change_request_list(arguments, config).await?)
        }
        "change_request_get" => {
            serde_json::to_value(forge::change_request_get(arguments, config).await?)
        }
        "change_request_create" => {
            serde_json::to_value(forge::change_request_create(arguments, config).await?)
        }
        "change_request_update" => {
            serde_json::to_value(forge::change_request_update(arguments, config).await?)
        }
        "change_request_checks" => {
            serde_json::to_value(forge::change_request_checks(arguments, config).await?)
        }
        "change_request_merge" => {
            serde_json::to_value(forge::change_request_merge(arguments, config).await?)
        }
        _ => return Ok(None),
    }
    .map_err(|_| McpError::Internal("failed to serialize git result".into()))?;
    let text = serde_json::to_string(&value)
        .map_err(|_| McpError::Internal("failed to serialize git result".into()))?;
    if text.len() > MAX_GIT_OUTPUT_BYTES + 32 * 1024 {
        return Err(McpError::InvalidRequest(
            "git result exceeds output maximum".into(),
        ));
    }
    Ok(Some(ToolCallResult::complete(vec![ToolResultContent {
        kind: "text",
        text,
    }])))
}

fn git_status(arguments: &Value, config: &ServerConfig) -> Result<GitStatusResult, McpError> {
    let repo = resolve_repo(arguments, config)?;
    let include_untracked = arguments
        .get("include_untracked")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let mut args = vec!["status", "--porcelain=v2", "--branch", "-z"];
    if !include_untracked {
        args.push("--untracked-files=no");
    }
    let output = run_git(&repo.root, &args, MAX_GIT_OUTPUT_BYTES)?;
    let hidden_staged_paths = protected_staged_rename_copy_paths(&repo.root)?;
    let mut result = GitStatusResult {
        repository_root: repo.relative_root,
        branch: None,
        detached: false,
        upstream: None,
        ahead: 0,
        behind: 0,
        staged: vec![],
        unstaged: vec![],
        untracked: vec![],
        conflicts: vec![],
        truncated: false,
    };
    let mut records = output.split(|b| *b == 0).filter(|r| !r.is_empty());
    while let Some(record) = records.next() {
        let text = std::str::from_utf8(record).map_err(|_| invalid_git_output())?;
        if let Some(v) = text.strip_prefix("# branch.head ") {
            if v == "(detached)" {
                result.detached = true
            } else {
                result.branch = Some(v.to_owned())
            }
        } else if let Some(v) = text.strip_prefix("# branch.upstream ") {
            result.upstream = Some(v.to_owned());
        } else if let Some(v) = text.strip_prefix("# branch.ab ") {
            for part in v.split_whitespace() {
                if let Some(n) = part.strip_prefix('+') {
                    result.ahead = n.parse().unwrap_or(0)
                } else if let Some(n) = part.strip_prefix('-') {
                    result.behind = n.parse().unwrap_or(0)
                }
            }
        } else if let Some(path) = text.strip_prefix("? ") {
            if !is_protected_git_path(&repo.root, path) {
                push_bounded(
                    &mut result.untracked,
                    path.to_owned(),
                    &mut result.truncated,
                );
            }
        } else if text.starts_with("u ") {
            if let Some(path) = status_path(text).filter(|path| {
                !is_protected_git_path(&repo.root, path) && !hidden_staged_paths.contains(path)
            }) {
                push_bounded(&mut result.conflicts, path, &mut result.truncated);
            }
        } else if text.starts_with("1 ") || text.starts_with("2 ") {
            let bytes = text.as_bytes();
            if bytes.len() > 4 {
                let x = bytes[2] as char;
                let y = bytes[3] as char;
                let renamed_from = if text.starts_with("2 ") {
                    let original = records.next().ok_or_else(invalid_git_output)?;
                    Some(std::str::from_utf8(original).map_err(|_| invalid_git_output())?)
                } else {
                    None
                };
                if let Some(path) = status_path(text).filter(|path| {
                    !is_protected_git_path(&repo.root, path)
                        && !hidden_staged_paths.contains(path)
                        && !renamed_from
                            .is_some_and(|original| is_protected_git_path(&repo.root, original))
                }) {
                    if x != '.' {
                        push_bounded(&mut result.staged, path.clone(), &mut result.truncated);
                    }
                    if y != '.' {
                        push_bounded(&mut result.unstaged, path, &mut result.truncated);
                    }
                }
            }
        }
    }
    Ok(result)
}

fn git_diff(arguments: &Value, config: &ServerConfig) -> Result<GitTextResult, McpError> {
    let repo = resolve_repo(arguments, config)?;
    let mode = arguments
        .get("mode")
        .and_then(Value::as_str)
        .unwrap_or("working");
    let context = arguments
        .get("context_lines")
        .and_then(Value::as_u64)
        .unwrap_or(3)
        .min(MAX_DIFF_CONTEXT);
    let mut owned = vec![
        "diff".to_string(),
        "--no-ext-diff".into(),
        "--no-textconv".into(),
        format!("--unified={context}"),
    ];
    match mode {
        "working" => {}
        "staged" => owned.push("--cached".into()),
        "refs" => {
            let base = resolve_commit_ref(&repo.root, &validated_ref(arguments, "base_ref")?)?;
            let head = resolve_commit_ref(&repo.root, &validated_ref(arguments, "head_ref")?)?;
            owned.push(base);
            owned.push(head);
        }
        _ => return Err(McpError::InvalidRequest("git diff mode is invalid".into())),
    }
    let requested_path = validated_optional_path(arguments, &repo, "path")?;
    if requested_path.is_some() {
        reject_protected_diff_renames(&repo.root, mode, &owned)?;
    } else {
        reject_protected_diff_changes(&repo.root, mode, &owned)?;
    }
    let snapshot = git_snapshot(&repo.root, mode, &owned)?;
    if let Some(path) = requested_path {
        owned.push("--".into());
        owned.push(path);
    } else {
        append_protected_exclusions(&mut owned);
    }
    let refs = owned.iter().map(String::as_str).collect::<Vec<_>>();
    let (text, source_truncated) = run_git_text_bounded(&repo.root, &refs, MAX_GIT_OUTPUT_BYTES)?;
    let (text, continuation) = paginate_git_text(
        arguments,
        config,
        "git_diff",
        &repo.root,
        text,
        Some(&snapshot),
    )?;
    Ok(GitTextResult {
        repository_root: repo.relative_root,
        text,
        truncated: continuation.is_some() || source_truncated,
        continuation,
    })
}

fn git_log(arguments: &Value, config: &ServerConfig) -> Result<GitLogResult, McpError> {
    let repo = resolve_repo(arguments, config)?;
    let max = bounded_results(arguments);
    let mut owned = vec![
        "log".to_string(),
        "--no-show-signature".into(),
        "--format=%H%x1f%P%x1f%ct%x1f%s%x1e".into(),
        format!("--max-count={}", crate::continuation::MAX_TOTAL_ENTRIES + 1),
    ];
    let log_ref = arguments
        .get("ref")
        .and_then(Value::as_str)
        .unwrap_or("HEAD");
    let resolved_log_ref = resolve_commit_ref(&repo.root, log_ref)?;
    owned.push(resolved_log_ref.clone());
    if let Some(path) = validated_optional_path(arguments, &repo, "path")? {
        owned.push("--".into());
        owned.push(path)
    } else {
        append_protected_exclusions(&mut owned);
    }
    let refs = owned.iter().map(String::as_str).collect::<Vec<_>>();
    let out = run_git(&repo.root, &refs, MAX_GIT_OUTPUT_BYTES)?;
    let text = std::str::from_utf8(&out).map_err(|_| invalid_git_output())?;
    let mut commits = Vec::new();
    for rec in text
        .split('\x1e')
        .filter(|r| !r.trim().is_empty())
        .take(crate::continuation::MAX_TOTAL_ENTRIES + 1)
    {
        let mut f = rec.trim_start_matches('\n').splitn(4, '\x1f');
        commits.push(GitCommit {
            sha: f.next().unwrap_or("").to_owned(),
            parents: f
                .next()
                .unwrap_or("")
                .split_whitespace()
                .map(str::to_owned)
                .collect(),
            timestamp: f.next().unwrap_or("0").parse().unwrap_or(0),
            subject: f.next().unwrap_or("").trim_end().to_owned(),
        });
    }
    let truncated = commits.len() > crate::continuation::MAX_TOTAL_ENTRIES;
    commits.truncate(crate::continuation::MAX_TOTAL_ENTRIES);
    let root_scope = repo.root.to_string_lossy().into_owned();
    let (commits, continuation) = crate::continuation::paginate(
        arguments,
        commits,
        max,
        config,
        "git_log",
        &root_scope,
        Some(&resolved_log_ref),
    )?;
    Ok(GitLogResult {
        repository_root: repo.relative_root,
        commits,
        truncated: continuation.is_some() || truncated,
        continuation,
    })
}

fn git_show(arguments: &Value, config: &ServerConfig) -> Result<GitTextResult, McpError> {
    let repo = resolve_repo(arguments, config)?;
    let reference = validated_ref(arguments, "ref")?;
    let include_patch = arguments
        .get("include_patch")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let mut owned = vec![
        "show".to_string(),
        "--no-ext-diff".into(),
        "--no-textconv".into(),
        "--no-show-signature".into(),
        "--format=fuller".into(),
    ];
    if !include_patch {
        owned.push("--no-patch".into())
    }
    let resolved_reference = resolve_commit_ref(&repo.root, &reference)?;
    let requested_path = validated_optional_path(arguments, &repo, "path")?;
    if include_patch {
        if requested_path.is_some() {
            reject_protected_commit_renames(&repo.root, &resolved_reference)?;
        } else {
            reject_protected_commit_changes(&repo.root, &resolved_reference)?;
        }
    }
    owned.push(resolved_reference.clone());
    if let Some(path) = requested_path {
        owned.push("--".into());
        owned.push(path)
    } else {
        append_protected_exclusions(&mut owned);
    }
    let refs = owned.iter().map(String::as_str).collect::<Vec<_>>();
    let (text, source_truncated) = run_git_text_bounded(&repo.root, &refs, MAX_GIT_OUTPUT_BYTES)?;
    let (text, continuation) = paginate_git_text(
        arguments,
        config,
        "git_show",
        &repo.root,
        text,
        Some(&resolved_reference),
    )?;
    Ok(GitTextResult {
        repository_root: repo.relative_root,
        text,
        truncated: continuation.is_some() || source_truncated,
        continuation,
    })
}

mod context;
pub(crate) use context::resolve_git_workspace;
use context::*;
mod parse;
use parse::*;
mod security;
use security::*;
mod process;
use process::*;
mod advanced;
mod branch;
mod forge;
mod forge_process;
mod mutation;
mod remote;
mod remote_process;
mod stash;
mod tag;
mod worktree;

fn append_protected_exclusions(args: &mut Vec<String>) {
    args.push("--".into());
    args.extend(relay_core::protected_paths::git_exclusion_pathspecs());
}
