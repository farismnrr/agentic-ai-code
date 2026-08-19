use super::super::*;
use super::common::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

const MAX_ISSUES: usize = 50;
const MAX_LABEL_FILTER_COUNT: usize = 10;
const MAX_LABEL_NAME_BYTES: usize = 128;
const MAX_LABELS_PER_ISSUE: usize = 50;
const MAX_TITLE_BYTES: usize = 256;
const MAX_BODY_BYTES: usize = 64 * 1024;

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
    comments: Vec<serde_json::Value>,
    #[serde(default)]
    is_pull_request: bool,
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
    pub comment_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::git) struct IssueDetail {
    #[serde(flatten)]
    pub summary: IssueSummary,
    pub body: String,
}

#[derive(Debug, Serialize)]
pub(in crate::git) struct IssueListResult {
    pub repository_root: String,
    pub forge: ForgeRepository,
    pub issues: Vec<IssueSummary>,
    pub truncated: bool,
}

#[derive(Debug, Serialize)]
pub(in crate::git) struct IssueResult {
    pub repository_root: String,
    pub forge: ForgeRepository,
    pub issue: IssueDetail,
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
    let label_filters = parse_label_filters(arguments)?;
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
    let number = super::common::requested_number(arguments, "issue")?;
    let repo_spec = repo_spec(&remote);
    let args = vec![
        "issue".into(),
        "view".into(),
        number.to_string(),
        "--repo".into(),
        repo_spec,
        "--json".into(),
        detail_fields().into(),
    ];
    let output = forge_process::run_gh(&repo.root, &args, &[]).await?;
    let raw_item: ProviderIssue = parse_json(&output)?;
    let detail = validate_issue_detail(&raw_item, &remote, number)?;
    Ok(IssueResult {
        repository_root: repo.relative_root,
        forge: forge_identity(&remote),
        issue: detail,
    })
}

fn validate_issue_summary(
    item: &ProviderIssue,
    remote: &remote::GitRemoteIdentity,
) -> Result<IssueSummary, McpError> {
    if item.is_pull_request {
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
    if item.url != expected_url || item.url.contains("/pull/") {
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
        comment_count: item.comments.len(),
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

fn parse_label_filters(arguments: &Value) -> Result<Vec<String>, McpError> {
    let Some(raw_labels) = arguments.get("labels") else {
        return Ok(Vec::new());
    };
    let array = raw_labels
        .as_array()
        .ok_or_else(|| McpError::InvalidRequest("issue labels are invalid".into()))?;
    if array.len() > MAX_LABEL_FILTER_COUNT {
        return Err(McpError::InvalidRequest(
            "issue labels exceed maximum".into(),
        ));
    }
    let mut labels = Vec::with_capacity(array.len());
    for item in array {
        let label = item
            .as_str()
            .ok_or_else(|| McpError::InvalidRequest("issue label is invalid".into()))?;
        if label.trim().is_empty()
            || label.len() > MAX_LABEL_NAME_BYTES
            || label.contains('\0')
            || label.contains('\n')
            || label.contains('\r')
        {
            return Err(McpError::InvalidRequest("issue label is invalid".into()));
        }
        labels.push(label.to_owned());
    }
    Ok(labels)
}

fn summary_fields() -> &'static str {
    "number,title,url,state,stateReason,author,labels,createdAt,updatedAt,closedAt,comments,isPullRequest"
}

fn detail_fields() -> &'static str {
    "number,title,url,state,stateReason,author,labels,createdAt,updatedAt,closedAt,comments,isPullRequest,body"
}
