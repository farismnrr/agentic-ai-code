use serde::Serialize;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedPrincipal {
    pub subject: String,
    pub login: String,
    pub avatar_url: Option<String>,
}

impl VerifiedPrincipal {
    pub fn new(subject: impl Into<String>, login: impl Into<String>, avatar_url: Option<String>) -> Self {
        Self {
            subject: subject.into(),
            login: login.into(),
            avatar_url,
        }
    }
}
