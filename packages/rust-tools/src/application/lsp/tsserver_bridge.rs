//! Bounded tsserver-request/response bridge for `@vue/language-server`
//! (Plan 039C PHASE-04, hardened by the 039C Vue Bridge Final Remediation).
//!
//! The installed, reviewed `@vue/language-server@3.3.8` build unconditionally
//! routes every TypeScript-backed operation on `.vue` files through a
//! companion TypeScript project/service that only the *client* can provide:
//! it sends a `tsserver/request` LSP notification `[requestId, command,
//! args]` (`command` a real `tsserver` command prefixed with `_vue:`) and
//! blocks internally on a matching `tsserver/response` notification
//! `[requestId, body]` — `getLanguageService` itself awaits `_vue:projectInfo`
//! before answering anything, so without an answering client every query
//! times out or hangs.
//!
//! This module is that bridge and nothing more: one sandboxed `tsserver.js`
//! child (from the reviewed `tsdk` directory already used for `--tsdk=`),
//! narrowly forwarding only the closed, explicitly reviewed set of
//! `_vue:`-prefixed commands the installed `@vue/language-server@3.3.8` /
//! `@vue/typescript-plugin@3.3.8` pair actually sends (verified against
//! every `sendTsServerRequest(...)` call site in `server.js` and every
//! `session.addProtocolHandler(...)` registration in the plugin's
//! `index.js`). Unlisted or malformed bridge messages are never forwarded;
//! any file-bearing argument is resolved through the same canonical
//! contained-path + protected-path policy the rest of the native workspace
//! surface uses before any host read; outstanding requests and spawned
//! forwarding work are bounded; every request has a hard timeout after
//! which the bridge answers `null` (a real `tsserver` "no info" shape)
//! instead of hanging the caller; and a fatal bridge condition
//! (oversized/malformed child output, stdin/stdout failure, or child exit)
//! faults the owning session (`SharedState::faulted`) exactly the way a
//! fatal condition on the primary LSP child does, so the parent Vue session
//! is never left silently degraded while still reporting healthy.

use super::framing::write_message;
use super::protocol::SharedState;
use super::LspError;
use crate::application::execution::sandbox;
use crate::application::workspace::reject_protected_target;
use crate::core::config::ServerConfig;
use crate::core::workspace_path::{resolve_existing_path, EntryKind};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::process::{Child, ChildStdin};
use tokio::sync::{oneshot, Mutex, Semaphore};
use tokio::time::{timeout, Duration};

mod commands;
mod framing;

/// Bound on outstanding tsserver requests, mirroring `MAX_LSP_PENDING_REQUESTS`.
const MAX_TSSERVER_PENDING: usize = 32;
/// A single tsserver protocol line (request or response) is small JSON;
/// this is generous headroom while still bounding an untrusted/misbehaving
/// child process's ability to grow memory before the bound is enforced.
const MAX_TSSERVER_LINE_BYTES: usize = 1024 * 1024;
const TSSERVER_REQUEST_TIMEOUT: Duration = Duration::from_secs(8);
/// tsserver only answers `_vue:*` commands about a file once that file has
/// been `open`ed in its own project model (otherwise every command fails
/// closed with "No Project"), mirroring `MAX_LSP_DOCUMENTS_PER_SESSION`.
const MAX_TSSERVER_OPEN_FILES: usize = 16;
const MAX_TSSERVER_OPEN_FILE_BYTES: usize = 1024 * 1024;

use commands::{extract_file_argument, ALLOWED_COMMANDS};

struct Pending {
    responses: Mutex<HashMap<i64, oneshot::Sender<Value>>>,
}

struct OpenedFile {
    content_hash: u64,
}

pub(super) struct TsServerBridge {
    writer: Mutex<ChildStdin>,
    pending: Arc<Pending>,
    next_seq: AtomicI64,
    child: Mutex<Child>,
    primary_child: Arc<Mutex<Child>>,
    workspace_root: PathBuf,
    opened_files: Mutex<HashMap<PathBuf, OpenedFile>>,
    /// Shared with the owning session's `SharedState::faulted`: a fatal
    /// bridge condition faults the whole Vue session, not just this
    /// component, so a degraded bridge can never masquerade as a healthy
    /// session that merely answers `null` forever.
    faulted: Arc<AtomicBool>,
    /// Bounds concurrently spawned forwarding tasks *before* any task is
    /// spawned (Required fix 4): a full semaphore means the caller answers
    /// `null` inline instead of spawning unbounded work.
    concurrency: Arc<Semaphore>,
}

