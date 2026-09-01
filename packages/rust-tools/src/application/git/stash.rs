//! Structured Git stash operations.

use super::context::resolve_repo;
use super::process::{run_git, validated_path_list};
use crate::core::config::ServerConfig;
use crate::core::error::McpError;
use serde::Serialize;
use serde_json::{json, Value};
use std::path::Path;

#[derive(Debug, Serialize, Clone)]
pub struct StashEntry {
    pub index: usize,
    pub stash_ref: String,
    pub commit_sha: String,
    pub message: String,
}

pub fn git_stash_list(arguments: &Value, config: &ServerConfig) -> Result<Value, McpError> {
    let repo = resolve_repo(arguments, config)?;
    let out = run_git(
        &repo.root,
        &["stash", "list", "--format=%gd%x00%H%x00%gs"],
        65536,
    )?;
    let text = std::str::from_utf8(&out).unwrap_or("");
    let mut stashes = Vec::new();

    for line in text.lines() {
        let parts: Vec<&str> = line.split('\0').collect();
        if parts.len() >= 3 {
            let stash_ref = parts[0];
            let index = stash_ref
                .trim_start_matches("stash@{")
                .trim_end_matches('}')
                .parse::<usize>()
                .unwrap_or(0);
            stashes.push(StashEntry {
                index,
                stash_ref: stash_ref.to_string(),
                commit_sha: parts[1].to_string(),
                message: parts[2].to_string(),
            });
        }
    }

    Ok(json!({
        "stashes": stashes,
        "total": stashes.len()
    }))
}

pub fn git_stash_push(arguments: &Value, config: &ServerConfig) -> Result<Value, McpError> {
    let repo = resolve_repo(arguments, config)?;
    super::mutation::validate_mutation_config(&repo.root)?;
    let mut args: Vec<String> = vec!["stash".into(), "push".into()];

    if let Some(msg) = arguments.get("message").and_then(Value::as_str) {
        if msg.len() > 4096 || msg.contains('\0') {
            return Err(McpError::InvalidRequest("stash message is invalid".into()));
        }
        args.extend(["-m".into(), msg.into()]);
    }
    if arguments
        .get("include_untracked")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        args.push("--include-untracked".into());
    }
    if arguments
        .get("keep_index")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        args.push("--keep-index".into());
    }

    args.push("--".into());
    if let Some(paths_val) = arguments
        .get("paths")
        .and_then(Value::as_array)
        .filter(|paths| !paths.is_empty())
    {
        args.extend(validated_path_list(paths_val, &repo)?);
    } else {
        args.push(".".into());
    }
    args.extend(crate::core::protected_paths::git_mutation_exclusion_pathspecs());

    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let out = run_git(&repo.root, &args_ref, 8192)?;
    let output_msg = std::str::from_utf8(&out).unwrap_or("").trim().to_string();

    Ok(json!({
        "pushed": true,
        "message": output_msg,
    }))
}

fn stash_ref(arguments: &Value) -> Result<(u64, String), McpError> {
    let index = arguments.get("index").and_then(Value::as_u64).unwrap_or(0);
    if index > 10_000 {
        return Err(McpError::InvalidRequest(
            "stash index exceeds maximum".into(),
        ));
    }
    Ok((index, format!("stash@{{{index}}}")))
}

fn reject_protected_stash_changes(root: &Path, reference: &str) -> Result<(), McpError> {
    let parents = run_git(root, &["rev-list", "--parents", "-n", "1", reference], 512)?;
    let parents = std::str::from_utf8(&parents)
        .map_err(|_| McpError::InvalidRequest("stash metadata is invalid".into()))?;
    let commits = parents.split_whitespace().collect::<Vec<_>>();
    if commits.len() < 2 {
        return Err(McpError::InvalidRequest("stash metadata is invalid".into()));
    }
    let tracked = run_git(
        root,
        &["diff", "--name-only", "-z", commits[1], commits[0]],
        65536,
    )?;
    reject_protected_path_bytes(&tracked)?;
    if commits.len() >= 4 {
        let untracked = run_git(
            root,
            &["ls-tree", "-r", "--name-only", "-z", commits[3]],
            65536,
        )?;
        reject_protected_path_bytes(&untracked)?;
    }
    Ok(())
}

fn reject_protected_path_bytes(paths: &[u8]) -> Result<(), McpError> {
    for path in paths
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
    {
        let path = std::str::from_utf8(path)
            .map_err(|_| McpError::InvalidRequest("stash path metadata is invalid".into()))?;
        if crate::core::protected_paths::is_protected_relative(Path::new(path)) {
            return Err(McpError::InvalidRequest(
                "stash references a protected credential path".into(),
            ));
        }
    }
    Ok(())
}

pub fn git_stash_pop(arguments: &Value, config: &ServerConfig) -> Result<Value, McpError> {
    let repo = resolve_repo(arguments, config)?;
    super::mutation::validate_mutation_config(&repo.root)?;
    let (index, stash_ref) = stash_ref(arguments)?;
    reject_protected_stash_changes(&repo.root, &stash_ref)?;
    run_git(&repo.root, &["stash", "pop", &stash_ref], 8192)?;

    Ok(json!({
        "popped": true,
        "index": index,
    }))
}

pub fn git_stash_apply(arguments: &Value, config: &ServerConfig) -> Result<Value, McpError> {
    let repo = resolve_repo(arguments, config)?;
    super::mutation::validate_mutation_config(&repo.root)?;
    let (index, stash_ref) = stash_ref(arguments)?;
    reject_protected_stash_changes(&repo.root, &stash_ref)?;
    run_git(&repo.root, &["stash", "apply", &stash_ref], 8192)?;

    Ok(json!({
        "applied": true,
        "index": index,
    }))
}

pub fn git_stash_drop(arguments: &Value, config: &ServerConfig) -> Result<Value, McpError> {
    let repo = resolve_repo(arguments, config)?;
    super::mutation::validate_mutation_config(&repo.root)?;
    let (index, stash_ref) = stash_ref(arguments)?;
    reject_protected_stash_changes(&repo.root, &stash_ref)?;
    run_git(&repo.root, &["stash", "drop", &stash_ref], 8192)?;

    Ok(json!({
        "dropped": true,
        "index": index,
    }))
}
