use super::super::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub(in crate::git) const MAX_TITLE_BYTES: usize = 256;
pub(in crate::git) const MAX_BODY_BYTES: usize = 64 * 1024;
pub(in crate::git) const MAX_LABEL_NAME_BYTES: usize = 128;
pub(in crate::git) const MAX_ISSUES: usize = 50;
pub(in crate::git) const MAX_LABEL_FILTER_COUNT: usize = 10;
pub(in crate::git) const MAX_LABELS_PER_ISSUE: usize = 50;

#[derive(Debug, Clone, Serialize)]
pub(in crate::git) struct ForgeRepository {
    pub(in crate::git) provider: &'static str,
    pub(in crate::git) owner: String,
    pub(in crate::git) repository: String,
}

pub(in crate::git) fn forge_identity(remote: &remote::GitRemoteIdentity) -> ForgeRepository {
    ForgeRepository {
        provider: remote.provider,
        owner: remote.owner.clone(),
        repository: remote.repository.clone(),
    }
}

pub(in crate::git) fn repo_spec(remote: &remote::GitRemoteIdentity) -> String {
    format!("{}/{}", remote.owner, remote.repository)
}

pub(in crate::git) fn parse_json<T: serde::de::DeserializeOwned>(
    output: &[u8],
) -> Result<T, McpError> {
    serde_json::from_slice(output)
        .map_err(|_| McpError::InvalidRequest("forge output is invalid".into()))
}

pub(in crate::git) fn bounded_text(
    arguments: &Value,
    key: &str,
    max: usize,
    allow_empty: bool,
) -> Result<String, McpError> {
    let value = arguments
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::InvalidRequest(format!("{key} is required")))?;
    if value.len() > max || value.contains('\0') || (!allow_empty && value.trim().is_empty()) {
        return Err(McpError::InvalidRequest(format!("{key} is invalid")));
    }
    Ok(value.to_owned())
}

pub(in crate::git) fn requested_number(
    arguments: &Value,
    entity_name: &str,
) -> Result<u64, McpError> {
    arguments
        .get("number")
        .and_then(Value::as_u64)
        .filter(|number| *number > 0)
        .ok_or_else(|| McpError::InvalidRequest(format!("{entity_name} number is required")))
}

pub(in crate::git) fn parse_label_array(
    arguments: &Value,
    key: &str,
    max_count: usize,
) -> Result<Vec<String>, McpError> {
    let Some(raw) = arguments.get(key) else {
        return Ok(Vec::new());
    };
    let array = raw
        .as_array()
        .ok_or_else(|| McpError::InvalidRequest("issue labels are invalid".into()))?;
    if array.len() > max_count {
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::git) struct ProviderAuthor {
    pub(in crate::git) login: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::git) struct ProviderLabel {
    pub(in crate::git) name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::git) struct ProviderIssue {
    pub(in crate::git) number: u64,
    pub(in crate::git) title: String,
    pub(in crate::git) url: String,
    pub(in crate::git) state: String,
    #[serde(default)]
    pub(in crate::git) state_reason: Option<String>,
    #[serde(default)]
    pub(in crate::git) author: Option<ProviderAuthor>,
    #[serde(default)]
    pub(in crate::git) labels: Vec<ProviderLabel>,
    #[serde(default)]
    pub(in crate::git) created_at: String,
    #[serde(default)]
    pub(in crate::git) updated_at: String,
    #[serde(default)]
    pub(in crate::git) closed_at: Option<String>,
    #[serde(default)]
    pub(in crate::git) body: Option<String>,
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

pub(in crate::git) fn validate_issue_summary(
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

pub(in crate::git) fn validate_issue_detail(
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

pub(in crate::git) fn parse_created_issue_number(
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

pub(in crate::git) fn parse_comment_url(
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

pub(in crate::git) fn verify_closed_state(
    detail: &IssueDetail,
    requested_reason: &str,
) -> Result<(), McpError> {
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

pub(in crate::git) fn summary_fields() -> &'static str {
    "number,title,url,state,stateReason,author,labels,createdAt,updatedAt,closedAt"
}

pub(in crate::git) fn detail_fields() -> &'static str {
    "number,title,url,state,stateReason,author,labels,createdAt,updatedAt,closedAt,body"
}
