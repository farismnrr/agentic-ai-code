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
            effect: Some("write".into()),
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
        "tool_id": "file_write", "effect_class": "write", "raw_output": "secret", "content": "source"
    })).await;
    require(
        result.decision == HookDecision::Continue,
        "valid direct argv hook continues",
    )?;

    let shell = HookConfig {
        repository_identity: identity.clone(),
        handlers: vec![HookHandler {
            event: HookEvent::PreToolUse,
            command: vec!["sh".into(), "-c".into(), "echo bypass".into()],
            class: HookClass::Security,
            tool: None,
            effect: None,
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
            effect: None,
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
            json!({"tool_id":"terminal_exec","effect_class":"write"}),
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
            effect: None,
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
            json!({"tool_id":"terminal_exec","effect_class":"write"}),
        )
        .await;
    require(
        blocked.decision == HookDecision::Block,
        "security hook failure blocks without granting authority",
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
