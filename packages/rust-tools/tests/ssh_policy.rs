use ai_tools::core::ssh_policy::{
    openssh_args, openssh_args_for_chain, resolve_connection_chain, resolve_connection_spec,
    validate_alias, validate_remote_command,
};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

struct Fixture {
    root: PathBuf,
    config: PathBuf,
}

impl Fixture {
    fn new(config: &str) -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("relay-ssh-policy-{}-{stamp}", std::process::id()));
        fs::create_dir_all(&root).expect("create fixture root");
        fs::write(root.join("id_ed25519"), "fixture-key-placeholder").expect("write key fixture");
        fs::write(root.join("known_hosts"), "example.test ssh-ed25519 fixture")
            .expect("write known hosts");
        let path = root.join("config");
        fs::write(&path, config).expect("write config");
        Self { root, config: path }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn safe_config(extra: &str) -> Fixture {
    Fixture::new(&format!(
        "Host smart-*\n  User ops\n  Port 2222\n\nHost smart-meeting\n  HostName example.test\n  IdentityFile ~/.ssh/id_ed25519\n  UserKnownHostsFile ~/.ssh/known_hosts\n{extra}\nHost *\n  User fallback\n"
    ))
}

#[test]
fn alias_validation_is_bounded_and_option_safe() {
    assert!(validate_alias("smart-meeting").is_ok());
    assert!(validate_alias("-oProxyCommand=evil").is_err());
    assert!(validate_alias("smart meeting").is_err());
    assert!(validate_alias("user@host").is_err());
}

#[test]
fn ssh_config_resolves_safe_subset_with_first_value_precedence() {
    let fixture = safe_config("");
    let spec = resolve_connection_spec(&fixture.root, &fixture.config, "smart-meeting")
        .expect("safe alias resolves");
    assert_eq!(spec.hostname, "example.test");
    assert_eq!(spec.user.as_deref(), Some("ops"));
    assert_eq!(spec.port, 2222);
    assert_eq!(spec.identity_file, fixture.root.join("id_ed25519"));
    assert_eq!(spec.known_hosts_file, fixture.root.join("known_hosts"));
}

#[test]
fn unused_ssh_capability_directives_are_inert_instead_of_poisoning_alias_resolution() {
    for directive in [
        "  ProxyCommand nc %h %p",
        "  LocalCommand touch /tmp/x",
        "  KnownHostsCommand helper",
        "  RemoteCommand sh",
        "  IdentityAgent /tmp/agent.sock",
        "  ControlMaster auto",
        "  LocalForward 1234 localhost:80",
        "  ForwardAgent yes",
    ] {
        let fixture = safe_config(directive);
        let spec = resolve_connection_spec(&fixture.root, &fixture.config, "smart-meeting")
            .unwrap_or_else(|_| panic!("unused raw directive must be inert: {directive}"));
        assert_eq!(spec.hostname, "example.test");
    }
}

#[test]
fn proxyjump_aliases_are_parsed_as_bounded_relay_owned_hops() {
    let fixture = safe_config("  ProxyJump arch,bastion");
    let spec = resolve_connection_spec(&fixture.root, &fixture.config, "smart-meeting")
        .expect("safe ProxyJump aliases resolve");
    assert_eq!(spec.proxy_jump, vec!["arch", "bastion"]);
}

#[test]
fn ssh_include_files_under_credential_root_are_supported() {
    let fixture = Fixture::new(
        "Include ~/.ssh/config.d/*\nHost *\n  IdentityFile ~/.ssh/id_ed25519\n  UserKnownHostsFile ~/.ssh/known_hosts\n",
    );
    let include_dir = fixture.root.join("config.d");
    fs::create_dir_all(&include_dir).expect("create include dir");
    fs::write(
        include_dir.join("hosts.conf"),
        "Host arch\n  HostName arch.example.test\n  User ops\n",
    )
    .expect("write included config");
    let spec =
        resolve_connection_spec(&fixture.root, &fixture.config, "arch").expect("included alias");
    assert_eq!(spec.hostname, "arch.example.test");
    assert_eq!(spec.user.as_deref(), Some("ops"));
}

