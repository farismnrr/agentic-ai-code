//! Bounded tsserver-request/response bridge for `@vue/language-server`
//! (Plan 039C PHASE-04 remediation).
//!
//! The installed, reviewed `@vue/language-server@3.3.8` build (verified by
//! reading `@vue/language-server/lib/server.js`) unconditionally routes
//! every TypeScript-backed operation on `.vue` files through a companion
//! TypeScript project/service that only the *client* can provide: it sends
//! a `tsserver/request` LSP notification `[requestId, command, args]` where
//! `command` is a real `tsserver` command name prefixed with `_vue:`, and
//! blocks internally on a matching `tsserver/response` notification
//! `[requestId, body]`. Without a client that answers this, the server's
//! own internal promise for that request never resolves and any query
//! needing it (which, per `server.js`, is effectively all of them, since
//! `getLanguageService` itself awaits a `_vue:ProjectInfo` request) times
//! out or hangs.
//!
//! This module is that bridge and nothing more: it starts exactly one
//! sandboxed `tsserver.js` child process (from the same reviewed TypeScript
//! `tsdk` directory already passed to both language servers via
//! `--tsdk=`), narrowly forwards only `_vue:`-prefixed commands with their
//! `_vue:` prefix stripped, and returns the matching response body. It is
//! not a general subprocess bridge: unprefixed or malformed bridge
//! messages are never forwarded, outstanding requests are bounded, and
//! every request has a hard timeout after which the bridge answers `null`
//! (matching a real `tsserver` "no info" response) rather than hanging the
//! caller.

use super::framing::write_message;
use super::protocol::SharedState;
use super::LspError;
use crate::execution::sandbox;
use relay_core::config::ServerConfig;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin};
use tokio::sync::{oneshot, Mutex};
use tokio::time::{timeout, Duration};

/// Bound on outstanding tsserver requests, mirroring `MAX_LSP_PENDING_REQUESTS`.
const MAX_TSSERVER_PENDING: usize = 32;
/// A single tsserver protocol line (request or response) is small JSON;
/// this is generous headroom while still bounding an untrusted/misbehaving
/// child process's ability to grow memory.
const MAX_TSSERVER_LINE_BYTES: usize = 1024 * 1024;
const TSSERVER_REQUEST_TIMEOUT: Duration = Duration::from_secs(8);
/// tsserver only answers `_vue:*` commands about a file once that file has
/// been `open`ed in its own project model (otherwise every command fails
/// closed with "No Project"), mirroring `MAX_LSP_DOCUMENTS_PER_SESSION`.
const MAX_TSSERVER_OPEN_FILES: usize = 16;
const MAX_TSSERVER_OPEN_FILE_BYTES: usize = 1024 * 1024;

struct Pending {
    responses: Mutex<HashMap<i64, oneshot::Sender<Value>>>,
}

