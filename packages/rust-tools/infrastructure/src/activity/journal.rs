use super::crypto;
use relay_application::activity::{transition_allowed, ActivityEvent, Status};
use relay_core::config::ServerConfig;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

const SCHEMA: &str = "CREATE TABLE IF NOT EXISTS activity_journal (record_id TEXT PRIMARY KEY, activity_id TEXT NOT NULL, source_sequence INTEGER NOT NULL, status TEXT NOT NULL, envelope BLOB NOT NULL, checksum TEXT NOT NULL, acknowledged INTEGER NOT NULL DEFAULT 0, attempts INTEGER NOT NULL DEFAULT 0, next_attempt_ms INTEGER NOT NULL DEFAULT 0, created_at_ms INTEGER NOT NULL, updated_at_ms INTEGER NOT NULL); CREATE INDEX IF NOT EXISTS activity_journal_delivery_idx ON activity_journal (acknowledged, source_sequence, record_id);";
const MAX_PAYLOAD_BYTES: usize = 512 * 1024;

#[derive(Debug)]
pub enum JournalError {
    Full,
    Invalid,
    Io,
    Database,
    Crypto,
}

pub struct Journal {
    connection: Mutex<Connection>,
    path: PathBuf,
    key: [u8; crypto::KEY_BYTES],
    source_id: String,
    quota: u64,
    ack_retention_ms: u64,
}

#[derive(Debug, Clone)]
pub struct PendingRecord {
    pub record_id: String,
    pub event: ActivityEvent,
    pub payload: Option<Vec<u8>>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct JournalSnapshot {
    pub source_id: String,
    pub pending_records: u64,
    pub acknowledged_records: u64,
    pub degraded: bool,
}

impl Journal {
    pub fn open(config: &ServerConfig) -> Result<Self, JournalError> {
        let state_dir = state_dir(config)?;
        fs::create_dir_all(&state_dir).map_err(|_| JournalError::Io)?;
        set_owner_only(&state_dir, true)?;
        let source_path = state_dir.join("source-id");
        let key_path = state_dir.join("payload.key");
        let source_id = load_or_create_source(&source_path)?;
        let key = load_or_create_key(&key_path)?;
        let path = state_dir.join("activity.sqlite3");
        if path.exists()
            && fs::symlink_metadata(&path)
                .map_err(|_| JournalError::Io)?
                .file_type()
                .is_symlink()
        {
            return Err(JournalError::Io);
        }
        let connection = Connection::open(&path).map_err(|_| JournalError::Database)?;
        set_owner_only(&path, false)?;
        connection
            .execute_batch(
                "PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; PRAGMA foreign_keys=ON;",
            )
            .map_err(|_| JournalError::Database)?;
        connection
            .execute_batch(SCHEMA)
            .map_err(|_| JournalError::Database)?;
        let journal = Self {
            connection: Mutex::new(connection),
            path,
            key,
            source_id,
            quota: config.activity.spool_quota_bytes,
            ack_retention_ms: config.activity.acknowledged_retention_ms,
        };
        journal.recover_stale()?;
        Ok(journal)
    }

    pub fn source_id(&self) -> &str {
        &self.source_id
    }