impl TsServerBridge {
    /// Spawns the sandboxed `tsserver.js` child from the given reviewed
    /// `tsdk` directory (the same directory already used for `--tsdk=`;
    /// resolved once, up front, never re-derived from untrusted input) and
    /// the `node` executable resolved from the operator-approved toolchain
    /// PATH — no shell, no project-controlled executable selection.
    pub(super) async fn spawn(
        config: &ServerConfig,
        tsdk_dir: &Path,
        plugin_probe_location: Option<&Path>,
        workspace_root: PathBuf,
        faulted: Arc<AtomicBool>,
        primary_child: Arc<Mutex<Child>>,
    ) -> Result<Arc<Self>, LspError> {
        let node = sandbox::resolve_safe_executable(config, "node")
            .map_err(|_| LspError::ServerUnavailable)?;
        let tsserver_js = tsdk_dir.join("tsserver.js");
        if !tsserver_js.is_file() {
            return Err(LspError::ServerUnavailable);
        }
        let mut args = vec![tsserver_js.to_string_lossy().into_owned()];
        // Registers the already-installed, reviewed `@vue/typescript-plugin`
        // as a tsserver global plugin so tsserver treats `.vue` as a
        // project file extension (see `ApprovedServerSpec::
        // tsserver_bridge_plugin_probe` for why this is required). This is
        // the one reviewed plugin the installed `@vue/language-server`
        // itself depends on and ships in its own `node_modules` — not an
        // arbitrary/project-controlled plugin path.
        if let Some(probe) = plugin_probe_location {
            if probe.join("node_modules/@vue/typescript-plugin").is_dir() {
                args.push("--globalPlugins".to_owned());
                args.push("@vue/typescript-plugin".to_owned());
                args.push("--pluginProbeLocations".to_owned());
                args.push(probe.to_string_lossy().into_owned());
            }
        }
        let mut child = sandbox::spawn_lsp(config, node, args, workspace_root.clone())
            .map_err(|_| LspError::StartupFailed)?;
        let stdin = child.stdin.take().ok_or(LspError::StartupFailed)?;
        let stdout = child.stdout.take().ok_or(LspError::StartupFailed)?;
        // stderr is drained and discarded (not retained/exposed) purely to
        // stop the child from blocking on a full pipe; bridge failures are
        // observable through request timeouts and session faulting, not
        // raw child stderr.
        if let Some(mut stderr) = child.stderr.take() {
            tokio::spawn(async move {
                use tokio::io::AsyncReadExt;
                let mut sink = [0u8; 4096];
                loop {
                    match stderr.read(&mut sink).await {
                        Ok(0) | Err(_) => return,
                        Ok(_) => {}
                    }
                }
            });
        }
        let pending = Arc::new(Pending {
            responses: Mutex::new(HashMap::new()),
        });
        let bridge = Arc::new(Self {
            writer: Mutex::new(stdin),
            pending,
            next_seq: AtomicI64::new(1),
            child: Mutex::new(child),
            primary_child,
            workspace_root,
            opened_files: Mutex::new(HashMap::new()),
            faulted,
            concurrency: Arc::new(Semaphore::new(MAX_TSSERVER_PENDING)),
        });
        tokio::spawn(framing::read_loop(stdout, bridge.clone()));
        Ok(bridge)
    }

    /// Bounds concurrently forwarded requests *before* any forwarding work
    /// is spawned. Returns `None` at capacity; the caller must answer the
    /// bridged request with `null` without spawning anything further.
    pub(super) fn try_acquire(self: &Arc<Self>) -> Option<tokio::sync::OwnedSemaphorePermit> {
        self.concurrency.clone().try_acquire_owned().ok()
    }

    fn is_faulted(&self) -> bool {
        self.faulted.load(Ordering::SeqCst)
    }

