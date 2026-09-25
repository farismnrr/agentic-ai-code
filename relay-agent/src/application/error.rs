use std::{error::Error, fmt};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuthError {
    MissingAssertion,
    VerificationNotImplemented,
    InvalidAssertion,
}

impl fmt::Display for AuthError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingAssertion => formatter.write_str("callback assertion is required"),
            Self::VerificationNotImplemented => {
                formatter.write_str("signed assertion verification is not implemented")
            }
            Self::InvalidAssertion => formatter.write_str("signed assertion is invalid"),
        }
    }
}

impl Error for AuthError {}
