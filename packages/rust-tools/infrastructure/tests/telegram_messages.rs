use relay_application::dispatcher::{dispatch, Dispatch};
use relay_core::config::ServerConfig;
use relay_infrastructure::notifications::{
    format_telegram_message, payload_from_arguments, sanitize_telegram_message,
    telegram_send_message_body, validate_channel_target, validate_message_thread_id, QueueResult,
    TelegramMessageLedger, TelegramMessagePayload, TelegramMessageService, MAX_MESSAGE_BYTES,
};
use relay_interfaces::mcp::{tool_catalog_for_profile, DiscoverResult, Id, Request};
use rusqlite::Connection;
use serde_json::json;
use std::fs;
use uuid::Uuid;

fn payload() -> TelegramMessagePayload {
    TelegramMessagePayload {
        working_directory: "/workspace/ai-code".into(),
        message: "Implemented the feature and ran the focused tests.".into(),
    }
}

fn workspace_config() -> ServerConfig {
    let root = env!("CARGO_MANIFEST_DIR").to_string();
    ServerConfig {
        dir: Some(root.clone()),
        execution_root: Some(root),
        ..ServerConfig::default()
    }
}

#[test]
fn telegram_payload_is_bounded_redacted_and_includes_working_directory() {
    let sanitized = sanitize_telegram_message(payload()).expect("payload should be valid");
    assert_eq!(
        format_telegram_message(&sanitized),
        "Working directory: /workspace/ai-code\n\nImplemented the feature and ran the focused tests."
    );

    let unsafe_payload = TelegramMessagePayload {
        message: "Authorization: Bearer super-secret password=hidden".into(),
        ..payload()
    };
    let sanitized = sanitize_telegram_message(unsafe_payload).expect("payload should sanitize");
    assert!(sanitized.message.contains("Bearer [REDACTED]"));
    assert!(sanitized.message.contains("password=[REDACTED]"));
    assert!(!sanitized.message.contains("super-secret"));
    assert!(!sanitized.message.contains("hidden"));
    assert!(format_telegram_message(&sanitized).len() <= 4096);

    let oversized_combination = TelegramMessagePayload {
        working_directory: format!("/{}", "a".repeat(MAX_MESSAGE_BYTES - 1)),
        message: "x".into(),
    };
    assert!(
        sanitize_telegram_message(oversized_combination).is_err(),
        "the relay must reject a payload that cannot preserve the full canonical directory"
    );
}

#[test]
fn telegram_tool_arguments_require_authorized_absolute_working_directory() {
    let config = workspace_config();
    let canonical = fs::canonicalize(env!("CARGO_MANIFEST_DIR")).expect("canonical root");
    let valid = payload_from_arguments(
        &json!({
            "working_directory": canonical.to_string_lossy(),
            "message": "hello"
        }),
        &config,
    )
    .expect("authorized payload");
    assert_eq!(valid.working_directory, canonical.to_string_lossy());

    for invalid in [
        json!({"message":"hello"}),
        json!({"working_directory": canonical.to_string_lossy()}),
        json!({"working_directory":"relative/path","message":"hello"}),
        json!({"working_directory":"/definitely/not/an/authorized/workspace","message":"hello"}),
        json!({"working_directory":canonical.to_string_lossy(),"message":"hello","chat_id":"@override"}),
    ] {
        assert!(payload_from_arguments(&invalid, &config).is_err());
    }
}

