//! MCP `2026-07-28` JSON-RPC protocol core.
//!
//! Types and pure logic only — no transport/axum concerns here (kept in
//! `transport.rs`) so the protocol layer remains transport-independent.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use relay_core::error::McpError;

pub const PROTOCOL_VERSION: &str = "2026-07-28";
pub const LEGACY_PROTOCOL_VERSIONS: &[&str] = &["2025-06-18", "2025-03-26", "2024-11-05"];

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

/// Build the server identity metadata required on successful MCP results.
pub fn server_info_meta() -> Value {
    json!({
        "io.modelcontextprotocol/serverInfo": {
            "name": "relay-agent",
            "version": env!("CARGO_PKG_VERSION")
        }
    })
}

/// Add the server identity to a result without discarding other result
/// metadata supplied by a handler.
pub fn with_server_info_meta(mut result: Value) -> Value {
    let Some(result_object) = result.as_object_mut() else {
        return result;
    };

    let meta = result_object
        .entry("_meta")
        .or_insert_with(|| Value::Object(serde_json::Map::new()));
    if let Some(meta_object) = meta.as_object_mut() {
        meta_object.insert(
            "io.modelcontextprotocol/serverInfo".to_string(),
            server_info_meta()["io.modelcontextprotocol/serverInfo"].clone(),
        );
    } else {
        *meta = server_info_meta();
    }
    result
}

