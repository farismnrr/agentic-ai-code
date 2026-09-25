use axum::{
    routing::{get, post},
    Router,
};
use tower_http::trace::TraceLayer;

use super::{auth, frontend, health, AuthHttpState};

pub fn build_router(auth_state: AuthHttpState) -> Router {
    Router::new()
        .route("/health", get(health::get))
        .route("/auth/github", get(auth::start))
        .route("/auth/github/callback", get(auth::callback))
        .route("/auth/logout", post(auth::logout))
        .route("/api/session", get(auth::session))
        .fallback(frontend::serve)
        .with_state(auth_state)
        .layer(TraceLayer::new_for_http())
}
