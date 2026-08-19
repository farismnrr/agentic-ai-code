use relay_core::config::ServerConfig;
use relay_core::error::McpError;
mod actions;
mod actions_mutations;
mod change_requests;
mod common;
mod issues;
mod security;

pub(super) use actions::{
    workflow_get, workflow_job_log_preview, workflow_list, workflow_run_get, workflow_run_jobs,
    workflow_run_list,
};
pub(super) use actions_mutations::{workflow_dispatch, workflow_run_cancel, workflow_run_rerun};
pub(super) use change_requests::{
    change_request_checks, change_request_create, change_request_get, change_request_list,
    change_request_merge, change_request_update,
};
pub(super) use issues::{
    issue_close, issue_comment, issue_create, issue_get, issue_list, issue_reopen, issue_update,
};
pub(super) use security::{
    code_scanning_alert_get, code_scanning_alert_list, dependabot_alert_get, dependabot_alert_list,
    secret_scanning_alert_get, secret_scanning_alert_list, secret_scanning_alert_locations,
};

pub(super) async fn dispatch_forge_tool(
    name: &str,
    arguments: &serde_json::Value,
    config: &ServerConfig,
) -> Result<Option<serde_json::Value>, McpError> {
    let value = match name {
        "change_request_list" => {
            serde_json::to_value(change_request_list(arguments, config).await?)
        }
        "change_request_get" => serde_json::to_value(change_request_get(arguments, config).await?),
        "change_request_create" => {
            serde_json::to_value(change_request_create(arguments, config).await?)
        }
        "change_request_update" => {
            serde_json::to_value(change_request_update(arguments, config).await?)
        }
        "change_request_checks" => {
            serde_json::to_value(change_request_checks(arguments, config).await?)
        }
        "change_request_merge" => {
            serde_json::to_value(change_request_merge(arguments, config).await?)
        }
        "issue_list" => serde_json::to_value(issue_list(arguments, config).await?),
        "issue_get" => serde_json::to_value(issue_get(arguments, config).await?),
        "issue_create" => serde_json::to_value(issue_create(arguments, config).await?),
        "issue_update" => serde_json::to_value(issue_update(arguments, config).await?),
        "issue_comment" => serde_json::to_value(issue_comment(arguments, config).await?),
        "issue_close" => serde_json::to_value(issue_close(arguments, config).await?),
        "issue_reopen" => serde_json::to_value(issue_reopen(arguments, config).await?),
        "workflow_list" => serde_json::to_value(workflow_list(arguments, config).await?),
        "workflow_get" => serde_json::to_value(workflow_get(arguments, config).await?),
        "workflow_run_list" => serde_json::to_value(workflow_run_list(arguments, config).await?),
        "workflow_run_get" => serde_json::to_value(workflow_run_get(arguments, config).await?),
        "workflow_run_jobs" => serde_json::to_value(workflow_run_jobs(arguments, config).await?),
        "workflow_job_log_preview" => {
            serde_json::to_value(workflow_job_log_preview(arguments, config).await?)
        }
        "dependabot_alert_list" => {
            serde_json::to_value(dependabot_alert_list(arguments, config).await?)
        }
        "dependabot_alert_get" => {
            serde_json::to_value(dependabot_alert_get(arguments, config).await?)
        }
        "code_scanning_alert_list" => {
            serde_json::to_value(code_scanning_alert_list(arguments, config).await?)
        }
        "code_scanning_alert_get" => {
            serde_json::to_value(code_scanning_alert_get(arguments, config).await?)
        }
        "secret_scanning_alert_list" => {
            serde_json::to_value(secret_scanning_alert_list(arguments, config).await?)
        }
        "secret_scanning_alert_get" => {
            serde_json::to_value(secret_scanning_alert_get(arguments, config).await?)
        }
        "secret_scanning_alert_locations" => {
            serde_json::to_value(secret_scanning_alert_locations(arguments, config).await?)
        }
        "workflow_dispatch" => serde_json::to_value(workflow_dispatch(arguments, config).await?),
        "workflow_run_rerun" => serde_json::to_value(workflow_run_rerun(arguments, config).await?),
        "workflow_run_cancel" => {
            serde_json::to_value(workflow_run_cancel(arguments, config).await?)
        }
        _ => return Ok(None),
    }
    .map_err(|_| McpError::Internal("failed to serialize forge result".into()))?;
    Ok(Some(value))
}
