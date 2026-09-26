use std::{error::Error, sync::Arc};

use crate::{
    application::{ConnectCallbackUseCase, ConnectStartUseCase, ConnectionStatusUseCase},
    infrastructure::{
        config::AppConfig, connection_store::InMemoryConnectionRepository,
        random_token::SecureTokenGenerator, sso::SsoConnectUrlBuilder,
        verification::SignedRelayAssertionVerifier,
    },
    interfaces::http::{build_router, DiscoveryDocument, RelayHttpState},
};

pub async fn run() -> Result<(), Box<dyn Error>> {
    let config = AppConfig::from_env()?;
    let address = config.listen_address();
    let client_id = config.relay_client_id();

    let connections = Arc::new(InMemoryConnectionRepository::new(
        config.connection_attempt_ttl_seconds(),
    ));
    let sso_connect = Arc::new(SsoConnectUrlBuilder::new(
        config.sso_base_url(),
        client_id.clone(),
    )?);
    let verifier = Arc::new(SignedRelayAssertionVerifier::new(
        config.relay_assertion_secret(),
        config.sso_issuer(),
        client_id.clone(),
    )?);

    let public_url = config.relay_public_url();
    let discovery = DiscoveryDocument::new(
        client_id,
        public_url.join("/connections/start")?.to_string(),
        format!(
            "{}/connections/{{connectionId}}",
            public_url.as_str().trim_end_matches('/')
        ),
    );

    let http_state = RelayHttpState::new(
        Arc::new(ConnectStartUseCase::new(
            connections.clone(),
            Arc::new(SecureTokenGenerator),
            sso_connect,
        )),
        Arc::new(ConnectCallbackUseCase::new(verifier, connections.clone())),
        Arc::new(ConnectionStatusUseCase::new(connections)),
        discovery,
    );

    let listener = tokio::net::TcpListener::bind(address).await?;
    eprintln!("relay-agent listening on {address}");
    axum::serve(listener, build_router(http_state)).await?;
    Ok(())
}
