use axum::{routing::get, Router};

use super::{auth, AuthHttpState};

pub fn build_router(state: AuthHttpState) -> Router {
    Router::new()
        .route("/auth/login", get(auth::login))
        .route("/auth/callback", get(auth::callback))
        .with_state(state)
}
