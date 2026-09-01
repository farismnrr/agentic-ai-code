use super::MAX_PAYLOAD_BYTES;
use serde_json::{json, Value};

pub(crate) fn bounded_context(output: &[u8]) -> Option<Value> {
    let value: Value = serde_json::from_slice(output).ok()?;
    let context = value.get("context")?.as_object()?;
    let repository_identity = context.get("repository_identity")?.as_str()?;
    Some(json!({ "repository_identity": bounded_string(repository_identity, 512) }))
}

pub(super) fn bounded_payload(mut payload: Value) -> Value {
    if let Some(object) = payload.as_object_mut() {
        for key in [
            "raw_output",
            "content",
            "prompt",
            "secrets",
            "environment",
            "command_output",
        ] {
            object.remove(key);
        }
    }
    if serde_json::to_vec(&payload).map_or(true, |encoded| encoded.len() > MAX_PAYLOAD_BYTES) {
        json!({ "hook_payload_truncated": true })
    } else {
        payload
    }
}

pub(super) fn bounded_string(value: &str, limit: usize) -> String {
    value.chars().take(limit).collect()
}
