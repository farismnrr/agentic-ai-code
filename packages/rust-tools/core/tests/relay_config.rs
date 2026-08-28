use clap::Parser;
use relay_core::config::{Cli, SecurityMode, ServerConfig};

fn remote_config() -> ServerConfig {
    let root = env!("CARGO_MANIFEST_DIR").to_string();
    ServerConfig {
        mode: SecurityMode::Remote,
        dir: Some(root.clone()),
        execution_root: Some(root),
        origin: Some("http://100.99.88.53:3333".into()),
        oauth_issuer: Some("https://issuer.example/".into()),
        oauth_audience: Some("https://relay.example/mcp".into()),
        oauth_owner_subject: Some("owner".into()),
        bind_host: "0.0.0.0".into(),
        ..ServerConfig::default()
    }
}

#[test]
fn cli_accepts_explicit_bind_host_and_preserves_it_in_server_config() {
    let cli = Cli::try_parse_from([
        "ai-tools",
        "--mode",
        "remote",
        "--bind-host",
        "0.0.0.0",
        "--origin",
        "http://100.99.88.53:3333",
    ])
    .expect("bind-host should be a supported relay option");

    assert_eq!(ServerConfig::from(&cli).bind_host, "0.0.0.0");
}

#[test]
fn telegram_credentials_are_not_cli_arguments() {
    let parsed = Cli::try_parse_from([
        "ai-tools",
        "--telegram-enabled",
        "--telegram-bot-token",
        "token",
        "--telegram-chat-id",
        "-100123",
    ]);
    assert!(parsed.is_err());
}

#[test]
fn local_mode_rejects_non_loopback_bind() {
    let config = ServerConfig {
        bind_host: "0.0.0.0".into(),
        ..ServerConfig::default()
    };

    assert!(config.validate().is_err());
}

#[test]
fn remote_non_loopback_bind_requires_explicit_origin() {
    let mut config = remote_config();
    config.origin = None;

    assert!(config.validate().is_err());
}

#[test]
fn remote_non_loopback_bind_keeps_loopback_trusted_proxy_explicit() {
    let mut config = remote_config();
    config.trusted_proxy = true;
    config.trusted_proxy_cidr = Some("127.0.0.1/32".into());

    assert!(config.validate().is_ok());
}

#[test]
fn remote_non_loopback_bind_rejects_unrelated_trusted_proxy_cidr() {
    let mut config = remote_config();
    config.trusted_proxy = true;
    config.trusted_proxy_cidr = Some("10.0.0.0/8".into());

    assert!(config.validate().is_err());
}
