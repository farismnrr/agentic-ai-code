use std::{error::Error, sync::Arc};

use crate::{
    application::{AuthCallbackUseCase, AuthStartUseCase},
    infrastructure::{
        config::AppConfig, sso::SsoLoginUrlBuilder,
        verification::PendingSignedAssertionVerifier,
    },
    interfaces::http::{build_router, AuthHttpState},
};

pub async fn run() -> Result<(), Box<dyn Error>> {
    let config = AppConfig::from_env()?;
    let address = config.listen_address();

    let sso_login = Arc::new(SsoLoginUrlBuilder::new(
        config.sso_base_url(),
        config.relay_public_url(),
    )?);
    let auth_state = AuthHttpState {
        start: Arc::new(AuthStartUseCase::new(sso_login)),
        callback: Arc::new(AuthCallbackUseCase::new(Arc::new(
            PendingSignedAssertionVerifier,
        ))),
    };

    let listener = tokio::net::TcpListener::bind(address).await?;
    eprintln!("relay-agent listening on {address}");
    axum::serve(listener, build_router(auth_state)).await?;
    Ok(())
}
