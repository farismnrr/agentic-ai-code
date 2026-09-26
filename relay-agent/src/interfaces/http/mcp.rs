use axum::{
    body::Bytes,
    extract::State,
    http::{
        header::{AUTHORIZATION, CONTENT_TYPE, WWW_AUTHENTICATE},
        HeaderMap, HeaderValue, StatusCode,
    },
    response::{IntoResponse, Response},
    Json,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::Deserialize;
use serde_json::{json, Map, Value};

use crate::domain::VerifiedPrincipal;

use super::{mcp_metadata::MCP_SCOPE, RelayHttpState};

const PROTOCOL_VERSION: &str = "2026-07-28";
const HEADER_MCP_PROTOCOL_VERSION: &str = "mcp-protocol-version";
const HEADER_MCP_METHOD: &str = "mcp-method";
const HEADER_MCP_NAME: &str = "mcp-name";

#[derive(Deserialize)]
pub struct JsonRpcRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

pub async fn protected_resource(State(state): State<RelayHttpState>) -> Response {
    Json(state.mcp_resource.clone()).into_response()
}

pub async fn protected_resource_root(State(state): State<RelayHttpState>) -> Response {
    Json(state.mcp_resource.clone()).into_response()
}

pub async fn post(
    State(state): State<RelayHttpState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let token = match bearer_token(&headers) {
        Some(token) => token,
        None => return oauth_challenge(&state, false),
    };
    let principal = match state.mcp_tokens.verify(token, MCP_SCOPE) {
        Ok(principal) => principal,
        Err(_) => return oauth_challenge(&state, true),
    };

    if !content_type_is_json(&headers) {
        return rpc_error_response(
            StatusCode::BAD_REQUEST,
            None,
            -32600,
            "Invalid Request",
            None,
        );
    }

    let request: JsonRpcRequest = match serde_json::from_slice(&body) {
        Ok(request) => request,
        Err(_) => {
            return rpc_error_response(
                StatusCode::BAD_REQUEST,
                None,
                -32700,
                "Parse error",
                None,
            )
        }
    };

    if request.jsonrpc != "2.0" {
        return rpc_error_response(
            StatusCode::BAD_REQUEST,
            request.id,
            -32600,
            "Invalid Request",
            None,
        );
    }

    if let Err(response) = validate_request(&headers, &request) {
        return response;
    }

    let Some(id) = request.id.clone() else {
        return StatusCode::ACCEPTED.into_response();
    };

    let result = match request.method.as_str() {
        "server/discover" => discover_result(),
        "tools/list" => tools_list_result(),
        "tools/call" => match tools_call_result(&principal, &request.params) {
            Ok(result) => result,
            Err(message) => {
                return rpc_error_response(
                    StatusCode::BAD_REQUEST,
                    Some(id),
                    -32602,
                    message,
                    None,
                )
            }
        },
        _ => {
            return rpc_error_response(
                StatusCode::NOT_FOUND,
                Some(id),
                -32601,
                "Method not found",
                None,
            )
        }
    };
    rpc_result(id, result)
}

fn validate_request(headers: &HeaderMap, request: &JsonRpcRequest) -> Result<(), Response> {
    let id = request.id.clone();
    let Some(meta) = request.params.get("_meta").and_then(Value::as_object) else {
        return Err(rpc_error_response(
            StatusCode::BAD_REQUEST,
            id,
            -32602,
            "Invalid params",
            None,
        ));
    };
    let Some(protocol_version) = meta
        .get("io.modelcontextprotocol/protocolVersion")
        .and_then(Value::as_str)
    else {
        return Err(rpc_error_response(
            StatusCode::BAD_REQUEST,
            id,
            -32602,
            "Invalid params",
            None,
        ));
    };
    if !meta
        .get("io.modelcontextprotocol/clientCapabilities")
        .is_some_and(Value::is_object)
    {
        return Err(rpc_error_response(
            StatusCode::BAD_REQUEST,
            id,
            -32602,
            "Invalid params",
            None,
        ));
    }

    if protocol_version != PROTOCOL_VERSION {
        return Err(rpc_error_response(
            StatusCode::BAD_REQUEST,
            id,
            -32022,
            "Unsupported protocol version",
            Some(json!({
                "supported": [PROTOCOL_VERSION],
                "requested": protocol_version
            })),
        ));
    }

    if header_string(headers, HEADER_MCP_PROTOCOL_VERSION) != Some(protocol_version)
        || header_string(headers, HEADER_MCP_METHOD) != Some(request.method.as_str())
    {
        return Err(header_mismatch(id));
    }

    if request.method == "tools/call" {
        let Some(name) = request.params.get("name").and_then(Value::as_str) else {
            return Err(rpc_error_response(
                StatusCode::BAD_REQUEST,
                id,
                -32602,
                "Invalid params",
                None,
            ));
        };
        let Some(header_name) = header_string(headers, HEADER_MCP_NAME)
            .and_then(decode_header_value)
        else {
            return Err(header_mismatch(id));
        };
        if header_name != name {
            return Err(header_mismatch(id));
        }
    }

    Ok(())
}

fn discover_result() -> Value {
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
        "cacheScope": "private",
        "_meta": server_meta()
    })
}

fn tools_call_result(
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

fn oauth_challenge(state: &RelayHttpState, invalid_token: bool) -> Response {
    let mut challenge = format!(
        "Bearer resource_metadata=\"{}\", scope=\"{}\"",
        state.mcp_resource_metadata_url, MCP_SCOPE
    );
    if invalid_token {
        challenge.push_str(", error=\"invalid_token\"");
    }

    let mut headers = HeaderMap::new();
    if let Ok(value) = HeaderValue::from_str(&challenge) {
        headers.insert(WWW_AUTHENTICATE, value);
    }
    (StatusCode::UNAUTHORIZED, headers).into_response()
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .filter(|value| !value.is_empty())
}

fn content_type_is_json(headers: &HeaderMap) -> bool {
    headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value
                .split(';')
                .next()
                .is_some_and(|media_type| media_type.trim() == "application/json")
        })
}

fn header_string<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name)?.to_str().ok()
}

fn decode_header_value(value: &str) -> Option<String> {
    let Some(encoded) = value
        .strip_prefix("=?base64?")
        .and_then(|value| value.strip_suffix("?="))
    else {
        return Some(value.to_string());
    };
    let decoded = STANDARD.decode(encoded).ok()?;
    String::from_utf8(decoded).ok()
}

fn header_mismatch(id: Option<Value>) -> Response {
    rpc_error_response(
        StatusCode::BAD_REQUEST,
        id,
        -32020,
        "Header mismatch",
        None,
    )
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

fn rpc_error_response(
    status: StatusCode,
    id: Option<Value>,
    code: i64,
    message: &'static str,
    data: Option<Value>,
) -> Response {
    let mut error = Map::new();
    error.insert("code".to_string(), json!(code));
    error.insert("message".to_string(), json!(message));
    if let Some(data) = data {
        error.insert("data".to_string(), data);
    }
    (
        status,
        Json(json!({
            "jsonrpc": "2.0",
            "id": id.unwrap_or(Value::Null),
            "error": Value::Object(error)
        })),
    )
        .into_response()
}
