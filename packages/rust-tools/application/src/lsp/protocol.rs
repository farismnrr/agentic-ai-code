use super::framing::{read_message, write_message};
use super::{
    ApprovedServerSpec, LspError, ServerCapabilities, WorkspaceIdentity, LSP_REQUEST_TIMEOUT,
    LSP_SHUTDOWN_TIMEOUT, LSP_STARTUP_TIMEOUT, MAX_LSP_PENDING_REQUESTS, MAX_LSP_STDERR_BYTES,
};
use crate::execution::sandbox;
use relay_core::config::ServerConfig;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::io::AsyncReadExt;
use tokio::process::{Child, ChildStdin};
use tokio::sync::{oneshot, Mutex, RwLock};
use tokio::time::timeout;

struct SharedState {
    writer: Mutex<ChildStdin>,
    pending: Mutex<HashMap<i64, oneshot::Sender<Result<Value, LspError>>>>,
    faulted: AtomicBool,
}

pub struct LspSession {
    identity: WorkspaceIdentity,
    language: String,
    capabilities: ServerCapabilities,
    state: Arc<SharedState>,
    child: Mutex<Child>,
    next_id: AtomicI64,
    last_used: RwLock<Instant>,
    stderr: Arc<Mutex<Vec<u8>>>,
    documents: Mutex<super::document::DocumentStore>,
}

impl LspSession {
    pub(super) async fn start(
        config: &ServerConfig,
        spec: &ApprovedServerSpec,
        identity: WorkspaceIdentity,
    ) -> Result<Arc<Self>, LspError> {
        let mut child = sandbox::spawn_lsp(
            config,
            spec.executable.clone(),
            spec.args.clone(),
            identity.root.clone(),
        )
        .map_err(|_| LspError::StartupFailed)?;
        let stdin = child.stdin.take().ok_or(LspError::StartupFailed)?;
        let stdout = child.stdout.take().ok_or(LspError::StartupFailed)?;
        let stderr_pipe = child.stderr.take().ok_or(LspError::StartupFailed)?;
        let state = Arc::new(SharedState {
            writer: Mutex::new(stdin),
            pending: Mutex::new(HashMap::new()),
            faulted: AtomicBool::new(false),
        });
        let stderr = Arc::new(Mutex::new(Vec::new()));
        tokio::spawn(drain_stderr(stderr_pipe, stderr.clone()));
        tokio::spawn(read_loop(stdout, state.clone()));

        let placeholder = ServerCapabilities {
            definition: false,
            references: false,
            implementation: false,
            hover: false,
            document_symbols: false,
            workspace_symbols: false,
            rename: false,
            diagnostic_pull: false,
            text_sync: super::TextDocumentSyncKind::None,
            open_close: false,
        };
        let mut session = Arc::new(Self {
            identity,
            language: spec.language.clone(),
            capabilities: placeholder,
            state,
            child: Mutex::new(child),
            next_id: AtomicI64::new(1),
            last_used: RwLock::new(Instant::now()),
            stderr,
            documents: Mutex::new(super::document::DocumentStore::default()),
        });
        let root_uri = url::Url::from_directory_path(&session.identity.root)
            .map_err(|_| LspError::StartupFailed)?
            .to_string();
        let initialize = timeout(
            LSP_STARTUP_TIMEOUT,
            session.request_with_timeout(
                "initialize",
                json!({
                    "processId": Value::Null,
                    "clientInfo": {"name":"ai-tools","version":env!("CARGO_PKG_VERSION")},
                    "rootUri": root_uri,
                    "workspaceFolders": [{"uri":root_uri,"name":"workspace"}],
                    "capabilities": {
                        "workspace": {"workspaceFolders": true},
                        "textDocument": {"synchronization":{"dynamicRegistration":false}}
                    }
                }),
                LSP_STARTUP_TIMEOUT,
            ),
        )
        .await
        .map_err(|_| LspError::Timeout)??;
        let capabilities = ServerCapabilities::from_initialize(&initialize);
        Arc::get_mut(&mut session)
            .ok_or(LspError::Internal)?
            .capabilities = capabilities;
        session
            .notify("initialized", json!({}))
            .await
            .map_err(|_| LspError::StartupFailed)?;
        Ok(session)
    }

    pub fn identity(&self) -> &WorkspaceIdentity {
        &self.identity
    }

    pub fn language(&self) -> &str {
        &self.language
    }

    pub fn capabilities(&self) -> &ServerCapabilities {
        &self.capabilities
    }

    pub fn is_faulted(&self) -> bool {
        self.state.faulted.load(Ordering::SeqCst)
    }

    pub(super) async fn idle_for(&self) -> std::time::Duration {
        self.last_used.read().await.elapsed()
    }

    pub async fn request(&self, method: &str, params: Value) -> Result<Value, LspError> {
        self.request_with_timeout(method, params, LSP_REQUEST_TIMEOUT)
            .await
    }

    pub async fn request_with_timeout_ms(
        &self,
        method: &str,
        params: Value,
        timeout_ms: u64,
    ) -> Result<Value, LspError> {
        let requested = std::time::Duration::from_millis(timeout_ms.max(1));
        self.request_with_timeout(method, params, requested.min(LSP_REQUEST_TIMEOUT))
            .await
    }