    pub fn append(
        &self,
        mut event: ActivityEvent,
        payload: Option<Vec<u8>>,
    ) -> Result<ActivityEvent, JournalError> {
        let connection = self.connection.lock().map_err(|_| JournalError::Database)?;
        if payload
            .as_ref()
            .is_some_and(|payload| payload.len() > MAX_PAYLOAD_BYTES)
        {
            return Err(JournalError::Invalid);
        }
        self.ensure_capacity(&connection, payload.as_ref().map_or(0, Vec::len))?;
        if event.source_id.is_empty() {
            event.source_id = self.source_id.clone();
        }
        if event.source_id != self.source_id {
            return Err(JournalError::Invalid);
        }
        let next_sequence: u64 = connection
            .query_row(
                "SELECT COALESCE(MAX(source_sequence), 0) + 1 FROM activity_journal",
                [],
                |row| row.get(0),
            )
            .map_err(|_| JournalError::Database)?;
        event.source_sequence = next_sequence;
        event.validate().map_err(|_| JournalError::Invalid)?;
        if let Some(previous) = connection
            .query_row(
                "SELECT status FROM activity_journal WHERE activity_id = ?1 ORDER BY source_sequence DESC LIMIT 1",
                params![event.activity_id],
                |row| row.get::<_, String>(0),
            )
            .ok()
            .and_then(|value| status_from_name(&value))
        {
            if !transition_allowed(previous, event.status) {
                return Err(JournalError::Invalid);
            }
        }
        let mut content = serde_json::to_vec(&event).map_err(|_| JournalError::Invalid)?;
        if let Some(payload) = payload {
            let payload =
                base64::Engine::encode(&base64::engine::general_purpose::STANDARD, payload);
            content = serde_json::to_vec(&serde_json::json!({"event": event, "payload": payload}))
                .map_err(|_| JournalError::Invalid)?;
        }
        let envelope = crypto::seal(&self.key, &content, event.activity_id.as_bytes())
            .map_err(|_| JournalError::Crypto)?;
        let checksum = checksum(&envelope);
        let now = now_ms();
        let record_id = format!("{}:{}", event.activity_id, event.source_sequence);
        connection
            .execute(
                "INSERT INTO activity_journal (record_id, activity_id, source_sequence, status, envelope, checksum, created_at_ms, updated_at_ms) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
                params![record_id, event.activity_id, event.source_sequence, status_name(event.status), envelope, checksum, now],
            )
            .map_err(|_| JournalError::Database)?;
        Ok(event)
    }

    pub fn pending(
        &self,
        max_records: usize,
        max_bytes: usize,
    ) -> Result<Vec<PendingRecord>, JournalError> {
        let connection = self.connection.lock().map_err(|_| JournalError::Database)?;
        let mut statement = connection
            .prepare("SELECT record_id, activity_id, envelope, checksum FROM activity_journal WHERE acknowledged = 0 AND next_attempt_ms <= ?1 ORDER BY source_sequence ASC, record_id ASC LIMIT ?2")
            .map_err(|_| JournalError::Database)?;
        let rows = statement
            .query_map(params![now_ms(), max_records as i64], |row| {
                let record_id: String = row.get(0)?;
                let activity_id: String = row.get(1)?;
                let envelope: Vec<u8> = row.get(2)?;
                let stored_checksum: String = row.get(3)?;
                Ok((record_id, activity_id, envelope, stored_checksum))
            })
            .map_err(|_| JournalError::Database)?;
        let mut result = Vec::new();
        let mut bytes = 0usize;
        for row in rows {
            let (record_id, activity_id, envelope, stored_checksum) =
                row.map_err(|_| JournalError::Database)?;
            if bytes.saturating_add(envelope.len()) > max_bytes && !result.is_empty() {
                break;
            }
            if checksum(&envelope) != stored_checksum {
                return Err(JournalError::Crypto);
            }
            let plaintext = crypto::open(&self.key, &envelope, activity_id.as_bytes())
                .map_err(|_| JournalError::Crypto)?;
            let (event, payload) = if let Ok(event) =
                serde_json::from_slice::<ActivityEvent>(&plaintext)
            {
                (event, None)
            } else {
                let value = serde_json::from_slice::<Value>(&plaintext)
                    .ok()
                    .ok_or(JournalError::Invalid)?;
                let event = value
                    .get("event")
                    .cloned()
                    .and_then(|value| serde_json::from_value(value).ok())
                    .ok_or(JournalError::Invalid)?;
                let payload = value
                    .get("payload")
                    .and_then(Value::as_str)
                    .map(|value| {
                        base64::Engine::decode(&base64::engine::general_purpose::STANDARD, value)
                            .map_err(|_| JournalError::Invalid)
                    })
                    .transpose()?;
                (event, payload)
            };
            bytes = bytes.saturating_add(envelope.len());
            result.push(PendingRecord {
                record_id,
                event,
                payload,
            });
        }
        Ok(result)
    }