#[test]
fn explicit_multi_hop_chain_builds_server_owned_proxy_transport() {
    let fixture = Fixture::new(
        "Host arch\n  HostName arch.example.test\n  User ops\n  IdentityFile ~/.ssh/id_ed25519\n  UserKnownHostsFile ~/.ssh/known_hosts\n\
         Host smart-meeting\n  HostName smart.example.test\n  User ops\n  IdentityFile ~/.ssh/id_ed25519\n  UserKnownHostsFile ~/.ssh/known_hosts\n\
         Host big\n  HostName big.example.test\n  User ops\n  IdentityFile ~/.ssh/id_ed25519\n  UserKnownHostsFile ~/.ssh/known_hosts\n",
    );
    let chain = resolve_connection_chain(
        &fixture.root,
        &fixture.config,
        "big",
        &["arch".into(), "smart-meeting".into()],
    )
    .expect("multi-hop aliases resolve");
    assert_eq!(
        chain
            .iter()
            .map(|spec| spec.alias.as_str())
            .collect::<Vec<_>>(),
        vec!["arch", "smart-meeting", "big"]
    );
    let remote = validate_remote_command("uptime", None, None).unwrap();
    let joined = openssh_args_for_chain(&chain, &remote).join(" ");
    assert!(joined.contains("ProxyCommand="));
    assert!(joined.contains("arch.example.test"));
    assert!(joined.contains("smart.example.test"));
    assert!(joined.contains("big.example.test"));
    assert!(joined.ends_with("big.example.test uptime"));
}

#[test]
fn credential_paths_cannot_escape_ssh_root() {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let outside = std::env::temp_dir().join(format!("relay-ssh-outside-{stamp}"));
    fs::write(&outside, "outside-key").expect("outside fixture");
    let fixture = Fixture::new(&format!(
        "Host smart-meeting\n HostName example.test\n IdentityFile {}\n UserKnownHostsFile ~/.ssh/known_hosts\n",
        outside.display()
    ));
    assert!(resolve_connection_spec(&fixture.root, &fixture.config, "smart-meeting").is_err());
    let _ = fs::remove_file(outside);
}

#[test]
fn openssh_argv_is_server_owned_and_non_interactive() {
    let fixture = safe_config("");
    let spec = resolve_connection_spec(&fixture.root, &fixture.config, "smart-meeting").unwrap();
    let remote = validate_remote_command("docker logs api --tail 20", None, None).unwrap();
    let args = openssh_args(&spec, &remote);
    let joined = args.join(" ");
    for expected in [
        "-F /dev/null",
        "BatchMode=yes",
        "PasswordAuthentication=no",
        "KbdInteractiveAuthentication=no",
        "IdentitiesOnly=yes",
        "IdentityAgent=none",
        "ClearAllForwardings=yes",
        "ControlMaster=no",
        "ControlPersist=no",
        "RequestTTY=no",
        "StdinNull=yes",
        "StrictHostKeyChecking=yes",
        "ConnectionAttempts=1",
    ] {
        assert!(joined.contains(expected), "missing {expected}: {joined}");
    }
    assert!(!joined.contains(fixture.config.to_string_lossy().as_ref()));
}

#[test]
fn redis_password_flow_keeps_final_ssh_stdin_available() {
    let fixture = safe_config("");
    let spec = resolve_connection_spec(&fixture.root, &fixture.config, "smart-meeting").unwrap();
    let remote = validate_remote_command(
        "docker exec redis redis-cli PING",
        None,
        Some("relay_readonly"),
    )
    .unwrap();
    let joined = openssh_args(&spec, &remote).join(" ");
    assert!(joined.contains("StdinNull=no"));
    assert!(!joined.contains("StdinNull=yes"));
}

#[test]
fn docker_read_diagnostics_are_normalized_and_bounded() {
    let logs = validate_remote_command("docker logs api", None, None).unwrap();
    assert_eq!(logs.rendered, "docker logs --tail 200 api");

    let stats = validate_remote_command("docker stats api", None, None).unwrap();
    assert_eq!(stats.rendered, "docker stats --no-stream api");

    let pipeline = validate_remote_command(
        "docker logs api --tail 100 | grep ERROR | tail -20",
        None,
        None,
    )
    .unwrap();
    assert!(pipeline
        .rendered
        .contains("docker logs --tail 100 api | grep ERROR | tail -20"));

    let inspect = validate_remote_command("docker inspect api", None, None).unwrap();
    assert!(inspect.rendered.contains("--format"));
    assert!(!inspect.rendered.to_ascii_lowercase().contains("config.env"));

    let top = validate_remote_command("docker top api", None, None).unwrap();
    assert!(top.rendered.contains("pid,ppid,user,stat,etime,comm"));
    assert!(!top.rendered.contains("args"));
}

