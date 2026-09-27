use axum::{
    http::Request,
    routing::{get, post},
    Router,
};
use tower_http::trace::{DefaultOnRequest, DefaultOnResponse, TraceLayer};
use tracing::Level;

use super::{connections, discovery, mcp, RelayHttpState};

pub fn build_router(state: RelayHttpState) -> Router {
    Router::new()
        .route("/.well-known/relay.json", get(discovery::get))
        .route(
            "/.well-known/oauth-protected-resource/mcp",
            get(mcp::protected_resource),
        )
        .route(
            "/.well-known/oauth-protected-resource",
            get(mcp::protected_resource_root),
        )
        .route("/mcp", post(mcp::post))
        .route("/connections/start", get(connections::start))
        .route("/connections/callback", get(connections::callback))
        .route("/connections/{id}", get(connections::status))
        .route("/auth/login", get(connections::start))
        .route("/auth/callback", get(connections::callback))
        .with_state(state)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &Request<_>| {
                    tracing::info_span!(
                        "http_request",
                        method = %request.method(),
                        path = %request.uri().path()
                    )
                })
                .on_request(DefaultOnRequest::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
}
