use relay_application::dispatcher::{dispatch, Dispatch};
use relay_core::config::ServerConfig;
use relay_infrastructure::notifications::{
    format_task_completion_message, sanitize_task_completion, telegram_send_message_body,
    validate_channel_target, validate_message_thread_id, NotificationLedger, TaskCompletionPayload,
};
use relay_interfaces::mcp::{tool_catalog_for_profile, DiscoverResult};
use rusqlite::Connection;
use serde_json::json;
use std::fs;
use uuid::Uuid;

fn payload() -> TaskCompletionPayload {
    TaskCompletionPayload {
        task_id: "og_123".into(),
        title: "Ship Telegram completion notice".into(),
        summary: "Implemented the feature and ran the focused tests.".into(),
        completed_at: "2026-08-28T16:00:00.000Z".into(),
        result_url: Some("https://ai-code.example/tasks/og_123".into()),
        source: "external-mcp".into(),
    }
}

#[test]
fn completion_payload_is_bounded_and_redacted() {
    let sanitized = sanitize_task_completion(payload()).expect("payload should be valid");
    assert_eq!(sanitized.title, "Ship Telegram completion notice");
    assert_eq!(
        format_task_completion_message(&sanitized),
        "✅ Ship Telegram completion notice\nImplemented the feature and ran the focused tests.\nResult: https://ai-code.example/tasks/og_123"
    );

    let mut unsafe_payload = payload();
    unsafe_payload.summary = "Authorization: Bearer super-secret password=hidden".into();
    let sanitized = sanitize_task_completion(unsafe_payload).expect("payload should sanitize");
    assert!(sanitized.summary.contains("Bearer [REDACTED]"));
    assert!(sanitized.summary.contains("password=[REDACTED]"));
    assert!(!sanitized.summary.contains("super-secret"));
    assert!(!sanitized.summary.contains("hidden"));
}

#[test]
fn completion_payload_rejects_invalid_fields() {
    let mut missing_id = payload();
    missing_id.task_id.clear();
    assert!(sanitize_task_completion(missing_id).is_err());

    let mut invalid_url = payload();
    invalid_url.result_url = Some("http://example.test/task".into());
    assert!(sanitize_task_completion(invalid_url).is_err());
}

#[test]
fn ledger_deduplicates_and_recovers_inflight_rows() {
    let directory = std::env::temp_dir().join(format!("ai-tools-notifications-{}", Uuid::new_v4()));
    fs::create_dir_all(&directory).expect("temp directory");
    let path = directory.join("notifications.sqlite3");
    let ledger = NotificationLedger::open_at(&path).expect("ledger");
    let event = sanitize_task_completion(payload()).expect("payload");

    assert!(ledger.enqueue(&event).expect("enqueue"));
    assert!(!ledger.enqueue(&event).expect("duplicate enqueue"));
    assert_eq!(ledger.pending(10).expect("pending").len(), 1);

    let claimed = ledger.claim_next(0).expect("claim").expect("row");
    assert_eq!(claimed.task_id, "og_123");
    drop(ledger);

    let reopened = NotificationLedger::open_at(&path).expect("reopen");
    assert_eq!(reopened.pending(10).expect("recovered pending").len(), 1);
    let recovered = reopened
        .claim_next(0)
        .expect("claim recovered")
        .expect("recovered row");
    assert_eq!(recovered.task_id, "og_123");
    reopened.mark_sent("og_123").expect("acknowledge");
    assert!(reopened.claim_next(0).expect("empty pending").is_none());
    assert!(!reopened.enqueue(&event).expect("sent duplicate"));
    drop(reopened);
    fs::remove_dir_all(directory).expect("remove temp directory");
}

#[test]
fn env_telegram_credentials_are_imported_encrypted_and_survive_restart() {
    let directory =
        std::env::temp_dir().join(format!("ai-tools-telegram-import-{}", Uuid::new_v4()));
    fs::create_dir_all(&directory).expect("temp directory");
    let database = directory.join("notifications.sqlite3");
    let env_file = directory.join("telegram.env");
    fs::write(
        &env_file,
        "TELEGRAM_BOT_TOKEN=123456:EnvSecret\nTELEGRAM_ALLOWED_USERS=123456789\nTELEGRAM_HOME_CHANNEL=-1001234567890\n",
    )
    .expect("env file");

    let ledger = NotificationLedger::open_at(&database).expect("ledger");
    ledger
        .import_env_credentials(&env_file)
        .expect("import credentials");
    let stored = ledger
        .load_telegram_credentials()
        .expect("load credentials")
        .expect("stored credentials");
    assert_eq!(stored.chat_id, "-1001234567890");
    assert_eq!(stored.bot_token, "123456:EnvSecret");
    assert_eq!(stored.message_thread_id, None);
    drop(ledger);

    let raw_database = fs::read(&database).expect("database bytes");
    assert!(!String::from_utf8_lossy(&raw_database).contains("EnvSecret"));
    let reopened = NotificationLedger::open_at(&database).expect("reopen ledger");
    assert_eq!(
        reopened
            .load_telegram_credentials()
            .expect("load after restart")
            .expect("credentials after restart")
            .bot_token,
        "123456:EnvSecret"
    );
    fs::remove_dir_all(directory).expect("remove temp directory");
}

