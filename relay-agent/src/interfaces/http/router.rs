use axum::{
    routing::{get, post},
    Router,
};

use super::{connections, discovery, mcp, RelayHttpState};

pub fn build_router(state: RelayHttpState) -> Router {
    Router::new()
        .route("/.well-known/relay.json", get(discovery::get))
        .route(
            "/.well-known/oauth-protected-resource",
            get(mcp::protected_resource),
        )
        .route("/mcp", post(mcp::post))
        .route("/connections/start", get(connections::start))
        .route("/connections/callback", get(connections::callback))
        .route("/connections/{id}", get(connections::status))
        .route("/auth/login", get(connections::start))
        .route("/auth/callback", get(connections::callback))
        .with_state(state)
}
