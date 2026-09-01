use ai_tools::core::ssh_policy::{
    openssh_args, resolve_connection_spec, validate_alias, validate_remote_command,
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
fn dangerous_ssh_config_directives_fail_closed() {
    for directive in [
        "  ProxyCommand nc %h %p",
        "  ProxyJump bastion",
        "  LocalCommand touch /tmp/x",
        "  KnownHostsCommand helper",
        "  Match exec true",
        "  RemoteCommand sh",
        "  IdentityAgent /tmp/agent.sock",
        "  ControlMaster auto",
        "  LocalForward 1234 localhost:80",
        "  ForwardAgent yes",
        "  Include other.conf",
    ] {
        let fixture = safe_config(directive);
        assert!(
            resolve_connection_spec(&fixture.root, &fixture.config, "smart-meeting").is_err(),
            "directive should fail: {directive}"
        );
    }
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
        "docker compose exec api cat /tmp/x",
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
fn postgres_requires_readonly_identity_and_rejects_write_or_expensive_queries() {
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
    assert!(allowed.rendered.contains("--user relay_readonly"));

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
