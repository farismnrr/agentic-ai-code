use serde::Serialize;

pub const MCP_SCOPE: &str = "identity.read";

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
