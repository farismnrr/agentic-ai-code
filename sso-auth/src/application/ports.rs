use async_trait::async_trait;

use crate::domain::{AuthSession, AuthenticatedUser, ConnectedApp};

use super::AuthError;

#[async_trait]
pub trait OAuthProvider: Send + Sync {
    fn authorization_url(&self, state: &str) -> String;
    async fn authenticate(&self, code: &str) -> Result<AuthenticatedUser, AuthError>;
}

pub trait SessionCodec: Send + Sync {
    fn issue(&self, user: AuthenticatedUser) -> Result<(String, AuthSession), AuthError>;
    fn decode(&self, token: &str) -> Result<AuthSession, AuthError>;
}

pub trait StateGenerator: Send + Sync {
    fn generate(&self) -> String;
}

pub trait UserAccessPolicy: Send + Sync {
    fn is_allowed(&self, user: &AuthenticatedUser) -> bool;
}

pub trait ConnectionAssertionIssuer: Send + Sync {
    fn issue(
        &self,
        user: &AuthenticatedUser,
        audience: &str,
        state: &str,
        ttl_seconds: u64,
    ) -> Result<String, AuthError>;
}

pub trait ConnectedAppRepository: Send + Sync {
    fn list(&self) -> Result<Vec<ConnectedApp>, AuthError>;
    fn find(&self, client_id: &str) -> Result<Option<ConnectedApp>, AuthError>;
    fn save(&self, app: ConnectedApp) -> Result<ConnectedApp, AuthError>;
}
