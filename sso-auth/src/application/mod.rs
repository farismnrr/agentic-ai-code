mod auth_service;
mod connected_app_service;
mod connection_service;
mod error;
mod mcp_oauth_service;
mod ports;

pub use auth_service::{AuthService, LoginCompletion};
pub use connected_app_service::ConnectedAppService;
pub use connection_service::{ConnectionHandoff, ConnectionService};
pub use error::AuthError;
pub use mcp_oauth_service::{
    McpAuthorizationRequest, McpOAuthService, McpTokenRequest, McpTokenResponse, CHATGPT_CLIENT_ID,
    CHATGPT_REDIRECT_URI, MCP_SCOPE,
};
pub use ports::ConnectedAppRepository;
pub use ports::ConnectionAssertionIssuer;
pub use ports::McpAccessTokenIssuer;
pub use ports::OAuthProvider;
pub use ports::SessionCodec;
pub use ports::StateGenerator;
pub use ports::UserAccessPolicy;
