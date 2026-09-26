use axum::{
    http::{header::CACHE_CONTROL, HeaderValue, StatusCode},
    response::{IntoResponse, Redirect, Response},
    Json,
};
use serde::Serialize;
use url::Url;

pub(super) fn authorization_redirect_error(
    redirect_uri: &str,
    error: &'static str,
    state: Option<&str>,
    issuer: &str,
) -> Response {
    let Ok(mut redirect) = Url::parse(redirect_uri) else {
        return direct_authorization_error("invalid_request", issuer);
    };
    let mut query = redirect.query_pairs_mut();
    query.append_pair("error", error);
    if let Some(state) = state.filter(|value| !value.is_empty()) {
        query.append_pair("state", state);
    }
    query.append_pair("iss", issuer);
    drop(query);
    no_store_redirect(redirect.as_str())
}

pub(super) fn direct_authorization_error(error: &'static str, issuer: &str) -> Response {
    no_store_json(
        StatusCode::BAD_REQUEST,
        serde_json::json!({ "error": error, "iss": issuer }),
    )
}

pub(super) fn login_redirect(return_to: &str) -> Response {
    let query = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("return_to", return_to)
        .finish();
    no_store_redirect(&format!("/auth/github?{query}"))
}

pub(super) fn token_error(error: &'static str) -> Response {
    no_store_json(
        StatusCode::BAD_REQUEST,
        serde_json::json!({ "error": error }),
    )
}

pub(super) fn no_store_redirect(location: &str) -> Response {
    let mut response = Redirect::to(location).into_response();
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

pub(super) fn no_store_json<T: Serialize>(status: StatusCode, body: T) -> Response {
    let mut response = (status, Json(body)).into_response();
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

#[cfg(test)]
mod tests {
    use axum::http::{header::LOCATION, StatusCode};

    use super::authorization_redirect_error;

    #[test]
    fn authorization_error_redirect_includes_state_and_issuer() {
        let response = authorization_redirect_error(
            "https://chatgpt.com/connector_platform_oauth_redirect",
            "invalid_request",
            Some("state-123"),
            "https://sso.farismnrr.com",
        );

        assert!(response.status().is_redirection());
        assert_ne!(response.status(), StatusCode::BAD_REQUEST);
        let location = response
            .headers()
            .get(LOCATION)
            .and_then(|value| value.to_str().ok())
            .expect("redirect location");
        assert!(location.contains("error=invalid_request"));
        assert!(location.contains("state=state-123"));
        assert!(location.contains("iss=https%3A%2F%2Fsso.farismnrr.com"));
    }
}
