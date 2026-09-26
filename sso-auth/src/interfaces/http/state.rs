use std::sync::Arc;

use crate::application::{AuthService, ConnectedAppService, ConnectionService};

#[derive(Clone)]
pub struct AuthHttpState {
    pub auth: Arc<AuthService>,
    pub connections: Arc<ConnectionService>,
    pub connected_apps: Arc<ConnectedAppService>,
    pub cookie_secure: bool,
    pub session_ttl_seconds: u64,
}
