use super::{
    dotenv, format_telegram_message, now_ms, validate_bot_token, validate_message_thread_id,
    NotificationError, TelegramMessagePayload,
};
use crate::activity::{crypto, state_dir};
use relay_core::config::ServerConfig;
use rusqlite::{params, Connection, OptionalExtension};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;
use uuid::Uuid;

// Keep the legacy task_notifications table untouched for state-file backward
// compatibility. Plan 063 writes only to telegram_messages so pending rows from
// the superseded automatic completion pipeline can never be replayed as new
// explicit messages after upgrade.
const SCHEMA: &str = "CREATE TABLE IF NOT EXISTS telegram_messages (message_id TEXT PRIMARY KEY, message TEXT NOT NULL, status TEXT NOT NULL, attempts INTEGER NOT NULL DEFAULT 0, next_attempt_ms INTEGER NOT NULL DEFAULT 0, last_error TEXT, created_at_ms INTEGER NOT NULL, updated_at_ms INTEGER NOT NULL, sent_at_ms INTEGER); CREATE INDEX IF NOT EXISTS telegram_messages_delivery_idx ON telegram_messages (status, next_attempt_ms, created_at_ms); CREATE TABLE IF NOT EXISTS telegram_configuration (id INTEGER PRIMARY KEY CHECK (id = 1), bot_token_envelope BLOB NOT NULL, chat_id TEXT NOT NULL, message_thread_id INTEGER, updated_at_ms INTEGER NOT NULL);";
const MAX_ERROR_CATEGORY_BYTES: usize = 64;
const TELEGRAM_CONFIG_ID: i64 = 1;
const TELEGRAM_TOKEN_AAD: &[u8] = b"ai-tools:telegram-bot-token:v1";
// The existing filename is intentionally retained because it encrypts the
// already-provisioned Telegram credential envelope.
const TELEGRAM_KEY_FILE: &str = "telegram-task-notifications.key";

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct ClaimedTelegramMessage {
    pub message_id: String,
    pub message: String,
    pub attempts: u32,
}

#[derive(Clone, PartialEq, Eq)]
pub struct TelegramCredentials {
    pub bot_token: String,
    pub chat_id: String,
    pub message_thread_id: Option<i64>,
}

pub struct TelegramMessageLedger {
    connection: Mutex<Connection>,
    path: PathBuf,
    key: [u8; crypto::KEY_BYTES],
}

impl TelegramMessageLedger {
    pub fn open(config: &ServerConfig) -> Result<Self, NotificationError> {
        let directory = state_dir(config).map_err(|_| NotificationError::Io)?;
        std::fs::create_dir_all(&directory).map_err(|_| NotificationError::Io)?;
        set_owner_only(&directory, true)?;
        // Keep the established state database path so credential provisioning
        // survives the contract migration without an operator re-import.
        let path = directory.join("telegram-task-notifications.sqlite3");
        let key_path = directory.join(TELEGRAM_KEY_FILE);
        Self::open_at_with_key(&path, &key_path)
    }

    pub fn open_at(path: &Path) -> Result<Self, NotificationError> {
        let key_path = path.with_file_name(format!(
            "{}.key",
            path.file_name()
                .and_then(|name| name.to_str())
                .ok_or(NotificationError::Io)?
        ));
        Self::open_at_with_key(path, &key_path)
    }

