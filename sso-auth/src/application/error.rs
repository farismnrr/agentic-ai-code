use std::{error::Error, fmt};

#[derive(Debug)]
pub enum AuthError {
    ExternalProvider,
    InvalidSession,
    InvalidConfiguration(&'static str),
}

impl fmt::Display for AuthError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ExternalProvider => {
                formatter.write_str("external authentication provider failed")
            }
            Self::InvalidSession => formatter.write_str("authentication session is invalid"),
            Self::InvalidConfiguration(message) => formatter.write_str(message),
        }
    }
}

impl Error for AuthError {}
