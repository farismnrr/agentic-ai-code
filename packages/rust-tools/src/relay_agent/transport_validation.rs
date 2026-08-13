use super::error::McpError;
use super::mcp::{self, decode_header_value, extract_meta};
use axum::http::HeaderMap;

pub fn validate_content_type(value: Option<&str>) -> Result<(), McpError> {
    let content_type = value.unwrap_or_default();
    if content_type.starts_with("application/json") {
        Ok(())
    } else {
        Err(McpError::InvalidRequest(format!(
            "unsupported Content-Type '{content_type}', expected application/json"
        )))
    }
}

pub fn body_within_limit(body: &[u8], max_bytes: usize) -> bool {
    body.len() <= max_bytes
}

pub fn validate_routing_headers(
    headers: &HeaderMap,
    request: &mcp::Request,
) -> Result<(), McpError> {
    let protocol = match headers
        .get("mcp-protocol-version")
        .and_then(|v| v.to_str().ok())
    {
        None => {
            return Err(McpError::HeaderMismatch(
                "required standard header 'mcp-protocol-version' is missing".into(),
            ))
        }
        Some(v) if v != mcp::PROTOCOL_VERSION => {
            return Err(McpError::UnsupportedProtocolVersion {
                supported: vec![mcp::PROTOCOL_VERSION.to_string()],
                requested: v.to_string(),
            })
        }
        Some(v) => v,
    };
    let meta = extract_meta(request.params.as_ref());
    match meta.as_ref().and_then(|m| m.protocol_version.as_deref()) {
        Some(v) if v == protocol => {}
        Some(v) => return Err(McpError::HeaderMismatch(format!("'mcp-protocol-version' header value '{protocol}' does not match body params._meta['io.modelcontextprotocol/protocolVersion'] value '{v}'"))),
        None => return Err(McpError::HeaderMismatch("required params._meta['io.modelcontextprotocol/protocolVersion'] is missing from the request body".into())),
    }
    if meta
        .as_ref()
        .and_then(|m| m.client_capabilities.as_ref())
        .is_none()
    {
        return Err(McpError::HeaderMismatch("required params._meta['io.modelcontextprotocol/clientCapabilities'] is missing from the request body".into()));
    }
    match headers.get("mcp-method").and_then(|v| v.to_str().ok()) {
        None => {
            return Err(McpError::HeaderMismatch(
                "required standard header 'mcp-method' is missing".into(),
            ))
        }
        Some(v) if v != request.method => {
            return Err(McpError::HeaderMismatch(format!(
                "'mcp-method' header value '{v}' does not match body method '{}'",
                request.method
            )))
        }
        Some(_) => {}
    }
    if request.method == "tools/call" {
        let expected = request
            .params
            .as_ref()
            .and_then(|p| p.get("name"))
            .and_then(|v| v.as_str());
        let raw = headers
            .get("mcp-name")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| {
                McpError::HeaderMismatch(
                    "required standard header 'mcp-name' is missing for method 'tools/call'".into(),
                )
            })?;
        let decoded = decode_header_value(raw).ok_or_else(|| {
            McpError::HeaderMismatch(format!(
                "'mcp-name' header value '{raw}' is not valid Base64-sentinel or ASCII"
            ))
        })?;
        if Some(decoded.as_str()) != expected {
            return Err(McpError::HeaderMismatch(format!(
                "'mcp-name' header value '{decoded}' does not match body params.name"
            )));
        }
    }
    Ok(())
}

pub fn is_legacy_tools_list(headers: &HeaderMap, request: &mcp::Request) -> bool {
    if request.method != "tools/list"
        || request
            .params
            .as_ref()
            .and_then(|p| p.get("_meta"))
            .is_some()
    {
        return false;
    }
    match headers
        .get("mcp-protocol-version")
        .and_then(|v| v.to_str().ok())
    {
        None => true,
        Some(v) => mcp::LEGACY_PROTOCOL_VERSIONS.contains(&v),
    }
}
