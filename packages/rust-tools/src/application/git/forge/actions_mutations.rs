use super::super::*;
use super::common::*;
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Serialize)]
pub(in crate::application::git) struct WorkflowMutationResult {
    repository_root: String,
    forge: ForgeRepository,
    workflow_id: u64,
    operation: String,
}
#[derive(Debug, Serialize)]
pub(in crate::application::git) struct RunMutationResult {
    repository_root: String,
    forge: ForgeRepository,
    run_id: u64,
    operation: String,
}

fn workflow_id(a: &Value) -> Result<u64, McpError> {
    a.get("workflow_id")
        .and_then(Value::as_u64)
        .filter(|v| *v > 0)
        .ok_or_else(|| McpError::InvalidRequest("workflow_id is invalid".into()))
}
fn run_id(a: &Value) -> Result<u64, McpError> {
    a.get("run_id")
        .and_then(Value::as_u64)
        .filter(|v| *v > 0)
        .ok_or_else(|| McpError::InvalidRequest("run_id is invalid".into()))
}
fn repo_args(remote: &remote::GitRemoteIdentity) -> Vec<String> {
    vec!["--repo".into(), repo_spec(remote)]
}

pub(in crate::application::git) async fn workflow_dispatch(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<WorkflowMutationResult, McpError> {
    let repo = resolve_repo(arguments, config)?;
    let remote = remote::requested_remote(&repo.root, arguments)?;
    let id = workflow_id(arguments)?;
    let ref_name = arguments
        .get("ref")
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::InvalidRequest("ref is required".into()))?;
    if ref_name.is_empty() || ref_name.len() > 256 || ref_name.starts_with('-') {
        return Err(McpError::InvalidRequest("ref is invalid".into()));
    }
    let mut args = vec![
        "workflow".into(),
        "run".into(),
        id.to_string(),
        "--ref".into(),
        ref_name.into(),
    ];
    args.extend(repo_args(&remote));
    if let Some(inputs) = arguments.get("inputs") {
        let obj = inputs
            .as_object()
            .ok_or_else(|| McpError::InvalidRequest("inputs is invalid".into()))?;
        if obj.len() > 20 {
            return Err(McpError::InvalidRequest("too many workflow inputs".into()));
        }
        let mut pairs: Vec<_> = obj.iter().collect();
        pairs.sort_by_key(|(k, _)| *k);
        for (k, v) in pairs {
            if k.is_empty() || k.len() > 64 {
                return Err(McpError::InvalidRequest(
                    "workflow input key is invalid".into(),
                ));
            }
            let s = v.as_str().ok_or_else(|| {
                McpError::InvalidRequest("workflow input value is invalid".into())
            })?;
            if s.len() > 1024 {
                return Err(McpError::InvalidRequest(
                    "workflow input value is invalid".into(),
                ));
            }
            args.push("-f".into());
            args.push(format!("{k}={s}"));
        }
    }
    forge_process::run_gh(&repo.root, &args, &[]).await?;
    Ok(WorkflowMutationResult {
        repository_root: repo.relative_root,
        forge: forge_identity(&remote),
        workflow_id: id,
        operation: "dispatched".into(),
    })
}

pub(in crate::application::git) async fn workflow_run_rerun(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<RunMutationResult, McpError> {
    run_mut(arguments, config, "rerun").await
}
pub(in crate::application::git) async fn workflow_run_cancel(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<RunMutationResult, McpError> {
    run_mut(arguments, config, "cancel").await
}
async fn run_mut(
    arguments: &Value,
    config: &ServerConfig,
    op: &str,
) -> Result<RunMutationResult, McpError> {
    let repo = resolve_repo(arguments, config)?;
    let remote = remote::requested_remote(&repo.root, arguments)?;
    let id = run_id(arguments)?;
    let mut args = vec!["run".into(), op.into(), id.to_string()];
    args.extend(repo_args(&remote));
    forge_process::run_gh(&repo.root, &args, &[]).await?;
    Ok(RunMutationResult {
        repository_root: repo.relative_root,
        forge: forge_identity(&remote),
        run_id: id,
        operation: op.into(),
    })
}
