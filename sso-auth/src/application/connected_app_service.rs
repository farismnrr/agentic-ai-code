use std::sync::Arc;

use crate::domain::ConnectedApp;

use super::{AuthError, ConnectedAppRepository};

pub struct ConnectedAppService {
    apps: Arc<dyn ConnectedAppRepository>,
}

impl ConnectedAppService {
    pub fn new(apps: Arc<dyn ConnectedAppRepository>) -> Self {
        Self { apps }
    }

    pub fn list(&self) -> Result<Vec<ConnectedApp>, AuthError> {
        self.apps.list()
    }

    pub fn save(&self, app: ConnectedApp) -> Result<ConnectedApp, AuthError> {
        self.apps.save(app)
    }

    pub fn delete(&self, client_id: &str) -> Result<bool, AuthError> {
        self.apps.delete(client_id)
    }
}