impl Response {
    pub fn new(id: Id, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: with_server_info_meta(result),
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
    pub instructions: &'static str,
    /// Required on the wire per SEP-2549 (confirmed against a real MCP
    /// client SDK's `DiscoverResult` type, which rejects a response
    /// missing either field) — not optional caching metadata a
    /// non-caching server can skip. `0` means "not cached, always fresh".
    #[serde(rename = "ttlMs")]
    pub ttl_ms: u64,
    /// `"private"` per SEP-2549: conservative default for a server that
    /// implements no caching logic of its own.
    #[serde(rename = "cacheScope")]
    pub cache_scope: &'static str,
}

impl DiscoverResult {
    pub fn current() -> Self {
        Self {
            result_type: "complete",
            supported_versions: vec![PROTOCOL_VERSION],
            capabilities: json!({ "tools": { "listChanged": false } }),
            instructions: "Coding server providing a sandboxed coding terminal, configured HTTP requests, and web search within the configured workspace policy.",
            ttl_ms: 0,
            cache_scope: "private",
        }
    }
}

/// A single MCP tool definition: stable name, human description, and a
/// JSON Schema 2020-12-compatible `inputSchema`.
#[derive(Debug, Clone, Serialize)]
pub struct Tool {
    pub name: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<&'static str>,
    pub description: &'static str,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<ToolAnnotations>,
    #[serde(rename = "securitySchemes")]
    pub security_schemes: Vec<ToolSecurityScheme>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolAnnotations {
    pub read_only_hint: bool,
    pub destructive_hint: bool,
    pub idempotent_hint: bool,
    pub open_world_hint: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolSecurityScheme {
    #[serde(rename = "type")]
    pub scheme_type: &'static str,
    pub scopes: Vec<&'static str>,
}

/// The canonical MCP tool catalog, mapping 1:1 onto the Plan 027 Rust CLI
/// binaries. Execution is deliberately not wired here (Phase 3) — this only
/// describes the surface a client can discover and validate calls against.
pub fn tool_catalog() -> Vec<Tool> {
    vec![
        Tool {
            name: "terminal_exec",
            title: Some("Sandboxed Coding Terminal"),
            description: "Run a full sandboxed coding-terminal command in the workspace; supports shells, scripts, builds, package managers, Git, and interpreters. Returns stdout, stderr, and exit status.",
            input_schema: json!({
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "type": "object",
                "properties": {
                    "command": { "type": "string", "minLength": 1, "maxLength": 65536 },
                    "args": {
                        "type": "array",
                        "items": { "type": "string", "maxLength": 65536 },
                        "maxItems": 100,
                        "default": []
                    },
                    "cwd": { "type": "string" },
                    "timeout_ms": {
                        "type": "integer",
                        "minimum": 0,
                        "maximum": 300000,
                        "default": 30000
                    }
                },
                "required": ["command"],
                "additionalProperties": false
            }),
            annotations: Some(ToolAnnotations {
                read_only_hint: false,
                destructive_hint: true,
                idempotent_hint: false,
                open_world_hint: true,
            }),
            security_schemes: vec![ToolSecurityScheme {
                scheme_type: "oauth2",
                scopes: vec!["relay.coding"],
            }],
        },
        Tool {
            name: "http_fetch",
            title: Some("HTTP Fetch"),
            description: "Make an HTTP(S) request and return the response; methods may mutate remote state.",
            input_schema: json!({
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "type": "object",
                "properties": {
                    "url": { "type": "string", "format": "uri", "maxLength": 65536 },
                    "method": {
                        "type": "string",
                        "enum": ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"],
                        "default": "GET"
                    },
                    "headers": {
                        "type": "object",
                        "maxProperties": 100,
                        "additionalProperties": { "type": "string", "maxLength": 65536 }
                    },
                    "data": { "type": "string", "maxLength": 65536 },
                    "timeout_ms": {
                        "type": "integer",
                        "minimum": 0,
                        "maximum": 300000,
                        "default": 30000
                    }
                },
                "required": ["url"],
                "additionalProperties": false
            }),
            annotations: Some(ToolAnnotations {
                read_only_hint: false,
                destructive_hint: true,
                idempotent_hint: false,
                open_world_hint: true,
            }),
            security_schemes: vec![ToolSecurityScheme {
                scheme_type: "oauth2",
                scopes: vec!["relay.coding"],
            }],
        },
        Tool {
            name: "web_search",
            title: Some("Web Search"),
            description: "Search the web through the configured SearxNG backend and return matching results.",
            input_schema: json!({
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "type": "object",
                "properties": {
                    "query": { "type": "string", "minLength": 1, "maxLength": 65536 }
                },
                "required": ["query"],
                "additionalProperties": false
            }),
            annotations: Some(ToolAnnotations {
                read_only_hint: true,
                destructive_hint: false,
                idempotent_hint: true,
                open_world_hint: true,
            }),
            security_schemes: vec![ToolSecurityScheme {
                scheme_type: "oauth2",
                scopes: vec!["relay.coding"],
            }],
        },
    ]
}

pub fn find_tool(name: &str) -> Option<Tool> {
    tool_catalog().into_iter().find(|t| t.name == name)
}

/// Validate `arguments` against a tool's declared `inputSchema` (JSON
/// Schema 2020-12) — the enforcement boundary required before execution:
/// `tools/call` validates argument *shape*, not just that a tool with this
/// name exists. Returns the first validation error's message on failure.
pub fn validate_tool_arguments(tool: &Tool, arguments: &Value) -> Result<(), McpError> {
    let validator = jsonschema::validator_for(&tool.input_schema).map_err(|e| {
        McpError::Internal(format!(
            "tool '{}' has an invalid inputSchema: {e}",
            tool.name
        ))
    })?;

    if let Some(first_error) = validator.iter_errors(arguments).next() {
        return Err(McpError::InvalidParams(format!(
            "arguments for tool '{}' failed schema validation at '{}': {}",
            tool.name,
            first_error.instance_path(),
            first_error
        )));
    }

    Ok(())
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
/// protocol error — see the MCP contract.
#[derive(Debug, Clone, Serialize)]
pub struct ToolCallResult {
    #[serde(rename = "resultType")]
    result_type: &'static str,
    pub content: Vec<ToolResultContent>,
    #[serde(rename = "isError")]
    pub is_error: bool,
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<Value>,
}

impl ToolCallResult {
    pub fn complete(content: Vec<ToolResultContent>) -> Self {
        Self::new(content, false)
    }

    pub fn error(content: Vec<ToolResultContent>) -> Self {
        Self::new(content, true)
    }

    pub fn with_meta(mut self, meta: Value) -> Self {
        self.meta = Some(meta);
        self
    }

    pub fn not_implemented(tool_name: &str) -> Self {
        Self::error(vec![ToolResultContent {
                kind: "text",
                text: format!(
                    "tool '{tool_name}' is registered but execution is not implemented yet (Plan 028 Phase 3)"
                ),
            }])
    }

    fn new(content: Vec<ToolResultContent>, is_error: bool) -> Self {
        Self {
            result_type: "complete",
            content,
            is_error,
            meta: None,
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
