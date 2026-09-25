use std::collections::HashSet;

use crate::{
    application::{AuthError, UserAccessPolicy},
    domain::AuthenticatedUser,
};

pub struct GitHubUserAllowlist {
    allowed_user_ids: HashSet<u64>,
}

impl GitHubUserAllowlist {
    pub fn new(allowed_user_ids: impl IntoIterator<Item = u64>) -> Result<Self, AuthError> {
        let allowed_user_ids = allowed_user_ids.into_iter().collect::<HashSet<_>>();
        if allowed_user_ids.is_empty() {
            return Err(AuthError::InvalidConfiguration(
                "ALLOWED_GITHUB_USER_IDS must contain at least one GitHub user ID",
            ));
        }

        Ok(Self { allowed_user_ids })
    }
}

impl UserAccessPolicy for GitHubUserAllowlist {
    fn is_allowed(&self, user: &AuthenticatedUser) -> bool {
        self.allowed_user_ids.contains(&user.id)
    }
}
