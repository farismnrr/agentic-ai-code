use super::super::*;
use super::common::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

const MAX_CHANGE_REQUESTS: usize = 50;
const MAX_CHECKS: usize = 100;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChangeRequestSummary {
    number: u64,
    title: String,
    url: String,
    state: String,
    is_draft: bool,
    base_ref_name: String,
    base_ref_oid: String,
    head_ref_name: String,
    head_ref_oid: String,
    mergeable: String,
    merge_state_status: String,
    review_decision: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct ChangeRequestListResult {
    repository_root: String,
    forge: ForgeRepository,
    change_requests: Vec<ChangeRequestSummary>,
    truncated: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct ChangeRequestResult {
    repository_root: String,
    forge: ForgeRepository,
    change_request: ChangeRequestSummary,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CheckSummary {
    bucket: String,
    name: String,
    state: String,
    workflow: String,
    link: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct ChangeRequestChecksResult {
    repository_root: String,
    forge: ForgeRepository,
    number: u64,
    checks: Vec<CheckSummary>,
    pass: usize,
    fail: usize,
    pending: usize,
    skipping: usize,
    cancelled: usize,
    truncated: bool,
}

pub(crate) async fn change_request_list(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<ChangeRequestListResult, McpError> {
    let repo = resolve_repo(arguments, config)?;
    let remote = remote::requested_remote(&repo.root, arguments)?;
    let state = arguments
        .get("state")
        .and_then(Value::as_str)
        .unwrap_or("open");
    if !matches!(state, "open" | "closed" | "merged" | "all") {
        return Err(McpError::InvalidRequest(
            "change request state is invalid".into(),
        ));
    }
    let repo_spec = repo_spec(&remote);
    let args = vec![
        "pr".into(),
        "list".into(),
        "--repo".into(),
        repo_spec,
        "--state".into(),
        state.into(),
        "--limit".into(),
        (MAX_CHANGE_REQUESTS + 1).to_string(),
        "--json".into(),
        summary_fields().into(),
    ];
    let output = forge_process::run_gh(&repo.root, &args, &[]).await?;
    let mut items: Vec<ChangeRequestSummary> = parse_json(&output)?;
    let truncated = items.len() > MAX_CHANGE_REQUESTS;
    items.truncate(MAX_CHANGE_REQUESTS);
    for item in &items {
        validate_summary(item, &remote)?;
    }
    Ok(ChangeRequestListResult {
        repository_root: repo.relative_root,
        forge: forge_identity(&remote),
        change_requests: items,
        truncated,
    })
}

pub(crate) async fn change_request_get(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<ChangeRequestResult, McpError> {
    let repo = resolve_repo(arguments, config)?;
    let remote = remote::requested_remote(&repo.root, arguments)?;
    let number = requested_number(arguments)?;
    let summary = get_summary(&repo.root, &remote, number).await?;
    Ok(ChangeRequestResult {
        repository_root: repo.relative_root,
        forge: forge_identity(&remote),
        change_request: summary,
    })
}

pub(crate) async fn change_request_create(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<ChangeRequestResult, McpError> {
    let repo = resolve_repo(arguments, config)?;
    let remote = remote::requested_remote(&repo.root, arguments)?;
    let head = requested_branch(arguments, "head_branch")?;
    let base = requested_branch(arguments, "base_branch")?;
    let title = bounded_text(arguments, "title", MAX_TITLE_BYTES, false)?;
    let body = bounded_text(arguments, "body", MAX_BODY_BYTES, true)?;
    let local_head = resolve_commit_ref(&repo.root, &format!("refs/heads/{head}"))?;
    let remote_head = remote::remote_branch_head(&repo.root, &remote, &head)
        .await?
        .ok_or_else(|| {
            McpError::InvalidRequest("change request head branch is not pushed".into())
        })?;
    if local_head != remote_head {
        return Err(McpError::InvalidRequest(
            "change request head branch does not match local branch".into(),
        ));
    }
    if remote::remote_branch_head(&repo.root, &remote, &base)
        .await?
        .is_none()
    {
        return Err(McpError::InvalidRequest(
            "change request base branch does not exist".into(),
        ));
    }
    let mut args = vec![
        "pr".into(),
        "create".into(),
        "--repo".into(),
        repo_spec(&remote),
        "--head".into(),
        head,
        "--base".into(),
        base,
        "--title".into(),
        title,
        "--body".into(),
        body,
    ];
    if arguments
        .get("draft")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        args.push("--draft".into());
    }
    let output = forge_process::run_gh(&repo.root, &args, &[]).await?;
    let number = parse_created_number(&output, &remote)?;
    let summary = get_summary(&repo.root, &remote, number).await?;
    Ok(ChangeRequestResult {
        repository_root: repo.relative_root,
        forge: forge_identity(&remote),
        change_request: summary,
    })
}

pub(crate) async fn change_request_update(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<ChangeRequestResult, McpError> {
    let repo = resolve_repo(arguments, config)?;
    let remote = remote::requested_remote(&repo.root, arguments)?;
    let number = requested_number(arguments)?;
    let mut args = vec![
        "pr".into(),
        "edit".into(),
        number.to_string(),
        "--repo".into(),
        repo_spec(&remote),
    ];
    let mut changed = false;
    if arguments.get("title").is_some() {
        args.push("--title".into());
        args.push(bounded_text(arguments, "title", MAX_TITLE_BYTES, false)?);
        changed = true;
    }
    if arguments.get("body").is_some() {
        args.push("--body".into());
        args.push(bounded_text(arguments, "body", MAX_BODY_BYTES, true)?);
        changed = true;
    }
    if arguments.get("base_branch").is_some() {
        let base = requested_branch(arguments, "base_branch")?;
        if remote::remote_branch_head(&repo.root, &remote, &base)
            .await?
            .is_none()
        {
            return Err(McpError::InvalidRequest(
                "change request base branch does not exist".into(),
            ));
        }
        args.push("--base".into());
        args.push(base);
        changed = true;
    }
    if !changed {
        return Err(McpError::InvalidRequest(
            "no change request update was supplied".into(),
        ));
    }
    forge_process::run_gh(&repo.root, &args, &[]).await?;
    let summary = get_summary(&repo.root, &remote, number).await?;
    Ok(ChangeRequestResult {
        repository_root: repo.relative_root,
        forge: forge_identity(&remote),
        change_request: summary,
    })
}

pub(crate) async fn change_request_checks(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<ChangeRequestChecksResult, McpError> {
    let repo = resolve_repo(arguments, config)?;
    let remote = remote::requested_remote(&repo.root, arguments)?;
    let number = requested_number(arguments)?;
    checks_result(repo, remote, number).await
}

pub(crate) async fn change_request_merge(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<ChangeRequestResult, McpError> {
    let repo = resolve_repo(arguments, config)?;
    let remote = remote::requested_remote(&repo.root, arguments)?;
    let number = requested_number(arguments)?;
    let expected = arguments
        .get("expected_head_sha")
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::InvalidRequest("expected_head_sha is required".into()))?;
    remote::validate_sha(expected)?;
    let strategy = arguments
        .get("strategy")
        .and_then(Value::as_str)
        .unwrap_or("squash");
    let strategy_flag = match strategy {
        "merge" => "--merge",
        "squash" => "--squash",
        "rebase" => "--rebase",
        _ => return Err(McpError::InvalidRequest("merge strategy is invalid".into())),
    };
    let before = get_summary(&repo.root, &remote, number).await?;
    if before.state != "OPEN" || before.head_ref_oid != expected {
        return Err(McpError::InvalidRequest(
            "change request state or head changed before merge".into(),
        ));
    }
    if before.is_draft
        || before.mergeable != "MERGEABLE"
        || before.review_decision == "CHANGES_REQUESTED"
    {
        return Err(McpError::InvalidRequest(
            "change request is not eligible for merge".into(),
        ));
    }
    let checks = checks_result(
        RepoContext {
            root: repo.root.clone(),
            relative_root: repo.relative_root.clone(),
            execution_root: repo.execution_root.clone(),
        },
        remote.clone(),
        number,
    )
    .await?;
    if checks.fail > 0 || checks.pending > 0 || checks.cancelled > 0 {
        return Err(McpError::InvalidRequest(
            "change request checks do not permit merge".into(),
        ));
    }
    let args = vec![
        "pr".into(),
        "merge".into(),
        number.to_string(),
        "--repo".into(),
        repo_spec(&remote),
        strategy_flag.into(),
        "--match-head-commit".into(),
        expected.into(),
    ];
    forge_process::run_gh(&repo.root, &args, &[]).await?;
    let after = get_summary(&repo.root, &remote, number).await?;
    if after.state != "MERGED" {
        return Err(McpError::InvalidRequest(
            "change request merge was not verified".into(),
        ));
    }
    Ok(ChangeRequestResult {
        repository_root: repo.relative_root,
        forge: forge_identity(&remote),
        change_request: after,
    })
}

async fn checks_result(
    repo: RepoContext,
    remote: remote::GitRemoteIdentity,
    number: u64,
) -> Result<ChangeRequestChecksResult, McpError> {
    let args = vec![
        "pr".into(),
        "checks".into(),
        number.to_string(),
        "--repo".into(),
        repo_spec(&remote),
        "--json".into(),
        "bucket,name,state,workflow,link".into(),
    ];
    let output = forge_process::run_gh(&repo.root, &args, &[8]).await?;
    let mut checks: Vec<CheckSummary> = parse_json(&output)?;
    let truncated = checks.len() > MAX_CHECKS;
    checks.truncate(MAX_CHECKS);
    let count = |bucket: &str| checks.iter().filter(|check| check.bucket == bucket).count();
    Ok(ChangeRequestChecksResult {
        repository_root: repo.relative_root,
        forge: forge_identity(&remote),
        number,
        pass: count("pass"),
        fail: count("fail"),
        pending: count("pending"),
        skipping: count("skipping"),
        cancelled: count("cancel"),
        checks,
        truncated,
    })
}

async fn get_summary(
    root: &std::path::Path,
    remote: &remote::GitRemoteIdentity,
    number: u64,
) -> Result<ChangeRequestSummary, McpError> {
    let args = vec![
        "pr".into(),
        "view".into(),
        number.to_string(),
        "--repo".into(),
        repo_spec(remote),
        "--json".into(),
        summary_fields().into(),
    ];
    let output = forge_process::run_gh(root, &args, &[]).await?;
    let summary: ChangeRequestSummary = parse_json(&output)?;
    validate_summary(&summary, remote)?;
    Ok(summary)
}

fn validate_summary(
    summary: &ChangeRequestSummary,
    remote: &remote::GitRemoteIdentity,
) -> Result<(), McpError> {
    if summary.number == 0
        || summary.title.len() > MAX_TITLE_BYTES
        || branch::validate_branch_name(&summary.base_ref_name).is_err()
        || branch::validate_branch_name(&summary.head_ref_name).is_err()
    {
        return Err(invalid_git_output());
    }
    remote::validate_sha(&summary.base_ref_oid)?;
    remote::validate_sha(&summary.head_ref_oid)?;
    let prefix = format!(
        "https://github.com/{}/{}/pull/",
        remote.owner, remote.repository
    );
    if !summary.url.starts_with(&prefix) {
        return Err(McpError::InvalidRequest(
            "change request repository identity mismatch".into(),
        ));
    }
    Ok(())
}

fn requested_number(arguments: &Value) -> Result<u64, McpError> {
    super::common::requested_number(arguments, "change request")
}

fn requested_branch(arguments: &Value, key: &str) -> Result<String, McpError> {
    branch::validate_branch_name(
        arguments
            .get(key)
            .and_then(Value::as_str)
            .ok_or_else(|| McpError::InvalidRequest(format!("{key} is required")))?,
    )
}

fn parse_created_number(
    output: &[u8],
    remote: &remote::GitRemoteIdentity,
) -> Result<u64, McpError> {
    let text = std::str::from_utf8(output)
        .map_err(|_| invalid_git_output())?
        .trim();
    let prefix = format!(
        "https://github.com/{}/{}/pull/",
        remote.owner, remote.repository
    );
    let number = text
        .strip_prefix(&prefix)
        .and_then(|value| value.trim_end_matches('/').parse::<u64>().ok())
        .filter(|value| *value > 0)
        .ok_or_else(|| {
            McpError::InvalidRequest("created change request identity is invalid".into())
        })?;
    Ok(number)
}

fn summary_fields() -> &'static str {
    "number,title,url,state,isDraft,baseRefName,baseRefOid,headRefName,headRefOid,mergeable,mergeStateStatus,reviewDecision"
}