    fn open_at_with_key(path: &Path, key_path: &Path) -> Result<Self, NotificationError> {
        if path.exists()
            && std::fs::symlink_metadata(path)
                .map_err(|_| NotificationError::Io)?
                .file_type()
                .is_symlink()
        {
            return Err(NotificationError::Io);
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|_| NotificationError::Io)?;
            set_owner_only(parent, true)?;
        }
        let key = load_or_create_key(key_path)?;
        let connection = Connection::open(path).map_err(|_| NotificationError::Database)?;
        set_owner_only(path, false)?;
        connection
            .execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;")
            .map_err(|_| NotificationError::Database)?;
        connection
            .execute_batch(SCHEMA)
            .map_err(|_| NotificationError::Database)?;
        migrate_schema(&connection)?;
        let ledger = Self {
            connection: Mutex::new(connection),
            path: path.to_owned(),
            key,
        };
        ledger.recover_inflight()?;
        Ok(ledger)
    }

    pub fn import_env_credentials(&self, path: &Path) -> Result<(), NotificationError> {
        let credentials = dotenv::load_credentials(path)?;
        self.store_telegram_credentials(TelegramCredentials {
            bot_token: credentials.bot_token,
            chat_id: credentials.channel,
            message_thread_id: credentials.message_thread_id,
        })
    }

    pub fn load_telegram_credentials(
        &self,
    ) -> Result<Option<TelegramCredentials>, NotificationError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| NotificationError::Database)?;
        let row = connection
            .query_row(
                "SELECT bot_token_envelope, chat_id, message_thread_id FROM telegram_configuration WHERE id = ?1",
                params![TELEGRAM_CONFIG_ID],
                |row| {
                    Ok((
                        row.get::<_, Vec<u8>>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<i64>>(2)?,
                    ))
                },
            )
            .optional()
            .map_err(|_| NotificationError::Database)?;
        let Some((envelope, chat_id, message_thread_id)) = row else {
            return Ok(None);
        };
        let plaintext = crypto::open(&self.key, &envelope, TELEGRAM_TOKEN_AAD)
            .map_err(|_| NotificationError::Invalid)?;
        let bot_token = String::from_utf8(plaintext).map_err(|_| NotificationError::Invalid)?;
        validate_bot_token(&bot_token)?;
        super::validate_channel_target(&chat_id)?;
        validate_message_thread_id(message_thread_id)?;
        Ok(Some(TelegramCredentials {
            bot_token,
            chat_id,
            message_thread_id,
        }))
    }

    fn store_telegram_credentials(
        &self,
        credentials: TelegramCredentials,
    ) -> Result<(), NotificationError> {
        validate_bot_token(&credentials.bot_token)?;
        super::validate_channel_target(&credentials.chat_id)?;
        validate_message_thread_id(credentials.message_thread_id)?;
        let envelope = crypto::seal(
            &self.key,
            credentials.bot_token.as_bytes(),
            TELEGRAM_TOKEN_AAD,
        )
        .map_err(|_| NotificationError::Invalid)?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| NotificationError::Database)?;
        connection
            .execute(
                "INSERT INTO telegram_configuration (id, bot_token_envelope, chat_id, message_thread_id, updated_at_ms) VALUES (?1, ?2, ?3, ?4, ?5) ON CONFLICT(id) DO UPDATE SET bot_token_envelope = excluded.bot_token_envelope, chat_id = excluded.chat_id, message_thread_id = excluded.message_thread_id, updated_at_ms = excluded.updated_at_ms",
                params![
                    TELEGRAM_CONFIG_ID,
                    envelope,
                    credentials.chat_id,
                    credentials.message_thread_id,
                    now_ms()
                ],
            )
            .map_err(|_| NotificationError::Database)?;
        Ok(())
    }

    pub fn enqueue(&self, payload: &TelegramMessagePayload) -> Result<String, NotificationError> {
        let message = format_telegram_message(payload);
        if message.is_empty() || message.len() > super::MAX_MESSAGE_BYTES {
            return Err(NotificationError::Invalid);
        }
        let message_id = Uuid::new_v4().to_string();
        let connection = self
            .connection
            .lock()
            .map_err(|_| NotificationError::Database)?;
        let now = now_ms();
        connection
            .execute(
                "INSERT INTO telegram_messages (message_id, message, status, created_at_ms, updated_at_ms) VALUES (?1, ?2, 'pending', ?3, ?3)",
                params![message_id, message, now],
            )
            .map_err(|_| NotificationError::Database)?;
        Ok(message_id)
    }

    pub fn pending(&self, limit: usize) -> Result<Vec<ClaimedTelegramMessage>, NotificationError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| NotificationError::Database)?;
        let mut statement = connection
            .prepare("SELECT message_id, message, attempts FROM telegram_messages WHERE status = 'pending' ORDER BY created_at_ms, message_id LIMIT ?1")
            .map_err(|_| NotificationError::Database)?;
        let rows = statement
            .query_map(params![limit.min(64) as i64], |row| {
                Ok(ClaimedTelegramMessage {
                    message_id: row.get(0)?,
                    message: row.get(1)?,
                    attempts: row.get::<_, i64>(2)?.max(0) as u32,
                })
            })
            .map_err(|_| NotificationError::Database)?;
        rows.map(|row| row.map_err(|_| NotificationError::Database))
            .collect()
    }

    pub fn claim_next(
        &self,
        now: i64,
    ) -> Result<Option<ClaimedTelegramMessage>, NotificationError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| NotificationError::Database)?;
        let transaction = connection
            .unchecked_transaction()
            .map_err(|_| NotificationError::Database)?;
        let candidate = transaction
            .query_row(
                "SELECT message_id, message, attempts FROM telegram_messages WHERE status = 'pending' AND next_attempt_ms <= ?1 ORDER BY created_at_ms, message_id LIMIT 1",
                params![now],
                |row| {
                    Ok(ClaimedTelegramMessage {
                        message_id: row.get(0)?,
                        message: row.get(1)?,
                        attempts: row.get::<_, i64>(2)?.max(0) as u32,
                    })
                },
            )
            .optional()
            .map_err(|_| NotificationError::Database)?;
        let Some(candidate) = candidate else {
            transaction
                .commit()
                .map_err(|_| NotificationError::Database)?;
            return Ok(None);
        };
        transaction
            .execute(
                "UPDATE telegram_messages SET status = 'sending', updated_at_ms = ?1 WHERE message_id = ?2 AND status = 'pending'",
                params![now, candidate.message_id],
            )
            .map_err(|_| NotificationError::Database)?;
        transaction
            .commit()
            .map_err(|_| NotificationError::Database)?;
        Ok(Some(candidate))
    }

    pub fn mark_sent(&self, message_id: &str) -> Result<(), NotificationError> {
        self.update_status(message_id, "sent", None, true)
    }

    pub fn mark_retry(
        &self,
        message_id: &str,
        category: &str,
        delay: Duration,
    ) -> Result<(), NotificationError> {
        let category = bounded_error_category(category);
        let connection = self
            .connection
            .lock()
            .map_err(|_| NotificationError::Database)?;
        let now = now_ms();
        connection
            .execute(
                "UPDATE telegram_messages SET status = 'pending', attempts = attempts + 1, next_attempt_ms = ?1, last_error = ?2, updated_at_ms = ?3 WHERE message_id = ?4 AND status = 'sending'",
                params![now.saturating_add(delay.as_millis().min(i64::MAX as u128) as i64), category, now, message_id],
            )
            .map_err(|_| NotificationError::Database)?;
        Ok(())
    }

    pub fn mark_failed(&self, message_id: &str, category: &str) -> Result<(), NotificationError> {
        self.update_status(
            message_id,
            "failed",
            Some(&bounded_error_category(category)),
            false,
        )
    }

    fn update_status(
        &self,
        message_id: &str,
        status: &str,
        error: Option<&str>,
        sent: bool,
    ) -> Result<(), NotificationError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| NotificationError::Database)?;
        let now = now_ms();
        if sent {
            connection
                .execute(
                    "UPDATE telegram_messages SET status = 'sent', last_error = NULL, sent_at_ms = ?1, updated_at_ms = ?1 WHERE message_id = ?2 AND status = 'sending'",
                    params![now, message_id],
                )
                .map_err(|_| NotificationError::Database)?;
        } else {
            connection
                .execute(
                    "UPDATE telegram_messages SET status = ?1, last_error = ?2, updated_at_ms = ?3 WHERE message_id = ?4 AND status = 'sending'",
                    params![status, error, now, message_id],
                )
                .map_err(|_| NotificationError::Database)?;
        }
        Ok(())
    }

    fn recover_inflight(&self) -> Result<(), NotificationError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| NotificationError::Database)?;
        connection
            .execute(
                "UPDATE telegram_messages SET status = 'pending', next_attempt_ms = 0, updated_at_ms = ?1 WHERE status = 'sending'",
                params![now_ms()],
            )
            .map_err(|_| NotificationError::Database)?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn database_path(&self) -> &Path {
        &self.path
    }
}

