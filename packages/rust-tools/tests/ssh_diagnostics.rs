use ai_tools::application::{activity, hooks};
use ai_tools::core::config::ServerConfig;
use serde_json::json;
use std::path::PathBuf;

fn fixture_config() -> ServerConfig {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    ServerConfig {
        dir: Some(root.to_string_lossy().into_owned()),
        execution_root: Some(root.to_string_lossy().into_owned()),
        ..ServerConfig::default()
    }
}

#[test]
fn dedicated_ssh_effects_are_read_only_remote_diagnostics() {
    let effects = hooks::effect_classes_for_call(
        "ssh_readonly_exec",
        false,
        true,
        &json!({"alias":"fixture","command":"docker","args":["logs","api","--tail","20"]}),
    );
    assert_eq!(
        effects,
        vec!["process_exec", "network_read", "privileged_bridge"]
    );
    assert!(!effects.contains(&"workspace_write"));
    assert!(!effects.contains(&"external_mutation"));

    let ordinary = hooks::effect_classes_for_call(
        "terminal_exec",
        true,
        true,
        &json!({"command":"cargo test"}),
    );
    assert!(ordinary.contains(&"workspace_write"));
    assert!(ordinary.contains(&"external_mutation"));
}

#[test]
fn ssh_activity_persists_metadata_not_remote_query_literals() {
    let arguments = json!({
        "alias":"prod",
        "command":"docker",
        "args":["exec","postgres","psql","-d","app","-c","SELECT email FROM users WHERE email=\"secret@example.com\""]
    });
    let event = activity::event_for_tool(
        &fixture_config(),
        "ssh_readonly_exec",
        &["process_exec", "network_read", "privileged_bridge"],
        &arguments,
        None,
    );
    let action = event.presentation.action.expect("activity action");
    assert_eq!(action, "SSH read-only · prod · docker exec psql");
    assert!(!action.contains("SELECT"));
    assert!(!action.contains("secret@example.com"));
    assert!(!action.contains("postgres"));
}

#[tokio::test]
async fn dedicated_ssh_tool_fails_closed_when_operator_capability_is_disabled() {
    use ai_tools::application::execution::{start_tool_task, JobManager};
    use ai_tools::interfaces::mcp::find_tool;

    let config = fixture_config();
    let manager = JobManager::new(config.clone());
    let tool = find_tool("ssh_readonly_exec").expect("dedicated SSH tool");
    let error = start_tool_task(
        &tool,
        &json!({"alias":"fixture","command":"docker","args":["ps"]}),
        &config,
        &manager,
        None,
        "disabled-ssh".into(),
    )
    .await
    .expect_err("disabled SSH capability must fail before spawn");
    assert!(error.to_string().contains("SSH diagnostics are disabled"));
}

#[tokio::test]
async fn generic_terminal_rejects_direct_ssh_before_spawn() {
    use ai_tools::application::execution::{start_terminal_job, JobManager};

    let config = fixture_config();
    let manager = JobManager::new(config.clone());
    let error = start_terminal_job(
        &json!({"command":"ssh","args":["example","uptime"]}),
        &config,
        &manager,
    )
    .await
    .expect_err("generic terminal must reject direct SSH");
    assert!(error.to_string().contains("ssh_readonly_exec"));
}

#[tokio::test]
async fn generic_terminal_shell_cannot_reach_masked_ssh_clients() {
    use ai_tools::application::execution::{start_terminal_job, JobManager, JobState};

    let mut config = fixture_config();
    config.allow_terminal_network = true;
    let manager = JobManager::new(config.clone());
    let task = start_terminal_job(
        &json!({
            "command": "sh",
            "args": ["-lc", "test ! -x /usr/bin/ssh && test ! -x /usr/bin/scp && test ! -x /usr/bin/sftp"]
        }),
        &config,
        &manager,
    )
    .await
    .expect("generic shell job admitted");
    let snapshot = manager
        .wait(&task)
        .await
        .expect("generic shell job completed");
    assert_eq!(snapshot.state, JobState::Completed);
    assert_eq!(snapshot.exit_code, Some(0));
}

#[tokio::test]
#[ignore = "requires an operator-provided disposable key-only SSH fixture"]
async fn opt_in_real_client_smoke_uses_the_relay_ssh_path() {
    use ai_tools::application::execution::{start_tool_task, JobManager, JobState};
    use ai_tools::interfaces::mcp::find_tool;
    use std::time::Duration;

    let ssh_root = std::env::var("RELAY_SSH_SMOKE_ROOT")
        .expect("set RELAY_SSH_SMOKE_ROOT to a disposable fixture credential directory");
    let alias = std::env::var("RELAY_SSH_SMOKE_ALIAS")
        .expect("set RELAY_SSH_SMOKE_ALIAS to the disposable fixture Host alias");
    let mut config = fixture_config();
    config.allow_ssh = true;
    config.ssh_root = Some(ssh_root.clone());
    config.ssh_config = Some(
        PathBuf::from(&ssh_root)
            .join("config")
            .to_string_lossy()
            .into_owned(),
    );
    config
        .validate()
        .expect("valid disposable SSH fixture config");

    let manager = JobManager::new(config.clone());
    let tool = find_tool("ssh_readonly_exec").expect("dedicated SSH tool");
    let task = start_tool_task(
        &tool,
        &json!({"alias": alias, "command": "docker", "args": ["ps"], "timeout_ms": 30_000}),
        &config,
        &manager,
        None,
        "ssh-smoke".into(),
    )
    .await
    .expect("SSH smoke job admitted");

    for _ in 0..100 {
        let snapshot = manager.get(&task).await.expect("retained SSH smoke job");
        match snapshot.state {
            JobState::Completed => {
                assert_eq!(snapshot.exit_code, Some(0));
                return;
            }
            JobState::Failed | JobState::TimedOut | JobState::Cancelled => {
                panic!("SSH smoke failed: {}", snapshot.stderr);
            }
            JobState::Queued | JobState::Running => {
                tokio::time::sleep(Duration::from_millis(100)).await
            }
        }
    }
    panic!("SSH smoke did not complete within the fixture wait bound");
}
