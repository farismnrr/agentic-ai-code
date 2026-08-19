mod model;
mod validation;

pub(in crate::git) use model::*;
use validation::*;

use super::super::*;
use super::common::*;
use serde_json::Value;

pub(in crate::git) async fn issue_list(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<IssueListResult, McpError> {
    let repo = resolve_repo(arguments, config)?;
    let remote = remote::requested_remote(&repo.root, arguments)?;
    let state = arguments
        .get("state")
        .and_then(Value::as_str)
        .unwrap_or("open");
    if !matches!(state, "open" | "closed" | "all") {
        return Err(McpError::InvalidRequest("issue state is invalid".into()));
    }
    let label_filters = parse_label_array(arguments, "labels", MAX_LABEL_FILTER_COUNT)?;
    let repo_spec = repo_spec(&remote);
    let mut args = vec![
        "issue".into(),
        "list".into(),
        "--repo".into(),
        repo_spec,
        "--state".into(),
        state.into(),
        "--limit".into(),
        (MAX_ISSUES + 1).to_string(),
        "--json".into(),
        summary_fields().into(),
    ];
    for label in &label_filters {
        args.push("--label".into());
        args.push(label.clone());
    }
    let output = forge_process::run_gh(&repo.root, &args, &[]).await?;
    let raw_list: Vec<ProviderIssue> = parse_json(&output)?;
    let truncated = raw_list.len() > MAX_ISSUES;
    let mut issues = Vec::with_capacity(raw_list.len().min(MAX_ISSUES));
    for item in raw_list.into_iter().take(MAX_ISSUES) {
        issues.push(validate_issue_summary(&item, &remote)?);
    }
    Ok(IssueListResult {
        repository_root: repo.relative_root,
        forge: forge_identity(&remote),
        issues,
        truncated,
    })
}

pub(in crate::git) async fn issue_get(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<IssueResult, McpError> {
    let repo = resolve_repo(arguments, config)?;
    let remote = remote::requested_remote(&repo.root, arguments)?;
    let number = requested_number(arguments, "issue")?;
    let issue = get_detail(&repo.root, &remote, number).await?;
    Ok(IssueResult {
        repository_root: repo.relative_root,
        forge: forge_identity(&remote),
        issue,
    })
}

pub(in crate::git) async fn issue_create(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<IssueResult, McpError> {
    let repo = resolve_repo(arguments, config)?;
    let remote = remote::requested_remote(&repo.root, arguments)?;
    let title = bounded_text(arguments, "title", MAX_TITLE_BYTES, false)?;
    let body = arguments
        .get("body")
        .map(|_| bounded_text(arguments, "body", MAX_BODY_BYTES, true))
        .transpose()?
        .unwrap_or_default();
    let labels = parse_label_array(arguments, "labels", MAX_LABELS_PER_ISSUE)?;
    let repo_spec = repo_spec(&remote);
    let mut args = vec![
        "issue".into(),
        "create".into(),
        "--repo".into(),
        repo_spec,
        "--title".into(),
        title,
        "--body".into(),
        body,
    ];
    for label in labels {
        args.push("--label".into());
        args.push(label);
    }
    let output = forge_process::run_gh(&repo.root, &args, &[]).await?;
    let issue_number = parse_created_issue_number(&output, &remote)?;
    let issue = get_detail(&repo.root, &remote, issue_number).await?;
    Ok(IssueResult {
        repository_root: repo.relative_root,
        forge: forge_identity(&remote),
        issue,
    })
}

pub(in crate::git) async fn issue_update(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<IssueResult, McpError> {
    let repo = resolve_repo(arguments, config)?;
    let remote = remote::requested_remote(&repo.root, arguments)?;
    let number = requested_number(arguments, "issue")?;
    let title = arguments
        .get("title")
        .map(|_| bounded_text(arguments, "title", MAX_TITLE_BYTES, false))
        .transpose()?;
    let body = arguments
        .get("body")
        .map(|_| bounded_text(arguments, "body", MAX_BODY_BYTES, true))
        .transpose()?;
    let add_labels = parse_label_array(arguments, "add_labels", MAX_LABELS_PER_ISSUE)?;
    let remove_labels = parse_label_array(arguments, "remove_labels", MAX_LABELS_PER_ISSUE)?;
    let changed =
        title.is_some() || body.is_some() || !add_labels.is_empty() || !remove_labels.is_empty();
    if !changed {
        return Err(McpError::InvalidRequest(
            "no issue update was supplied".into(),
        ));
    }
    let repo_spec = repo_spec(&remote);
    let mut args = vec![
        "issue".into(),
        "edit".into(),
        number.to_string(),
        "--repo".into(),
        repo_spec,
    ];
    if let Some(t) = title {
        args.push("--title".into());
        args.push(t);
    }
    if let Some(b) = body {
        args.push("--body".into());
        args.push(b);
    }
    for l in add_labels {
        args.push("--add-label".into());
        args.push(l);
    }
    for l in remove_labels {
        args.push("--remove-label".into());
        args.push(l);
    }
    let _ = forge_process::run_gh(&repo.root, &args, &[]).await?;
    let issue = get_detail(&repo.root, &remote, number).await?;
    Ok(IssueResult {
        repository_root: repo.relative_root,
        forge: forge_identity(&remote),
        issue,
    })
}

pub(in crate::git) async fn issue_comment(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<IssueCommentResult, McpError> {
    let repo = resolve_repo(arguments, config)?;
    let remote = remote::requested_remote(&repo.root, arguments)?;
    let number = requested_number(arguments, "issue")?;
    let body = bounded_text(arguments, "body", MAX_BODY_BYTES, false)?;
    let repo_spec = repo_spec(&remote);
    let args = vec![
        "issue".into(),
        "comment".into(),
        number.to_string(),
        "--repo".into(),
        repo_spec,
        "--body".into(),
        body,
    ];
    let output = forge_process::run_gh(&repo.root, &args, &[]).await?;
    let comment_url = parse_comment_url(&output, &remote, number)?;
    Ok(IssueCommentResult {
        repository_root: repo.relative_root,
        forge: forge_identity(&remote),
        issue_number: number,
        comment_url,
    })
}

pub(in crate::git) async fn issue_close(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<IssueResult, McpError> {
    let repo = resolve_repo(arguments, config)?;
    let remote = remote::requested_remote(&repo.root, arguments)?;
    let number = requested_number(arguments, "issue")?;
    let reason = arguments
        .get("reason")
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::InvalidRequest("close reason is required".into()))?;
    if !matches!(reason, "completed" | "not_planned" | "duplicate") {
        return Err(McpError::InvalidRequest("close reason is invalid".into()));
    }
    let duplicate_number = if reason == "duplicate" {
        let raw = arguments.get("duplicate_of").ok_or_else(|| {
            McpError::InvalidRequest("duplicate_of is required for duplicate close".into())
        })?;
        let target = raw
            .as_u64()
            .filter(|n| *n > 0)
            .ok_or_else(|| McpError::InvalidRequest("duplicate_of is invalid".into()))?;
        if target == number {
            return Err(McpError::InvalidRequest(
                "duplicate_of cannot reference the issue itself".into(),
            ));
        }
        Some(target)
    } else {
        if arguments.get("duplicate_of").is_some() {
            return Err(McpError::InvalidRequest(
                "duplicate_of is only valid for duplicate close".into(),
            ));
        }
        None
    };
    let comment = arguments
        .get("comment")
        .map(|_| bounded_text(arguments, "comment", MAX_BODY_BYTES, false))
        .transpose()?;
    let repo_spec = repo_spec(&remote);
    let mut args = vec![
        "issue".into(),
        "close".into(),
        number.to_string(),
        "--repo".into(),
        repo_spec,
    ];
    if reason == "completed" {
        args.push("--reason".into());
        args.push("completed".into());
    } else if reason == "not_planned" {
        args.push("--reason".into());
        args.push("not planned".into());
    } else if let Some(target) = duplicate_number {
        args.push("--duplicate-of".into());
        args.push(target.to_string());
    }
    if let Some(c) = comment {
        args.push("--comment".into());
        args.push(c);
    }
    let _ = forge_process::run_gh(&repo.root, &args, &[]).await?;
    let detail = get_detail(&repo.root, &remote, number).await?;
    verify_closed_state(&detail, reason)?;
    Ok(IssueResult {
        repository_root: repo.relative_root,
        forge: forge_identity(&remote),
        issue: detail,
    })
}

pub(in crate::git) async fn issue_reopen(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<IssueResult, McpError> {
    let repo = resolve_repo(arguments, config)?;
    let remote = remote::requested_remote(&repo.root, arguments)?;
    let number = requested_number(arguments, "issue")?;
    let comment = arguments
        .get("comment")
        .map(|_| bounded_text(arguments, "comment", MAX_BODY_BYTES, false))
        .transpose()?;
    let repo_spec = repo_spec(&remote);
    let mut args = vec![
        "issue".into(),
        "reopen".into(),
        number.to_string(),
        "--repo".into(),
        repo_spec,
    ];
    if let Some(c) = comment {
        args.push("--comment".into());
        args.push(c);
    }
    let _ = forge_process::run_gh(&repo.root, &args, &[]).await?;
    let detail = get_detail(&repo.root, &remote, number).await?;
    if detail.summary.state != "OPEN" {
        return Err(McpError::InvalidRequest(
            "issue reopen post-state is not open".into(),
        ));
    }
    Ok(IssueResult {
        repository_root: repo.relative_root,
        forge: forge_identity(&remote),
        issue: detail,
    })
}

async fn get_detail(
    root: &std::path::Path,
    remote: &remote::GitRemoteIdentity,
    number: u64,
) -> Result<IssueDetail, McpError> {
    let args = vec![
        "issue".into(),
        "view".into(),
        number.to_string(),
        "--repo".into(),
        repo_spec(remote),
        "--json".into(),
        detail_fields().into(),
    ];
    let output = forge_process::run_gh(root, &args, &[]).await?;
    let raw: ProviderIssue = parse_json(&output)?;
    validate_issue_detail(&raw, remote, number)
}