#[test]
fn stored_credentials_are_not_refreshed_or_replaced_at_runtime() {
    let directory = std::env::temp_dir().join(format!("ai-tools-telegram-sync-{}", Uuid::new_v4()));
    fs::create_dir_all(&directory).expect("temp directory");
    let database = directory.join("notifications.sqlite3");
    let env_file = directory.join("telegram.env");
    fs::write(
        &env_file,
        "TELEGRAM_BOT_TOKEN=123456:FirstSecret\nTELEGRAM_HOME_CHANNEL=-1001234567890\n",
    )
    .expect("first env file");

    let ledger = NotificationLedger::open_at(&database).expect("ledger");
    ledger
        .import_env_credentials(&env_file)
        .expect("first import");
    assert_eq!(
        ledger
            .load_telegram_credentials()
            .expect("first load")
            .expect("first credentials")
            .chat_id,
        "-1001234567890"
    );

    fs::write(
        &env_file,
        "TELEGRAM_BOT_TOKEN=123456:SecondSecret\nTELEGRAM_HOME_CHANNEL=@release_updates\n",
    )
    .expect("second env file");
    let stored = ledger
        .load_telegram_credentials()
        .expect("load without runtime import")
        .expect("stored credentials");
    assert_eq!(stored.bot_token, "123456:FirstSecret");
    assert_eq!(stored.chat_id, "-1001234567890");
    assert_eq!(stored.message_thread_id, None);

    fs::write(
        &env_file,
        "TELEGRAM_BOT_TOKEN=123456:InvalidSecret\nTELEGRAM_HOME_CHANNEL=123456789\n",
    )
    .expect("invalid env file");
    assert!(ledger.import_env_credentials(&env_file).is_err());
    fs::remove_dir_all(directory).expect("remove temp directory");
}

#[test]
fn telegram_forum_topic_id_is_imported_and_survives_restart() {
    let directory =
        std::env::temp_dir().join(format!("ai-tools-telegram-topic-{}", Uuid::new_v4()));
    fs::create_dir_all(&directory).expect("temp directory");
    let database = directory.join("notifications.sqlite3");
    let env_file = directory.join("telegram.env");
    fs::write(
        &env_file,
        "TELEGRAM_BOT_TOKEN=123456:TopicSecret\nTELEGRAM_HOME_CHANNEL=-1001234567890\nTELEGRAM_HOME_CHANNEL_THREAD_ID=3775\n",
    )
    .expect("env file");

    let ledger = NotificationLedger::open_at(&database).expect("ledger");
    ledger
        .import_env_credentials(&env_file)
        .expect("import credentials");
    assert_eq!(
        ledger
            .load_telegram_credentials()
            .expect("load credentials")
            .expect("stored credentials")
            .message_thread_id,
        Some(3775)
    );
    drop(ledger);

    let reopened = NotificationLedger::open_at(&database).expect("reopen ledger");
    let stored = reopened
        .load_telegram_credentials()
        .expect("load after restart")
        .expect("credentials after restart");
    assert_eq!(stored.chat_id, "-1001234567890");
    assert_eq!(stored.message_thread_id, Some(3775));
    let raw_database = fs::read(&database).expect("database bytes");
    assert!(!String::from_utf8_lossy(&raw_database).contains("TopicSecret"));
    fs::remove_dir_all(directory).expect("remove temp directory");
}

#[test]
fn legacy_telegram_schema_migrates_with_topic_column() {
    let directory =
        std::env::temp_dir().join(format!("ai-tools-telegram-migration-{}", Uuid::new_v4()));
    fs::create_dir_all(&directory).expect("temp directory");
    let database = directory.join("notifications.sqlite3");
    let env_file = directory.join("telegram.env");
    let connection = Connection::open(&database).expect("legacy database");
    connection
        .execute_batch(
            "CREATE TABLE telegram_configuration (id INTEGER PRIMARY KEY CHECK (id = 1), bot_token_envelope BLOB NOT NULL, chat_id TEXT NOT NULL, updated_at_ms INTEGER NOT NULL);",
        )
        .expect("legacy schema");
    drop(connection);
    fs::write(
        &env_file,
        "TELEGRAM_BOT_TOKEN=123456:MigrationSecret\nTELEGRAM_HOME_CHANNEL=-1001234567890\nTELEGRAM_HOME_CHANNEL_THREAD_ID=3775\n",
    )
    .expect("env file");

    let ledger = NotificationLedger::open_at(&database).expect("migrated ledger");
    ledger
        .import_env_credentials(&env_file)
        .expect("import after migration");
    let stored = ledger
        .load_telegram_credentials()
        .expect("load migrated credentials")
        .expect("stored migrated credentials");
    assert_eq!(stored.message_thread_id, Some(3775));
    fs::remove_dir_all(directory).expect("remove directory");
}

