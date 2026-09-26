use url::Url;

use crate::{application::AuthError, domain::ConnectedApp};

pub(super) fn validate_app(app: &ConnectedApp) -> Result<(), AuthError> {
    let valid_client_id = (3..=64).contains(&app.client_id.len())
        && app
            .client_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'));
    if !valid_client_id {
        return Err(AuthError::InvalidConnectedApp(
            "client ID must use 3-64 URL-safe characters",
        ));
    }

    if app.name.trim().is_empty() || app.name.len() > 80 {
        return Err(AuthError::InvalidConnectedApp(
            "app name must contain 1-80 characters",
        ));
    }
    if app.description.len() > 240 {
        return Err(AuthError::InvalidConnectedApp(
            "app description must be at most 240 characters",
        ));
    }
    if !(30..=300).contains(&app.assertion_ttl_seconds) {
        return Err(AuthError::InvalidConnectedApp(
            "assertion TTL must be between 30 and 300 seconds",
        ));
    }

    let callback = Url::parse(&app.callback_url)
        .map_err(|_| AuthError::InvalidConnectedApp("callback URL must be valid"))?;
    if !matches!(callback.scheme(), "http" | "https")
        || callback.cannot_be_a_base()
        || callback.query().is_some()
        || callback.fragment().is_some()
    {
        return Err(AuthError::InvalidConnectedApp(
            "callback URL must be an absolute HTTP(S) URL without query or fragment",
        ));
    }

    Ok(())
}