    pub fn acknowledge(&self, record_ids: &[String]) -> Result<(), JournalError> {
        let connection = self.connection.lock().map_err(|_| JournalError::Database)?;
        let transaction = connection
            .unchecked_transaction()
            .map_err(|_| JournalError::Database)?;
        for record_id in record_ids.iter().take(64) {
            let Some((activity_id, status)) = transaction
                .query_row(
                    "SELECT activity_id, status FROM activity_journal WHERE record_id = ?1",
                    params![record_id],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
                )
                .optional()
                .map_err(|_| JournalError::Database)?
            else {
                continue;
            };
            if status_from_name(&status).is_some_and(|status| {
                matches!(
                    status,
                    Status::Ok
                        | Status::Error
                        | Status::Denied
                        | Status::Cancelled
                        | Status::Interrupted
                )
            }) {
                transaction
                    .execute(
                        "UPDATE activity_journal SET acknowledged = 1, updated_at_ms = ?1 WHERE activity_id = ?2",
                        params![now_ms(), activity_id],
                    )
                    .map_err(|_| JournalError::Database)?;
            }
        }
        transaction.commit().map_err(|_| JournalError::Database)
    }

    pub fn mark_attempt(&self, record_ids: &[String]) -> Result<u64, JournalError> {
        let connection = self.connection.lock().map_err(|_| JournalError::Database)?;
        let transaction = connection
            .unchecked_transaction()
            .map_err(|_| JournalError::Database)?;
        let mut delay_ms = 5_000;
        for record_id in record_ids.iter().take(64) {
            transaction
                .execute(
                    "UPDATE activity_journal SET next_attempt_ms = ?1 + MIN(300000, 5000 * (1 << MIN(attempts, 6))), attempts = attempts + 1, updated_at_ms = ?1 WHERE record_id = ?2 AND acknowledged = 0",
                    params![now_ms(), record_id],
                )
                .map_err(|_| JournalError::Database)?;
            let attempts: u32 = transaction
                .query_row(
                    "SELECT attempts FROM activity_journal WHERE record_id = ?1",
                    params![record_id],
                    |row| row.get(0),
                )
                .map_err(|_| JournalError::Database)?;
            let exponent = attempts.saturating_sub(1).min(6);
            delay_ms = delay_ms.max(5_000_u64.saturating_mul(1_u64 << exponent));
        }
        transaction
            .commit()
            .map(|_| delay_ms)
            .map_err(|_| JournalError::Database)
    }

    pub fn prune_acknowledged(&self) -> Result<(), JournalError> {
        let connection = self.connection.lock().map_err(|_| JournalError::Database)?;
        let cutoff = now_ms().saturating_sub(self.ack_retention_ms as i64);
        connection
            .execute(
                "DELETE FROM activity_journal WHERE acknowledged = 1 AND updated_at_ms < ?1",
                params![cutoff],
            )
            .map_err(|_| JournalError::Database)?;
        Ok(())
    }

    pub fn snapshot(&self) -> Result<JournalSnapshot, JournalError> {
        let connection = self.connection.lock().map_err(|_| JournalError::Database)?;
        let pending = count(&connection, "acknowledged = 0")?;
        let acknowledged = count(&connection, "acknowledged = 1")?;
        Ok(JournalSnapshot {
            source_id: self.source_id.clone(),
            pending_records: pending,
            acknowledged_records: acknowledged,
            degraded: false,
        })
    }

    fn recover_stale(&self) -> Result<(), JournalError> {
        let pending = self.pending(256, 1024 * 1024)?;
        let mut latest = HashMap::new();
        for record in pending {
            latest.insert(record.event.activity_id.clone(), record);
        }
        for record in latest
            .into_values()
            .filter(|record| matches!(record.event.status, Status::Started | Status::Running))
        {
            let mut event = record.event;
            event.status = Status::Interrupted;
            event.presentation.summary =
                Some("relay restarted before the operation reached a terminal state".into());
            self.append(event, None)?;
        }
        Ok(())
    }

