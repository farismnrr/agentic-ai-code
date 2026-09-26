use serde_json::{json, Value};

use crate::domain::VerifiedPrincipal;

use super::{super::mcp_metadata::MCP_SCOPE, protocol::PROTOCOL_VERSION, server_meta};

pub(super) fn discover_result() -> Value {
    json!({
        "resultType": "complete",
        "supportedVersions": [PROTOCOL_VERSION],
        "capabilities": { "tools": {} },
        "instructions": "Use get_profile to identify the authenticated Masih Awam account.",
        "ttlMs": 300000,
        "cacheScope": "public",
        "_meta": server_meta()
    })
}

pub(super) fn tools_list_result() -> Value {
    json!({
        "resultType": "complete",
        "tools": [{
            "name": "get_profile",
            "title": "Get Masih Awam profile",
            "description": "Use this when the current authenticated Masih Awam account needs to be identified.",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "additionalProperties": false
            },
            "outputSchema": {
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "type": "object",
                "properties": {
                    "id": {
                        "type": "string",
                        "minLength": 1,
                        "pattern": "\\S",
                        "description": "Stable opaque profile identifier for the authenticated account."
                    },
                    "name": {
                        "type": "string",
                        "description": "Display name for the authenticated profile."
                    },
                    "nickname": {
                        "type": "string",
                        "description": "Short account label."
                    }
                },
                "required": ["id"],
                "additionalProperties": false
            },
            "annotations": {
                "readOnlyHint": true,
                "destructiveHint": false,
                "openWorldHint": false
            },
            "securitySchemes": [{ "type": "oauth2", "scopes": [MCP_SCOPE] }],
            "_meta": {
                "securitySchemes": [{ "type": "oauth2", "scopes": [MCP_SCOPE] }],
                "openai/profile": true
            }
        }],
        "ttlMs": 300000,
        "cacheScope": "private",
        "_meta": server_meta()
    })
}

pub(super) fn tools_call_result(
    principal: &VerifiedPrincipal,
    params: &Value,
) -> Result<Value, &'static str> {
    if params.get("name").and_then(Value::as_str) != Some("get_profile") {
        return Err("Unknown tool");
    }
    if let Some(arguments) = params.get("arguments") {
        let Some(arguments) = arguments.as_object() else {
            return Err("Invalid tool arguments");
        };
        if !arguments.is_empty() {
            return Err("Invalid tool arguments");
        }
    }

    let profile = json!({
        "id": principal.subject,
        "name": principal.login,
        "nickname": principal.login
    });
    Ok(json!({
        "resultType": "complete",
        "content": [{ "type": "text", "text": profile.to_string() }],
        "structuredContent": profile,
        "isError": false,
        "_meta": server_meta()
    }))
}
