use std::{error::Error, sync::Arc};

use crate::{
    application::{AuthService, ConnectedAppService, ConnectionService},
    infrastructure::{
        access::GitHubUserAllowlist, config::AppConfig, connected_apps::FileConnectedAppRepository,
        github::GitHubOAuthClient, random_state::SecureStateGenerator,
        relay_assertion::SignedRelayAssertionIssuer, session::SignedSessionCodec,
    },
    interfaces::http::{build_router, AuthHttpState},
};

pub async fn run() -> Result<(), Box<dyn Error>> {
    init_tracing();

    let config = AppConfig::from_env()?;
    let address = config.listen_address();
    let oauth = Arc::new(GitHubOAuthClient::new(
        config.github_client_id(),
        config.github_client_secret(),
        config.github_callback_url(),
        config.github_authorize_url(),
        config.github_token_url(),
        config.github_api_url(),
    )?);
    let sessions = Arc::new(SignedSessionCodec::new(
        config.session_secret(),
        config.session_ttl_seconds(),
    )?);
    let access_policy = Arc::new(GitHubUserAllowlist::new(config.allowed_github_user_ids())?);
    let auth = Arc::new(AuthService::new(
        oauth,
        sessions,
        Arc::new(SecureStateGenerator),
        access_policy,
    ));

    let app_registry = Arc::new(FileConnectedAppRepository::new(
        config.connected_apps_path(),
    )?);
    let assertions = Arc::new(SignedRelayAssertionIssuer::new(
        config.relay_assertion_secret(),
        config.sso_issuer(),
    )?);
    let connections = Arc::new(ConnectionService::new(assertions, app_registry.clone()));
    let connected_apps = Arc::new(ConnectedAppService::new(app_registry));

    let http_state = AuthHttpState {
        auth,
        connections,
        connected_apps,
        cookie_secure: config.cookie_secure(),
        session_ttl_seconds: config.session_ttl_seconds(),
    };

    let listener = tokio::net::TcpListener::bind(address).await?;
    tracing::info!(%address, "sso-auth listening");
    axum::serve(listener, build_router(http_state)).await?;
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
