use crate::domain::{Connection, VerifiedConnectionAssertion, VerifiedPrincipal};

use super::AuthError;

pub trait SsoConnectUrlProvider: Send + Sync {
    fn connect_url(&self, state: &str) -> String;
}

pub trait SignedAssertionVerifier: Send + Sync {
    fn verify(&self, assertion: &str) -> Result<VerifiedConnectionAssertion, AuthError>;
}

pub trait McpAccessTokenVerifier: Send + Sync {
    fn verify(&self, token: &str, required_scope: &str) -> Result<VerifiedPrincipal, AuthError>;
}

pub trait ConnectionRepository: Send + Sync {
    fn create(&self, connection: Connection) -> Result<(), AuthError>;
    fn find(&self, id: &str) -> Result<Connection, AuthError>;
    fn complete_by_state(
        &self,
        state: &str,
        principal: VerifiedPrincipal,
    ) -> Result<Connection, AuthError>;
}

pub trait TokenGenerator: Send + Sync {
    fn generate(&self) -> String;
}