pub(super) struct TsServerBridge {
    writer: Mutex<ChildStdin>,
    pending: Arc<Pending>,
    next_seq: AtomicI64,
    child: Mutex<Child>,
    workspace_root: PathBuf,
    opened_files: Mutex<std::collections::HashSet<String>>,
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
        // observable through request timeouts, not raw child stderr.
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
        tokio::spawn(read_loop(stdout, pending.clone()));
        Ok(Arc::new(Self {
            writer: Mutex::new(stdin),
            pending,
            next_seq: AtomicI64::new(1),
            child: Mutex::new(child),
            workspace_root,
            opened_files: Mutex::new(std::collections::HashSet::new()),
        }))
    }

    /// Ensures tsserver has `open`ed the given file before any `_vue:`
    /// command about it is forwarded — without this, tsserver answers
    /// every such command with a "No Project" failure, since it has no
    /// other way to learn about a file (the `.vue` extension is not one
    /// tsserver recognizes on its own; only the `open` command's inline
    /// `fileContent` makes the file visible to it here). Only files inside
    /// this session's own workspace root are ever opened; only a bounded
    /// number of distinct files are tracked at once.
    async fn ensure_open(&self, file: &str) {
        let path = PathBuf::from(file);
        if !path.starts_with(&self.workspace_root) {
            return;
        }
        {
            let mut opened = self.opened_files.lock().await;
            if opened.contains(file) || opened.len() >= MAX_TSSERVER_OPEN_FILES {
                return;
            }
            opened.insert(file.to_owned());
        }
        let Ok(content) = tokio::fs::read_to_string(&path).await else {
            return;
        };
        if content.len() > MAX_TSSERVER_OPEN_FILE_BYTES {
            return;
        }
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
        let Ok(mut line) = serde_json::to_vec(&open) else {
            return;
        };
        line.push(b'\n');
        {
            let mut writer = self.writer.lock().await;
            let _ = writer.write_all(&line).await;
            let _ = writer.flush().await;
        }
        // `open` has no matching `response` message in the default
        // (non-verbose) tsserver protocol; a brief settle delay is the
        // simplest bounded way to let tsserver finish processing it before
        // the next `_vue:` command depends on the file being known.
        tokio::time::sleep(Duration::from_millis(150)).await;
    }

    /// Forwards one `_vue:`-stripped tsserver command and returns its
    /// response `body` (or `Value::Null` on timeout/failure — the same
    /// shape a real `tsserver` "no info available" response has, so a
    /// caller waiting on `tsserver/response` is always unblocked instead
    /// of hung).
    pub(super) async fn request(&self, command: &str, arguments: Value) -> Value {
        if let Some(file) = arguments.get("file").and_then(Value::as_str) {
            self.ensure_open(file).await;
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
                self.pending.responses.lock().await.remove(&seq);
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

    pub(super) async fn shutdown(&self) {
        let mut child = self.child.lock().await;
        crate::execution::kill_process_group(&mut child).await;
        let _ = child.wait().await;
    }
}

async fn read_loop(stdout: tokio::process::ChildStdout, pending: Arc<Pending>) {
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    loop {
        line.clear();
        let read = match reader.read_line(&mut line).await {
            Ok(0) => return,
            Ok(count) => count,
            Err(_) => return,
        };
        if read > MAX_TSSERVER_LINE_BYTES {
            return;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Ok(value) = serde_json::from_str::<Value>(trimmed) else {
            continue;
        };
        if value.get("type").and_then(Value::as_str) != Some("response") {
            continue;
        }
        let Some(request_seq) = value.get("request_seq").and_then(Value::as_i64) else {
            continue;
        };
        let body = if value.get("success").and_then(Value::as_bool) == Some(true) {
            value.get("body").cloned().unwrap_or(Value::Null)
        } else {
            Value::Null
        };
        if let Some(sender) = pending.responses.lock().await.remove(&request_seq) {
            let _ = sender.send(body);
        }
    }
}

/// Answers one `@vue/language-server` `tsserver/request` notification
/// `[requestId, command, args]` by forwarding only `_vue:`-prefixed
/// commands (with the prefix stripped) to the session's bounded
/// `tsserver_bridge`, then always replying with a matching
/// `tsserver/response` notification `[requestId, body]` so the server's
/// internal promise is never left hanging — on any validation failure,
/// missing bridge, or non-`_vue:` command the reply body is `null` and
/// nothing is forwarded to the child process.
pub(super) fn handle_tsserver_request(state: &Arc<SharedState>, params: Option<&Value>) {
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
    let Some(command) = params[1].as_str() else {
        return;
    };
    let arguments = params[2].clone();
    let forward = state
        .tsserver_bridge
        .clone()
        .zip(command.strip_prefix("_vue:").map(str::to_owned));
    let state = state.clone();
    tokio::spawn(async move {
        let body = match forward {
            Some((bridge, command)) => bridge.request(&command, arguments).await,
            None => Value::Null,
        };
        let response = json!({
            "jsonrpc": "2.0",
            "method": "tsserver/response",
            "params": [[request_id, body]],
        });
        let _ = write_message(&mut *state.writer.lock().await, &response).await;
    });
}
