use crate::application::creative::graph::CreativeJobRecord;
use crate::application::creative::store;
use crate::core::config::ServerConfig;
use crate::core::error::McpError;

pub(super) fn owned_jobs(
    cwd: Option<&str>,
    config: &ServerConfig,
    owner: &str,
    project_id: &str,
) -> Result<Vec<CreativeJobRecord>, McpError> {
    Ok(store::list_jobs(cwd, config, project_id)?
        .into_iter()
        .filter(|job| job.owner == owner)
        .collect())
}

pub(super) fn enforce_owner(job: &CreativeJobRecord, owner: &str) -> Result<(), McpError> {
    if job.owner != owner {
        return Err(McpError::InvalidRequest(
            "creative job belongs to a different owner".into(),
        ));
    }
    Ok(())
}

pub(super) fn validate_owner(owner: &str) -> Result<(), McpError> {
    if owner.is_empty() || owner.len() > 512 || owner.chars().any(char::is_control) {
        return Err(McpError::InvalidRequest(
            "creative job owner is invalid".into(),
        ));
    }
    Ok(())
}
