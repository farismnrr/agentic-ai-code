mod frontend;
mod health;

use axum::Router;
use tower_http::trace::TraceLayer;

use crate::infrastructure::config::AppConfig;

pub fn build_router(_config: AppConfig) -> Router {
    Router::new()
        .route("/health", axum::routing::get(health::get))
        .fallback(frontend::serve)
        .layer(TraceLayer::new_for_http())
}
