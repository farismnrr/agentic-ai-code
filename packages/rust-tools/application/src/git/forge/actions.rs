use super::super::*;
use super::common::*;
use relay_core::redaction::redact_credentials;
use serde::{Deserialize, Serialize};
use serde_json::Value;

const MAX_RUNS: usize = 50;
const MAX_WORKFLOWS: usize = 50;
const MAX_JOBS: usize = 100;
const MAX_LOG_LINES: usize = 200;
const MAX_LOG_BYTES: usize = 64 * 1024;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProviderWorkflow {
    id: u64,
    name: String,
    path: String,
    state: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::git) struct WorkflowSummary {
    id: u64,
    name: String,
    path: String,
    state: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::git) struct WorkflowListResult {
    repository_root: String,
    forge: ForgeRepository,
    workflows: Vec<WorkflowSummary>,
    truncated: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProviderRun {
    database_id: u64,
    name: String,
    #[serde(default)]
    workflow_name: String,
    #[serde(default)]
    display_title: String,
    #[serde(default)]
    event: String,
    #[serde(default)]
    head_branch: String,
    #[serde(default)]
    head_sha: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    conclusion: String,
    #[serde(default)]
    created_at: String,
    #[serde(default)]
    started_at: String,
    #[serde(default)]
    updated_at: String,
    url: String,
    #[serde(default)]
    attempt: u64,
    #[serde(default)]
    number: u64,
    #[serde(default)]
    jobs: Vec<ProviderJob>,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProviderJob {
    database_id: u64,
    name: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    conclusion: String,
    #[serde(default)]
    started_at: String,
    #[serde(default)]
    completed_at: String,
    #[serde(default)]
    url: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::git) struct RunSummary {
    id: u64,
    name: String,
    workflow_name: String,
    display_title: String,
    event: String,
    head_branch: String,
    head_sha: String,
    status: String,
    conclusion: String,
    created_at: String,
    started_at: String,
    updated_at: String,
    url: String,
    attempt: u64,
    number: u64,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::git) struct JobSummary {
    id: u64,
    name: String,
    status: String,
    conclusion: String,
    started_at: String,
    completed_at: String,
    url: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::git) struct RunListResult {
    repository_root: String,
    forge: ForgeRepository,
    runs: Vec<RunSummary>,
    truncated: bool,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::git) struct RunResult {
    repository_root: String,
    forge: ForgeRepository,
    run: RunSummary,
    jobs: Vec<JobSummary>,
    jobs_truncated: bool,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::git) struct JobResult {
    repository_root: String,
    forge: ForgeRepository,
    job: JobSummary,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::git) struct RunLogResult {
    repository_root: String,
    forge: ForgeRepository,
    run_id: u64,
    job_id: Option<u64>,
    lines: Vec<String>,
    truncated: bool,
    redacted: bool,
}

fn run_fields(include_jobs: bool) -> &'static str {
    if include_jobs {
        "databaseId,name,workflowName,displayTitle,event,headBranch,headSha,status,conclusion,createdAt,startedAt,updatedAt,url,attempt,number,jobs"
    } else {
        "databaseId,name,workflowName,displayTitle,event,headBranch,headSha,status,conclusion,createdAt,startedAt,updatedAt,url,attempt,number"
    }
}
fn validate_run(
    item: ProviderRun,
    remote: &remote::GitRemoteIdentity,
) -> Result<RunSummary, McpError> {
    if item.database_id == 0
        || item.name.len() > 256
        || item.display_title.len() > 512
        || item.head_sha.len() > 64
        || item.url.len() > 512
        || item.name.contains('\0')
        || item.display_title.contains('\0')
    {
        return Err(invalid_git_output());
    }
    let expected = format!(
        "https://github.com/{}/{}/actions/runs/{}",
        remote.owner, remote.repository, item.database_id
    );
    if item.url != expected {
        return Err(McpError::InvalidRequest(
            "workflow run repository identity mismatch".into(),
        ));
    }
    Ok(RunSummary {
        id: item.database_id,
        name: item.name,
        workflow_name: item.workflow_name,
        display_title: item.display_title,
        event: item.event,
        head_branch: item.head_branch,
        head_sha: item.head_sha,
        status: item.status,
        conclusion: item.conclusion,
        created_at: item.created_at,
        started_at: item.started_at,
        updated_at: item.updated_at,
        url: item.url,
        attempt: item.attempt,
        number: item.number,
    })
}
fn validate_job(
    item: ProviderJob,
    remote: &remote::GitRemoteIdentity,
) -> Result<JobSummary, McpError> {
    if item.database_id == 0
        || item.name.is_empty()
        || item.name.len() > 256
        || item.name.contains('\0')
        || item.url.len() > 512
    {
        return Err(invalid_git_output());
    }
    if !item.url.is_empty() {
        let prefix = format!(
            "https://github.com/{}/{}/actions/runs/",
            remote.owner, remote.repository
        );
        if !item.url.starts_with(&prefix) {
            return Err(McpError::InvalidRequest(
                "workflow job repository identity mismatch".into(),
            ));
        }
    }
    Ok(JobSummary {
        id: item.database_id,
        name: item.name,
        status: item.status,
        conclusion: item.conclusion,
        started_at: item.started_at,
        completed_at: item.completed_at,
        url: item.url,
    })
}
fn string_filter(arguments: &Value, key: &str, max: usize) -> Result<Option<String>, McpError> {
    match arguments.get(key) {
        None => Ok(None),
        Some(v) => {
            let s = v
                .as_str()
                .filter(|s| !s.is_empty() && s.len() <= max && !s.contains(['\0', '\n', '\r']))
                .ok_or_else(|| McpError::InvalidRequest(format!("{key} filter is invalid")))?;
            Ok(Some(s.into()))
        }
    }
}

pub(in crate::git) async fn workflow_list(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<WorkflowListResult, McpError> {
    let repo = resolve_repo(arguments, config)?;
    let remote = remote::requested_remote(&repo.root, arguments)?;
    let args = vec![
        "workflow".into(),
        "list".into(),
        "--repo".into(),
        repo_spec(&remote),
        "--all".into(),
        "--limit".into(),
        (MAX_WORKFLOWS + 1).to_string(),
        "--json".into(),
        "id,name,path,state".into(),
    ];
    let raw: Vec<ProviderWorkflow> =
        parse_json(&forge_process::run_gh(&repo.root, &args, &[]).await?)?;
    let truncated = raw.len() > MAX_WORKFLOWS;
    let mut workflows = Vec::new();
    for w in raw.into_iter().take(MAX_WORKFLOWS) {
        if w.id == 0
            || w.name.is_empty()
            || w.name.len() > 256
            || w.path.is_empty()
            || w.path.len() > 512
            || w.name.contains('\0')
            || w.path.contains('\0')
        {
            return Err(invalid_git_output());
        }
        workflows.push(WorkflowSummary {
            id: w.id,
            name: w.name,
            path: w.path,
            state: w.state,
        });
    }
    Ok(WorkflowListResult {
        repository_root: repo.relative_root,
        forge: forge_identity(&remote),
        workflows,
        truncated,
    })
}
pub(in crate::git) async fn workflow_run_list(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<RunListResult, McpError> {
    let repo = resolve_repo(arguments, config)?;
    let remote = remote::requested_remote(&repo.root, arguments)?;
    let mut args = vec![
        "run".into(),
        "list".into(),
        "--repo".into(),
        repo_spec(&remote),
        "--limit".into(),
        (MAX_RUNS + 1).to_string(),
        "--json".into(),
        run_fields(false).into(),
    ];
    for (key, flag) in [
        ("workflow", "--workflow"),
        ("branch", "--branch"),
        ("status", "--status"),
    ] {
        if let Some(v) = string_filter(arguments, key, 256)? {
            args.push(flag.into());
            args.push(v)
        }
    }
    let raw: Vec<ProviderRun> = parse_json(&forge_process::run_gh(&repo.root, &args, &[]).await?)?;
    let truncated = raw.len() > MAX_RUNS;
    let mut runs = Vec::new();
    for r in raw.into_iter().take(MAX_RUNS) {
        runs.push(validate_run(r, &remote)?)
    }
    Ok(RunListResult {
        repository_root: repo.relative_root,
        forge: forge_identity(&remote),
        runs,
        truncated,
    })
}
pub(in crate::git) async fn workflow_run_get(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<RunResult, McpError> {
    let repo = resolve_repo(arguments, config)?;
    let remote = remote::requested_remote(&repo.root, arguments)?;
    let id = requested_number(arguments, "run")?;
    let args = vec![
        "run".into(),
        "view".into(),
        id.to_string(),
        "--repo".into(),
        repo_spec(&remote),
        "--json".into(),
        run_fields(true).into(),
    ];
    let mut raw: ProviderRun = parse_json(&forge_process::run_gh(&repo.root, &args, &[]).await?)?;
    if raw.database_id != id {
        return Err(McpError::InvalidRequest(
            "workflow run identity mismatch".into(),
        ));
    }
    let jobs_truncated = raw.jobs.len() > MAX_JOBS;
    let jobs_raw = std::mem::take(&mut raw.jobs);
    let run = validate_run(raw, &remote)?;
    let mut jobs = Vec::new();
    for j in jobs_raw.into_iter().take(MAX_JOBS) {
        jobs.push(validate_job(j, &remote)?)
    }
    Ok(RunResult {
        repository_root: repo.relative_root,
        forge: forge_identity(&remote),
        run,
        jobs,
        jobs_truncated,
    })
}
pub(in crate::git) async fn workflow_job_get(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<JobResult, McpError> {
    let repo = resolve_repo(arguments, config)?;
    let remote = remote::requested_remote(&repo.root, arguments)?;
    let id = requested_number(arguments, "job")?;
    let args = vec![
        "run".into(),
        "view".into(),
        "--job".into(),
        id.to_string(),
        "--repo".into(),
        repo_spec(&remote),
        "--json".into(),
        "jobs".into(),
    ];
    let raw: ProviderRun = parse_json(&forge_process::run_gh(&repo.root, &args, &[]).await?)?;
    let item = raw
        .jobs
        .into_iter()
        .find(|j| j.database_id == id)
        .ok_or_else(|| McpError::InvalidRequest("workflow job identity mismatch".into()))?;
    let job = validate_job(item, &remote)?;
    Ok(JobResult {
        repository_root: repo.relative_root,
        forge: forge_identity(&remote),
        job,
    })
}
pub(in crate::git) async fn workflow_run_job_log(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<RunLogResult, McpError> {
    let repo = resolve_repo(arguments, config)?;
    let remote = remote::requested_remote(&repo.root, arguments)?;
    let run_id = requested_number(arguments, "run")?;
    let job_id = match arguments.get("job_id") {
        None => None,
        Some(v) => Some(
            v.as_u64()
                .filter(|n| *n > 0)
                .ok_or_else(|| McpError::InvalidRequest("job id is required".into()))?,
        ),
    };
    let max_lines = arguments
        .get("max_lines")
        .and_then(Value::as_u64)
        .unwrap_or(100) as usize;
    if max_lines == 0 || max_lines > MAX_LOG_LINES {
        return Err(McpError::InvalidRequest("max_lines is invalid".into()));
    }
    let mut args = vec![
        "run".into(),
        "view".into(),
        run_id.to_string(),
        "--repo".into(),
        repo_spec(&remote),
        "--log-failed".into(),
    ];
    if let Some(id) = job_id {
        args.push("--job".into());
        args.push(id.to_string())
    }
    let output = forge_process::run_gh(&repo.root, &args, &[]).await?;
    let text = std::str::from_utf8(&output).map_err(|_| invalid_git_output())?;
    let mut lines = Vec::new();
    let mut bytes = 0usize;
    let mut truncated = false;
    let mut redacted = false;
    for line in text.lines() {
        if lines.len() >= max_lines {
            truncated = true;
            break;
        }
        let clean = redact_credentials(line);
        redacted |= clean != line;
        bytes += clean.len();
        if bytes > MAX_LOG_BYTES {
            truncated = true;
            break;
        }
        lines.push(clean);
    }
    Ok(RunLogResult {
        repository_root: repo.relative_root,
        forge: forge_identity(&remote),
        run_id,
        job_id,
        lines,
        truncated,
        redacted,
    })
}
