//! MCP `2026-07-28` JSON-RPC protocol core.
//!
//! Types and pure logic only — no transport/axum concerns here (kept in
//! `transport.rs`) so the protocol layer is independently testable per the
//! plan's module-separation requirement.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::error::McpError;

pub const PROTOCOL_VERSION: &str = "2026-07-28";

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum Id {
    Number(i64),
    String(String),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Request {
    pub jsonrpc: String,
    pub id: Id,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Notification {
    pub jsonrpc: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Response {
    pub jsonrpc: String,
    pub id: Id,
    pub result: Value,
}

impl Response {
    pub fn new(id: Id, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ErrorResponse {
    pub jsonrpc: String,
    pub id: Option<Id>,
    pub error: RpcError,
}

impl ErrorResponse {
    pub fn new(id: Option<Id>, error: &McpError) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            error: RpcError::from(error),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl From<&McpError> for RpcError {
    fn from(err: &McpError) -> Self {
        Self {
            code: err.code(),
            message: err.message(),
            data: err.data(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ClientInfo {
    pub name: String,
    pub version: String,
}

/// The `_meta` object modern MCP (`2026-07-28`) requires on every request's
/// `params`, per the spec's `RequestMetaObject` (`schema#requestmetaobject`).
/// There is no `initialize` handshake anymore — this is how protocol
/// version, client identity, and capabilities travel, self-contained, on
/// every single request.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct RequestMeta {
    /// Required by spec. Mirrored in and cross-checked against the
    /// `MCP-Protocol-Version` HTTP header — see `transport.rs`.
    #[serde(rename = "io.modelcontextprotocol/protocolVersion")]
    pub protocol_version: Option<String>,

    /// Optional by spec.
    #[serde(rename = "io.modelcontextprotocol/clientInfo")]
    pub client_info: Option<ClientInfo>,

    /// Required by spec (may be an empty object).
    #[serde(rename = "io.modelcontextprotocol/clientCapabilities")]
    pub client_capabilities: Option<Value>,
}

/// Extract and parse `params._meta` from a request, if present. Absence (or
/// a `params` with no `_meta` key) is represented as `None` — callers
/// distinguish "meta object present but a required field is empty" from
/// "no meta object at all" so error messages stay precise.
pub fn extract_meta(params: Option<&Value>) -> Option<RequestMeta> {
    let meta_val = params?.get("_meta")?;
    serde_json::from_value(meta_val.clone()).ok()
}

/// Decode a header value per the spec's Base64 sentinel format
/// (`streamable-http#value-encoding`): `=?base64?{Base64EncodedValue}?=`.
/// Values that don't match the sentinel pattern are returned as-is (they
/// were sent as plain ASCII). Returns `None` only when the value *looks*
/// like a sentinel but fails to decode as valid UTF-8 Base64 — that is a
/// malformed header, not a plain value.
pub fn decode_header_value(raw: &str) -> Option<String> {
    const PREFIX: &str = "=?base64?";
    const SUFFIX: &str = "?=";
    match raw
        .strip_prefix(PREFIX)
        .and_then(|s| s.strip_suffix(SUFFIX))
    {
        Some(encoded) => {
            use base64::Engine;
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(encoded)
                .ok()?;
            String::from_utf8(bytes).ok()
        }
        None => Some(raw.to_string()),
    }
}

/// The result of `server/discover` (`server/discover#discoverresult`).
/// `server/discover` is the modern replacement for the removed
/// `initialize` handshake: servers **MUST** implement it, but calling it is
/// optional for clients (any RPC can be invoked inline).
#[derive(Debug, Clone, Serialize)]
pub struct DiscoverResult {
    #[serde(rename = "resultType")]
    pub result_type: &'static str,
    #[serde(rename = "supportedVersions")]
    pub supported_versions: Vec<&'static str>,
    pub capabilities: Value,
    #[serde(rename = "_meta")]
    pub meta: Value,
    pub instructions: &'static str,
}

impl DiscoverResult {
    pub fn current() -> Self {
        Self {
            result_type: "complete",
            supported_versions: vec![PROTOCOL_VERSION],
            capabilities: json!({ "tools": { "listChanged": false } }),
            meta: json!({
                "io.modelcontextprotocol/serverInfo": {
                    "name": "relay-agent",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }),
            instructions: "Local relay-agent MCP server: exposes terminal_exec, http_fetch, and web_search tools backed by the Plan 027 Rust CLI binaries.",
        }
    }
}

/// A single MCP tool definition: stable name, human description, and a
/// JSON Schema 2020-12-compatible `inputSchema`.
#[derive(Debug, Clone, Serialize)]
pub struct Tool {
    pub name: &'static str,
    pub description: &'static str,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

/// The canonical MCP tool catalog, mapping 1:1 onto the Plan 027 Rust CLI
/// binaries per `.agents/plans/028-phase0-contract-audit.md` section 4.
///
/// Execution is deliberately not wired here (Phase 3) — this only describes
/// the surface a client can discover and validate calls against.
pub fn tool_catalog() -> Vec<Tool> {
    vec![
        Tool {
            name: "terminal_exec",
            description: "Run an executable command in a working directory and return its stdout/stderr/exit status. Maps to the terminal-tool Rust CLI binary.",
            input_schema: json!({
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "type": "object",
                "properties": {
                    "command": { "type": "string", "minLength": 1, "maxLength": 65536 },
                    "args": {
                        "type": "array",
                        "items": { "type": "string", "maxLength": 65536 },
                        "default": []
                    },
                    "cwd": { "type": "string" },
                    "timeout_ms": { "type": "integer", "minimum": 1, "default": 30000 }
                },
                "required": ["command"],
                "additionalProperties": false
            }),
        },
        Tool {
            name: "http_fetch",
            description: "Fetch a URL over HTTP(S) and return the response. Maps to the curl-tool Rust CLI binary.",
            input_schema: json!({
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "type": "object",
                "properties": {
                    "url": { "type": "string", "format": "uri", "maxLength": 65536 },
                    "method": { "type": "string", "default": "GET" },
                    "headers": {
                        "type": "object",
                        "additionalProperties": { "type": "string" }
                    },
                    "data": { "type": "string", "maxLength": 65536 },
                    "timeout_ms": { "type": "integer", "minimum": 0, "default": 30000 }
                },
                "required": ["url"],
                "additionalProperties": false
            }),
        },
        Tool {
            name: "web_search",
            description: "Search the web via a local SearxNG instance. Maps to the searxng-search-tool Rust CLI binary.",
            input_schema: json!({
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "type": "object",
                "properties": {
                    "query": { "type": "string", "minLength": 1, "maxLength": 65536 },
                    "base_url": { "type": "string", "format": "uri", "default": "http://127.0.0.1:8888" }
                },
                "required": ["query"],
                "additionalProperties": false
            }),
        },
    ]
}

pub fn find_tool(name: &str) -> Option<Tool> {
    tool_catalog().into_iter().find(|t| t.name == name)
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToolsCallParams {
    pub name: String,
    #[serde(default)]
    pub arguments: Value,
}

/// `tools/call` result content block (MCP text-content convention).
#[derive(Debug, Clone, Serialize)]
pub struct ToolResultContent {
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub text: String,
}

/// `tools/call` result envelope. A failing tool call (including "not
/// implemented yet") is a *result* with `is_error: true`, not a JSON-RPC
/// protocol error — see the audit doc section 3.
#[derive(Debug, Clone, Serialize)]
pub struct ToolCallResult {
    pub content: Vec<ToolResultContent>,
    #[serde(rename = "isError")]
    pub is_error: bool,
}

impl ToolCallResult {
    pub fn not_implemented(tool_name: &str) -> Self {
        Self {
            content: vec![ToolResultContent {
                kind: "text",
                text: format!(
                    "tool '{tool_name}' is registered but execution is not implemented yet (Plan 028 Phase 3)"
                ),
            }],
            is_error: true,
        }
    }
}

/// Parse and validate an incoming JSON-RPC request body.
///
/// Distinguishes three cases the transport layer must handle differently:
/// - a well-formed request (has `id` + `method`) → `Ok(Request)`
/// - a well-formed notification (`method`, no `id`) → `Err(None)` (transport
///   should reply with no body / 202-equivalent, never a JSON-RPC error)
/// - anything else → `Err(Some(McpError::ParseError | InvalidRequest))`
pub fn parse_request(payload: &Value) -> Result<Request, Option<McpError>> {
    if payload.get("id").is_none() && payload.get("method").is_some() {
        return Err(None);
    }

    let request: Request = serde_json::from_value(payload.clone())
        .map_err(|e| Some(McpError::InvalidRequest(e.to_string())))?;

    if request.jsonrpc != "2.0" {
        return Err(Some(McpError::InvalidRequest(
            "jsonrpc must be \"2.0\"".to_string(),
        )));
    }

    Ok(request)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn catalog_has_one_tool_per_plan_027_binary() {
        let names: Vec<&str> = tool_catalog().iter().map(|t| t.name).collect();
        assert_eq!(names, vec!["terminal_exec", "http_fetch", "web_search"]);
    }

    #[test]
    fn find_tool_returns_none_for_unknown_name() {
        assert!(find_tool("does_not_exist").is_none());
    }

    #[test]
    fn parse_request_accepts_well_formed_request() {
        let payload = json!({"jsonrpc":"2.0","id":1,"method":"tools/list"});
        let req = parse_request(&payload).expect("should parse");
        assert_eq!(req.method, "tools/list");
    }

    #[test]
    fn parse_request_treats_id_less_method_as_notification() {
        let payload = json!({"jsonrpc":"2.0","method":"notifications/initialized"});
        let err = parse_request(&payload).unwrap_err();
        assert!(err.is_none());
    }

    #[test]
    fn parse_request_rejects_wrong_jsonrpc_version() {
        let payload = json!({"jsonrpc":"1.0","id":1,"method":"tools/list"});
        let err = parse_request(&payload).unwrap_err();
        assert!(matches!(err, Some(McpError::InvalidRequest(_))));
    }

    #[test]
    fn parse_request_rejects_malformed_shape() {
        let payload = json!({"not":"a request"});
        let err = parse_request(&payload).unwrap_err();
        assert!(matches!(err, Some(McpError::InvalidRequest(_))));
    }

    #[test]
    fn extract_meta_reads_reserved_keys() {
        let params = json!({
            "_meta": {
                "io.modelcontextprotocol/protocolVersion": "2026-07-28",
                "io.modelcontextprotocol/clientInfo": { "name": "c", "version": "1.0" },
                "io.modelcontextprotocol/clientCapabilities": {}
            }
        });
        let meta = extract_meta(Some(&params)).expect("meta present");
        assert_eq!(meta.protocol_version.as_deref(), Some("2026-07-28"));
        assert_eq!(meta.client_info.unwrap().name, "c");
        assert!(meta.client_capabilities.is_some());
    }

    #[test]
    fn extract_meta_returns_none_when_absent() {
        assert!(extract_meta(Some(&json!({}))).is_none());
        assert!(extract_meta(None).is_none());
    }

    #[test]
    fn decode_header_value_passes_through_plain_ascii() {
        assert_eq!(
            decode_header_value("get_weather").as_deref(),
            Some("get_weather")
        );
    }

    #[test]
    fn decode_header_value_decodes_base64_sentinel() {
        // "Hello, 世界" per the spec's own worked example.
        let encoded = "=?base64?SGVsbG8sIOS4lueVjA==?=";
        assert_eq!(decode_header_value(encoded).as_deref(), Some("Hello, 世界"));
    }

    #[test]
    fn decode_header_value_rejects_invalid_base64_sentinel() {
        assert!(decode_header_value("=?base64?not valid base64?=").is_none());
    }

    #[test]
    fn discover_result_advertises_current_protocol_version() {
        let result = DiscoverResult::current();
        assert_eq!(result.supported_versions, vec![PROTOCOL_VERSION]);
        assert_eq!(result.result_type, "complete");
    }
}
