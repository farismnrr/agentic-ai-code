use axum::{routing::get, Router};

use super::{connections, discovery, RelayHttpState};

pub fn build_router(state: RelayHttpState) -> Router {
    Router::new()
        .route("/.well-known/relay.json", get(discovery::get))
        .route("/connections/start", get(connections::start))
        .route("/connections/callback", get(connections::callback))
        .route("/connections/{id}", get(connections::status))
        .route("/auth/login", get(connections::start))
        .route("/auth/callback", get(connections::callback))
        .with_state(state)
}
