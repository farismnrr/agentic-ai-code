use std::sync::Arc;

use crate::domain::AuthSession;

use super::{AuthError, OAuthProvider, SessionCodec, StateGenerator, UserAccessPolicy};

pub struct LoginStart {
    pub authorization_url: String,
    pub state: String,
}

pub struct AuthService {
    oauth: Arc<dyn OAuthProvider>,
    sessions: Arc<dyn SessionCodec>,
    states: Arc<dyn StateGenerator>,
    access_policy: Arc<dyn UserAccessPolicy>,
}

impl AuthService {
    pub fn new(
        oauth: Arc<dyn OAuthProvider>,
        sessions: Arc<dyn SessionCodec>,
        states: Arc<dyn StateGenerator>,
        access_policy: Arc<dyn UserAccessPolicy>,
    ) -> Self {
        Self {
            oauth,
            sessions,
            states,
            access_policy,
        }
    }

    pub fn begin_login(&self) -> LoginStart {
        let state = self.states.generate();
        LoginStart {
            authorization_url: self.oauth.authorization_url(&state),
            state,
        }
    }

    pub async fn complete_login(&self, code: &str) -> Result<String, AuthError> {
        let user = self.oauth.authenticate(code).await?;
        if !self.access_policy.is_allowed(&user) {
            return Err(AuthError::Forbidden);
        }
        self.sessions.issue(user).map(|(token, _)| token)
    }

    pub fn read_session(&self, token: &str) -> Result<AuthSession, AuthError> {
        self.sessions.decode(token)
    }
}