    /// Ensures tsserver has `open`ed the given file (or reopened it with
    /// fresh content, if changed since the last observed version) before
    /// any `_vue:` command about it is forwarded — without this, tsserver
    /// answers every such command with a "No Project" failure, since it has
    /// no other way to learn about a file (the `.vue` extension is not one
    /// tsserver recognizes on its own; only the `open` command's inline
    /// `fileContent` makes the file visible to it here).
    ///
    /// `file` is resolved through the same canonical contained-path +
    /// protected-path policy the rest of the native workspace surface uses
    /// (Required fix 1): lexical `starts_with` checks and unguarded
    /// `read_to_string` calls are never used here, so `../` traversal,
    /// outside absolute paths, outside-target symlinks, and protected
    /// targets (`.ssh`, `.aws`, etc.) are all rejected before any host
    /// read. Returns the resolved absolute path on success so callers use
    /// the same canonical form tsserver was told about.
    async fn ensure_open(&self, file: &str) -> Option<String> {
        let target = resolve_bridge_file(&self.workspace_root, file)?;
        let content = tokio::fs::read_to_string(&target).await.ok()?;
        if content.len() > MAX_TSSERVER_OPEN_FILE_BYTES {
            return None;
        }
        let hash = content_hash(&content);
        let canonical = target.to_string_lossy().into_owned();
        {
            let mut opened = self.opened_files.lock().await;
            match opened.get(&target) {
                Some(existing) if existing.content_hash == hash => return Some(canonical),
                Some(_) => {
                    // Content changed since this file was last opened:
                    // close and reopen with fresh content so the same
                    // Vue session/bridge observes native edits without a
                    // full restart (Required fix 5).
                    self.close_file(&canonical).await;
                }
                None if opened.len() >= MAX_TSSERVER_OPEN_FILES => return None,
                None => {}
            }
            opened.insert(target.clone(), OpenedFile { content_hash: hash });
        }
        self.open_file(&canonical, &content).await;
        Some(canonical)
    }

    async fn open_file(&self, file: &str, content: &str) {
        let seq = self.next_seq.fetch_add(1, Ordering::Relaxed);
        let open = json!({
            "seq": seq,
            "type": "request",
            "command": "open",
            "arguments": {
                "file": file,
                "fileContent": content,
                "projectRootPath": self.workspace_root.to_string_lossy(),
            },
        });
        self.send_line(&open).await;
        // `open` has no matching `response` message in the default
        // (non-verbose) tsserver protocol; a brief settle delay is the
        // simplest bounded way to let tsserver finish processing it before
        // the next `_vue:` command depends on the file being known.
        tokio::time::sleep(Duration::from_millis(150)).await;
    }

    async fn close_file(&self, file: &str) {
        let seq = self.next_seq.fetch_add(1, Ordering::Relaxed);
        let close = json!({
            "seq": seq,
            "type": "request",
            "command": "close",
            "arguments": {"file": file},
        });
        self.send_line(&close).await;
    }

    async fn send_line(&self, value: &Value) {
        let Ok(mut line) = serde_json::to_vec(value) else {
            return;
        };
        line.push(b'\n');
        let mut writer = self.writer.lock().await;
        if writer.write_all(&line).await.is_err() || writer.flush().await.is_err() {
            drop(writer);
            self.mark_fatal().await;
        }
    }

    /// Forwards one allowlisted, exact `_vue:` tsserver command and
    /// returns its response `body` (or `Value::Null` on timeout/failure —
    /// the same shape a real `tsserver` "no info available" response has,
    /// so a caller waiting on `tsserver/response` is always unblocked
    /// instead of hung). The caller (`handle_tsserver_request`) has already
    /// checked the command against `ALLOWED_COMMANDS`; this only re-derives
    /// and validates any file-bearing argument before forwarding.
    pub(super) async fn request(&self, command: &str, arguments: Value) -> Value {
        if self.is_faulted() {
            return Value::Null;
        }
        if let Some(file) = extract_file_argument(command, &arguments) {
            if self.ensure_open(&file).await.is_none() {
                return Value::Null;
            }
        }
        let seq = self.next_seq.fetch_add(1, Ordering::Relaxed);
        let (sender, receiver) = oneshot::channel();
        {
            let mut responses = self.pending.responses.lock().await;
            if responses.len() >= MAX_TSSERVER_PENDING {
                return Value::Null;
            }
            responses.insert(seq, sender);
        }
        let request = json!({
            "seq": seq,
            "type": "request",
            "command": command,
            "arguments": arguments,
        });
        let Ok(mut line) = serde_json::to_vec(&request) else {
            self.pending.responses.lock().await.remove(&seq);
            return Value::Null;
        };
        line.push(b'\n');
        {
            let mut writer = self.writer.lock().await;
            if writer.write_all(&line).await.is_err() || writer.flush().await.is_err() {
                drop(writer);
                self.pending.responses.lock().await.remove(&seq);
                self.mark_fatal().await;
                return Value::Null;
            }
        }
        match timeout(TSSERVER_REQUEST_TIMEOUT, receiver).await {
            Ok(Ok(body)) => body,
            Ok(Err(_)) | Err(_) => {
                self.pending.responses.lock().await.remove(&seq);
                Value::Null
            }
        }
    }

