use std::sync::Arc;

use crate::domain::VerifiedPrincipal;

use super::{AuthError, SignedAssertionVerifier};

pub struct AuthCallbackUseCase {
    verifier: Arc<dyn SignedAssertionVerifier>,
}

impl AuthCallbackUseCase {
    pub fn new(verifier: Arc<dyn SignedAssertionVerifier>) -> Self {
        Self { verifier }
    }

    pub fn execute(&self, assertion: Option<&str>) -> Result<VerifiedPrincipal, AuthError> {
        let assertion = assertion
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or(AuthError::MissingAssertion)?;

        self.verifier.verify(assertion)
    }
}
