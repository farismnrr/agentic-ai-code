//! Advanced Git repository operations: restore, clean, cherry-pick, revert, reset, branch rename.

use super::context::resolve_repo;
use super::process::{resolve_commit_ref, run_git, validate_ref, validated_path_list};
use super::security::{reject_protected_commit_changes, reject_protected_diff_changes};
use relay_core::config::ServerConfig;
use relay_core::error::McpError;
use serde_json::{json, Value};

pub fn git_branch_rename(arguments: &Value, config: &ServerConfig) -> Result<Value, McpError> {
    let repo = resolve_repo(arguments, config)?;
    super::mutation::validate_mutation_config(&repo.root)?;
    let new_name = arguments
        .get("new_name")
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::InvalidRequest("new_name is required".into()))?;
    validate_ref(new_name)?;

    let old_name = arguments.get("old_name").and_then(Value::as_str);
    if let Some(old) = old_name {
        validate_ref(old)?;
    }

    let force = arguments
        .get("force")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let flag = if force { "-M" } else { "-m" };

    let mut args: Vec<&str> = vec!["branch", flag];
    if let Some(old) = old_name {
        args.push(old);
    }
    args.push(new_name);

    run_git(&repo.root, &args, 8192)?;

    Ok(json!({
        "new_name": new_name,
        "renamed": true,
    }))
}

pub fn git_restore(arguments: &Value, config: &ServerConfig) -> Result<Value, McpError> {
    let repo = resolve_repo(arguments, config)?;
    super::mutation::validate_mutation_config(&repo.root)?;
    let paths_val = arguments
        .get("paths")
        .and_then(Value::as_array)
        .ok_or_else(|| McpError::InvalidRequest("paths array is required".into()))?;

    let paths = validated_path_list(paths_val, &repo)?;

    let staged = arguments
        .get("staged")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let source = arguments.get("source").and_then(Value::as_str);
    if let Some(src) = source {
        validate_ref(src)?;
    }

    let mut args: Vec<String> = vec!["restore".into()];
    if staged {
        args.push("--staged".into());
    }
    if let Some(src) = source {
        args.extend(["--source".into(), src.into()]);
    }
    args.push("--".into());

    args.extend(paths);
    args.extend(relay_core::protected_paths::git_mutation_exclusion_pathspecs());

    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    run_git(&repo.root, &args_ref, 8192)?;

    Ok(json!({
        "restored": true,
        "staged": staged,
        "total_paths": paths_val.len(),
    }))
}

pub fn git_clean(arguments: &Value, config: &ServerConfig) -> Result<Value, McpError> {
    let repo = resolve_repo(arguments, config)?;
    super::mutation::validate_mutation_config(&repo.root)?;
    let dry_run = arguments
        .get("dry_run")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let directories = arguments
        .get("directories")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let mut args: Vec<String> = vec!["clean".into()];
    if dry_run {
        args.push("-n".into());
    } else {
        args.push("-f".into());
    }
    if directories {
        args.push("-d".into());
    }

    for pattern in relay_core::protected_paths::git_clean_exclusion_patterns() {
        args.extend(["-e".into(), pattern]);
    }
    if let Some(paths_val) = arguments.get("paths").and_then(Value::as_array) {
        if !paths_val.is_empty() {
            args.push("--".into());
            args.extend(validated_path_list(paths_val, &repo)?);
        }
    }

    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let out = run_git(&repo.root, &args_ref, 65536)?;
    let text = std::str::from_utf8(&out).unwrap_or("").trim().to_string();

    Ok(json!({
        "cleaned": !dry_run,
        "dry_run": dry_run,
        "output": text,
    }))
}

pub fn git_cherry_pick(arguments: &Value, config: &ServerConfig) -> Result<Value, McpError> {
    let repo = resolve_repo(arguments, config)?;
    super::mutation::validate_mutation_config(&repo.root)?;
    let commit = arguments
        .get("commit")
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::InvalidRequest("commit is required".into()))?;
    let commit = resolve_commit_ref(&repo.root, commit)?;
    reject_protected_commit_changes(&repo.root, &commit)?;

    let no_commit = arguments
        .get("no_commit")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let mut args: Vec<&str> = vec!["cherry-pick"];
    if no_commit {
        args.push("--no-commit");
    }
    args.push(&commit);

    run_git(&repo.root, &args, 8192)?;

    Ok(json!({
        "cherry_picked": true,
        "commit": commit,
        "no_commit": no_commit,
    }))
}

pub fn git_revert(arguments: &Value, config: &ServerConfig) -> Result<Value, McpError> {
    let repo = resolve_repo(arguments, config)?;
    super::mutation::validate_mutation_config(&repo.root)?;
    let commit = arguments
        .get("commit")
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::InvalidRequest("commit is required".into()))?;
    let commit = resolve_commit_ref(&repo.root, commit)?;
    reject_protected_commit_changes(&repo.root, &commit)?;

    let no_commit = arguments
        .get("no_commit")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let mut args: Vec<&str> = vec!["revert", "--no-edit"];
    if no_commit {
        args.push("--no-commit");
    }
    args.push(&commit);

    run_git(&repo.root, &args, 8192)?;

    Ok(json!({
        "reverted": true,
        "commit": commit,
        "no_commit": no_commit,
    }))
}

