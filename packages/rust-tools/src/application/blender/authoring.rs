use super::bridge;
use crate::core::config::ServerConfig;
use crate::core::error::McpError;
use serde_json::{json, Value};

const MAX_AUTHORING_RESULT_BYTES: usize = 1024 * 1024;

pub async fn execute_python(
    config: &ServerConfig,
    code: &str,
    result_mode: &str,
) -> Result<Value, McpError> {
    if code.is_empty() || code.len() > super::MAX_BLENDER_PYTHON_BYTES {
        return Err(McpError::InvalidRequest(
            "Blender Python code exceeds allowed bounds".into(),
        ));
    }
    if code.chars().any(|ch| ch == '\0') {
        return Err(McpError::InvalidRequest(
            "Blender Python code contains an unsupported NUL byte".into(),
        ));
    }
    let strict_json = match result_mode {
        "json" => true,
        "text" => false,
        _ => {
            return Err(McpError::InvalidRequest(
                "Blender Python result_mode must be json or text".into(),
            ))
        }
    };
    let reply = bridge::execute(config, code, strict_json).await?;
    let result = match result_mode {
        "json" => {
            enforce_result_bound(&reply.result)?;
            json!({
                "result_mode":"json",
                "result":reply.result,
                "stdout":reply.stdout,
                "sandboxed":false,
                "authority":"blender_host_user"
            })
        }
        "text" => {
            let text = match reply.result {
                Value::String(value) => value,
                value => serde_json::to_string(&value).map_err(|_| {
                    McpError::Internal("Blender Python text result could not be encoded".into())
                })?,
            };
            if text.len() > MAX_AUTHORING_RESULT_BYTES {
                return Err(McpError::InvalidRequest(
                    "Blender Python result exceeds allowed bounds".into(),
                ));
            }
            json!({
                "result_mode":"text",
                "result":text,
                "stdout":reply.stdout,
                "sandboxed":false,
                "authority":"blender_host_user"
            })
        }
        _ => unreachable!(),
    };
    Ok(result)
}

fn enforce_result_bound(value: &Value) -> Result<(), McpError> {
    let encoded = serde_json::to_vec(value)
        .map_err(|_| McpError::Internal("Blender Python result could not be encoded".into()))?;
    if encoded.len() > MAX_AUTHORING_RESULT_BYTES {
        return Err(McpError::InvalidRequest(
            "Blender Python result exceeds allowed bounds".into(),
        ));
    }
    Ok(())
}
