use super::{resolve_project_root, validate_blender_relative_path};
use crate::core::config::ServerConfig;
use crate::core::error::McpError;
use crate::interfaces::mcp::{ToolCallResult, ToolResultContent};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde_json::{json, Value};
use std::fs;

const MAX_INLINE_SCREENSHOT_BYTES: usize = 16 * 1024 * 1024;
const MAX_INLINE_ANIMATION_PREVIEW_BYTES: usize = 20 * 1024 * 1024;

pub(super) fn complete(value: Value) -> Result<ToolCallResult, McpError> {
    let text = serde_json::to_string(&value)
        .map_err(|_| McpError::Internal("Blender result could not be serialized".into()))?;
    Ok(ToolCallResult::complete(vec![ToolResultContent {
        kind: "text",
        text,
    }]))
}

pub(super) fn complete_animation_preview(
    cwd: &str,
    config: &ServerConfig,
    mut value: Value,
) -> Result<ToolCallResult, McpError> {
    let root = resolve_project_root(Some(cwd), config)?;
    let previews = value
        .get_mut("previews")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| McpError::Internal("Blender animation previews are missing".into()))?;

    let mut encoded = Vec::new();
    let mut resource_links = Vec::new();
    let mut inline_total = 0usize;
    for preview in previews.iter_mut() {
        let relative_path = preview
            .get("relative_path")
            .and_then(Value::as_str)
            .ok_or_else(|| McpError::Internal("Blender animation preview path is missing".into()))?
            .to_owned();
        validate_blender_relative_path(&relative_path)?;
        let resource_uri = preview
            .get("resource_uri")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                McpError::Internal("Blender animation preview resource URI is missing".into())
            })?
            .to_owned();
        resource_links.push(resource_uri);
        let bytes = fs::read(root.join(&relative_path)).map_err(|_| {
            McpError::InvalidRequest("Blender animation preview is unavailable".into())
        })?;
        let inline = bytes.len() <= MAX_INLINE_SCREENSHOT_BYTES
            && inline_total.saturating_add(bytes.len()) <= MAX_INLINE_ANIMATION_PREVIEW_BYTES;
        if let Some(object) = preview.as_object_mut() {
            object.insert("inline_image".into(), json!(inline));
        }
        if inline {
            inline_total = inline_total.saturating_add(bytes.len());
            encoded.push(STANDARD.encode(bytes));
        }
    }

    let text = serde_json::to_string(&value)
        .map_err(|_| McpError::Internal("Blender result could not be serialized".into()))?;
    let mut content = vec![ToolResultContent { kind: "text", text }];
    content.extend(
        resource_links
            .into_iter()
            .map(ToolResultContent::image_resource_link),
    );
    content.extend(encoded.into_iter().map(ToolResultContent::png));
    Ok(ToolCallResult::complete(content))
}

pub(super) fn complete_screenshot(
    cwd: &str,
    config: &ServerConfig,
    mut value: Value,
) -> Result<ToolCallResult, McpError> {
    let relative_path = value
        .pointer("/preview/relative_path")
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::Internal("Blender screenshot path is missing".into()))?;
    validate_blender_relative_path(relative_path)?;
    let root = resolve_project_root(Some(cwd), config)?;
    let path = root.join(relative_path);
    let bytes = fs::read(&path).map_err(|_| {
        McpError::InvalidRequest("Blender screenshot preview is unavailable".into())
    })?;

    let inline = bytes.len() <= MAX_INLINE_SCREENSHOT_BYTES;
    let resource_uri = value
        .pointer("/preview/resource_uri")
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::Internal("Blender screenshot resource URI is missing".into()))?
        .to_owned();
    if let Some(preview) = value.get_mut("preview").and_then(Value::as_object_mut) {
        preview.insert("inline_image".into(), json!(inline));
    }

    let text = serde_json::to_string(&value)
        .map_err(|_| McpError::Internal("Blender result could not be serialized".into()))?;
    let mut content = vec![
        ToolResultContent { kind: "text", text },
        ToolResultContent::image_resource_link(resource_uri),
    ];
    if inline {
        content.push(ToolResultContent::png(STANDARD.encode(bytes)));
    }
    Ok(ToolCallResult::complete(content))
}
