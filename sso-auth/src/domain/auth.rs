use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AuthenticatedUser {
    pub id: u64,
    pub login: String,
    pub avatar_url: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AuthSession {
    pub user: AuthenticatedUser,
    pub issued_at: u64,
    pub expires_at: u64,
}
