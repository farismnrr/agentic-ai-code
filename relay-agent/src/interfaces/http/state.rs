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
    pub mcp_root_resource: ProtectedResourceMetadata,
    pub mcp_resource_metadata_url: String,
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
        mcp_root_resource: ProtectedResourceMetadata,
        mcp_resource_metadata_url: String,
    ) -> Self {
        Self {
            start,
            callback,
            status,
            discovery,
            sso_dashboard_url,
            mcp_tokens,
            mcp_resource,
            mcp_root_resource,
            mcp_resource_metadata_url,
        }
    }
}
