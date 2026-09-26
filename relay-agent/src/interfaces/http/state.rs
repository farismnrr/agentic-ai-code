use std::sync::Arc;

use crate::application::{ConnectCallbackUseCase, ConnectStartUseCase, ConnectionStatusUseCase};
use url::Url;

use super::DiscoveryDocument;

#[derive(Clone)]
pub struct RelayHttpState {
    pub start: Arc<ConnectStartUseCase>,
    pub callback: Arc<ConnectCallbackUseCase>,
    pub status: Arc<ConnectionStatusUseCase>,
    pub discovery: DiscoveryDocument,
    pub sso_dashboard_url: Url,
}

impl RelayHttpState {
    pub fn new(
        start: Arc<ConnectStartUseCase>,
        callback: Arc<ConnectCallbackUseCase>,
        status: Arc<ConnectionStatusUseCase>,
        discovery: DiscoveryDocument,
        sso_dashboard_url: Url,
    ) -> Self {
        Self {
            start,
            callback,
            status,
            discovery,
            sso_dashboard_url,
        }
    }
}
