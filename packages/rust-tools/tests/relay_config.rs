use ai_tools::core::config::{Cli, SecurityMode, ServerConfig};
use ai_tools::interfaces::mcp::{LEGACY_PROTOCOL_VERSIONS, PROTOCOL_VERSION};
use clap::Parser;

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
fn relay_advertises_only_the_fully_implemented_modern_protocol() {
    assert_eq!(PROTOCOL_VERSION, "2026-07-28");
    assert!(LEGACY_PROTOCOL_VERSIONS.is_empty());
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

#[cfg(unix)]
fn chmod(path: &std::path::Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
}

fn ssh_fixture() -> (ServerConfig, std::path::PathBuf) {
    let root = std::env::temp_dir().join(format!(
        "relay-config-ssh-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    #[cfg(unix)]
    chmod(&root, 0o700);
    let config_path = root.join("config");
    let key = root.join("id_ed25519");
    let known = root.join("known_hosts");
    std::fs::write(&key, "dummy").unwrap();
    std::fs::write(&known, "dummy").unwrap();
    std::fs::write(
        &config_path,
        "Host fixture\n HostName example.invalid\n User diagnostic\n IdentityFile id_ed25519\n UserKnownHostsFile known_hosts\n",
    )
    .unwrap();
    #[cfg(unix)]
    for path in [&config_path, &key, &known] {
        chmod(path, 0o600);
    }
    let workspace = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let config = ServerConfig {
        dir: Some(workspace.to_string_lossy().into_owned()),
        execution_root: Some(workspace.to_string_lossy().into_owned()),
        allow_ssh: true,
        ssh_root: Some(root.to_string_lossy().into_owned()),
        ssh_config: Some(config_path.to_string_lossy().into_owned()),
        ssh_readonly_db_user: Some("relay_reader".into()),
        ssh_readonly_redis_user: Some("relay_reader".into()),
        ..ServerConfig::default()
    };
    (config, root)
}

#[test]
fn ssh_configuration_is_opt_in_and_does_not_enable_terminal_network() {
    let (config, root) = ssh_fixture();
    assert!(config.validate().is_ok());
    assert!(!config.allow_terminal_network);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn ssh_specific_configuration_is_rejected_when_ssh_is_disabled() {
    let (mut config, root) = ssh_fixture();
    config.allow_ssh = false;
    assert!(config.validate().is_err());
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn ssh_root_must_not_be_group_or_world_writable() {
    let (config, root) = ssh_fixture();
    #[cfg(unix)]
    chmod(&root, 0o777);
    assert!(config.validate().is_err());
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn ssh_database_principals_are_bounded_identity_names() {
    let (mut config, root) = ssh_fixture();
    config.ssh_readonly_db_user = Some("reader;DROP TABLE users".into());
    assert!(config.validate().is_err());
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn explicit_toolchain_paths_may_be_outside_the_execution_root() {
    let (mut config, root) = ssh_fixture();
    let toolchain = root.join("toolchain");
    std::fs::create_dir(&toolchain).unwrap();
    #[cfg(unix)]
    chmod(&toolchain, 0o700);
    config.toolchain_paths = vec![toolchain.to_string_lossy().into_owned()];

    assert!(config.validate().is_ok());
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn protected_ssh_directory_at_execution_root_boundary_is_allowed() {
    let root = std::env::temp_dir().join(format!(
        "relay-config-home-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let ssh_root = root.join(".ssh");
    let workspace = root.join("workspace");
    std::fs::create_dir_all(&ssh_root).unwrap();
    std::fs::create_dir(&workspace).unwrap();
    #[cfg(unix)]
    for path in [&root, &ssh_root, &workspace] {
        chmod(path, 0o700);
    }
    let config_path = ssh_root.join("config");
    let key = ssh_root.join("id_ed25519");
    let known = ssh_root.join("known_hosts");
    std::fs::write(&key, "dummy").unwrap();
    std::fs::write(&known, "dummy").unwrap();
    std::fs::write(
        &config_path,
        "Host fixture\n HostName example.invalid\n User diagnostic\n IdentityFile id_ed25519\n UserKnownHostsFile known_hosts\n",
    )
    .unwrap();
    #[cfg(unix)]
    for path in [&config_path, &key, &known] {
        chmod(path, 0o600);
    }
    let config = ServerConfig {
        dir: Some(workspace.to_string_lossy().into_owned()),
        execution_root: Some(root.to_string_lossy().into_owned()),
        allow_ssh: true,
        ssh_root: Some(ssh_root.to_string_lossy().into_owned()),
        ssh_config: Some(config_path.to_string_lossy().into_owned()),
        ..ServerConfig::default()
    };

    assert!(config.validate().is_ok());
    let _ = std::fs::remove_dir_all(root);
}