#[test]
fn telegram_forum_topic_id_validation_is_bounded() {
    for valid in [None, Some(1), Some(3775), Some(i32::MAX as i64)] {
        assert!(validate_message_thread_id(valid).is_ok());
    }
    for invalid in [Some(-1), Some(0), Some(i32::MAX as i64 + 1)] {
        assert!(validate_message_thread_id(invalid).is_err());
    }
}

#[test]
fn telegram_send_payload_targets_only_configured_topic() {
    let root_payload = telegram_send_message_body("-1001234567890", None, "root");
    assert_eq!(root_payload["chat_id"], json!("-1001234567890"));
    assert_eq!(root_payload["text"], json!("root"));
    assert!(root_payload.get("message_thread_id").is_none());

    let topic_payload = telegram_send_message_body("-1001234567890", Some(3775), "topic");
    assert_eq!(topic_payload["chat_id"], json!("-1001234567890"));
    assert_eq!(topic_payload["text"], json!("topic"));
    assert_eq!(topic_payload["message_thread_id"], json!(3775));
}

#[test]
fn env_user_chat_is_rejected_as_a_notification_target() {
    let directory = std::env::temp_dir().join(format!("ai-tools-telegram-dm-{}", Uuid::new_v4()));
    fs::create_dir_all(&directory).expect("temp directory");
    let database = directory.join("notifications.sqlite3");
    let env_file = directory.join("telegram.env");
    fs::write(
        &env_file,
        "TELEGRAM_BOT_TOKEN=123456:EnvSecret\nTELEGRAM_ALLOWED_USERS=123456789\nTELEGRAM_HOME_CHANNEL=123456789\n",
    )
    .expect("env file");

    assert!(validate_channel_target("123456789").is_err());
    let ledger = NotificationLedger::open_at(&database).expect("ledger");
    assert!(ledger.import_env_credentials(&env_file).is_err());
    assert!(ledger
        .load_telegram_credentials()
        .expect("load missing credentials")
        .is_none());
    fs::remove_dir_all(directory).expect("remove temp directory");
}

#[test]
fn telegram_config_keeps_credentials_out_of_serialized_server_config() {
    let config = ServerConfig {
        dir: Some(env!("CARGO_MANIFEST_DIR").to_string()),
        execution_root: Some(env!("CARGO_MANIFEST_DIR").to_string()),
        telegram_enabled: true,
        ..ServerConfig::default()
    };
    assert!(config.validate().is_ok());
    let serialized = serde_json::to_string(&config).expect("config serialization");
    assert!(!serialized.contains("telegram_enabled"));
}

#[test]
fn completion_signal_is_bounded_and_private_method_is_not_a_telegram_tool() {
    let tool = tool_catalog_for_profile(relay_core::config::ToolProfile::Primary)
        .into_iter()
        .find(|tool| tool.name == "task_completed")
        .expect("primary profile exposes the completion signal");
    let schema = tool.input_schema.to_string();
    assert!(schema.contains("taskId"));
    assert!(schema.contains("summary"));
    assert!(!schema.contains("chatId"));
    assert!(!schema.contains("botToken"));
    assert_eq!(
        dispatch(&relay_interfaces::mcp::Request {
            jsonrpc: "2.0".into(),
            id: relay_interfaces::mcp::Id::String("1".into()),
            method: "server/task_completed".into(),
            params: None,
        }),
        Dispatch::TaskCompleted
    );
    let extensions = DiscoverResult::current().capabilities["extensions"].to_string();
    assert!(extensions.contains("server/task_completed"));
}

#[tokio::test]
async fn enabled_service_uses_only_bootstrapped_relay_state() {
    let directory =
        std::env::temp_dir().join(format!("ai-tools-telegram-runtime-{}", Uuid::new_v4()));
    fs::create_dir_all(&directory).expect("temp directory");
    let config = ServerConfig {
        telegram_enabled: true,
        activity: relay_core::config::ActivityConfig {
            state_dir: Some(directory.to_string_lossy().into_owned()),
            ..Default::default()
        },
        ..Default::default()
    };

    let service = relay_infrastructure::notifications::TaskNotificationService::open(&config)
        .expect("enabled service");
    assert_eq!(
        service
            .enqueue(sanitize_task_completion(payload()).expect("payload"))
            .await
            .expect("enqueue"),
        relay_infrastructure::notifications::QueueResult::Disabled
    );
    drop(service);
    fs::remove_dir_all(directory).expect("remove temp directory");
}

#[tokio::test]
async fn disabled_service_does_not_attempt_delivery() {
    let service = relay_infrastructure::notifications::TaskNotificationService::open(
        &ServerConfig::default(),
    )
    .expect("disabled service");
    assert_eq!(
        service
            .enqueue(sanitize_task_completion(payload()).expect("payload"))
            .await
            .expect("enqueue"),
        relay_infrastructure::notifications::QueueResult::Disabled
    );
}
