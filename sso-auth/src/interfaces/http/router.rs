use axum::{
    routing::{get, post, put},
    Router,
};
use tower_http::trace::TraceLayer;

use super::{auth, connected_apps, frontend, health, oauth, session, AuthHttpState};

pub fn build_router(auth_state: AuthHttpState) -> Router {
    Router::new()
        .route("/health", get(health::get))
        .route("/auth/github", get(auth::start))
        .route("/auth/github/callback", get(auth::callback))
        .route("/auth/logout", post(session::logout))
        .route(
            "/.well-known/oauth-authorization-server",
            get(oauth::metadata),
        )
        .route("/oauth/authorize", get(oauth::authorize))
        .route("/oauth/token", post(oauth::token))
        .route("/api/session", get(session::get))
        .route(
            "/api/connect/apps/{client_id}",
            get(connected_apps::connection_info),
        )
        .route("/api/connected-apps", get(connected_apps::list))
        .route(
            "/api/connected-apps/{client_id}",
            put(connected_apps::save).delete(connected_apps::delete),
        )
        .fallback(frontend::serve)
        .with_state(auth_state)
        .layer(TraceLayer::new_for_http())
}
