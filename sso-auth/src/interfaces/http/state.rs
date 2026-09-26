use std::sync::Arc;

use url::Url;

use crate::application::{AuthService, ConnectionService};

#[derive(Clone)]
pub struct AuthHttpState {
    pub auth: Arc<AuthService>,
    pub connections: Arc<ConnectionService>,
    pub relay_callback_url: Url,
    pub cookie_secure: bool,
    pub session_ttl_seconds: u64,
}
