use axum::{
    http::{header::CONTENT_TYPE, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::Deserialize;
use serde_json::{json, Map, Value};

pub(super) const PROTOCOL_VERSION: &str = "2026-07-28";
const HEADER_MCP_PROTOCOL_VERSION: &str = "mcp-protocol-version";
const HEADER_MCP_METHOD: &str = "mcp-method";
const HEADER_MCP_NAME: &str = "mcp-name";

#[derive(Deserialize)]
pub(super) struct JsonRpcRequest {
    pub(super) jsonrpc: String,
    pub(super) id: Option<Value>,
    pub(super) method: String,
    #[serde(default)]
    pub(super) params: Value,
}

pub(super) fn validate_request(
    headers: &HeaderMap,
    request: &JsonRpcRequest,
) -> Result<(), Response> {
    let id = request.id.clone();
    let Some(meta) = request.params.get("_meta").and_then(Value::as_object) else {
        return Err(invalid_params(id));
    };
    let Some(protocol_version) = meta
        .get("io.modelcontextprotocol/protocolVersion")
        .and_then(Value::as_str)
    else {
        return Err(invalid_params(id));
    };
    if !meta
        .get("io.modelcontextprotocol/clientCapabilities")
        .is_some_and(Value::is_object)
    {
        return Err(invalid_params(id));
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
            return Err(invalid_params(id));
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

pub(super) fn content_type_is_json(headers: &HeaderMap) -> bool {
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

fn invalid_params(id: Option<Value>) -> Response {
    rpc_error_response(
        StatusCode::BAD_REQUEST,
        id,
        -32602,
        "Invalid params",
        None,
    )
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

pub(super) fn rpc_error_response(
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
