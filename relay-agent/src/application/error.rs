use std::{error::Error, fmt};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuthError {
    MissingAssertion,
    InvalidAssertion,
    InvalidAccessToken,
    ConnectionNotFound,
    ConnectionStateMismatch,
    ConnectionConflict,
    StorageUnavailable,
}

impl fmt::Display for AuthError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingAssertion => formatter.write_str("callback assertion is required"),
            Self::InvalidAssertion => formatter.write_str("signed assertion is invalid"),
            Self::InvalidAccessToken => formatter.write_str("MCP access token is invalid"),
            Self::ConnectionNotFound => formatter.write_str("connection was not found"),
            Self::ConnectionStateMismatch => formatter.write_str("connection state is invalid"),
            Self::ConnectionConflict => formatter.write_str("connection identifier already exists"),
            Self::StorageUnavailable => formatter.write_str("connection storage is unavailable"),
        }
    }
}

impl Error for AuthError {}
