use super::bounded_context;
use super::{
    HookDecision, HookEvent, HookHandler, HookManager, HookResult, APPROVAL_EXIT, BLOCK_EXIT,
    DEFAULT_TIMEOUT_MS, MAX_CONTEXT_BYTES, MAX_PAYLOAD_BYTES, MAX_TIMEOUT_MS,
};
use crate::execution::sandbox;
use serde_json::Value;
use std::time::Instant;
use tokio::io::{AsyncRead, AsyncWriteExt};
use tokio::time::{timeout, Duration};

pub(super) async fn run(
    manager: &HookManager,
    handler: &HookHandler,
    payload: &Value,
) -> HookResult {
    let started = Instant::now();
    let executable = match sandbox::resolve_safe_executable(&manager.config, &handler.command[0]) {
        Ok(path) => path,
        Err(_) => return failed(started, handler.event),
    };
    let writable = payload
        .get("effect_classes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .any(|effect| {
            matches!(
                effect,
                "workspace_write" | "workspace_delete" | "external_mutation"
            )
        });
    let mut child = match sandbox::spawn_hook(
        &manager.config,
        executable,
        handler.command[1..].to_vec(),
        manager.root.clone(),
        if writable {
            sandbox::WorkspaceAccess::Writable
        } else {
            sandbox::WorkspaceAccess::ReadOnly
        },
    ) {
        Ok(child) => child,
        Err(_) => return failed(started, handler.event),
    };
    let stdout_task = child
        .stdout
        .take()
        .map(|stream| tokio::spawn(drain_output(stream)));
    let stderr_task = child
        .stderr
        .take()
        .map(|stream| tokio::spawn(drain_output(stream)));
    let input = serde_json::to_vec(payload).unwrap_or_default();
    if input.len() > MAX_PAYLOAD_BYTES {
        return failed(started, handler.event);
    }
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(&input).await;
        let _ = stdin.shutdown().await;
    }
    let deadline = if handler.timeout_ms == 0 {
        DEFAULT_TIMEOUT_MS
    } else {
        handler.timeout_ms.clamp(1, MAX_TIMEOUT_MS)
    };
    let status = match timeout(Duration::from_millis(deadline), child.wait()).await {
        Ok(Ok(status)) => status,
        _ => {
            crate::execution::kill_process_group(&mut child).await;
            if let Some(task) = stdout_task {
                let _ = task.await;
            }
            if let Some(task) = stderr_task {
                let _ = task.await;
            }
            return failed(started, handler.event);
        }
    };
    let stdout = if let Some(task) = stdout_task {
        task.await.unwrap_or_default()
    } else {
        Vec::new()
    };
    if let Some(task) = stderr_task {
        let _ = task.await;
    }
    let decision = match status.code() {
        Some(0) => HookDecision::Continue,
        Some(BLOCK_EXIT) => HookDecision::Block,
        Some(APPROVAL_EXIT) => HookDecision::RequestApproval,
        _ => return failed(started, handler.event),
    };
    let context = (handler.event == HookEvent::SessionStart)
        .then(|| bounded_context(&stdout))
        .flatten();
    tracing::info!(event = "relay.hook", hook_event = handler.event.name(), decision = ?decision, duration_ms = started.elapsed().as_millis() as u64, reason = "handler_result");
    HookResult {
        decision,
        reason: if decision == HookDecision::Continue {
            "continued"
        } else {
            "handler_decision"
        },
        duration_ms: started.elapsed().as_millis() as u64,
        context,
        approval_checkpoint: None,
    }
}

fn failed(started: Instant, event: HookEvent) -> HookResult {
    tracing::info!(
        event = "relay.hook",
        hook_event = event.name(),
        decision = "failure",
        duration_ms = started.elapsed().as_millis() as u64,
        reason = "hook_failure"
    );
    HookResult {
        decision: HookDecision::Continue,
        reason: "hook_failure",
        duration_ms: started.elapsed().as_millis() as u64,
        context: None,
        approval_checkpoint: None,
    }
}

async fn drain_output<R: AsyncRead + Unpin>(mut stream: R) -> Vec<u8> {
    let mut retained = 0usize;
    let mut output = Vec::new();
    let mut buffer = [0u8; 4096];
    loop {
        match tokio::io::AsyncReadExt::read(&mut stream, &mut buffer).await {
            Ok(0) | Err(_) => return output,
            Ok(bytes) => {
                let remaining = MAX_CONTEXT_BYTES.saturating_sub(retained);
                output.extend_from_slice(&buffer[..bytes.min(remaining)]);
                retained = retained.saturating_add(bytes).min(MAX_CONTEXT_BYTES);
                if retained >= MAX_CONTEXT_BYTES {
                    return output;
                }
            }
        }
    }
}
