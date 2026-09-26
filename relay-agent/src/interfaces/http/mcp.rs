use axum::{
    extract::State,
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::RelayHttpState;

const MCP_SCOPE: &str = "identity.read";
const PROTOCOL_MODERN: &str = "2026-07-28";
const PROTOCOL_LEGACY: &str = "2025-11-25";

#[derive(Clone, Serialize)]
pub struct ProtectedResourceMetadata {
    pub resource: String,
    pub authorization_servers: Vec<String>,
    pub scopes_supported: Vec<String>,
}

impl ProtectedResourceMetadata {
    pub fn new(resource: String, authorization_server: String) -> Self {
        Self {
            resource,
            authorization_servers: vec![authorization_server],
            scopes_supported: vec![MCP_SCOPE.to_string()],
        }
    }
}

#[derive(Deserialize)]
pub struct JsonRpcRequest {
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

pub async fn protected_resource(State(state): State<RelayHttpState>) -> Response {
    Json(state.mcp_resource.clone()).into_response()
}

pub async fn post(
    State(state): State<RelayHttpState>,
    headers: HeaderMap,
    Json(request): Json<JsonRpcRequest>,
) -> Response {
    let Some(id) = request.id.clone() else {
        return StatusCode::ACCEPTED.into_response();
    };

    let result = match request.method.as_str() {
        "server/discover" => discover_result(),
        "initialize" => initialize_result(&request.params),
        "tools/list" => tools_list_result(),
        "tools/call" => tools_call_result(&state, &headers, &request.params),
        _ => return rpc_error(id, -32601, "Method not found"),
    };
    rpc_result(id, result)
}

fn discover_result() -> Value {
    json!({
        "resultType": "complete",
        "supportedVersions": [PROTOCOL_MODERN, PROTOCOL_LEGACY],
        "capabilities": { "tools": {} },
        "instructions": "Use get_profile to identify the authenticated Masih Awam account.",
        "ttlMs": 300000,
        "cacheScope": "public",
        "_meta": server_meta()
    })
}

fn initialize_result(params: &Value) -> Value {
    let requested = params
        .get("protocolVersion")
        .and_then(Value::as_str)
        .unwrap_or(PROTOCOL_LEGACY);
    let protocol = if requested == PROTOCOL_MODERN {
        PROTOCOL_MODERN
    } else {
        PROTOCOL_LEGACY
    };
    json!({
        "protocolVersion": protocol,
        "capabilities": { "tools": {} },
        "serverInfo": { "name": "masih-awam-relay", "version": "0.1.0" },
        "instructions": "Use get_profile to identify the authenticated Masih Awam account."
    })
}

fn tools_list_result() -> Value {
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
        "cacheScope": "public",
        "_meta": server_meta()
    })
}

fn tools_call_result(state: &RelayHttpState, headers: &HeaderMap, params: &Value) -> Value {
    if params.get("name").and_then(Value::as_str) != Some("get_profile") {
        return json!({
            "resultType": "complete",
            "content": [{ "type": "text", "text": "Unknown tool." }],
            "isError": true,
            "_meta": server_meta()
        });
    }

    let principal =
        bearer_token(headers).and_then(|token| state.mcp_tokens.verify(token, MCP_SCOPE).ok());
    let Some(principal) = principal else {
        let challenge = format!(
            "Bearer resource_metadata=\"{}/.well-known/oauth-protected-resource\", scope=\"{}\", error=\"insufficient_scope\", error_description=\"Login required\"",
            state.mcp_resource.resource.trim_end_matches('/'),
            MCP_SCOPE
        );
        return json!({
            "resultType": "complete",
            "content": [{ "type": "text", "text": "Authentication required." }],
            "isError": true,
            "_meta": {
                "mcp/www_authenticate": [challenge],
                "io.modelcontextprotocol/serverInfo": {
                    "name": "masih-awam-relay",
                    "version": "0.1.0"
                }
            }
        });
    };

    let profile = json!({
        "id": principal.subject,
        "name": principal.login,
        "nickname": principal.login
    });
    json!({
        "resultType": "complete",
        "content": [{ "type": "text", "text": profile.to_string() }],
        "structuredContent": profile,
        "isError": false,
        "_meta": server_meta()
    })
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .filter(|value| !value.is_empty())
}

fn server_meta() -> Value {
    json!({
        "io.modelcontextprotocol/serverInfo": {
            "name": "masih-awam-relay",
            "version": "0.1.0"
        }
    })
}

fn rpc_result(id: Value, result: Value) -> Response {
    Json(json!({ "jsonrpc": "2.0", "id": id, "result": result })).into_response()
}

fn rpc_error(id: Value, code: i64, message: &'static str) -> Response {
    Json(json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message }
    }))
    .into_response()
}
