use super::super::*;
use serde::Serialize;
use serde_json::Value;

pub(in crate::git) const MAX_TITLE_BYTES: usize = 256;
pub(in crate::git) const MAX_BODY_BYTES: usize = 64 * 1024;

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