#[test]
fn explicit_message_ledger_keeps_distinct_repeated_sends_and_recovers_inflight() {
    let directory =
        std::env::temp_dir().join(format!("ai-tools-telegram-messages-{}", Uuid::new_v4()));
    fs::create_dir_all(&directory).expect("temp directory");
    let path = directory.join("messages.sqlite3");
    let ledger = TelegramMessageLedger::open_at(&path).expect("ledger");
    let event = sanitize_telegram_message(payload()).expect("payload");

    let first = ledger.enqueue(&event).expect("first enqueue");
    let second = ledger.enqueue(&event).expect("second enqueue");
    assert_ne!(first, second);
    assert_eq!(ledger.pending(10).expect("pending").len(), 2);

    let claimed = ledger.claim_next(0).expect("claim").expect("row");
    let claimed_id = claimed.message_id.clone();
    drop(ledger);

    let reopened = TelegramMessageLedger::open_at(&path).expect("reopen");
    assert_eq!(reopened.pending(10).expect("recovered pending").len(), 2);
    let recovered = reopened
        .claim_next(0)
        .expect("claim recovered")
        .expect("recovered row");
    assert_eq!(recovered.message_id, claimed_id);
    reopened.mark_sent(&claimed_id).expect("acknowledge");
    assert!(reopened.claim_next(0).expect("remaining").is_some());
    fs::remove_dir_all(directory).expect("remove temp directory");
}

#[test]
fn legacy_completion_rows_are_not_replayed_by_explicit_message_ledger() {
    let directory =
        std::env::temp_dir().join(format!("ai-tools-telegram-legacy-{}", Uuid::new_v4()));
    fs::create_dir_all(&directory).expect("temp directory");
    let database = directory.join("messages.sqlite3");
    let connection = Connection::open(&database).expect("legacy database");
    connection.execute_batch(
        "CREATE TABLE task_notifications (task_id TEXT PRIMARY KEY, message TEXT NOT NULL, status TEXT NOT NULL, attempts INTEGER NOT NULL DEFAULT 0, next_attempt_ms INTEGER NOT NULL DEFAULT 0, last_error TEXT, created_at_ms INTEGER NOT NULL, updated_at_ms INTEGER NOT NULL, sent_at_ms INTEGER); INSERT INTO task_notifications (task_id, message, status, created_at_ms, updated_at_ms) VALUES ('legacy', 'old completion', 'pending', 1, 1);"
    ).expect("legacy schema");
    drop(connection);

    let ledger = TelegramMessageLedger::open_at(&database).expect("new ledger");
    assert!(ledger.pending(10).expect("new queue").is_empty());
    fs::remove_dir_all(directory).expect("remove directory");
}

#[test]
fn env_telegram_credentials_are_imported_encrypted_and_topic_survives_restart() {
    let directory =
        std::env::temp_dir().join(format!("ai-tools-telegram-import-{}", Uuid::new_v4()));
    fs::create_dir_all(&directory).expect("temp directory");
    let database = directory.join("messages.sqlite3");
    let env_file = directory.join("telegram.env");
    fs::write(
        &env_file,
        "TELEGRAM_BOT_TOKEN=123456:EnvSecret\nTELEGRAM_ALLOWED_USERS=123456789\nTELEGRAM_HOME_CHANNEL=-1001234567890\nTELEGRAM_HOME_CHANNEL_THREAD_ID=3775\n",
    )
    .expect("env file");

    let ledger = TelegramMessageLedger::open_at(&database).expect("ledger");
    ledger
        .import_env_credentials(&env_file)
        .expect("import credentials");
    let stored = ledger
        .load_telegram_credentials()
        .expect("load")
        .expect("stored");
    assert_eq!(stored.chat_id, "-1001234567890");
    assert_eq!(stored.message_thread_id, Some(3775));
    assert_eq!(stored.bot_token, "123456:EnvSecret");
    drop(ledger);

    let raw_database = fs::read(&database).expect("database bytes");
    assert!(!String::from_utf8_lossy(&raw_database).contains("EnvSecret"));
    let reopened = TelegramMessageLedger::open_at(&database).expect("reopen ledger");
    assert_eq!(
        reopened
            .load_telegram_credentials()
            .expect("load")
            .expect("stored")
            .message_thread_id,
        Some(3775)
    );
    fs::remove_dir_all(directory).expect("remove temp directory");
}

