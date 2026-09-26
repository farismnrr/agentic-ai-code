use std::sync::Arc;

use crate::domain::{AuthenticatedUser, ConnectedApp};

use super::{AuthError, ConnectedAppRepository, ConnectionAssertionIssuer};

pub struct ConnectionHandoff {
    pub callback_url: String,
    pub assertion: String,
}

pub struct ConnectionService {
    assertions: Arc<dyn ConnectionAssertionIssuer>,
    apps: Arc<dyn ConnectedAppRepository>,
}

impl ConnectionService {
    pub fn new(
        assertions: Arc<dyn ConnectionAssertionIssuer>,
        apps: Arc<dyn ConnectedAppRepository>,
    ) -> Self {
        Self { assertions, apps }
    }

    pub fn authorize(&self, client_id: &str) -> Result<ConnectedApp, AuthError> {
        let app = self
            .apps
            .find(client_id)?
            .ok_or(AuthError::ConnectedAppNotFound)?;
        if !app.enabled {
            return Err(AuthError::ConnectedAppDisabled);
        }
        Ok(app)
    }

    pub fn issue_handoff(
        &self,
        user: &AuthenticatedUser,
        client_id: &str,
        state: &str,
    ) -> Result<ConnectionHandoff, AuthError> {
        let app = self.authorize(client_id)?;
        let assertion = self.assertions.issue(
            user,
            &app.client_id,
            state,
            app.assertion_ttl_seconds,
        )?;

        Ok(ConnectionHandoff {
            callback_url: app.callback_url,
            assertion,
        })
    }
}