#[test]
fn docker_mutation_interactivity_and_confidentiality_bypasses_are_denied() {
    for command in [
        "docker restart api",
        "docker stop api",
        "docker exec api sh",
        "docker exec -it api cat /tmp/x",
        "docker compose exec api sh -c 'touch /tmp/x'",
        "docker compose up -d",
        "docker compose config",
        "docker logs -f api",
        "docker logs api --tail 999999",
        "docker top api -eo pid,args",
    ] {
        assert!(
            validate_remote_command(command, None, None).is_err(),
            "must deny: {command}"
        );
    }
}

#[test]
fn docker_listing_accepts_bounded_custom_format_without_enabling_mutation() {
    let command = "docker ps --format '{{.Names}}\t{{.Status}}'";
    let allowed = validate_remote_command(command, None, None).expect("custom docker format");
    assert!(allowed.rendered.contains("--format"));
    assert!(allowed.rendered.contains("{{.Names}}"));
    assert!(validate_remote_command("docker ps --format '$HOME'", None, None).is_err());
}

#[test]
fn postgres_requires_readonly_identity_and_rejects_write_or_expensive_queries() {
    let direct = "psql -d app -c 'SELECT count(*) FROM users'";
    assert!(validate_remote_command(direct, None, None).is_err());
    let direct_allowed = validate_remote_command(direct, Some("relay_readonly"), None).unwrap();
    assert!(direct_allowed.rendered.contains("BEGIN READ ONLY"));

    let compose = "docker compose exec -T postgres psql -d app -c 'SELECT count(*) FROM users'";
    let compose_allowed = validate_remote_command(compose, Some("relay_readonly"), None).unwrap();
    assert!(compose_allowed
        .rendered
        .contains("docker compose exec -T postgres psql"));
    assert!(compose_allowed.rendered.contains("BEGIN READ ONLY"));

    let select = "docker exec postgres psql -d app -c 'SELECT count(*) FROM users'";
    assert!(validate_remote_command(select, None, None).is_err());
    let allowed = validate_remote_command(select, Some("relay_readonly"), None).unwrap();
    assert!(allowed.rendered.contains("BEGIN READ ONLY"));
    assert!(allowed.rendered.contains("statement_timeout"));
    assert!(allowed.rendered.contains("-w"));
    assert!(allowed.rendered.contains("-U relay_readonly"));

    for query in [
        "docker exec postgres psql -d app -c 'DELETE FROM users'",
        "docker exec postgres psql -d app -c 'WITH gone AS (DELETE FROM users RETURNING *) SELECT * FROM gone'",
        "docker exec postgres psql -d app -c 'SELECT pg_sleep(20)'",
        "docker exec postgres psql -d app -c 'SELECT * INTO temp_copy FROM users'",
    ] {
        assert!(
            validate_remote_command(query, Some("relay_readonly"), None).is_err(),
            "must deny: {query}"
        );
    }
}

#[test]
fn redis_requires_acl_identity_and_blocks_dangerous_or_unbounded_commands() {
    assert!(validate_remote_command("docker exec redis redis-cli GET foo", None, None).is_err());
    let allowed = validate_remote_command(
        "docker exec redis redis-cli GET foo",
        None,
        Some("relay_readonly"),
    )
    .unwrap();
    assert!(allowed.rendered.contains("docker exec -i redis redis-cli"));
    assert!(allowed.rendered.contains("--askpass"));
    assert!(allowed.rendered.contains("--user relay_readonly"));
    assert!(allowed.requires_redis_password);
    assert!(validate_remote_command(
        "docker exec redis redis-cli GET foo | head -n 1",
        None,
        Some("relay_readonly"),
    )
    .is_err());

    for command in [
        "docker exec redis redis-cli KEYS '*'",
        "docker exec redis redis-cli MONITOR",
        "docker exec redis redis-cli SET foo bar",
        "docker exec redis redis-cli EVAL 'return 1' 0",
    ] {
        assert!(
            validate_remote_command(command, None, Some("relay_readonly")).is_err(),
            "must deny: {command}"
        );
    }
}

