use super::*;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub(super) struct GitBranchListResult {
    repository_root: String,
    branches: Vec<GitBranch>,
    truncated: bool,
}

#[derive(Debug, Serialize)]
struct GitBranch {
    name: String,
    head: String,
    current: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    upstream: Option<String>,
}

#[derive(Debug, Serialize)]
pub(super) struct GitBranchMutationResult {
    repository_root: String,
    operation: &'static str,
    branch: String,
    head: String,
}

pub(super) fn git_branch_list(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<GitBranchListResult, McpError> {
    let repo = resolve_repo(arguments, config)?;
    let output = run_git(
        &repo.root,
        &[
            "for-each-ref",
            "--sort=refname",
            "--format=%(refname:short)%09%(objectname)%09%(HEAD)%09%(upstream:short)",
            "refs/heads",
        ],
        MAX_GIT_OUTPUT_BYTES,
    )?;
    let text = std::str::from_utf8(&output).map_err(|_| invalid_git_output())?;
    let mut branches = Vec::new();
    let mut truncated = false;
    for line in text.lines() {
        let mut fields = line.splitn(4, '\t');
        let name = fields.next().unwrap_or_default();
        let head = fields.next().unwrap_or_default();
        let current = fields.next().unwrap_or_default() == "*";
        let upstream = fields.next().unwrap_or_default();
        if validate_ref(name).is_err()
            || head.len() != 40
            || !head.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(invalid_git_output());
        }
        if branches.len() >= MAX_GIT_RESULTS {
            truncated = true;
            break;
        }
        branches.push(GitBranch {
            name: name.to_owned(),
            head: head.to_owned(),
            current,
            upstream: (!upstream.is_empty()).then(|| upstream.to_owned()),
        });
    }
    Ok(GitBranchListResult {
        repository_root: repo.relative_root,
        branches,
        truncated,
    })
}

pub(super) fn git_branch_create(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<GitBranchMutationResult, McpError> {
    let repo = resolve_repo(arguments, config)?;
    mutation::validate_mutation_config(&repo.root)?;
    let branch = validated_branch_name(arguments, "name")?;
    ensure_local_branch_name_available(&repo.root, &branch)?;
    let start_point = arguments
        .get("start_point")
        .and_then(Value::as_str)
        .unwrap_or("HEAD");
    let start_commit = resolve_commit_ref(&repo.root, start_point)?;
    run_git(
        &repo.root,
        &["branch", "--no-track", &branch, &start_commit],
        8192,
    )?;
    let head = resolve_commit_ref(&repo.root, &branch)?;
    Ok(GitBranchMutationResult {
        repository_root: repo.relative_root,
        operation: "create",
        branch,
        head,
    })
}

pub(super) fn git_branch_switch(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<GitBranchMutationResult, McpError> {
    let repo = resolve_repo(arguments, config)?;
    mutation::validate_mutation_config(&repo.root)?;
    let branch = validated_branch_name(arguments, "name")?;
    ensure_local_branch_exists(&repo.root, &branch)?;
    ensure_worktree_switchable(&repo.root)?;
    run_git(&repo.root, &["switch", "--no-guess", "--", &branch], 8192)?;
    let head = resolve_commit_ref(&repo.root, "HEAD")?;
    Ok(GitBranchMutationResult {
        repository_root: repo.relative_root,
        operation: "switch",
        branch,
        head,
    })
}

pub(super) fn validate_branch_name(value: &str) -> Result<String, McpError> {
    let branch = validate_ref(value)?;
    if branch == "HEAD" || branch.starts_with("refs/") {
        return Err(McpError::InvalidRequest(
            "git branch name is invalid".into(),
        ));
    }
    Ok(branch)
}

fn validated_branch_name(arguments: &Value, key: &str) -> Result<String, McpError> {
    validate_branch_name(
        arguments
            .get(key)
            .and_then(Value::as_str)
            .ok_or_else(|| McpError::InvalidRequest(format!("{key} is required")))?,
    )
}

fn ensure_local_branch_name_available(root: &Path, branch: &str) -> Result<(), McpError> {
    let full_ref = format!("refs/heads/{branch}");
    let status = git_command(root)
        .args(["show-ref", "--verify", "--quiet", &full_ref])
        .status()
        .map_err(|_| McpError::Internal("failed to inspect git branch".into()))?;
    if status.success() {
        return Err(McpError::InvalidRequest("git branch already exists".into()));
    }
    if status.code() == Some(1) {
        Ok(())
    } else {
        Err(McpError::InvalidRequest(
            "git branch availability could not be verified".into(),
        ))
    }
}

fn ensure_local_branch_exists(root: &Path, branch: &str) -> Result<(), McpError> {
    let full_ref = format!("refs/heads/{branch}");
    let status = git_command(root)
        .args(["show-ref", "--verify", "--quiet", &full_ref])
        .status()
        .map_err(|_| McpError::Internal("failed to inspect git branch".into()))?;
    if status.success() {
        Ok(())
    } else {
        Err(McpError::InvalidRequest(
            "local git branch does not exist".into(),
        ))
    }
}

fn ensure_worktree_switchable(root: &Path) -> Result<(), McpError> {
    let output = run_git(
        root,
        &["status", "--porcelain=v2", "-z"],
        MAX_GIT_OUTPUT_BYTES,
    )?;
    if output.is_empty() {
        Ok(())
    } else {
        Err(McpError::InvalidRequest(
            "git branch switch requires a clean worktree".into(),
        ))
    }
}
