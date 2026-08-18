//! Plan 039E deterministic acceptance for trust, direct argv, bounded hooks,
//! policy-subordinate decisions, and lifecycle ordering.

use relay_application::hooks::{
    HookClass, HookConfig, HookDecision, HookEvent, HookHandler, HookManager,
};
use relay_core::config::ServerConfig;
use serde_json::json;
use std::fs;
use std::path::Path;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("PLAN039E_HOOKS_ACCEPTANCE_FAIL: {error}");
        std::process::exit(1);
    }
    println!("PLAN039E_HOOKS_ACCEPTANCE_PASS");
}

async fn run() -> Result<(), String> {
    let root = std::env::current_dir()
        .map_err(io_error)?
        .join(format!("target/plan039e-hooks-{}", std::process::id()));
    fs::create_dir_all(root.join(".agents")).map_err(io_error)?;
    fs::create_dir_all(root.join(".git")).map_err(io_error)?;

    let nested_parent = root.join("owner");
    let nested = nested_parent.join("project");
    fs::create_dir_all(nested.join(".agents")).map_err(io_error)?;
    fs::create_dir_all(nested.join(".git")).map_err(io_error)?;
    let nested_identity = identity(&nested)?;
    let nested_valid = HookConfig {
        repository_identity: nested_identity.clone(),
        handlers: vec![HookHandler {
            event: HookEvent::SessionStart,
            command: vec!["true".into()],
            class: HookClass::Cosmetic,
            tool: None,
            effect_class: None,
            timeout_ms: 100,
        }],
    };
    fs::write(
        nested.join(".agents/hooks.json"),
        serde_json::to_vec(&nested_valid).unwrap(),
    )
    .map_err(io_error)?;
    let nested_config = ServerConfig {
        execution_root: Some(nested_parent.to_string_lossy().into_owned()),
        dir: Some(nested.to_string_lossy().into_owned()),
        enable_agent_hooks: true,
        ..ServerConfig::default()
    };
    let nested_manager = HookManager::load(Arc::new(nested_config)).map_err(|e| e.to_string())?;
    require(nested_manager.repository_identity().as_deref() == Some(nested_identity.as_str()), &format!("nested repository identity uses working repository: expected {nested_identity}, got {:?}", nested_manager.repository_identity()))?;

    fs::write(root.join(".agents/hooks.json"), b"not-json").map_err(io_error)?;
    let disabled = config(&root, false);
    require(
        HookManager::load(Arc::new(disabled)).is_ok(),
        "unknown hook config is inert by default",
    )?;

    let enabled = config(&root, true);
    require(
        HookManager::load(Arc::new(enabled.clone())).is_err(),
        "malformed enabled config fails closed",
    )?;

    let identity = identity(&root)?;
    let valid = HookConfig {
        repository_identity: identity.clone(),
        handlers: vec![HookHandler {
            event: HookEvent::PreToolUse,
            command: vec!["true".into()],
            class: HookClass::Security,
            tool: Some("file_write".into()),
            effect_class: Some("workspace_write".into()),
            timeout_ms: 1_000,
        }],
    };
    fs::write(
        root.join(".agents/hooks.json"),
        serde_json::to_vec(&valid).unwrap(),
    )
    .map_err(io_error)?;
    let manager = HookManager::load(Arc::new(enabled.clone())).map_err(|e| e.to_string())?;
    let result = manager.invoke(HookEvent::PreToolUse, json!({
        "tool_id": "file_write", "effect_classes": ["workspace_write"], "raw_output": "secret", "content": "source"
    })).await;
    require(
        result.decision == HookDecision::Continue,
        "valid direct argv hook continues",
    )?;
    let approval_token = manager.issue_approval("agent-a", "file_write").await;
    require(
        manager
            .consume_approval(&approval_token, "agent-a", "file_write")
            .await
            && !manager
                .consume_approval(&approval_token, "agent-a", "file_write")
                .await,
        "hook approval is one-use and scoped to the agent/tool",
    )?;

    let canary = root.join("HOOK_CANARY");
    let read_only = HookConfig {
        repository_identity: identity.clone(),
        handlers: vec![HookHandler {
            event: HookEvent::PreToolUse,
            command: vec!["touch".into(), canary.to_string_lossy().into_owned()],
            class: HookClass::Security,
            tool: Some("directory_list".into()),
            effect_class: Some("workspace_read".into()),
            timeout_ms: 1_000,
        }],
    };
    fs::write(
        root.join(".agents/hooks.json"),
        serde_json::to_vec(&read_only).unwrap(),
    )
    .map_err(io_error)?;
    let read_only_manager =
        HookManager::load(Arc::new(enabled.clone())).map_err(|e| e.to_string())?;
    let read_only_result = read_only_manager
        .invoke(
            HookEvent::PreToolUse,
            json!({
                "tool_id": "directory_list", "effect_classes": ["workspace_read"]
            }),
        )
        .await;
    require(
        read_only_result.decision == HookDecision::Block && !canary.exists(),
        "read-only tool cannot mutate through hook",
    )?;

    fs::write(
        root.join(".agents/hooks.json"),
        serde_json::to_vec(&valid).unwrap(),
    )
    .map_err(io_error)?;
    let session_manager =
        HookManager::load(Arc::new(enabled.clone())).map_err(|e| e.to_string())?;
    session_manager.start_session("agent-a", &identity).await;
    session_manager.start_session("agent-a", &identity).await;
    session_manager.start_session("agent-b", &identity).await;
    require(
        session_manager.started_session_count().await == 2,
        "session start is per agent session",
    )?;

    let context_handler = HookConfig {
        repository_identity: identity.clone(),
        handlers: vec![HookHandler {
            event: HookEvent::SessionStart,
            command: vec![
                "echo".into(),
                r#"{"context":{"repository_identity":"bounded-context"}}"#.into(),
            ],
            class: HookClass::Cosmetic,
            tool: None,
            effect_class: None,
            timeout_ms: 1_000,
        }],
    };
    fs::write(
        root.join(".agents/hooks.json"),
        serde_json::to_vec(&context_handler).unwrap(),
    )
    .map_err(io_error)?;
    let context_manager =
        HookManager::load(Arc::new(enabled.clone())).map_err(|e| e.to_string())?;
    let context = context_manager
        .start_session("agent-context", &identity)
        .await;
    require(
        context == Some(json!({"repository_identity": "bounded-context"})),
        "session-start returns bounded structured context",
    )?;

    let traversal = ServerConfig {
        agent_hooks_config: Some(".agents/../hooks.json".into()),
        ..enabled.clone()
    };
    require(
        HookManager::load(Arc::new(traversal)).is_err(),
        "lexical hook traversal is rejected",
    )?;
    let outside = root.join("outside-hooks.json");
    fs::write(&outside, serde_json::to_vec(&valid).unwrap()).map_err(io_error)?;
    std::os::unix::fs::symlink(&outside, root.join(".agents/link.json")).map_err(io_error)?;
    let symlink = ServerConfig {
        agent_hooks_config: Some(".agents/link.json".into()),
        ..enabled.clone()
    };
    require(
        HookManager::load(Arc::new(symlink)).is_err(),
        "symlink hook escape is rejected",
    )?;

    let shell = HookConfig {
        repository_identity: identity.clone(),
        handlers: vec![HookHandler {
            event: HookEvent::PreToolUse,
            command: vec!["sh".into(), "-c".into(), "echo bypass".into()],
            class: HookClass::Security,
            tool: None,
            effect_class: None,
            timeout_ms: 0,
        }],
    };
    fs::write(
        root.join(".agents/hooks.json"),
        serde_json::to_vec(&shell).unwrap(),
    )
    .map_err(io_error)?;
    require(
        HookManager::load(Arc::new(enabled.clone())).is_err(),
        "shell indirection is rejected",
    )?;

    let mismatch = HookConfig {
        repository_identity: "wrong-repository".into(),
        handlers: valid.handlers.clone(),
    };
    fs::write(
        root.join(".agents/hooks.json"),
        serde_json::to_vec(&mismatch).unwrap(),
    )
    .map_err(io_error)?;
    require(
        HookManager::load(Arc::new(enabled.clone())).is_err(),
        "identity mismatch fails closed",
    )?;

    let cosmetic_failure = HookConfig {
        repository_identity: identity.clone(),
        handlers: vec![HookHandler {
            event: HookEvent::PreToolUse,
            command: vec!["false".into()],
            class: HookClass::Cosmetic,
            tool: None,
            effect_class: None,
            timeout_ms: 100,
        }],
    };
    fs::write(
        root.join(".agents/hooks.json"),
        serde_json::to_vec(&cosmetic_failure).unwrap(),
    )
    .map_err(io_error)?;
    let manager = HookManager::load(Arc::new(enabled)).map_err(|e| e.to_string())?;
    let continued = manager
        .invoke(
            HookEvent::PreToolUse,
            json!({"tool_id":"terminal_exec","effect_classes":["process_exec","workspace_write"]}),
        )
        .await;
    require(
        continued.decision == HookDecision::Continue,
        "cosmetic hook failure is explicitly fail-open",
    )?;

    let security_failure = HookConfig {
        repository_identity: identity,
        handlers: vec![HookHandler {
            event: HookEvent::PreToolUse,
            command: vec!["false".into()],
            class: HookClass::Security,
            tool: None,
            effect_class: None,
            timeout_ms: 100,
        }],
    };
    fs::write(
        root.join(".agents/hooks.json"),
        serde_json::to_vec(&security_failure).unwrap(),
    )
    .map_err(io_error)?;
    let manager = HookManager::load(Arc::new(config(&root, true))).map_err(|e| e.to_string())?;
    let blocked = manager
        .invoke(
            HookEvent::PreToolUse,
            json!({"tool_id":"terminal_exec","effect_classes":["process_exec","workspace_write"]}),
        )
        .await;
    require(
        blocked.decision == HookDecision::Block,
        "security hook failure blocks without granting authority",
    )?;
    let stop_config = HookConfig {
        repository_identity: security_failure.repository_identity.clone(),
        handlers: vec![HookHandler {
            event: HookEvent::PreAgentStop,
            command: vec!["false".into()],
            class: HookClass::Security,
            tool: None,
            effect_class: None,
            timeout_ms: 100,
        }],
    };
    fs::write(
        root.join(".agents/hooks.json"),
        serde_json::to_vec(&stop_config).unwrap(),
    )
    .map_err(io_error)?;
    let stop_manager =
        HookManager::load(Arc::new(config(&root, true))).map_err(|e| e.to_string())?;
    require(
        !stop_manager.pre_agent_stop("agent-stop").await,
        "first stop boundary blocks for remediation",
    )?;
    require(
        stop_manager.pre_agent_stop("agent-stop").await,
        "second stop boundary forces completion after one remediation turn",
    )?;

    fs::remove_dir_all(&root).map_err(io_error)?;
    Ok(())
}

fn config(root: &Path, enabled: bool) -> ServerConfig {
    ServerConfig {
        execution_root: Some(root.to_string_lossy().into_owned()),
        dir: Some(root.to_string_lossy().into_owned()),
        enable_agent_hooks: enabled,
        ..ServerConfig::default()
    }
}

fn identity(root: &Path) -> Result<String, String> {
    let root = fs::canonicalize(root).map_err(io_error)?;
    let git = fs::canonicalize(root.join(".git")).map_err(io_error)?;
    Ok(format!("{}|{}", root.display(), git.display()))
}

fn require(condition: bool, message: &str) -> Result<(), String> {
    condition.then_some(()).ok_or_else(|| message.into())
}

fn io_error(error: std::io::Error) -> String {
    error.to_string()
}
