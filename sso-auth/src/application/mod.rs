mod auth_service;
mod connected_app_service;
mod connection_service;
mod mcp_oauth_service;
mod shared;

pub use auth_service::{AuthService, LoginCompletion};
pub use connected_app_service::ConnectedAppService;
pub use connection_service::{ConnectionHandoff, ConnectionService};
pub use mcp_oauth_service::{McpAuthorizationRequest, McpOAuthService, McpTokenRequest, MCP_SCOPE};
pub use shared::{
    AuthError, ConnectedAppRepository, ConnectionAssertionIssuer, McpAccessTokenIssuer,
    McpClientMetadata, McpClientMetadataResolver, OAuthProvider, SessionCodec, StateGenerator,
    UserAccessPolicy,
};
