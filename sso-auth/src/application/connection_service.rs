use std::sync::Arc;

use crate::domain::AuthenticatedUser;

use super::{AuthError, ConnectionAssertionIssuer};

pub struct ConnectionService {
    assertions: Arc<dyn ConnectionAssertionIssuer>,
}

impl ConnectionService {
    pub fn new(assertions: Arc<dyn ConnectionAssertionIssuer>) -> Self {
        Self { assertions }
    }

    pub fn issue_assertion(
        &self,
        user: &AuthenticatedUser,
        state: &str,
    ) -> Result<String, AuthError> {
        self.assertions.issue(user, state)
    }
}
