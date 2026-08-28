use super::{format_task_completion_message, now_ms, NotificationError, TaskCompletionPayload};
use crate::activity::state_dir;
use relay_core::config::ServerConfig;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

const SCHEMA: &str = "CREATE TABLE IF NOT EXISTS task_notifications (task_id TEXT PRIMARY KEY, message TEXT NOT NULL, status TEXT NOT NULL, attempts INTEGER NOT NULL DEFAULT 0, next_attempt_ms INTEGER NOT NULL DEFAULT 0, last_error TEXT, created_at_ms INTEGER NOT NULL, updated_at_ms INTEGER NOT NULL, sent_at_ms INTEGER); CREATE INDEX IF NOT EXISTS task_notifications_delivery_idx ON task_notifications (status, next_attempt_ms, created_at_ms);";
const MAX_ERROR_CATEGORY_BYTES: usize = 64;

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct ClaimedNotification {
    pub task_id: String,
    pub message: String,
    pub attempts: u32,
}

pub struct NotificationLedger {
    connection: Mutex<Connection>,
    path: PathBuf,
}

impl NotificationLedger {
    pub fn open(config: &ServerConfig) -> Result<Self, NotificationError> {
        let directory = state_dir(config).map_err(|_| NotificationError::Io)?;
        std::fs::create_dir_all(&directory).map_err(|_| NotificationError::Io)?;
        set_owner_only(&directory, true)?;
        let path = directory.join("telegram-task-notifications.sqlite3");
        Self::open_at(&path)
    }

    pub fn open_at(path: &Path) -> Result<Self, NotificationError> {
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
        let connection = Connection::open(path).map_err(|_| NotificationError::Database)?;
        set_owner_only(path, false)?;
        connection
            .execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;")
            .map_err(|_| NotificationError::Database)?;
        connection
            .execute_batch(SCHEMA)
            .map_err(|_| NotificationError::Database)?;
        let ledger = Self {
            connection: Mutex::new(connection),
            path: path.to_owned(),
        };
        ledger.recover_inflight()?;
        Ok(ledger)
    }

    pub fn enqueue(&self, payload: &TaskCompletionPayload) -> Result<bool, NotificationError> {
        let message = format_task_completion_message(payload);
        if message.is_empty() || message.len() > super::MAX_MESSAGE_BYTES {
            return Err(NotificationError::Invalid);
        }
        let connection = self
            .connection
            .lock()
            .map_err(|_| NotificationError::Database)?;
        let now = now_ms();
        let inserted = connection
            .execute(
                "INSERT INTO task_notifications (task_id, message, status, created_at_ms, updated_at_ms) VALUES (?1, ?2, 'pending', ?3, ?3) ON CONFLICT(task_id) DO NOTHING",
                params![payload.task_id, message, now],
            )
            .map_err(|_| NotificationError::Database)?;
        Ok(inserted == 1)
    }

    pub fn pending(&self, limit: usize) -> Result<Vec<ClaimedNotification>, NotificationError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| NotificationError::Database)?;
        let mut statement = connection
            .prepare("SELECT task_id, message, attempts FROM task_notifications WHERE status = 'pending' ORDER BY created_at_ms, task_id LIMIT ?1")
            .map_err(|_| NotificationError::Database)?;
        let rows = statement
            .query_map(params![limit.min(64) as i64], |row| {
                Ok(ClaimedNotification {
                    task_id: row.get(0)?,
                    message: row.get(1)?,
                    attempts: row.get::<_, i64>(2)?.max(0) as u32,
                })
            })
            .map_err(|_| NotificationError::Database)?;
        rows.map(|row| row.map_err(|_| NotificationError::Database))
            .collect()
    }

    pub fn claim_next(&self, now: i64) -> Result<Option<ClaimedNotification>, NotificationError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| NotificationError::Database)?;
        let transaction = connection
            .unchecked_transaction()
            .map_err(|_| NotificationError::Database)?;
        let candidate = transaction
            .query_row(
                "SELECT task_id, message, attempts FROM task_notifications WHERE status = 'pending' AND next_attempt_ms <= ?1 ORDER BY created_at_ms, task_id LIMIT 1",
                params![now],
                |row| {
                    Ok(ClaimedNotification {
                        task_id: row.get(0)?,
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
                "UPDATE task_notifications SET status = 'sending', updated_at_ms = ?1 WHERE task_id = ?2 AND status = 'pending'",
                params![now, candidate.task_id],
            )
            .map_err(|_| NotificationError::Database)?;
        transaction
            .commit()
            .map_err(|_| NotificationError::Database)?;
        Ok(Some(candidate))
    }

    pub fn mark_sent(&self, task_id: &str) -> Result<(), NotificationError> {
        self.update_status(task_id, "sent", None, true)
    }

    pub fn mark_retry(
        &self,
        task_id: &str,
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
                "UPDATE task_notifications SET status = 'pending', attempts = attempts + 1, next_attempt_ms = ?1, last_error = ?2, updated_at_ms = ?3 WHERE task_id = ?4 AND status = 'sending'",
                params![now.saturating_add(delay.as_millis().min(i64::MAX as u128) as i64), category, now, task_id],
            )
            .map_err(|_| NotificationError::Database)?;
        Ok(())
    }

    pub fn mark_failed(&self, task_id: &str, category: &str) -> Result<(), NotificationError> {
        self.update_status(
            task_id,
            "failed",
            Some(&bounded_error_category(category)),
            false,
        )
    }

    fn update_status(
        &self,
        task_id: &str,
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
                    "UPDATE task_notifications SET status = 'sent', last_error = NULL, sent_at_ms = ?1, updated_at_ms = ?1 WHERE task_id = ?2 AND status = 'sending'",
                    params![now, task_id],
                )
                .map_err(|_| NotificationError::Database)?;
        } else {
            connection
                .execute(
                    "UPDATE task_notifications SET status = ?1, last_error = ?2, updated_at_ms = ?3 WHERE task_id = ?4 AND status = 'sending'",
                    params![status, error, now, task_id],
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
                "UPDATE task_notifications SET status = 'pending', next_attempt_ms = 0, updated_at_ms = ?1 WHERE status = 'sending'",
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

fn bounded_error_category(category: &str) -> String {
    category
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '_' || *ch == '-')
        .take(MAX_ERROR_CATEGORY_BYTES)
        .collect()
}

fn set_owner_only(path: &Path, directory: bool) -> Result<(), NotificationError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = if directory { 0o700 } else { 0o600 };
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
            .map_err(|_| NotificationError::Io)?;
    }
    Ok(())
}