#[test]
fn legacy_telegram_configuration_schema_still_migrates_topic_column() {
    let directory =
        std::env::temp_dir().join(format!("ai-tools-telegram-migration-{}", Uuid::new_v4()));
    fs::create_dir_all(&directory).expect("temp directory");
    let database = directory.join("messages.sqlite3");
    let env_file = directory.join("telegram.env");
    let connection = Connection::open(&database).expect("legacy database");
    connection.execute_batch(
        "CREATE TABLE telegram_configuration (id INTEGER PRIMARY KEY CHECK (id = 1), bot_token_envelope BLOB NOT NULL, chat_id TEXT NOT NULL, updated_at_ms INTEGER NOT NULL);"
    ).expect("legacy schema");
    drop(connection);
    fs::write(
        &env_file,
        "TELEGRAM_BOT_TOKEN=123456:MigrationSecret\nTELEGRAM_HOME_CHANNEL=-1001234567890\nTELEGRAM_HOME_CHANNEL_THREAD_ID=3775\n",
    ).expect("env file");

    let ledger = TelegramMessageLedger::open_at(&database).expect("migrated ledger");
    ledger
        .import_env_credentials(&env_file)
        .expect("import after migration");
    assert_eq!(
        ledger
            .load_telegram_credentials()
            .expect("load")
            .expect("stored")
            .message_thread_id,
        Some(3775)
    );
    fs::remove_dir_all(directory).expect("remove directory");
}

#[test]
fn telegram_destination_contract_remains_fixed_and_bounded() {
    for valid in [None, Some(1), Some(3775), Some(i32::MAX as i64)] {
        assert!(validate_message_thread_id(valid).is_ok());
    }
    for invalid in [Some(-1), Some(0), Some(i32::MAX as i64 + 1)] {
        assert!(validate_message_thread_id(invalid).is_err());
    }
    assert!(validate_channel_target("123456789").is_err());

    let topic_payload = telegram_send_message_body("-1001234567890", Some(3775), "topic");
    assert_eq!(topic_payload["chat_id"], json!("-1001234567890"));
    assert_eq!(topic_payload["text"], json!("topic"));
    assert_eq!(topic_payload["message_thread_id"], json!(3775));
}

#[test]
fn explicit_telegram_tool_replaces_completion_contract() {
    let tools = tool_catalog_for_profile(relay_core::config::ToolProfile::Primary);
    let tool = tools
        .iter()
        .find(|tool| tool.name == "telegram_send_message")
        .expect("primary profile exposes Telegram messaging");
    let schema = tool.input_schema.to_string();
    assert!(schema.contains("working_directory"));
    assert!(schema.contains("message"));
    assert!(!schema.contains("chatId"));
    assert!(!schema.contains("botToken"));
    assert!(!tools.iter().any(|tool| tool.name == "task_completed"));

    assert_eq!(
        dispatch(&Request {
            jsonrpc: "2.0".into(),
            id: Id::String("1".into()),
            method: "server/task_completed".into(),
            params: None,
        }),
        Dispatch::Unknown("server/task_completed".into())
    );
    let extensions = DiscoverResult::current().capabilities["extensions"].to_string();
    assert!(!extensions.contains("server/task_completed"));
    assert!(!extensions.contains("task-completion-notifications"));
}

#[tokio::test]
async fn service_requires_bootstrapped_relay_state_when_enabled() {
    let directory =
        std::env::temp_dir().join(format!("ai-tools-telegram-runtime-{}", Uuid::new_v4()));
    fs::create_dir_all(&directory).expect("temp directory");
    let config = ServerConfig {
        telegram_enabled: true,
        activity: relay_core::config::ActivityConfig {
            state_dir: Some(directory.to_string_lossy().into_owned()),
            ..Default::default()
        },
        ..ServerConfig::default()
    };

    let service = TelegramMessageService::open(&config).expect("enabled service");
    assert_eq!(
        service.enqueue(payload()).await.expect("enqueue"),
        QueueResult::Disabled
    );
    fs::remove_dir_all(directory).expect("remove temp directory");
}

#[tokio::test]
async fn disabled_service_does_not_attempt_delivery() {
    let service = TelegramMessageService::open(&ServerConfig::default()).expect("disabled service");
    assert_eq!(
        service.enqueue(payload()).await.expect("enqueue"),
        QueueResult::Disabled
    );
}
