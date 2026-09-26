use std::sync::Arc;

use url::Url;

use crate::application::{
    ConnectCallbackUseCase, ConnectStartUseCase, ConnectionStatusUseCase, McpAccessTokenVerifier,
};

use super::{DiscoveryDocument, ProtectedResourceMetadata};

#[derive(Clone)]
pub struct RelayHttpState {
    pub start: Arc<ConnectStartUseCase>,
    pub callback: Arc<ConnectCallbackUseCase>,
    pub status: Arc<ConnectionStatusUseCase>,
    pub discovery: DiscoveryDocument,
    pub sso_dashboard_url: Url,
    pub mcp_tokens: Arc<dyn McpAccessTokenVerifier>,
    pub mcp_resource: ProtectedResourceMetadata,
}

impl RelayHttpState {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        start: Arc<ConnectStartUseCase>,
        callback: Arc<ConnectCallbackUseCase>,
        status: Arc<ConnectionStatusUseCase>,
        discovery: DiscoveryDocument,
        sso_dashboard_url: Url,
        mcp_tokens: Arc<dyn McpAccessTokenVerifier>,
        mcp_resource: ProtectedResourceMetadata,
    ) -> Self {
        Self {
            start,
            callback,
            status,
            discovery,
            sso_dashboard_url,
            mcp_tokens,
            mcp_resource,
        }
    }
}
