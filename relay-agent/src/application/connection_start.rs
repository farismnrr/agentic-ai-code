use std::sync::Arc;

use crate::domain::Connection;

use super::{AuthError, ConnectionRepository, SsoConnectUrlProvider, TokenGenerator};

pub struct ConnectStart {
    pub authorization_url: String,
}

pub struct ConnectStartUseCase {
    connections: Arc<dyn ConnectionRepository>,
    tokens: Arc<dyn TokenGenerator>,
    sso_connect: Arc<dyn SsoConnectUrlProvider>,
}

impl ConnectStartUseCase {
    pub fn new(
        connections: Arc<dyn ConnectionRepository>,
        tokens: Arc<dyn TokenGenerator>,
        sso_connect: Arc<dyn SsoConnectUrlProvider>,
    ) -> Self {
        Self {
            connections,
            tokens,
            sso_connect,
        }
    }

    pub fn execute(&self) -> Result<ConnectStart, AuthError> {
        let id = self.tokens.generate();
        let state = self.tokens.generate();
        self.connections
            .create(Connection::pending(id, state.clone()))?;

        Ok(ConnectStart {
            authorization_url: self.sso_connect.connect_url(&state),
        })
    }
}
