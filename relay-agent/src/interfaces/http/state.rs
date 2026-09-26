use std::sync::Arc;

use crate::application::{ConnectCallbackUseCase, ConnectStartUseCase, ConnectionStatusUseCase};

use super::DiscoveryDocument;

#[derive(Clone)]
pub struct RelayHttpState {
    pub start: Arc<ConnectStartUseCase>,
    pub callback: Arc<ConnectCallbackUseCase>,
    pub status: Arc<ConnectionStatusUseCase>,
    pub discovery: DiscoveryDocument,
}

impl RelayHttpState {
    pub fn new(
        start: Arc<ConnectStartUseCase>,
        callback: Arc<ConnectCallbackUseCase>,
        status: Arc<ConnectionStatusUseCase>,
        discovery: DiscoveryDocument,
    ) -> Self {
        Self {
            start,
            callback,
            status,
            discovery,
        }
    }
}
