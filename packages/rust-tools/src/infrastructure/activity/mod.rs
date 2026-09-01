//! Relay-local durable activity journal and asynchronous delivery.
//!
//! This module is deliberately infrastructure-owned. The application crate
//! only sees `ActivityRecorder`; SQLite, key files, encryption, and HTTP
//! delivery never cross that boundary.

pub(crate) mod crypto;
mod journal;

use crate::application::activity::{ActivityError, ActivityEvent, ActivityRecorder, Status};
use crate::core::config::{ActivityMode, ServerConfig};
use journal::Journal;
use ring::rand::SecureRandom;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use tokio::time::{sleep, Duration};

pub(crate) use journal::state_dir;
pub use journal::JournalSnapshot;

const MAX_EXPORT_RECORDS: usize = 25;
const MAX_EXPORT_BYTES: usize = 512 * 1024;
const BOOTSTRAP_CONFIG_FILE: &str = "activity-bootstrap.json";

#[derive(Debug, Clone, Deserialize, Serialize)]
struct PersistedActivityBootstrap {
    sink_url: String,
    source_token: String,
}

#[derive(Clone)]
pub struct ReloadableActivityRecorder {
    base_config: ServerConfig,
    inner: Arc<RwLock<Arc<ActivityRuntime>>>,
}

impl ReloadableActivityRecorder {
    pub fn open(config: &ServerConfig) -> Result<Arc<Self>, ActivityError> {
        let effective = persisted_activity_config(config).unwrap_or_else(|| config.clone());
        let inner = ActivityRuntime::open(&effective)?;
        Ok(Arc::new(Self {
            base_config: config.clone(),
            inner: Arc::new(RwLock::new(inner)),
        }))
    }

    pub fn configure(&self, sink_url: String, source_token: String) -> Result<(), ActivityError> {
        let mut effective = self.base_config.clone();
        effective.activity.mode = ActivityMode::Required;
        effective.activity.sink_url = Some(sink_url.clone());
        effective.activity.source_token = Some(source_token.clone());
        effective
            .validate()
            .map_err(|_| ActivityError::InvalidEvent)?;
        persist_bootstrap(
            &effective,
            &PersistedActivityBootstrap {
                sink_url,
                source_token,
            },
        )?;
        let runtime = ActivityRuntime::open(&effective)?;
        let mut guard = self
            .inner
            .write()
            .map_err(|_| ActivityError::StorageUnavailable)?;
        *guard = runtime;
        Ok(())
    }

    pub fn status(&self) -> Result<(bool, Option<String>), ActivityError> {
        let runtime = self.current()?;
        Ok((
            runtime.journal.is_some()
                && runtime.sink_url.is_some()
                && runtime.source_token.is_some(),
            runtime.source_id.clone(),
        ))
    }

    fn current(&self) -> Result<Arc<ActivityRuntime>, ActivityError> {
        self.inner
            .read()
            .map(|guard| Arc::clone(&guard))
            .map_err(|_| ActivityError::StorageUnavailable)
    }
}

impl ActivityRecorder for ReloadableActivityRecorder {
    fn required(&self) -> bool {
        self.current()
            .map(|recorder| recorder.required())
            .unwrap_or(true)
    }

    fn record_start(
        &self,
        event: ActivityEvent,
        payload: Option<Vec<u8>>,
    ) -> Result<ActivityEvent, ActivityError> {
        self.current()?.record_start(event, payload)
    }

    fn record_outcome(
        &self,
        event: ActivityEvent,
        payload: Option<Vec<u8>>,
    ) -> Result<(), ActivityError> {
        self.current()?.record_outcome(event, payload)
    }
}

fn persisted_activity_config(config: &ServerConfig) -> Option<ServerConfig> {
    let path = bootstrap_path(config).ok()?;
    if !path.exists() {
        return None;
    }
    let metadata = fs::symlink_metadata(&path).ok()?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return None;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            return None;
        }
    }
    let persisted: PersistedActivityBootstrap =
        serde_json::from_slice(&fs::read(path).ok()?).ok()?;
    let mut effective = config.clone();
    effective.activity.mode = ActivityMode::Required;
    effective.activity.sink_url = Some(persisted.sink_url);
    effective.activity.source_token = Some(persisted.source_token);
    effective.validate().ok()?;
    Some(effective)
}