    async fn request_with_timeout(
        &self,
        method: &str,
        params: Value,
        request_timeout: std::time::Duration,
    ) -> Result<Value, LspError> {
        if self.is_faulted() {
            return Err(LspError::Crashed);
        }
        *self.last_used.write().await = Instant::now();
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (sender, receiver) = oneshot::channel();
        {
            let mut pending = self.state.pending.lock().await;
            if pending.len() >= MAX_LSP_PENDING_REQUESTS {
                return Err(LspError::CapacityReached);
            }
            pending.insert(id, sender);
        }
        let value = json!({"jsonrpc":"2.0","id":id,"method":method,"params":params});
        if let Err(error) = write_message(&mut *self.state.writer.lock().await, &value).await {
            self.state.pending.lock().await.remove(&id);
            mark_faulted(&self.state, error).await;
            return Err(error);
        }
        match timeout(request_timeout, receiver).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(LspError::Crashed),
            Err(_) => {
                self.state.pending.lock().await.remove(&id);
                Err(LspError::Timeout)
            }
        }
    }

    pub async fn notify(&self, method: &str, params: Value) -> Result<(), LspError> {
        if self.is_faulted() {
            return Err(LspError::Crashed);
        }
        *self.last_used.write().await = Instant::now();
        let value = json!({"jsonrpc":"2.0","method":method,"params":params});
        write_message(&mut *self.state.writer.lock().await, &value).await
    }

    pub async fn sync_document(&self, path: &str) -> Result<u64, LspError> {
        super::document::sync_document(self, path).await
    }

    pub async fn shutdown(&self) {
        if !self.is_faulted() {
            let _ = self
                .request_with_timeout("shutdown", Value::Null, LSP_SHUTDOWN_TIMEOUT)
                .await;
            let _ = self.notify("exit", Value::Null).await;
        }
        let mut child = self.child.lock().await;
        match timeout(LSP_SHUTDOWN_TIMEOUT, child.wait()).await {
            Ok(_) => {}
            Err(_) => sandbox_kill(&mut child).await,
        }
        mark_faulted(&self.state, LspError::Crashed).await;
    }

    pub async fn retained_stderr_bytes(&self) -> usize {
        self.stderr.lock().await.len()
    }

    pub(super) fn documents(&self) -> &Mutex<super::document::DocumentStore> {
        &self.documents
    }
}

async fn read_loop(mut stdout: tokio::process::ChildStdout, state: Arc<SharedState>) {
    loop {
        let message = match read_message(&mut stdout).await {
            Ok(value) => value,
            Err(error) => {
                mark_faulted(&state, error).await;
                return;
            }
        };
        let Some(object) = message.as_object() else {
            mark_faulted(&state, LspError::MalformedResponse).await;
            return;
        };
        if object.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
            mark_faulted(&state, LspError::MalformedResponse).await;
            return;
        }
        if let Some(method) = object.get("method") {
            if method.as_str().is_none() {
                mark_faulted(&state, LspError::MalformedResponse).await;
                return;
            }
            if let Some(id) = object.get("id") {
                if !id.is_i64() && !id.is_u64() && !id.is_string() {
                    mark_faulted(&state, LspError::MalformedResponse).await;
                    return;
                }
                let response = json!({
                    "jsonrpc":"2.0",
                    "id":id,
                    "error":{"code":-32601,"message":"Method not found"}
                });
                if write_message(&mut *state.writer.lock().await, &response)
                    .await
                    .is_err()
                {
                    mark_faulted(&state, LspError::Crashed).await;
                    return;
                }
            }
            continue;
        }
        let Some(id) = object.get("id").and_then(Value::as_i64) else {
            mark_faulted(&state, LspError::MalformedResponse).await;
            return;
        };
        if object.contains_key("result") == object.contains_key("error") {
            mark_faulted(&state, LspError::MalformedResponse).await;
            return;
        }
        let sender = state.pending.lock().await.remove(&id);
        let Some(sender) = sender else {
            mark_faulted(&state, LspError::MalformedResponse).await;
            return;
        };
        let result = if let Some(value) = object.get("result") {
            Ok(value.clone())
        } else if object.contains_key("error") {
            Err(LspError::Internal)
        } else {
            Err(LspError::MalformedResponse)
        };
        let _ = sender.send(result);
    }
}

async fn mark_faulted(state: &Arc<SharedState>, error: LspError) {
    state.faulted.store(true, Ordering::SeqCst);
    let mut pending = state.pending.lock().await;
    for (_, sender) in pending.drain() {
        let _ = sender.send(Err(error));
    }
}

async fn drain_stderr(mut pipe: tokio::process::ChildStderr, buffer: Arc<Mutex<Vec<u8>>>) {
    let mut chunk = [0u8; 4096];
    loop {
        match pipe.read(&mut chunk).await {
            Ok(0) | Err(_) => return,
            Ok(count) => {
                let mut output = buffer.lock().await;
                output.extend_from_slice(&chunk[..count]);
                if output.len() > MAX_LSP_STDERR_BYTES {
                    let excess = output.len() - MAX_LSP_STDERR_BYTES;
                    output.drain(..excess);
                }
            }
        }
    }
}

async fn sandbox_kill(child: &mut Child) {
    if let Some(pid) = child.id() {
        #[cfg(unix)]
        unsafe {
            libc::kill(-(pid as i32), libc::SIGKILL);
        }
    }
    let _ = child.wait().await;
}
