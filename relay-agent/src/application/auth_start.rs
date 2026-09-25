use std::sync::Arc;

use super::SsoLoginUrlProvider;

pub struct AuthStartUseCase {
    sso_login: Arc<dyn SsoLoginUrlProvider>,
}

impl AuthStartUseCase {
    pub fn new(sso_login: Arc<dyn SsoLoginUrlProvider>) -> Self {
        Self { sso_login }
    }

    pub fn execute(&self) -> String {
        self.sso_login.login_url()
    }
}
