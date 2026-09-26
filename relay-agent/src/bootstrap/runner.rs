use std::{error::Error, sync::Arc};

use crate::{
    application::{ConnectCallbackUseCase, ConnectStartUseCase, ConnectionStatusUseCase},
    infrastructure::{
        config::AppConfig, connection_store::InMemoryConnectionRepository,
        mcp_token::SignedMcpAccessTokenVerifier, random_token::SecureTokenGenerator,
        sso::SsoConnectUrlBuilder, verification::SignedRelayAssertionVerifier,
    },
    interfaces::http::{
        build_router, DiscoveryDocument, ProtectedResourceMetadata, RelayHttpState,
    },
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
        client_id.to_string(),
    )?);
    let verifier = Arc::new(SignedRelayAssertionVerifier::new(
        config.relay_assertion_secret(),
        config.sso_issuer(),
        client_id.to_string(),
    )?);

    let sso_dashboard_url = config.sso_base_url();
    let public_url = config.relay_public_url();
    let mcp_resource_url = public_url.as_str().trim_end_matches('/').to_string();
    let mcp_tokens = Arc::new(SignedMcpAccessTokenVerifier::new(
        config.relay_assertion_secret(),
        config.sso_issuer(),
        mcp_resource_url.clone(),
    )?);
    let mcp_resource = ProtectedResourceMetadata::new(mcp_resource_url, config.sso_issuer());

    let discovery = DiscoveryDocument::new(
        client_id.to_string(),
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
        sso_dashboard_url,
        mcp_tokens,
        mcp_resource,
    );

    let listener = tokio::net::TcpListener::bind(address).await?;
    eprintln!("relay-agent listening on {address}");
    axum::serve(listener, build_router(http_state)).await?;
    Ok(())
}
