use async_trait::async_trait;

use crate::domain::{AuthenticatedUser, AuthSession};

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
