use axum::{
    extract::{Form, OriginalUri, Query, State},
    http::{header::CACHE_CONTROL, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Redirect, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::application::{McpAuthorizationRequest, McpTokenRequest, MCP_SCOPE};

use super::{
    auth_cookie::{cookie_value, SESSION_COOKIE},
    AuthHttpState,
};

#[derive(Clone, Serialize)]
pub struct OAuthServerMetadata {
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub authorization_response_iss_parameter_supported: bool,
    pub client_id_metadata_document_supported: bool,
    pub token_endpoint_auth_methods_supported: Vec<String>,
    pub code_challenge_methods_supported: Vec<String>,
    pub scopes_supported: Vec<String>,
    pub response_types_supported: Vec<String>,
    pub grant_types_supported: Vec<String>,
}

impl OAuthServerMetadata {
    pub fn new(issuer: &str) -> Self {
        let base = issuer.trim_end_matches('/');
        Self {
            issuer: base.to_string(),
            authorization_endpoint: format!("{base}/oauth/authorize"),
            token_endpoint: format!("{base}/oauth/token"),
            authorization_response_iss_parameter_supported: true,
            client_id_metadata_document_supported: true,
            token_endpoint_auth_methods_supported: vec!["none".to_string()],
            code_challenge_methods_supported: vec!["S256".to_string()],
            scopes_supported: vec![MCP_SCOPE.to_string()],
            response_types_supported: vec!["code".to_string()],
            grant_types_supported: vec!["authorization_code".to_string()],
        }
    }
}

#[derive(Deserialize)]
pub struct AuthorizeQuery {
    response_type: String,
    client_id: String,
    redirect_uri: String,
    code_challenge: String,
    code_challenge_method: String,
    scope: String,
    state: String,
    resource: String,
}

#[derive(Deserialize)]
pub struct TokenForm {
    grant_type: String,
    code: String,
    redirect_uri: String,
    client_id: String,
    code_verifier: String,
    resource: String,
}

#[derive(Serialize)]
struct TokenResponse {
    access_token: String,
    token_type: &'static str,
    expires_in: u64,
    scope: String,
}

pub async fn metadata(State(state): State<AuthHttpState>) -> Response {
    no_store_json(StatusCode::OK, state.oauth_metadata.clone())
}

pub async fn authorize(
    State(state): State<AuthHttpState>,
    OriginalUri(uri): OriginalUri,
    headers: HeaderMap,
    Query(query): Query<AuthorizeQuery>,
) -> Response {
    if query.response_type != "code" {
        return oauth_error("unsupported_response_type");
    }

    let request = McpAuthorizationRequest {
        client_id: query.client_id,
        redirect_uri: query.redirect_uri,
        code_challenge: query.code_challenge,
        code_challenge_method: query.code_challenge_method,
        scope: query.scope,
        state: query.state,
        resource: query.resource,
    };
    if state
        .oauth
        .validate_authorization_request(&request)
        .is_err()
    {
        return oauth_error("invalid_request");
    }

    let session = cookie_value(&headers, SESSION_COOKIE)
        .and_then(|token| state.auth.read_session(&token).ok());
    let Some(session) = session else {
        let return_to = uri
            .path_and_query()
            .map(|value| value.as_str())
            .unwrap_or("/oauth/authorize");
        return login_redirect(return_to);
    };

    let Ok(code) = state.oauth.issue_code(session.user, request.clone()) else {
        return oauth_error("server_error");
    };
    let Ok(mut redirect) = Url::parse(&request.redirect_uri) else {
        return oauth_error("invalid_request");
    };
    redirect
        .query_pairs_mut()
        .append_pair("code", &code)
        .append_pair("state", &request.state)
        .append_pair("iss", &state.oauth_metadata.issuer);
    no_store_redirect(redirect.as_str())
}

pub async fn token(State(state): State<AuthHttpState>, Form(form): Form<TokenForm>) -> Response {
    let request = McpTokenRequest {
        grant_type: form.grant_type,
        code: form.code,
        redirect_uri: form.redirect_uri,
        client_id: form.client_id,
        code_verifier: form.code_verifier,
        resource: form.resource,
    };

    match state.oauth.exchange(request) {
        Ok(token) => no_store_json(
            StatusCode::OK,
            TokenResponse {
                access_token: token.access_token,
                token_type: "Bearer",
                expires_in: token.expires_in,
                scope: token.scope,
            },
        ),
        Err(_) => oauth_error("invalid_grant"),
    }
}

fn login_redirect(return_to: &str) -> Response {
    let query = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("return_to", return_to)
        .finish();
    no_store_redirect(&format!("/auth/github?{query}"))
}

fn oauth_error(error: &'static str) -> Response {
    no_store_json(
        StatusCode::BAD_REQUEST,
        serde_json::json!({ "error": error }),
    )
}

fn no_store_redirect(location: &str) -> Response {
    let mut response = Redirect::to(location).into_response();
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

fn no_store_json<T: Serialize>(status: StatusCode, body: T) -> Response {
    let mut response = (status, Json(body)).into_response();
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}
