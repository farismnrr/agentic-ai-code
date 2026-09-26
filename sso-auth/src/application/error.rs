use std::{error::Error, fmt};

#[derive(Debug)]
pub enum AuthError {
    ExternalProvider,
    InvalidSession,
    InvalidConnectionAssertion,
    InvalidOAuthRequest,
    InvalidAuthorizationCode,
    ConnectedAppNotFound,
    ConnectedAppDisabled,
    InvalidConnectedApp(&'static str),
    StorageUnavailable,
    Forbidden,
    InvalidConfiguration(&'static str),
}

impl fmt::Display for AuthError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ExternalProvider => {
                formatter.write_str("external authentication provider failed")
            }
            Self::InvalidSession => formatter.write_str("authentication session is invalid"),
            Self::InvalidConnectionAssertion => {
                formatter.write_str("connection assertion could not be issued")
            }
            Self::InvalidOAuthRequest => formatter.write_str("OAuth request is invalid"),
            Self::InvalidAuthorizationCode => {
                formatter.write_str("OAuth authorization code is invalid")
            }
            Self::ConnectedAppNotFound => formatter.write_str("connected app was not found"),
            Self::ConnectedAppDisabled => formatter.write_str("connected app is disabled"),
            Self::InvalidConnectedApp(message) => formatter.write_str(message),
            Self::StorageUnavailable => formatter.write_str("connected app storage is unavailable"),
            Self::Forbidden => formatter.write_str("authenticated user is not allowed"),
            Self::InvalidConfiguration(message) => formatter.write_str(message),
        }
    }
}

impl Error for AuthError {}
