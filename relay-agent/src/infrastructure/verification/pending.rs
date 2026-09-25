use crate::{
    application::{AuthError, SignedAssertionVerifier},
    domain::VerifiedPrincipal,
};

pub struct PendingSignedAssertionVerifier;

impl SignedAssertionVerifier for PendingSignedAssertionVerifier {
    fn verify(&self, _assertion: &str) -> Result<VerifiedPrincipal, AuthError> {
        Err(AuthError::VerificationNotImplemented)
    }
}
