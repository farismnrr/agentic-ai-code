use super::super::*;
use super::common::*;
use relay_core::redaction::redact_credentials;
use serde_json::Value;

mod model;
mod validation;
use model::*;
use validation::*;

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
        workflows.push(validate_workflow(w, &remote)?)
    }
    Ok(WorkflowListResult {
        repository_root: repo.relative_root,
        forge: forge_identity(&remote),
        workflows,
        truncated,
    })
}

pub(in crate::git) async fn workflow_get(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<WorkflowResult, McpError> {
    let repo = resolve_repo(arguments, config)?;
    let remote = remote::requested_remote(&repo.root, arguments)?;
    let id = positive(arguments, "workflow_id")?;
    let endpoint = format!(
        "repos/{}/{}/actions/workflows/{id}",
        remote.owner, remote.repository
    );
    let args = vec!["api".into(), endpoint];
    let raw: ProviderWorkflow = parse_json(&forge_process::run_gh(&repo.root, &args, &[]).await?)?;
    if raw.id != id {
        return Err(McpError::InvalidRequest(
            "workflow identity mismatch".into(),
        ));
    }
    let workflow = validate_workflow(raw, &remote)?;
    Ok(WorkflowResult {
        repository_root: repo.relative_root,
        forge: forge_identity(&remote),
        workflow,
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
        run_fields().into(),
    ];
    if let Some(id) = arguments.get("workflow_id") {
        let id = id
            .as_u64()
            .filter(|n| *n > 0)
            .ok_or_else(|| McpError::InvalidRequest("workflow_id is invalid".into()))?;
        args.push("--workflow".into());
        args.push(id.to_string())
    }
    if let Some(v) = optional_string(arguments, "branch", 256)? {
        args.push("--branch".into());
        args.push(v)
    }
    if let Some(v) = commit_sha(arguments)? {
        args.push("--commit".into());
        args.push(v)
    }
    if let Some(v) = status(arguments)? {
        args.push("--status".into());
        args.push(v)
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
    let id = positive(arguments, "run_id")?;
    let args = vec![
        "run".into(),
        "view".into(),
        id.to_string(),
        "--repo".into(),
        repo_spec(&remote),
        "--json".into(),
        run_fields().into(),
    ];
    let raw: ProviderRun = parse_json(&forge_process::run_gh(&repo.root, &args, &[]).await?)?;
    if raw.database_id != id {
        return Err(McpError::InvalidRequest(
            "workflow run identity mismatch".into(),
        ));
    }
    let run = validate_run(raw, &remote)?;
    Ok(RunResult {
        repository_root: repo.relative_root,
        forge: forge_identity(&remote),
        run,
    })
}

pub(in crate::git) async fn workflow_run_jobs(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<RunJobsResult, McpError> {
    let repo = resolve_repo(arguments, config)?;
    let remote = remote::requested_remote(&repo.root, arguments)?;
    let id = positive(arguments, "run_id")?;
    let fields = format!("{},jobs", run_fields());
    let args = vec![
        "run".into(),
        "view".into(),
        id.to_string(),
        "--repo".into(),
        repo_spec(&remote),
        "--json".into(),
        fields,
    ];
    let mut raw: ProviderRun = parse_json(&forge_process::run_gh(&repo.root, &args, &[]).await?)?;
    if raw.database_id != id {
        return Err(McpError::InvalidRequest(
            "workflow run identity mismatch".into(),
        ));
    }
    let _ = validate_run(raw.clone(), &remote)?;
    let truncated = raw.jobs.len() > MAX_JOBS;
    let mut jobs = Vec::new();
    for j in raw.jobs.drain(..).take(MAX_JOBS) {
        jobs.push(validate_job(j, &remote)?)
    }
    Ok(RunJobsResult {
        repository_root: repo.relative_root,
        forge: forge_identity(&remote),
        run_id: id,
        jobs,
        truncated,
    })
}

pub(in crate::git) async fn workflow_job_log_preview(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<JobLogPreviewResult, McpError> {
    let repo = resolve_repo(arguments, config)?;
    let remote = remote::requested_remote(&repo.root, arguments)?;
    let job_id = positive(arguments, "job_id")?;
    let failed_only = arguments
        .get("failed_only")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    if arguments.get("failed_only").is_some() && !arguments.get("failed_only").unwrap().is_boolean()
    {
        return Err(McpError::InvalidRequest("failed_only is invalid".into()));
    }
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
        "--job".into(),
        job_id.to_string(),
        "--repo".into(),
        repo_spec(&remote),
    ];
    args.push(if failed_only { "--log-failed" } else { "--log" }.into());
    let capture = forge_process::run_gh_log_preview(&repo.root, &args).await?;
    let text = std::str::from_utf8(&capture.output).map_err(|_| invalid_git_output())?;
    let mut lines = Vec::new();
    let mut retained = 0usize;
    let mut truncated = capture.truncated;
    let mut redacted = false;
    for raw in text.lines() {
        if lines.len() >= max_lines {
            truncated = true;
            break;
        }
        let normalized: String = raw
            .chars()
            .filter(|c| !c.is_control() || *c == '\t')
            .collect();
        let clean = redact_credentials(&normalized);
        redacted |= clean != normalized;
        let bounded: String = clean.chars().take(MAX_LOG_LINE_BYTES).collect();
        if bounded.len() < clean.len() {
            truncated = true
        }
        retained += bounded.len();
        if retained > MAX_LOG_BYTES {
            truncated = true;
            break;
        }
        lines.push(bounded)
    }
    let returned_lines = lines.len();
    Ok(JobLogPreviewResult {
        repository_root: repo.relative_root,
        forge: forge_identity(&remote),
        job_id,
        failed_only,
        returned_lines,
        truncated,
        redacted,
        lines,
    })
}