pub fn git_reset(arguments: &Value, config: &ServerConfig) -> Result<Value, McpError> {
    let repo = resolve_repo(arguments, config)?;
    super::mutation::validate_mutation_config(&repo.root)?;
    let target = arguments
        .get("target")
        .and_then(Value::as_str)
        .unwrap_or("HEAD");
    let target = resolve_commit_ref(&repo.root, target)?;

    let mode = arguments
        .get("mode")
        .and_then(Value::as_str)
        .unwrap_or("mixed");
    if mode != "soft" && mode != "mixed" {
        return Err(McpError::InvalidRequest(
            "reset mode must be 'soft' or 'mixed' (hard reset is not permitted)".into(),
        ));
    }

    let mut args: Vec<String> = vec!["reset".into()];
    let path_values = arguments.get("paths").and_then(Value::as_array);
    if let Some(paths_val) = path_values.filter(|values| !values.is_empty()) {
        if mode != "mixed" {
            return Err(McpError::InvalidRequest(
                "path-limited reset only supports mixed mode".into(),
            ));
        }
        let paths = validated_path_list(paths_val, &repo)?;
        args.push(target.clone());
        args.push("--".into());
        args.extend(paths);
        args.extend(relay_core::protected_paths::git_mutation_exclusion_pathspecs());
    } else {
        let head = resolve_commit_ref(&repo.root, "HEAD")?;
        reject_protected_diff_changes(&repo.root, "refs", &[head, target.clone()])?;
        args.push(if mode == "soft" { "--soft" } else { "--mixed" }.into());
        args.push(target.clone());
    }

    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    run_git(&repo.root, &args_ref, 8192)?;

    Ok(json!({
        "reset": true,
        "target": target,
        "mode": mode,
    }))
}

pub fn git_remote_add(arguments: &Value, config: &ServerConfig) -> Result<Value, McpError> {
    let repo = resolve_repo(arguments, config)?;
    super::mutation::validate_mutation_config(&repo.root)?;
    let name = arguments
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::InvalidRequest("name is required".into()))?;
    validate_ref(name)?;

    let url = arguments
        .get("url")
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::InvalidRequest("url is required".into()))?;
    validate_remote_url(url)?;

    run_git(&repo.root, &["remote", "add", name, url], 8192)?;

    Ok(json!({
        "name": name,
        "url": url,
        "added": true,
    }))
}

pub fn git_remote_remove(arguments: &Value, config: &ServerConfig) -> Result<Value, McpError> {
    let repo = resolve_repo(arguments, config)?;
    super::mutation::validate_mutation_config(&repo.root)?;
    let name = arguments
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::InvalidRequest("name is required".into()))?;
    validate_ref(name)?;

    run_git(&repo.root, &["remote", "remove", name], 8192)?;

    Ok(json!({
        "name": name,
        "removed": true,
    }))
}

pub fn git_remote_set_url(arguments: &Value, config: &ServerConfig) -> Result<Value, McpError> {
    let repo = resolve_repo(arguments, config)?;
    super::mutation::validate_mutation_config(&repo.root)?;
    let name = arguments
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::InvalidRequest("name is required".into()))?;
    validate_ref(name)?;

    let url = arguments
        .get("url")
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::InvalidRequest("url is required".into()))?;
    validate_remote_url(url)?;

    run_git(&repo.root, &["remote", "set-url", name, url], 8192)?;

    Ok(json!({
        "name": name,
        "url": url,
        "updated": true,
    }))
}

fn validate_remote_url(url_str: &str) -> Result<(), McpError> {
    if url_str.is_empty() || url_str.len() > 2048 || url_str.contains(['\0', '\n', '\r']) {
        return Err(McpError::InvalidRequest("invalid remote url".into()));
    }
    if let Some(scp) = url_str.strip_prefix("git@") {
        let Some((host, path)) = scp.split_once(':') else {
            return Err(McpError::InvalidRequest("invalid SSH remote url".into()));
        };
        if host.is_empty()
            || path.is_empty()
            || host.contains(['/', '\\', '@', ' ', '\t'])
            || path.contains("..")
            || path.contains(['\\', ' ', '\t'])
        {
            return Err(McpError::InvalidRequest("invalid SSH remote url".into()));
        }
        return Ok(());
    }
    let parsed = url::Url::parse(url_str)
        .map_err(|_| McpError::InvalidRequest("malformed remote URL".into()))?;
    if !matches!(parsed.scheme(), "https" | "ssh")
        || parsed.host_str().is_none()
        || parsed.password().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
        || (parsed.scheme() == "https" && !parsed.username().is_empty())
        || parsed.path().is_empty()
        || parsed.path() == "/"
    {
        return Err(McpError::InvalidRequest(
            "remote URL must use credential-free HTTPS or SSH".into(),
        ));
    }
    Ok(())
}