fn persist_bootstrap(
    config: &ServerConfig,
    persisted: &PersistedActivityBootstrap,
) -> Result<(), ActivityError> {
    let path = bootstrap_path(config).map_err(|_| ActivityError::StorageUnavailable)?;
    let parent = path.parent().ok_or(ActivityError::StorageUnavailable)?;
    fs::create_dir_all(parent).map_err(|_| ActivityError::StorageUnavailable)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(parent, fs::Permissions::from_mode(0o700))
            .map_err(|_| ActivityError::StorageUnavailable)?;
    }
    let bytes = serde_json::to_vec(persisted).map_err(|_| ActivityError::StorageUnavailable)?;
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, bytes).map_err(|_| ActivityError::StorageUnavailable)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&temporary, fs::Permissions::from_mode(0o600))
            .map_err(|_| ActivityError::StorageUnavailable)?;
    }
    fs::rename(&temporary, &path).map_err(|_| ActivityError::StorageUnavailable)?;
    Ok(())
}

fn bootstrap_path(config: &ServerConfig) -> Result<PathBuf, journal::JournalError> {
    Ok(journal::state_dir(config)?.join(BOOTSTRAP_CONFIG_FILE))
}

#[derive(Clone)]
pub struct ActivityRuntime {
    journal: Option<Arc<Journal>>,
    source_id: Option<String>,
    sink_url: Option<String>,
    source_token: Option<String>,
    degraded: Arc<AtomicBool>,
}

impl ActivityRuntime {
    pub fn open(config: &ServerConfig) -> Result<Arc<Self>, ActivityError> {
        if config.activity.mode == ActivityMode::Off {
            return Ok(Arc::new(Self {
                journal: None,
                source_id: None,
                sink_url: None,
                source_token: None,
                degraded: Arc::new(AtomicBool::new(false)),
            }));
        }

        let journal = Journal::open(config).map_err(|_| ActivityError::StorageUnavailable)?;
        let source_id = journal.source_id().to_owned();
        let runtime = Arc::new(Self {
            journal: Some(Arc::new(journal)),
            source_id: Some(source_id),
            sink_url: config.activity.sink_url.clone(),
            source_token: config.activity.source_token.clone(),
            degraded: Arc::new(AtomicBool::new(false)),
        });
        runtime.spawn_exporter();
        Ok(runtime)
    }

    pub fn snapshot(&self) -> Option<JournalSnapshot> {
        self.journal
            .as_ref()
            .and_then(|journal| journal.snapshot().ok())
            .map(|mut snapshot| {
                snapshot.degraded = self.degraded.load(Ordering::Acquire);
                snapshot
            })
    }

    fn spawn_exporter(self: &Arc<Self>) {
        if self.sink_url.is_none() || self.source_token.is_none() {
            return;
        }
        let Ok(handle) = tokio::runtime::Handle::try_current() else {
            return;
        };
        let runtime = Arc::clone(self);
        handle.spawn(async move {
            let client = match reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
            {
                Ok(client) => client,
                Err(_) => return,
            };
            loop {
                if runtime.degraded.load(Ordering::Acquire) {
                    return;
                }
                let result = runtime.flush_once(&client).await;
                let delay = match result {
                    FlushResult::Delivered => Duration::from_secs(2),
                    FlushResult::Empty => Duration::from_secs(5),
                    FlushResult::Retry(delay) => delay,
                    FlushResult::Revoked => return,
                };
                sleep(delay).await;
            }
        });
    }

    async fn flush_once(&self, client: &reqwest::Client) -> FlushResult {
        let Some(journal) = &self.journal else {
            return FlushResult::Empty;
        };
        let records = match journal.pending(MAX_EXPORT_RECORDS, MAX_EXPORT_BYTES, true, false) {
            Ok(records) => records,
            Err(_) => return FlushResult::Retry(Duration::from_secs(10)),
        };
        if records.is_empty() {
            let _ = journal.prune_acknowledged();
            return FlushResult::Empty;
        }
        let Some(url) = &self.sink_url else {
            return FlushResult::Empty;
        };
        let Some(token) = &self.source_token else {
            return FlushResult::Empty;
        };
        let record_ids = records
            .iter()
            .map(|record| record.record_id.clone())
            .collect::<Vec<_>>();
        let retry_delay = match journal.mark_attempt(&record_ids) {
            Ok(delay_ms) => retry_delay(delay_ms),
            Err(_) => return FlushResult::Retry(Duration::from_secs(10)),
        };
        let body = json!({
            "contractVersion": crate::application::activity::CONTRACT_VERSION,
            "sourceId": self.source_id,
            "events": records.iter().map(export_record).collect::<Vec<_>>()
        });
        let response = match client
            .post(url)
            .bearer_auth(token)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .json(&body)
            .send()
            .await
        {
            Ok(response) => response,
            Err(_) => return FlushResult::Retry(retry_delay),
        };
        let retry_delay = response_retry_delay(&response, retry_delay);
        if response.status() == reqwest::StatusCode::UNAUTHORIZED
            || response.status() == reqwest::StatusCode::FORBIDDEN
        {
            self.degraded.store(true, Ordering::Release);
            return FlushResult::Revoked;
        }
        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS
            || response.status().is_server_error()
        {
            return FlushResult::Retry(retry_delay);
        }
        if !response.status().is_success() {
            return FlushResult::Retry(retry_delay);
        }
        let (mut accepted, duplicates) = match response.json::<AckResponse>().await {
            Ok(ack) => (ack.accepted, ack.duplicates),
            Err(_) => return FlushResult::Retry(retry_delay),
        };
        accepted.extend(duplicates);
        if accepted.is_empty() {
            return FlushResult::Retry(retry_delay);
        }
        if journal.acknowledge(&accepted).is_err() {
            return FlushResult::Retry(retry_delay);
        }
        let _ = journal.prune_acknowledged();
        FlushResult::Delivered
    }
}