#[test]
fn shell_write_escape_and_sensitive_read_surfaces_are_denied() {
    for command in [
        "cat /app/.env",
        "cat /root/.ssh/id_ed25519",
        "cat /proc/1/environ",
        "cat /run/secrets/db_password",
        "cat /tmp/x > /tmp/y",
        "tail -f /tmp/app.log",
        "head -n 100000 /tmp/app.log",
        "grep -R ERROR /var/log",
        "docker logs api &",
        "docker logs api; rm -rf /tmp/x",
        "echo $(touch /tmp/x)",
        "bash -c 'docker logs api'",
        "python3 -c 'print(1)'",
    ] {
        assert!(
            validate_remote_command(command, None, None).is_err(),
            "must deny: {command}"
        );
    }
}

#[test]
fn common_remote_filesystem_observation_is_bounded_and_sensitive_paths_stay_denied() {
    for command in [
        "ls -lah /var/log",
        "du -sh /var/log",
        "readlink -f /var/log",
    ] {
        assert!(
            validate_remote_command(command, None, None).is_ok(),
            "must allow: {command}"
        );
    }
    for command in [
        "ls -R /",
        "du --max-depth=99 /var/log",
        "readlink /root/.ssh/id_ed25519",
    ] {
        assert!(
            validate_remote_command(command, None, None).is_err(),
            "must deny: {command}"
        );
    }
}

#[test]
fn git_read_only_subset_disables_pager_and_rejects_mutation() {
    let status = validate_remote_command("git status --short", None, None).unwrap();
    assert_eq!(status.rendered, "git --no-pager status --short");
    let log = validate_remote_command("git log --all", None, None).unwrap();
    assert!(log.rendered.contains("--no-patch"));
    assert!(log.rendered.contains("--max-count=100"));
    for command in [
        "git checkout main",
        "git -c core.pager=sh status",
        "git diff --output=/tmp/leak",
        "git log --ext-diff",
        "git show HEAD",
    ] {
        assert!(
            validate_remote_command(command, None, None).is_err(),
            "must deny: {command}"
        );
    }
}

#[test]
fn host_network_diagnostics_reject_mutation_and_streaming_modes() {
    for command in [
        "ip link set eth0 down",
        "ip addr add 10.0.0.2/24 dev eth0",
        "ip route flush table main",
        "ip monitor",
        "ss -K dst 10.0.0.1",
        "free -s 1",
    ] {
        assert!(
            validate_remote_command(command, None, None).is_err(),
            "must deny: {command}"
        );
    }
    assert!(validate_remote_command("ip addr show", None, None).is_ok());
    assert!(validate_remote_command("ss -ltn", None, None).is_ok());
}

#[test]
fn observability_commands_are_bounded_and_read_only() {
    let journal = validate_remote_command(
        "journalctl -u api.service -n 50 --since '10 minutes ago'",
        None,
        None,
    )
    .expect("bounded journal query");
    assert!(journal.rendered.contains("--no-pager"));
    assert!(journal.rendered.contains("api.service"));
    assert!(journal.rendered.contains("--lines 50") || journal.rendered.contains("-n 50"));

    let status =
        validate_remote_command("systemctl status api.service", None, None).expect("status query");
    assert!(status
        .rendered
        .contains("systemctl --no-pager status api.service"));

    for command in [
        "journalctl -f",
        "journalctl --vacuum-time=1d",
        "systemctl restart api.service",
        "systemctl stop api.service",
        "systemctl enable api.service",
    ] {
        assert!(
            validate_remote_command(command, None, None).is_err(),
            "must deny: {command}"
        );
    }
}

#[test]
fn curl_diagnostics_block_metadata_credentials_and_write_surfaces() {
    assert!(validate_remote_command("curl https://example.com/health", None, None).is_ok());
    for command in [
        "curl http://169.254.169.254/latest/meta-data/",
        "curl https://user:pass@example.com/",
        "curl -H 'Authorization: Bearer secret' https://example.com/",
        "curl -o /tmp/out https://example.com/",
        "curl -X POST https://example.com/",
    ] {
        assert!(
            validate_remote_command(command, None, None).is_err(),
            "must deny: {command}"
        );
    }
}
