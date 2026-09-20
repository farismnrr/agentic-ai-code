use super::bridge;
use crate::core::config::ServerConfig;
use crate::core::error::McpError;
use serde_json::Value;

const MAX_DOC_RESULT_BYTES: usize = 512 * 1024;

pub async fn lookup(
    config: &ServerConfig,
    query: &str,
    module: Option<&str>,
    limit: usize,
) -> Result<Value, McpError> {
    validate_query(query)?;
    let module = module.unwrap_or("bpy");
    validate_module(module)?;
    if !(1..=20).contains(&limit) {
        return Err(McpError::InvalidRequest(
            "Blender docs result limit is outside allowed bounds".into(),
        ));
    }
    let query_literal = serde_json::to_string(query)
        .map_err(|_| McpError::Internal("Blender docs query could not be encoded".into()))?;
    let module_literal = serde_json::to_string(module)
        .map_err(|_| McpError::Internal("Blender docs module could not be encoded".into()))?;
    let code = format!(
        r#"# MASIHAWAM_DOCS
import bpy
import inspect
_query = {query_literal}.lower()
_module_name = {module_literal}
_limit = {limit}
_parts = _module_name.split(".")
_obj = bpy
for _part in _parts[1:]:
    _obj = getattr(_obj, _part)
_matches = []
for _name in dir(_obj):
    if _name.startswith("_"):
        continue
    if _query not in _name.lower():
        continue
    try:
        _value = getattr(_obj, _name)
        _doc = inspect.getdoc(_value) or ""
        _matches.append({{
            "name": _module_name + "." + _name,
            "kind": type(_value).__name__,
            "doc": _doc[:2000],
        }})
    except Exception:
        continue
    if len(_matches) >= _limit:
        break
_major, _minor, _patch = bpy.app.version
result = {{
    "blender_version": bpy.app.version_string,
    "api_version": [_major, _minor, _patch],
    "docs_base_url": f"https://docs.blender.org/api/{{_major}}.{{_minor}}/",
    "module": _module_name,
    "query": {query_literal},
    "matches": _matches,
}}
"#
    );
    let reply = bridge::execute(config, &code, true).await?;
    let encoded = serde_json::to_vec(&reply.result)
        .map_err(|_| McpError::Internal("Blender docs result could not be encoded".into()))?;
    if encoded.len() > MAX_DOC_RESULT_BYTES {
        return Err(McpError::InvalidRequest(
            "Blender docs result exceeds allowed bounds".into(),
        ));
    }
    Ok(reply.result)
}

fn validate_query(query: &str) -> Result<(), McpError> {
    if query.is_empty() || query.len() > 512 || query.chars().any(char::is_control) {
        return Err(McpError::InvalidRequest(
            "Blender docs query exceeds allowed bounds".into(),
        ));
    }
    Ok(())
}

fn validate_module(module: &str) -> Result<(), McpError> {
    if module.is_empty() || module.len() > 128 || module.chars().any(char::is_control) {
        return Err(McpError::InvalidRequest(
            "Blender docs module exceeds allowed bounds".into(),
        ));
    }
    let mut parts = module.split('.');
    if parts.next() != Some("bpy")
        || parts.clone().count() > 4
        || parts.any(|part| {
            part.is_empty()
                || !part
                    .chars()
                    .all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
                || part.chars().next().is_some_and(|ch| ch.is_ascii_digit())
        })
    {
        return Err(McpError::InvalidRequest(
            "Blender docs module must be a bounded bpy namespace".into(),
        ));
    }
    Ok(())
}
