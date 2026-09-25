use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::{header::CACHE_CONTROL, HeaderValue, StatusCode},
    response::{IntoResponse, Redirect, Response},
};
use serde::Deserialize;

use crate::application::{AuthCallbackUseCase, AuthError, AuthStartUseCase};

#[derive(Clone)]
pub struct AuthHttpState {
    pub start: Arc<AuthStartUseCase>,
    pub callback: Arc<AuthCallbackUseCase>,
}

#[derive(Deserialize)]
pub struct CallbackQuery {
    assertion: Option<String>,
}

pub async fn login(State(state): State<AuthHttpState>) -> Response {
    Redirect::temporary(&state.start.execute()).into_response()
}

pub async fn callback(
    State(state): State<AuthHttpState>,
    Query(query): Query<CallbackQuery>,
) -> Response {
    match state.callback.execute(query.assertion.as_deref()) {
        Err(AuthError::MissingAssertion) => no_store(
            StatusCode::BAD_REQUEST,
            "callback assertion is required",
        ),
        Err(AuthError::VerificationNotImplemented) => no_store(
            StatusCode::NOT_IMPLEMENTED,
            "callback received; signed assertion verification is not implemented",
        ),
        Err(AuthError::InvalidAssertion) => {
            no_store(StatusCode::UNAUTHORIZED, "signed assertion is invalid")
        }
        Ok(_) => no_store(
            StatusCode::NOT_IMPLEMENTED,
            "assertion verified; relay session issuance is not implemented",
        ),
    }
}

fn no_store(status: StatusCode, message: &'static str) -> Response {
    let mut response = (status, message).into_response();
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}
