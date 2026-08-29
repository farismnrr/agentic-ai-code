use relay_application::activity::{
    complete_event, event_for_tool, ActivityEvent, ActivityRecorder, Category, Effect, Evidence,
    Presentation, Status,
};
use relay_core::config::{ActivityMode, ServerConfig};
use relay_infrastructure::activity::{ActivityRuntime, ReloadableActivityRecorder};
use rusqlite::Connection;
use serde_json::json;
use std::fs;
use std::path::PathBuf;

struct TempState(PathBuf);

impl TempState {
    fn new(label: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let path = std::env::temp_dir().join(format!(
            "activity-test-{label}-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&path)?;
        Ok(Self(path))
    }

    fn config(&self) -> ServerConfig {
        let mut config = ServerConfig::default();
        config.activity.mode = ActivityMode::Required;
        config.activity.state_dir = Some(self.0.to_string_lossy().into_owned());
        config
    }
}

impl Drop for TempState {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn event(activity_id: &str) -> ActivityEvent {
    ActivityEvent {
        contract_version: "activity.event.v1".into(),
        activity_id: activity_id.into(),
        source_id: String::new(),
        source_sequence: 0,
        status: Status::Started,
        tool_id: "file_write".into(),
        category: Category::Filesystem,
        effects: vec![Effect::WorkspaceWrite],
        workspace_root_fingerprint: None,
        actor: relay_application::activity::actor_or_external(None),
        client_info: None,
        occurred_at_ms: 1,
        ingested_at_ms: None,
        duration_ms: None,
        presentation: Presentation {
            target: Some("notes.txt".into()),
            action: Some("Create notes.txt".into()),
            summary: Some("canary".into()),
            result_detail: None,
            result_class: Some("started".into()),
            evidence: Evidence::NotApplicable,
            payload_reference: None,
            complete: false,
        },
    }
}

#[test]
fn relay_export_contract_excludes_ingestion_owned_timestamp(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut sample = event("activity-contract");
    sample.ingested_at_ms = Some(123);
    let value = serde_json::to_value(sample)?;
    assert!(
        value.get("ingested_at_ms").is_none(),
        "relay export envelopes must not serialize Nuxt-owned ingestion metadata"
    );
    Ok(())
}

#[test]
fn activity_action_summary_is_useful_and_redacts_credentials() {
    let config = ServerConfig::default();
    let event = event_for_tool(
        &config,
        "terminal_exec",
        &["process_exec"],
        &json!({
            "command": "sh",
            "args": ["-lc", "curl -H 'Authorization: Bearer activity-secret' https://example.test && echo done"],
        }),
        None,
    );
    let action = event.presentation.action.expect("terminal action summary");
    assert!(action.starts_with("sh -lc curl"));
    assert!(action.contains("Bearer [REDACTED]"));
    assert!(!action.contains("activity-secret"));
}

#[test]
fn activity_actions_include_specific_arguments_for_non_terminal_tools() {
    let config = ServerConfig::default();
    let event = event_for_tool(
        &config,
        "code_hover",
        &["workspace_read"],
        &json!({"path":"src/main.ts","line":12,"column":4}),
        None,
    );
    let action = event.presentation.action.expect("specific action");
    assert!(action.contains("code hover"));
    assert!(action.contains("path=src/main.ts"));
    assert!(action.contains("line=12"));
}

#[test]
fn terminal_job_start_uses_the_actual_command() {
    let config = ServerConfig::default();
    let event = event_for_tool(
        &config,
        "terminal_job_start",
        &["process_exec"],
        &json!({"command":"cat","args":["file.txt"]}),
        None,
    );
    assert_eq!(event.presentation.action.as_deref(), Some("cat file.txt"));
}

#[tokio::test]
async fn journal_persists_start_and_outcome_without_plaintext_payload(
) -> Result<(), Box<dyn std::error::Error>> {
    let state = TempState::new("durability")?;
    let config = state.config();
    let runtime = ActivityRuntime::open(&config)?;

    let started = runtime.record_start(
        event("activity-complete"),
        Some(serde_json::to_vec(&json!({"secret":"activity-canary"}))?),
    )?;
    runtime.record_outcome(
        complete_event(
            &started,
            Status::Ok,
            3,
            "canary complete",
            Some("full result"),
            Evidence::Summary,
            None,
        ),
        None,
    )?;
    drop(runtime);

    let connection = Connection::open(state.0.join("activity.sqlite3"))?;
    let count: u64 = connection.query_row("SELECT COUNT(*) FROM activity_journal", [], |row| {
        row.get(0)
    })?;
    assert_eq!(count, 2, "start and terminal outcome must be durable");
    let envelope: Vec<u8> =
        connection.query_row("SELECT envelope FROM activity_journal LIMIT 1", [], |row| {
            row.get(0)
        })?;
    assert!(
        !String::from_utf8_lossy(&envelope).contains("activity-canary"),
        "journal envelope must not contain plaintext event data"
    );
    Ok(())
}

#[tokio::test]
async fn restart_marks_only_unfinished_activity_interrupted(
) -> Result<(), Box<dyn std::error::Error>> {
    let state = TempState::new("restart")?;
    let config = state.config();

    let runtime = ActivityRuntime::open(&config)?;
    let _ = runtime.record_start(event("activity-unfinished"), None)?;
    drop(runtime);
    drop(ActivityRuntime::open(&config)?);

    let connection = Connection::open(state.0.join("activity.sqlite3"))?;
    let interrupted: u64 = connection.query_row(
        "SELECT COUNT(*) FROM activity_journal WHERE status = 'interrupted'",
        [],
        |row| row.get(0),
    )?;
    assert_eq!(
        interrupted, 1,
        "restart must recover the unfinished activity"
    );
    Ok(())
}

#[tokio::test]
async fn restart_recovers_unfinished_activity_even_when_retry_is_backed_off(
) -> Result<(), Box<dyn std::error::Error>> {
    let state = TempState::new("restart-backed-off")?;
    let config = state.config();

    let runtime = ActivityRuntime::open(&config)?;
    let _ = runtime.record_start(event("activity-backed-off"), None)?;
    drop(runtime);

    let database = state.0.join("activity.sqlite3");
    let connection = Connection::open(&database)?;
    connection.execute(
        "UPDATE activity_journal SET attempts = 8, next_attempt_ms = 9999999999999 WHERE status = 'started'",
        [],
    )?;
    drop(connection);

    drop(ActivityRuntime::open(&config)?);

    let connection = Connection::open(database)?;
    let interrupted: u64 = connection.query_row(
        "SELECT COUNT(*) FROM activity_journal WHERE status = 'interrupted'",
        [],
        |row| row.get(0),
    )?;
    assert_eq!(
        interrupted, 1,
        "restart must recover unfinished activity even when its retry is backed off"
    );
    Ok(())
}

#[tokio::test]
async fn bootstrap_hot_reload_persists_across_restart() -> Result<(), Box<dyn std::error::Error>> {
    let state = TempState::new("bootstrap")?;
    let mut config = ServerConfig::default();
    config.activity.state_dir = Some(state.0.to_string_lossy().into_owned());

    let recorder = ReloadableActivityRecorder::open(&config)?;
    assert!(!recorder.status()?.0);
    recorder.configure(
        "https://chat.example.test/api/activity/ingest".into(),
        "activity-source-token-0123456789abcdef".into(),
    )?;
    let (configured, source_id) = recorder.status()?;
    assert!(configured);
    assert!(source_id.is_some());
    drop(recorder);

    let reopened = ReloadableActivityRecorder::open(&config)?;
    let (reopened_configured, reopened_source_id) = reopened.status()?;
    assert!(reopened_configured);
    assert_eq!(reopened_source_id, source_id);

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = fs::metadata(state.0.join("activity-bootstrap.json"))?;
        assert_eq!(metadata.permissions().mode() & 0o077, 0);
    }
    Ok(())
}
