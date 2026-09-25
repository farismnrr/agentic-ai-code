use crate::domain::VerifiedPrincipal;

use super::AuthError;

pub trait SsoLoginUrlProvider: Send + Sync {
    fn login_url(&self) -> String;
}

pub trait SignedAssertionVerifier: Send + Sync {
    fn verify(&self, assertion: &str) -> Result<VerifiedPrincipal, AuthError>;
}
