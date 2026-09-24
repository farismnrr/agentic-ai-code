mod health;

use axum::Router;
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};

use crate::infrastructure::config::AppConfig;

pub fn build_router(config: AppConfig) -> Router {
    let index_path = format!("{}/index.html", config.static_dir());
    let static_files = ServeDir::new(config.static_dir())
        .not_found_service(ServeFile::new(index_path));

    Router::new()
        .route("/health", axum::routing::get(health::get))
        .fallback_service(static_files)
        .layer(TraceLayer::new_for_http())
}
