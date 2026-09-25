#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedPrincipal {
    pub subject: String,
}

impl VerifiedPrincipal {
    pub fn new(subject: impl Into<String>) -> Self {
        Self {
            subject: subject.into(),
        }
    }
}
