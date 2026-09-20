//! MCP `2026-07-28` JSON-RPC protocol core.
//!
//! Types and pure logic only — no transport/axum concerns here (kept in
//! `transport.rs`) so the protocol layer remains transport-independent.

use serde::{ser::SerializeStruct, Deserialize, Serialize, Serializer};
use serde_json::{json, Value};

use crate::core::error::McpError;

pub const PROTOCOL_VERSION: &str = "2026-07-28";
pub const TOOL_DESCRIPTION_REPORTING_SUFFIX: &str = "After tool-assisted work, report with sections Workspace, Issue(s), Work Completed, Verification, Next Steps, and Restart / Operator Action; never claim unperformed actions or checks.";
// The relay exposes one fully implemented wire contract. Older stateful MCP
// versions are intentionally not advertised until their complete session and
// task lifecycle is implemented in Plan 068; accepting only initialize for an
// older version would create a misleading half-supported protocol claim.
pub const LEGACY_PROTOCOL_VERSIONS: &[&str] = &[];

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

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub struct ToolTimingMeta {
    pub dispatch_ms: u64,
    pub server_total_ms: u64,
}

/// Add monotonic server timing metadata to a tool result value while preserving
/// any existing `_meta` entries.
pub fn with_timing_meta(mut result: Value, dispatch_ms: u64, server_total_ms: u64) -> Value {
    let Some(result_object) = result.as_object_mut() else {
        return result;
    };
    let meta = result_object
        .entry("_meta")
        .or_insert_with(|| Value::Object(serde_json::Map::new()));
    let timing = serde_json::to_value(ToolTimingMeta {
        dispatch_ms,
        server_total_ms,
    })
    .unwrap_or_else(|_| json!({}));
    if let Some(meta_object) = meta.as_object_mut() {
        meta_object.insert("timing".to_string(), timing);
    } else {
        *meta = json!({ "timing": timing });
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

pub const SERVER_INSTRUCTIONS: &str = "Masih Awam coding relay. Before repository mutation, resolve and verify the target workspace fresh for the current task; never treat a remembered cwd as write authority. Read the server's agent-guidance resource when available and follow repository-local guidance without weakening relay safety. For milestone, blocker, handoff, and completion reporting, use: Task Execution Report with Workspace, Issue(s), Work Completed, Verification, Next Steps, and Restart / Operator Action. Report only checks/actions that actually occurred; do not invent verification, commit, push, PR, merge, deployment, or restart status. Long-running work that exceeds a public tool deadline must be handed to the operator as an exact foreground command rather than hidden/background execution.";

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
            capabilities: json!({
                "tools": { "listChanged": false },
                "resources": {},
                "extensions": {
                    "io.masihawam/activity-bootstrap": { "version": "1" }
                }
            }),
            instructions: SERVER_INSTRUCTIONS,
            ttl_ms: 0,
            cache_scope: "private",
        }
    }
}

mod catalog;
pub use catalog::{
    blender_tool_catalog, find_tool, find_tool_for_profile, output_schema_for_tool,
    retained_tool_catalog, runtime_tool_catalog, tool_for_wire, validate_tool_arguments,
    validate_tool_output, Tool, ToolAnnotations, ToolSecurityScheme, CODING_SCOPE,
    PRIMARY_TOOL_NAMES,
};

#[derive(Debug, Clone, Deserialize)]
pub struct ToolsCallParams {
    pub name: String,
    #[serde(default)]
    pub arguments: Value,
}

pub mod resources {
    use serde::Serialize;

    #[derive(Debug, Clone, Serialize)]
    pub struct Resource {
        pub uri: String,
        pub name: String,
        pub description: String,
        #[serde(rename = "mimeType")]
        pub mime_type: &'static str,
    }

    #[derive(Debug, Clone, Serialize)]
    pub struct ResourceContent {
        pub uri: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub text: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub blob: Option<String>,
        #[serde(rename = "mimeType")]
        pub mime_type: String,
    }
}

/// `tools/call` result content block. Existing text callers retain the
/// in-memory `text` field; `kind="image"` serializes that field as MCP
/// base64 image data so binary previews can be returned without widening every
/// existing call site.
#[derive(Debug, Clone)]
pub struct ToolResultContent {
    pub kind: &'static str,
    pub text: String,
}

impl ToolResultContent {
    pub fn png(data_base64: String) -> Self {
        Self {
            kind: "image",
            text: data_base64,
        }
    }

    pub fn image_resource_link(uri: String) -> Self {
        Self {
            kind: "resource_link",
            text: uri,
        }
    }
}

impl Serialize for ToolResultContent {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self.kind {
            "image" => {
                let mut state = serializer.serialize_struct("ToolResultContent", 3)?;
                state.serialize_field("type", "image")?;
                state.serialize_field("data", &self.text)?;
                state.serialize_field("mimeType", "image/png")?;
                state.end()
            }
            "resource_link" => {
                let name = self
                    .text
                    .rsplit('/')
                    .next()
                    .filter(|value| !value.is_empty())
                    .unwrap_or("creative-image");
                let mut state = serializer.serialize_struct("ToolResultContent", 4)?;
                state.serialize_field("type", "resource_link")?;
                state.serialize_field("uri", &self.text)?;
                state.serialize_field("name", name)?;
                state.serialize_field("mimeType", "image/png")?;
                state.end()
            }
            _ => {
                let mut state = serializer.serialize_struct("ToolResultContent", 2)?;
                state.serialize_field("type", self.kind)?;
                state.serialize_field("text", &self.text)?;
                state.end()
            }
        }
    }
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
    #[serde(rename = "structuredContent", skip_serializing_if = "Option::is_none")]
    pub structured_content: Option<Value>,
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

    pub fn with_structured_content(mut self, structured_content: Value) -> Self {
        self.structured_content = Some(structured_content);
        self
    }

    pub fn with_meta(mut self, meta: Value) -> Self {
        self.meta = Some(meta);
        self
    }

    pub fn with_timing(mut self, dispatch_ms: u64, server_total_ms: u64) -> Self {
        let timing = serde_json::to_value(ToolTimingMeta {
            dispatch_ms,
            server_total_ms,
        })
        .unwrap_or_else(|_| json!({}));
        let meta = self.meta.get_or_insert_with(|| json!({}));
        if let Some(obj) = meta.as_object_mut() {
            obj.insert("timing".to_string(), timing);
        }
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
            structured_content: None,
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
        .map_err(|_| Some(McpError::InvalidRequest("invalid request".to_string())))?;

    if request.jsonrpc != "2.0" {
        return Err(Some(McpError::InvalidRequest(
            "jsonrpc must be \"2.0\"".to_string(),
        )));
    }

    Ok(request)
}
