//! Deterministic local acceptance for Plan 050's encrypted journal boundary.

use relay_application::activity::{
    complete_event, ActivityEvent, Category, Effect, Evidence, Presentation, Status,
};
use relay_core::config::{ActivityMode, ServerConfig};
use relay_infrastructure::activity::ActivityRuntime;
use rusqlite::Connection;
use serde_json::json;
use std::fs;
use std::path::PathBuf;

struct TempState(PathBuf);

impl TempState {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let path = std::env::temp_dir().join(format!("plan050-activity-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path)?;
        Ok(Self(path))
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
            summary: Some("canary".into()),
            result_class: Some("started".into()),
            evidence: Evidence::NotApplicable,
            payload_reference: None,
            complete: false,
        },
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let state = TempState::new()?;
    let mut config = ServerConfig::default();
    config.activity.mode = ActivityMode::Required;
    config.activity.state_dir = Some(state.0.to_string_lossy().into_owned());

    let runtime = ActivityRuntime::open(&config)?;
    let started = runtime.record_start(
        event("plan050-complete"),
        Some(serde_json::to_vec(&json!({"secret":"plan050-canary"}))?),
    )?;
    runtime.record_outcome(
        complete_event(
            &started,
            Status::Ok,
            3,
            "canary complete",
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
        !String::from_utf8_lossy(&envelope).contains("plan050-canary"),
        "journal envelope must not contain plaintext event data"
    );
    drop(connection);

    let runtime = ActivityRuntime::open(&config)?;
    let _ = runtime.record_start(event("plan050-unfinished"), None)?;
    drop(runtime);
    let runtime = ActivityRuntime::open(&config)?;
    drop(runtime);
    let connection = Connection::open(state.0.join("activity.sqlite3"))?;
    let interrupted: u64 = connection.query_row(
        "SELECT COUNT(*) FROM activity_journal WHERE status = 'interrupted'",
        [],
        |row| row.get(0),
    )?;
    assert_eq!(
        interrupted, 1,
        "restart must recover only the latest unfinished activity"
    );
    println!("Plan 050 journal acceptance: PASS");
    Ok(())
}
