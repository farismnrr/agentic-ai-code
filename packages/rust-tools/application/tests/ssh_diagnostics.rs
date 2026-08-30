use relay_application::{activity, hooks};
use relay_core::config::ServerConfig;
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
fn ssh_terminal_effects_are_read_only_remote_diagnostics() {
    let effects = hooks::effect_classes_for_call(
        "terminal_exec",
        true,
        true,
        &json!({"command":"ssh fixture docker logs api --tail 20"}),
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
        "command":"ssh prod docker exec postgres psql -d app -c 'SELECT email FROM users WHERE email=\"secret@example.com\"'"
    });
    let event = activity::event_for_tool(
        &fixture_config(),
        "terminal_exec",
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
#[ignore = "requires an operator-provided disposable key-only SSH fixture"]
async fn opt_in_real_client_smoke_uses_the_relay_ssh_path() {
    use relay_application::execution::{start_terminal_job, JobManager, JobState};
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
    let task = start_terminal_job(
        &json!({"command": format!("ssh {alias} docker ps"), "timeout_ms": 30_000}),
        &config,
        &manager,
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
