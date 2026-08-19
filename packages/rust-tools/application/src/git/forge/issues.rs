use super::super::*;
use super::common::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

const MAX_ISSUES: usize = 50;
const MAX_LABEL_FILTER_COUNT: usize = 10;
const MAX_LABELS_PER_ISSUE: usize = 50;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProviderAuthor {
    login: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProviderLabel {
    name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProviderIssue {
    number: u64,
    title: String,
    url: String,
    state: String,
    #[serde(default)]
    state_reason: Option<String>,
    #[serde(default)]
    author: Option<ProviderAuthor>,
    #[serde(default)]
    labels: Vec<ProviderLabel>,
    #[serde(default)]
    created_at: String,
    #[serde(default)]
    updated_at: String,
    #[serde(default)]
    closed_at: Option<String>,
    #[serde(default)]
    body: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::git) struct IssueSummary {
    pub number: u64,
    pub title: String,
    pub url: String,
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_reason: Option<String>,
    pub author: Option<String>,
    pub labels: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub closed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::git) struct IssueDetail {
    #[serde(flatten)]
    pub summary: IssueSummary,
    pub body: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::git) struct IssueListResult {
    pub repository_root: String,
    pub forge: ForgeRepository,
    pub issues: Vec<IssueSummary>,
    pub truncated: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::git) struct IssueResult {
    pub repository_root: String,
    pub forge: ForgeRepository,
    pub issue: IssueDetail,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::git) struct IssueCommentResult {
    pub repository_root: String,
    pub forge: ForgeRepository,
    pub issue_number: u64,
    pub comment_url: String,
}

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
    let mut raw_items: Vec<ProviderIssue> = parse_json(&output)?;
    let truncated = raw_items.len() > MAX_ISSUES;
    raw_items.truncate(MAX_ISSUES);
    let mut items = Vec::with_capacity(raw_items.len());
    for item in raw_items {
        items.push(validate_issue_summary(&item, &remote)?);
    }
    Ok(IssueListResult {
        repository_root: repo.relative_root,
        forge: forge_identity(&remote),
        issues: items,
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
    let detail = get_detail(&repo.root, &remote, number).await?;
    Ok(IssueResult {
        repository_root: repo.relative_root,
        forge: forge_identity(&remote),
        issue: detail,
    })
}

pub(in crate::git) async fn issue_create(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<IssueResult, McpError> {
    let repo = resolve_repo(arguments, config)?;
    let remote = remote::requested_remote(&repo.root, arguments)?;
    let title = bounded_text(arguments, "title", MAX_TITLE_BYTES, false)?;
    let body = match arguments.get("body") {
        Some(_) => bounded_text(arguments, "body", MAX_BODY_BYTES, true)?,
        None => String::new(),
    };
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
    for label in &labels {
        args.push("--label".into());
        args.push(label.clone());
    }
    let output = forge_process::run_gh(&repo.root, &args, &[]).await?;
    let number = parse_created_issue_number(&output, &remote)?;
    let detail = get_detail(&repo.root, &remote, number).await?;
    Ok(IssueResult {
        repository_root: repo.relative_root,
        forge: forge_identity(&remote),
        issue: detail,
    })
}

pub(in crate::git) async fn issue_update(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<IssueResult, McpError> {
    let repo = resolve_repo(arguments, config)?;
    let remote = remote::requested_remote(&repo.root, arguments)?;
    let number = requested_number(arguments, "issue")?;
    let repo_spec = repo_spec(&remote);
    let mut args = vec![
        "issue".into(),
        "edit".into(),
        number.to_string(),
        "--repo".into(),
        repo_spec,
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
    for label in parse_label_array(arguments, "add_labels", MAX_LABELS_PER_ISSUE)? {
        args.push("--add-label".into());
        args.push(label);
        changed = true;
    }
    for label in parse_label_array(arguments, "remove_labels", MAX_LABELS_PER_ISSUE)? {
        args.push("--remove-label".into());
        args.push(label);
        changed = true;
    }
    if !changed {
        return Err(McpError::InvalidRequest(
            "no issue update was supplied".into(),
        ));
    }
    forge_process::run_gh(&repo.root, &args, &[]).await?;
    let detail = get_detail(&repo.root, &remote, number).await?;
    Ok(IssueResult {
        repository_root: repo.relative_root,
        forge: forge_identity(&remote),
        issue: detail,
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
    let provider_reason = match reason {
        "completed" => "completed",
        "not_planned" => "not planned",
        "duplicate" => "duplicate",
        _ => return Err(McpError::InvalidRequest("close reason is invalid".into())),
    };
    let duplicate_of = arguments.get("duplicate_of");
    let duplicate_number: Option<u64> = match (reason, duplicate_of) {
        ("duplicate", Some(value)) => Some(
            value
                .as_u64()
                .filter(|number| *number > 0)
                .ok_or_else(|| McpError::InvalidRequest("duplicate_of is invalid".into()))?,
        ),
        ("duplicate", None) => {
            return Err(McpError::InvalidRequest(
                "duplicate_of is required for duplicate close".into(),
            ));
        }
        (_, Some(_)) => {
            return Err(McpError::InvalidRequest(
                "duplicate_of is only valid for duplicate close".into(),
            ));
        }
        (_, None) => None,
    };
    if duplicate_number == Some(number) {
        return Err(McpError::InvalidRequest(
            "duplicate_of cannot reference the issue itself".into(),
        ));
    }
    let repo_spec = repo_spec(&remote);
    let mut args = vec![
        "issue".into(),
        "close".into(),
        number.to_string(),
        "--repo".into(),
        repo_spec,
    ];
    if reason != "duplicate" {
        args.push("--reason".into());
        args.push(provider_reason.into());
    }
    if let Some(duplicate_number) = duplicate_number {
        args.push("--duplicate-of".into());
        args.push(duplicate_number.to_string());
    }
    if arguments.get("comment").is_some() {
        args.push("--comment".into());
        args.push(bounded_text(arguments, "comment", MAX_BODY_BYTES, false)?);
    }
    forge_process::run_gh(&repo.root, &args, &[]).await?;
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
    let repo_spec = repo_spec(&remote);
    let mut args = vec![
        "issue".into(),
        "reopen".into(),
        number.to_string(),
        "--repo".into(),
        repo_spec,
    ];
    if arguments.get("comment").is_some() {
        args.push("--comment".into());
        args.push(bounded_text(arguments, "comment", MAX_BODY_BYTES, false)?);
    }
    forge_process::run_gh(&repo.root, &args, &[]).await?;
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

fn validate_issue_summary(
    item: &ProviderIssue,
    remote: &remote::GitRemoteIdentity,
) -> Result<IssueSummary, McpError> {
    if item.url.contains("/pull/") {
        return Err(McpError::InvalidRequest(
            "pull request cannot be accessed as an issue".into(),
        ));
    }
    if item.number == 0
        || item.title.len() > MAX_TITLE_BYTES
        || item.title.contains('\0')
        || item.labels.len() > MAX_LABELS_PER_ISSUE
    {
        return Err(invalid_git_output());
    }
    let expected_url = format!(
        "https://github.com/{}/{}/issues/{}",
        remote.owner, remote.repository, item.number
    );
    if item.url != expected_url {
        return Err(McpError::InvalidRequest(
            "issue repository identity mismatch".into(),
        ));
    }
    let mut labels = Vec::with_capacity(item.labels.len());
    for label in &item.labels {
        if label.name.is_empty()
            || label.name.len() > MAX_LABEL_NAME_BYTES
            || label.name.contains('\0')
        {
            return Err(invalid_git_output());
        }
        labels.push(label.name.clone());
    }
    let author = item.author.as_ref().and_then(|a| {
        if a.login.is_empty() || a.login.contains('\0') {
            None
        } else {
            Some(a.login.clone())
        }
    });
    Ok(IssueSummary {
        number: item.number,
        title: item.title.clone(),
        url: item.url.clone(),
        state: item.state.clone(),
        state_reason: item.state_reason.clone().filter(|s| !s.is_empty()),
        author,
        labels,
        created_at: item.created_at.clone(),
        updated_at: item.updated_at.clone(),
        closed_at: item.closed_at.clone().filter(|s| !s.is_empty()),
    })
}

fn validate_issue_detail(
    item: &ProviderIssue,
    remote: &remote::GitRemoteIdentity,
    expected_number: u64,
) -> Result<IssueDetail, McpError> {
    if item.number != expected_number {
        return Err(McpError::InvalidRequest(
            "issue repository identity mismatch".into(),
        ));
    }
    let summary = validate_issue_summary(item, remote)?;
    let body = item.body.as_deref().unwrap_or("");
    if body.len() > MAX_BODY_BYTES || body.contains('\0') {
        return Err(invalid_git_output());
    }
    Ok(IssueDetail {
        summary,
        body: body.to_owned(),
    })
}

fn parse_created_issue_number(
    output: &[u8],
    remote: &remote::GitRemoteIdentity,
) -> Result<u64, McpError> {
    let text = std::str::from_utf8(output)
        .map_err(|_| invalid_git_output())?
        .trim();
    let prefix = format!(
        "https://github.com/{}/{}/issues/",
        remote.owner, remote.repository
    );
    let number = text
        .strip_prefix(&prefix)
        .and_then(|tail| tail.trim_end_matches('/').parse::<u64>().ok())
        .filter(|n| *n > 0)
        .ok_or_else(|| McpError::InvalidRequest("created issue identity is invalid".into()))?;
    Ok(number)
}

fn parse_comment_url(
    output: &[u8],
    remote: &remote::GitRemoteIdentity,
    issue_number: u64,
) -> Result<String, McpError> {
    let text = std::str::from_utf8(output)
        .map_err(|_| invalid_git_output())?
        .trim()
        .to_owned();
    let expected_prefix = format!(
        "https://github.com/{}/{}/issues/{}#issuecomment-",
        remote.owner, remote.repository, issue_number
    );
    if !text.starts_with(&expected_prefix) || text.len() > 512 || text.contains('\0') {
        return Err(McpError::InvalidRequest(
            "comment identity is invalid".into(),
        ));
    }
    let fragment = &text[expected_prefix.len()..];
    if fragment.is_empty() || !fragment.chars().all(|c| c.is_ascii_digit()) {
        return Err(McpError::InvalidRequest(
            "comment identity is invalid".into(),
        ));
    }
    Ok(text)
}

fn verify_closed_state(detail: &IssueDetail, requested_reason: &str) -> Result<(), McpError> {
    if detail.summary.state != "CLOSED" {
        return Err(McpError::InvalidRequest(
            "issue close post-state is not closed".into(),
        ));
    }
    let expected_reason = match requested_reason {
        "completed" => "COMPLETED",
        "not_planned" => "NOT_PLANNED",
        "duplicate" => "DUPLICATE",
        _ => return Err(McpError::Internal("close reason mapping is invalid".into())),
    };
    if detail.summary.state_reason.as_deref() != Some(expected_reason) {
        return Err(McpError::InvalidRequest(
            "issue close post-state reason does not match".into(),
        ));
    }
    Ok(())
}

fn summary_fields() -> &'static str {
    "number,title,url,state,stateReason,author,labels,createdAt,updatedAt,closedAt"
}

fn detail_fields() -> &'static str {
    "number,title,url,state,stateReason,author,labels,createdAt,updatedAt,closedAt,body"
}