    /// Fails the bridge closed (Required fix 6): faults the shared flag the
    /// owning `LspSession` also checks (so `LspSessionManager` treats the
    /// whole session as unhealthy and replaces it on next use, preserving
    /// restart budgets the same way a primary-child crash does), drains any
    /// outstanding requests with `null` instead of leaving them hanging to
    /// their own timeout, and reaps the child process.
    async fn mark_fatal(&self) {
        self.faulted.store(true, Ordering::SeqCst);
        let mut responses = self.pending.responses.lock().await;
        for (_, sender) in responses.drain() {
            let _ = sender.send(Value::Null);
        }
        drop(responses);
        let mut child = self.child.lock().await;
        crate::application::execution::kill_process_group(&mut child).await;
        let _ = child.wait().await;
        drop(child);
        if let Ok(mut primary) = self.primary_child.try_lock() {
            crate::application::execution::kill_process_group(&mut primary).await;
            let _ = primary.wait().await;
        } else {
            let primary_child = self.primary_child.clone();
            tokio::spawn(async move {
                let mut primary = primary_child.lock().await;
                crate::application::execution::kill_process_group(&mut primary).await;
                let _ = primary.wait().await;
            });
        }
    }

    pub(super) async fn shutdown(&self) {
        let mut child = self.child.lock().await;
        crate::application::execution::kill_process_group(&mut child).await;
        let _ = child.wait().await;
    }

    pub(super) async fn child_id(&self) -> Option<u32> {
        self.child.lock().await.id()
    }
}

/// Resolves a server-supplied bridge file path through the same canonical
/// contained-path + protected-path policy the rest of the native workspace
/// surface uses (Required fix 1). Pure/host-read-free except for the
/// filesystem metadata `resolve_existing_path` itself performs, so this is
/// directly unit-testable against fixture directories without spawning any
/// process.
fn resolve_bridge_file(workspace_root: &Path, file: &str) -> Option<PathBuf> {
    let target = resolve_existing_path(workspace_root, None, file, EntryKind::File).ok()?;
    reject_protected_target(workspace_root, &target).ok()?;
    Some(target)
}

fn content_hash(content: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    hasher.finish()
}

/// Answers one `@vue/language-server` `tsserver/request` notification
/// `[requestId, command, args]` by forwarding only allowlisted exact `_vue:`
/// commands to the session's bounded
/// `tsserver_bridge`, then always replying with a matching
/// `tsserver/response` notification `[requestId, body]` so the server's
/// internal promise is never left hanging — on any validation failure,
/// missing bridge, unlisted command, or capacity exhaustion the reply body
/// is `null` and nothing is forwarded to the child process. Concurrency is
/// bounded *before* any forwarding task is spawned (Required fix 2): at
/// capacity, the null reply is written inline.
pub(super) async fn handle_tsserver_request(state: &Arc<SharedState>, params: Option<&Value>) {
    // LSP notification params are a positional-args array; the actual
    // `[requestId, command, args]` tuple is the first (and only) element,
    // not `params` itself — verified against the installed
    // `@vue/language-server@3.3.8` build's own `connection.sendNotification`
    // call, which wraps its tuple the same way.
    let Some(params) = params
        .and_then(Value::as_array)
        .and_then(|outer| outer.first())
        .and_then(Value::as_array)
    else {
        return;
    };
    if params.len() != 3 {
        return;
    }
    let Some(request_id) = params[0].as_i64() else {
        return;
    };
    let Some(command) = params[1].as_str().map(str::to_owned) else {
        return;
    };
    let arguments = params[2].clone();
    let forward = state
        .tsserver_bridge
        .clone()
        .filter(|_| ALLOWED_COMMANDS.contains(&command.as_str()));
    let state = state.clone();
    match forward {
        Some(bridge) => match bridge.try_acquire() {
            Some(permit) => {
                tokio::spawn(async move {
                    let body = bridge.request(&command, arguments).await;
                    drop(permit);
                    reply(&state, request_id, body).await;
                });
            }
            None => {
                reply(&state, request_id, Value::Null).await;
            }
        },
        None => {
            reply(&state, request_id, Value::Null).await;
        }
    }
}

async fn reply(state: &Arc<SharedState>, request_id: i64, body: Value) {
    let response = json!({
        "jsonrpc": "2.0",
        "method": "tsserver/response",
        "params": [[request_id, body]],
    });
    if write_message(&mut *state.writer.lock().await, &response)
        .await
        .is_err()
    {
        super::protocol::fault_and_terminate(state, &state.primary_child, LspError::Crashed).await;
    }
}