fn migrate_schema(connection: &Connection) -> Result<(), NotificationError> {
    let mut statement = connection
        .prepare("PRAGMA table_info(telegram_configuration)")
        .map_err(|_| NotificationError::Database)?;
    let column_names = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|_| NotificationError::Database)?;
    let mut has_thread_column = false;
    for column_name in column_names {
        if column_name.map_err(|_| NotificationError::Database)? == "message_thread_id" {
            has_thread_column = true;
            break;
        }
    }
    drop(statement);
    if !has_thread_column {
        connection
            .execute(
                "ALTER TABLE telegram_configuration ADD COLUMN message_thread_id INTEGER",
                [],
            )
            .map_err(|_| NotificationError::Database)?;
    }
    Ok(())
}

fn load_or_create_key(path: &Path) -> Result<[u8; crypto::KEY_BYTES], NotificationError> {
    if path.exists() {
        ensure_private_file(path)?;
        let bytes = fs::read(path).map_err(|_| NotificationError::Io)?;
        return bytes.try_into().map_err(|_| NotificationError::Invalid);
    }
    let key = crypto::random_key().map_err(|_| NotificationError::Invalid)?;
    write_private(path, &key)?;
    Ok(key)
}

fn ensure_private_file(path: &Path) -> Result<(), NotificationError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| NotificationError::Io)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(NotificationError::Io);
    }
    set_owner_only(path, false)
}

fn write_private(path: &Path, content: &[u8]) -> Result<(), NotificationError> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    let mut file = options.open(path).map_err(|_| NotificationError::Io)?;
    file.write_all(content).map_err(|_| NotificationError::Io)?;
    file.sync_all().map_err(|_| NotificationError::Io)?;
    set_owner_only(path, false)
}

fn set_owner_only(path: &Path, directory: bool) -> Result<(), NotificationError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = if directory { 0o700 } else { 0o600 };
        fs::set_permissions(path, fs::Permissions::from_mode(mode))
            .map_err(|_| NotificationError::Io)?;
    }
    Ok(())
}

fn bounded_error_category(category: &str) -> String {
    category
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '_' || *ch == '-')
        .take(MAX_ERROR_CATEGORY_BYTES)
        .collect()
}
