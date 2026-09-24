use axum::{routing::get, Json, Router};
use serde_json::json;
use std::{env, net::SocketAddr};
use tower_http::{services::{ServeDir, ServeFile}, trace::TraceLayer};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "sso_auth=info,tower_http=info".into()),
        )
        .init();

    let port = env::var("PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(3000);

    let static_files = ServeDir::new("dist")
        .not_found_service(ServeFile::new("dist/index.html"));

    let app = Router::new()
        .route("/health", get(health))
        .fallback_service(static_files)
        .layer(TraceLayer::new_for_http());

    let address = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!(%address, "sso-auth listening");

    let listener = tokio::net::TcpListener::bind(address)
        .await
        .expect("failed to bind sso-auth listener");

    axum::serve(listener, app)
        .await
        .expect("sso-auth server failed");
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok" }))
}
