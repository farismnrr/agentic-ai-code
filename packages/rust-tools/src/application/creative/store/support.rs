use crate::core::error::McpError;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

pub fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

pub(super) fn new_id(prefix: &str) -> String {
    format!("{prefix}_{}", Uuid::new_v4().simple())
}

pub(super) fn validate_media_type(value: &str) -> Result<(), McpError> {
    let allowed = value.starts_with("image/")
        || value.starts_with("video/")
        || value.starts_with("model/")
        || matches!(
            value,
            "application/json" | "application/octet-stream" | "application/x-blender"
        );
    if value.is_empty() || value.len() > 128 || value.chars().any(char::is_control) || !allowed {
        return Err(McpError::InvalidRequest(
            "creative asset media type is unsupported".into(),
        ));
    }
    Ok(())
}
