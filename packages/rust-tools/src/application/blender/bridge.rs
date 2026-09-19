use super::{bridge_address, MAX_BLENDER_REQUEST_BYTES, MAX_BLENDER_RESPONSE_BYTES};
use crate::core::config::ServerConfig;
use crate::core::error::McpError;
use serde_json::{json, Value};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{timeout_at, Instant};

const READY_PROBE_CODE: &str = r#"import bpy
result = {
    "ready": True,
    "version": bpy.app.version_string,
    "scene": bpy.context.scene.name if bpy.context.scene else None,
}
"#;

#[derive(Debug, Clone, PartialEq)]
pub struct BridgeReply {
    pub result: Value,
    pub stdout: Option<String>,
}

pub async fn probe(config: &ServerConfig) -> Result<BridgeReply, McpError> {
    let reply = execute(config, READY_PROBE_CODE, true).await?;
    let ready = reply
        .result
        .get("ready")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let version = reply.result.get("version").and_then(Value::as_str);
    if !ready || version.is_none_or(str::is_empty) {
        return Err(McpError::InvalidRequest(
            "Blender bridge readiness payload is incompatible".into(),
        ));
    }
    Ok(reply)
}

pub async fn execute(
    config: &ServerConfig,
    code: &str,
    strict_json: bool,
) -> Result<BridgeReply, McpError> {
    if code.is_empty() || code.len() > super::MAX_BLENDER_PYTHON_BYTES {
        return Err(McpError::InvalidRequest(
            "Blender bridge code exceeds allowed bounds".into(),
        ));
    }
    let request = serde_json::to_vec(&json!({
        "type": "execute",
        "code": code,
        "strict_json": strict_json,
    }))
    .map_err(|_| McpError::Internal("failed to encode Blender bridge request".into()))?;
    if request.len().saturating_add(1) > MAX_BLENDER_REQUEST_BYTES {
        return Err(McpError::InvalidRequest(
            "Blender bridge request exceeds maximum frame size".into(),
        ));
    }

    let deadline = (config.blender_bridge_timeout_ms > 0)
        .then(|| Instant::now() + Duration::from_millis(config.blender_bridge_timeout_ms));
    let address = bridge_address(config);
    let mut stream = if let Some(deadline) = deadline {
        timeout_at(deadline, TcpStream::connect(address))
            .await
            .map_err(|_| McpError::InvalidRequest("Blender bridge connection timed out".into()))?
            .map_err(|_| McpError::InvalidRequest("Blender bridge is unavailable".into()))?
    } else {
        TcpStream::connect(address)
            .await
            .map_err(|_| McpError::InvalidRequest("Blender bridge is unavailable".into()))?
    };

    let write = async {
        stream.write_all(&request).await?;
        stream.write_all(&[0]).await?;
        stream.flush().await
    };
    if let Some(deadline) = deadline {
        timeout_at(deadline, write)
            .await
            .map_err(|_| McpError::InvalidRequest("Blender bridge write timed out".into()))?
            .map_err(|_| McpError::InvalidRequest("Blender bridge write failed".into()))?;
    } else {
        write
            .await
            .map_err(|_| McpError::InvalidRequest("Blender bridge write failed".into()))?;
    }

    let mut response = Vec::new();
    let read_frame = async {
        let mut chunk = [0u8; 16 * 1024];
        loop {
            let read = stream.read(&mut chunk).await?;
            if read == 0 {
                return Ok::<Option<Vec<u8>>, std::io::Error>(None);
            }
            if let Some(index) = chunk[..read].iter().position(|byte| *byte == 0) {
                if response.len().saturating_add(index) > MAX_BLENDER_RESPONSE_BYTES {
                    return Ok(None);
                }
                response.extend_from_slice(&chunk[..index]);
                return Ok(Some(std::mem::take(&mut response)));
            }
            if response.len().saturating_add(read) > MAX_BLENDER_RESPONSE_BYTES {
                return Ok(None);
            }
            response.extend_from_slice(&chunk[..read]);
        }
    };
    let frame = if let Some(deadline) = deadline {
        timeout_at(deadline, read_frame)
            .await
            .map_err(|_| McpError::InvalidRequest("Blender bridge response timed out".into()))?
            .map_err(|_| McpError::InvalidRequest("Blender bridge response read failed".into()))?
    } else {
        read_frame
            .await
            .map_err(|_| McpError::InvalidRequest("Blender bridge response read failed".into()))?
    }
    .ok_or_else(|| {
        McpError::InvalidRequest("Blender bridge response is missing or exceeds bounds".into())
    })?;

    let value: Value = serde_json::from_slice(&frame)
        .map_err(|_| McpError::InvalidRequest("Blender bridge returned malformed JSON".into()))?;
    match value.get("status").and_then(Value::as_str) {
        Some("ok") => Ok(BridgeReply {
            result: value.get("result").cloned().unwrap_or(Value::Null),
            stdout: value
                .get("stdout")
                .and_then(Value::as_str)
                .map(|text| text.chars().take(16_384).collect()),
        }),
        Some("error") => Err(McpError::InvalidRequest(
            "Blender bridge reported an execution error".into(),
        )),
        _ => Err(McpError::InvalidRequest(
            "Blender bridge returned an incompatible response".into(),
        )),
    }
}
