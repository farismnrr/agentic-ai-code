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
    response_type: Option<String>,
    client_id: Option<String>,
    redirect_uri: Option<String>,
    code_challenge: Option<String>,
    code_challenge_method: Option<String>,
    scope: Option<String>,
    state: Option<String>,
    resource: Option<String>,
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
    let issuer = state.oauth_metadata.issuer.as_str();
    let Some(client_id) = query.client_id.as_deref() else {
        return direct_authorization_error("invalid_request", issuer);
    };
    let Some(redirect_uri) = query.redirect_uri.as_deref() else {
        return direct_authorization_error("invalid_request", issuer);
    };

    let client = match state.oauth.validate_client(client_id, redirect_uri).await {
        Ok(client) => client,
        Err(_) => return direct_authorization_error("invalid_request", issuer),
    };

    if query.response_type.as_deref() != Some("code") {
        return authorization_redirect_error(
            redirect_uri,
            "unsupported_response_type",
            query.state.as_deref(),
            issuer,
        );
    }

    let Some(code_challenge) = query.code_challenge.as_deref() else {
        return authorization_redirect_error(
            redirect_uri,
            "invalid_request",
            query.state.as_deref(),
            issuer,
        );
    };
    let Some(code_challenge_method) = query.code_challenge_method.as_deref() else {
        return authorization_redirect_error(
            redirect_uri,
            "invalid_request",
            query.state.as_deref(),
            issuer,
        );
    };
    let Some(scope) = query.scope.as_deref() else {
        return authorization_redirect_error(
            redirect_uri,
            "invalid_scope",
            query.state.as_deref(),
            issuer,
        );
    };
    let Some(state_value) = query.state.as_deref() else {
        return authorization_redirect_error(redirect_uri, "invalid_request", None, issuer);
    };
    let Some(resource) = query.resource.as_deref() else {
        return authorization_redirect_error(
            redirect_uri,
            "invalid_request",
            query.state.as_deref(),
            issuer,
        );
    };

    let request = McpAuthorizationRequest {
        client_id: client_id.to_string(),
        redirect_uri: redirect_uri.to_string(),
        code_challenge: code_challenge.to_string(),
        code_challenge_method: code_challenge_method.to_string(),
        scope: scope.to_string(),
        state: state_value.to_string(),
        resource: resource.to_string(),
    };
    let request = match state.oauth.validate_authorization_request(request, &client) {
        Ok(request) => request,
        Err(_) => {
            return authorization_redirect_error(
                redirect_uri,
                "invalid_request",
                query.state.as_deref(),
                issuer,
            )
        }
    };

    let session = cookie_value(&headers, SESSION_COOKIE)
        .and_then(|token| state.auth.read_session(&token).ok());
    let Some(session) = session else {
        let return_to = uri
            .path_and_query()
            .map(|value| value.as_str())
            .unwrap_or("/oauth/authorize");
        return login_redirect(return_to);
    };

    let code = match state.oauth.issue_code(session.user, request) {
        Ok(code) => code,
        Err(_) => {
            return authorization_redirect_error(
                redirect_uri,
                "server_error",
                query.state.as_deref(),
                issuer,
            )
        }
    };
    let Ok(mut redirect) = Url::parse(redirect_uri) else {
        return direct_authorization_error("invalid_request", issuer);
    };
    redirect
        .query_pairs_mut()
        .append_pair("code", &code)
        .append_pair("state", state_value)
        .append_pair("iss", issuer);
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
        Err(_) => token_error("invalid_grant"),
    }
}

fn authorization_redirect_error(
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

fn direct_authorization_error(error: &'static str, issuer: &str) -> Response {
    no_store_json(
        StatusCode::BAD_REQUEST,
        serde_json::json!({ "error": error, "iss": issuer }),
    )
}

fn login_redirect(return_to: &str) -> Response {
    let query = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("return_to", return_to)
        .finish();
    no_store_redirect(&format!("/auth/github?{query}"))
}

fn token_error(error: &'static str) -> Response {
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
