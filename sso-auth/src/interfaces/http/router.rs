use axum::{
    routing::{get, post, put},
    Router,
};
use tower_http::trace::TraceLayer;

use super::{auth, connected_apps, frontend, health, session, AuthHttpState};

pub fn build_router(auth_state: AuthHttpState) -> Router {
    Router::new()
        .route("/health", get(health::get))
        .route("/auth/github", get(auth::start))
        .route("/auth/github/callback", get(auth::callback))
        .route("/auth/logout", post(session::logout))
        .route("/api/session", get(session::get))
        .route("/api/connected-apps", get(connected_apps::list))
        .route("/api/connected-apps/{client_id}", put(connected_apps::save))
        .fallback(frontend::serve)
        .with_state(auth_state)
        .layer(TraceLayer::new_for_http())
}
