use super::model::*;
use crate::application::git::{invalid_git_output, remote, McpError};
use serde_json::Value;

pub(super) fn run_fields() -> &'static str {
    "databaseId,name,workflowName,workflowDatabaseId,displayTitle,event,headBranch,headSha,status,conclusion,createdAt,startedAt,updatedAt,url,attempt,number"
}
pub(super) fn validate_workflow(
    w: ProviderWorkflow,
    remote: &remote::GitRemoteIdentity,
) -> Result<WorkflowSummary, McpError> {
    if w.id == 0
        || w.name.is_empty()
        || w.name.len() > 256
        || w.path.is_empty()
        || w.path.len() > 512
        || w.name.contains('\0')
        || w.path.contains(['\0', '\n', '\r'])
    {
        return Err(invalid_git_output());
    }
    if !w.html_url.is_empty() {
        let file = w.path.rsplit('/').next().ok_or_else(invalid_git_output)?;
        let expected = format!(
            "https://github.com/{}/{}/actions/workflows/{file}",
            remote.owner, remote.repository
        );
        if w.html_url != expected {
            return Err(McpError::InvalidRequest(
                "workflow repository identity mismatch".into(),
            ));
        }
    }
    Ok(WorkflowSummary {
        id: w.id,
        name: w.name,
        path: w.path,
        state: w.state,
        url: w.html_url,
    })
}
pub(super) fn validate_run(
    r: ProviderRun,
    remote: &remote::GitRemoteIdentity,
) -> Result<RunSummary, McpError> {
    if r.database_id == 0
        || r.name.len() > 256
        || r.display_title.len() > 512
        || r.head_sha.len() > 64
        || r.url.len() > 512
        || r.name.contains('\0')
        || r.display_title.contains('\0')
    {
        return Err(invalid_git_output());
    }
    let expected = format!(
        "https://github.com/{}/{}/actions/runs/{}",
        remote.owner, remote.repository, r.database_id
    );
    if r.url != expected {
        return Err(McpError::InvalidRequest(
            "workflow run repository identity mismatch".into(),
        ));
    }
    Ok(RunSummary {
        id: r.database_id,
        number: r.number,
        attempt: r.attempt,
        workflow_id: r.workflow_database_id,
        workflow_name: r.workflow_name,
        name: r.name,
        display_title: r.display_title,
        event: r.event,
        head_branch: r.head_branch,
        head_sha: r.head_sha,
        status: r.status,
        conclusion: r.conclusion,
        created_at: r.created_at,
        started_at: r.started_at,
        updated_at: r.updated_at,
        url: r.url,
    })
}
pub(super) fn validate_job(
    mut j: ProviderJob,
    remote: &remote::GitRemoteIdentity,
) -> Result<JobSummary, McpError> {
    if j.database_id == 0
        || j.name.is_empty()
        || j.name.len() > 256
        || j.name.contains('\0')
        || j.url.len() > 512
    {
        return Err(invalid_git_output());
    }
    if !j.url.is_empty() {
        let prefix = format!(
            "https://github.com/{}/{}/actions/runs/",
            remote.owner, remote.repository
        );
        if !j.url.starts_with(&prefix) {
            return Err(McpError::InvalidRequest(
                "workflow job repository identity mismatch".into(),
            ));
        }
    }
    let steps_truncated = j.steps.len() > MAX_STEPS;
    let mut steps = Vec::new();
    for s in j.steps.drain(..).take(MAX_STEPS) {
        if s.number == 0 || s.name.is_empty() || s.name.len() > 256 || s.name.contains('\0') {
            return Err(invalid_git_output());
        }
        steps.push(StepSummary {
            number: s.number,
            name: s.name,
            status: s.status,
            conclusion: s.conclusion,
            started_at: s.started_at,
            completed_at: s.completed_at,
        })
    }
    Ok(JobSummary {
        id: j.database_id,
        name: j.name,
        status: j.status,
        conclusion: j.conclusion,
        started_at: j.started_at,
        completed_at: j.completed_at,
        url: j.url,
        steps,
        steps_truncated,
    })
}
pub(super) fn positive(arguments: &Value, key: &str) -> Result<u64, McpError> {
    arguments
        .get(key)
        .and_then(Value::as_u64)
        .filter(|n| *n > 0)
        .ok_or_else(|| McpError::InvalidRequest(format!("{key} is required")))
}
pub(super) fn optional_string(
    arguments: &Value,
    key: &str,
    max: usize,
) -> Result<Option<String>, McpError> {
    match arguments.get(key) {
        None => Ok(None),
        Some(v) => {
            let s = v
                .as_str()
                .filter(|s| !s.is_empty() && s.len() <= max && !s.contains(['\0', '\n', '\r']))
                .ok_or_else(|| McpError::InvalidRequest(format!("{key} is invalid")))?;
            Ok(Some(s.into()))
        }
    }
}
pub(super) fn status(arguments: &Value) -> Result<Option<String>, McpError> {
    let Some(s) = optional_string(arguments, "status", 32)? else {
        return Ok(None);
    };
    const ALLOWED: &[&str] = &[
        "queued",
        "in_progress",
        "completed",
        "requested",
        "waiting",
        "pending",
        "success",
        "failure",
        "cancelled",
        "skipped",
        "timed_out",
        "action_required",
        "neutral",
        "stale",
        "startup_failure",
    ];
    if !ALLOWED.contains(&s.as_str()) {
        return Err(McpError::InvalidRequest("status is invalid".into()));
    }
    Ok(Some(s))
}
pub(super) fn commit_sha(arguments: &Value) -> Result<Option<String>, McpError> {
    let Some(s) = optional_string(arguments, "commit_sha", 40)? else {
        return Ok(None);
    };
    if s.len() != 40 || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(McpError::InvalidRequest("commit_sha is invalid".into()));
    }
    Ok(Some(s))
}
