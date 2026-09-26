use std::sync::Arc;

use crate::domain::Connection;

use super::{AuthError, ConnectionRepository, SignedAssertionVerifier};

pub struct ConnectCallbackUseCase {
    verifier: Arc<dyn SignedAssertionVerifier>,
    connections: Arc<dyn ConnectionRepository>,
}

impl ConnectCallbackUseCase {
    pub fn new(
        verifier: Arc<dyn SignedAssertionVerifier>,
        connections: Arc<dyn ConnectionRepository>,
    ) -> Self {
        Self {
            verifier,
            connections,
        }
    }

    pub fn execute(&self, assertion: Option<&str>) -> Result<Connection, AuthError> {
        let assertion = assertion
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or(AuthError::MissingAssertion)?;

        let verified = self.verifier.verify(assertion)?;
        self.connections
            .complete_by_state(&verified.state, verified.principal)
    }
}