fn export_record(record: &journal::PendingRecord) -> serde_json::Value {
    let mut value = json!({
        "recordId": record.record_id,
        "event": record.event,
    });
    if let Some(payload) = record.payload.as_ref() {
        value["payload"] = json!({
            "kind": "activity_evidence",
            "version": "v1",
            "value": base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                payload,
            ),
            "byteCount": payload.len()
        });
    }
    value
}

#[derive(Debug, serde::Deserialize)]
struct AckResponse {
    #[serde(default)]
    accepted: Vec<String>,
    #[serde(default)]
    duplicates: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
enum FlushResult {
    Delivered,
    Empty,
    Retry(Duration),
    Revoked,
}

fn retry_delay(base_ms: u64) -> Duration {
    let mut bytes = [0_u8; 2];
    let jitter = ring::rand::SystemRandom::new()
        .fill(&mut bytes)
        .map(|_| u16::from_le_bytes(bytes) as u64 % (base_ms / 5 + 1))
        .unwrap_or(0);
    Duration::from_millis(base_ms.saturating_add(jitter).min(300_000))
}

fn response_retry_delay(response: &reqwest::Response, fallback: Duration) -> Duration {
    let Some(seconds) = response
        .headers()
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
    else {
        return fallback;
    };
    Duration::from_secs(seconds.min(300))
}

impl ActivityRecorder for ActivityRuntime {
    fn required(&self) -> bool {
        self.journal.is_some()
    }

    fn record_start(
        &self,
        mut event: ActivityEvent,
        payload: Option<Vec<u8>>,
    ) -> Result<ActivityEvent, ActivityError> {
        if self.journal.is_none() {
            return Ok(event);
        }
        if event.status != Status::Started {
            return Err(ActivityError::InvalidEvent);
        }
        if event.source_id.is_empty() {
            event.source_id = self.source_id.clone().ok_or(ActivityError::InvalidEvent)?;
        }
        let journal = self
            .journal
            .as_ref()
            .ok_or(ActivityError::StorageUnavailable)?;
        let assigned = journal
            .append(event, payload)
            .map_err(|error| match error {
                journal::JournalError::Full => ActivityError::AdmissionFailed,
                _ => ActivityError::StorageUnavailable,
            })?;
        Ok(assigned)
    }

    fn record_outcome(
        &self,
        mut event: ActivityEvent,
        payload: Option<Vec<u8>>,
    ) -> Result<(), ActivityError> {
        let Some(journal) = &self.journal else {
            return Ok(());
        };
        if event.source_id.is_empty() {
            event.source_id = self.source_id.clone().ok_or(ActivityError::InvalidEvent)?;
        }
        let is_terminal = matches!(
            event.status,
            Status::Ok | Status::Error | Status::Denied | Status::Cancelled | Status::Interrupted
        );
        match journal.append(event.clone(), payload) {
            Ok(_) => Ok(()),
            Err(error) => {
                if is_terminal {
                    event.status = Status::Interrupted;
                    event.presentation.summary =
                        Some("activity outcome could not be durably recorded".into());
                    event.presentation.result_class = Some("interrupted".into());
                    event.presentation.evidence =
                        crate::application::activity::Evidence::Unavailable;
                    event.presentation.payload_reference = None;
                    event.presentation.complete = true;
                    let _ = journal.append(event, None);
                }
                Err(map_journal_error(error))
            }
        }
    }
}

fn map_journal_error(error: journal::JournalError) -> ActivityError {
    match error {
        journal::JournalError::Full => ActivityError::AdmissionFailed,
        journal::JournalError::Invalid => ActivityError::InvalidEvent,
        journal::JournalError::Io
        | journal::JournalError::Database
        | journal::JournalError::Crypto => ActivityError::StorageUnavailable,
    }
}