    fn ensure_capacity(
        &self,
        connection: &Connection,
        additional_bytes: usize,
    ) -> Result<(), JournalError> {
        let size = database_size(&self.path);
        if size.saturating_add(additional_bytes as u64) <= self.quota {
            return Ok(());
        }
        let cutoff = now_ms().saturating_sub(self.ack_retention_ms as i64);
        connection
            .execute(
                "DELETE FROM activity_journal WHERE acknowledged = 1 AND updated_at_ms < ?1",
                params![cutoff],
            )
            .map_err(|_| JournalError::Database)?;
        if database_size(&self.path).saturating_add(additional_bytes as u64) > self.quota {
            return Err(JournalError::Full);
        }
        Ok(())
    }
}

pub(crate) fn state_dir(config: &ServerConfig) -> Result<PathBuf, JournalError> {
    if let Some(path) = config.activity.state_dir.as_deref() {
        let path = PathBuf::from(path);
        if !path.is_absolute() || path.as_os_str().is_empty() {
            return Err(JournalError::Io);
        }
        if path.exists()
            && fs::symlink_metadata(&path)
                .map_err(|_| JournalError::Io)?
                .file_type()
                .is_symlink()
        {
            return Err(JournalError::Io);
        }
        return Ok(path);
    }
    let home = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state")))
        .ok_or(JournalError::Io)?;
    Ok(home.join("ai-tools"))
}

fn load_or_create_source(path: &Path) -> Result<String, JournalError> {
    if path.exists() {
        ensure_private_file(path)?;
        let source = fs::read_to_string(path)
            .map_err(|_| JournalError::Io)?
            .trim()
            .to_owned();
        if source.len() <= 128
            && !source.is_empty()
            && source.chars().all(|ch| ch.is_ascii_graphic() && ch != '/')
        {
            return Ok(source);
        }
        return Err(JournalError::Invalid);
    }
    let source = Uuid::new_v4().to_string();
    write_private(path, source.as_bytes())?;
    Ok(source)
}

fn load_or_create_key(path: &Path) -> Result<[u8; crypto::KEY_BYTES], JournalError> {
    if path.exists() {
        ensure_private_file(path)?;
        let bytes = fs::read(path).map_err(|_| JournalError::Io)?;
        return bytes.try_into().map_err(|_| JournalError::Invalid);
    }
    let key = crypto::random_key().map_err(|_| JournalError::Crypto)?;
    write_private(path, &key)?;
    Ok(key)
}

fn ensure_private_file(path: &Path) -> Result<(), JournalError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| JournalError::Io)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(JournalError::Io);
    }
    set_owner_only(path, false)
}

fn write_private(path: &Path, content: &[u8]) -> Result<(), JournalError> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    let mut file = options.open(path).map_err(|_| JournalError::Io)?;
    use std::io::Write;
    file.write_all(content).map_err(|_| JournalError::Io)?;
    file.sync_all().map_err(|_| JournalError::Io)?;
    set_owner_only(path, false)
}

fn set_owner_only(path: &Path, directory: bool) -> Result<(), JournalError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = if directory { 0o700 } else { 0o600 };
        fs::set_permissions(path, fs::Permissions::from_mode(mode))
            .map_err(|_| JournalError::Io)?;
    }
    Ok(())
}

fn count(connection: &Connection, predicate: &str) -> Result<u64, JournalError> {
    connection
        .query_row(
            &format!("SELECT COUNT(*) FROM activity_journal WHERE {predicate}"),
            [],
            |row| row.get(0),
        )
        .map_err(|_| JournalError::Database)
}

fn status_name(status: Status) -> &'static str {
    match status {
        Status::Started => "started",
        Status::Running => "running",
        Status::Ok => "ok",
        Status::Error => "error",
        Status::Denied => "denied",
        Status::Cancelled => "cancelled",
        Status::Interrupted => "interrupted",
    }
}

fn status_from_name(status: &str) -> Option<Status> {
    Some(match status {
        "started" => Status::Started,
        "running" => Status::Running,
        "ok" => Status::Ok,
        "error" => Status::Error,
        "denied" => Status::Denied,
        "cancelled" => Status::Cancelled,
        "interrupted" => Status::Interrupted,
        _ => return None,
    })
}

fn checksum(value: &[u8]) -> String {
    ring::digest::digest(&ring::digest::SHA256, value)
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn database_size(path: &Path) -> u64 {
    let base = fs::metadata(path).map(|meta| meta.len()).unwrap_or(0);
    let wal = path.with_extension("sqlite3-wal");
    let shm = path.with_extension("sqlite3-shm");
    base.saturating_add(fs::metadata(wal).map(|meta| meta.len()).unwrap_or(0))
        .saturating_add(fs::metadata(shm).map(|meta| meta.len()).unwrap_or(0))
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(i64::MAX as u128) as i64
}
