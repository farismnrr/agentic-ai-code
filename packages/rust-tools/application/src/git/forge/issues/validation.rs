use super::model::*;
use crate::git::forge::common::{MAX_BODY_BYTES, MAX_LABEL_NAME_BYTES, MAX_TITLE_BYTES};
use crate::git::{invalid_git_output, remote, McpError};

pub(super) fn validate_issue_summary(
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

pub(super) fn validate_issue_detail(
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

pub(super) fn parse_created_issue_number(
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

pub(super) fn parse_comment_url(
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

pub(super) fn verify_closed_state(
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
