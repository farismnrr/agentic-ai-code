use std::error::Error;

use crate::{
    infrastructure::config::AppConfig,
    interfaces::http::build_router,
};

pub async fn run() -> Result<(), Box<dyn Error>> {
    init_tracing();

    let config = AppConfig::from_env()?;
    let address = config.listen_address();
    let listener = tokio::net::TcpListener::bind(address).await?;

    tracing::info!(%address, "sso-auth listening");
    axum::serve(listener, build_router(config)).await?;

    Ok(())
}

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "sso_auth=info,tower_http=info".into()),
        )
        .init();
}
