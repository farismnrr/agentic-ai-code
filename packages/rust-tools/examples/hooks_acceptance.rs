//! Plan 039E deterministic acceptance for trust, direct argv, bounded hooks,
//! policy-subordinate decisions, and lifecycle ordering.

use ai_tools::application::hooks::{
    HookClass, HookConfig, HookDecision, HookEvent, HookHandler, HookManager,
};
use ai_tools::core::config::ServerConfig;
use serde_json::json;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

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
    let approval_payload = json!({"tool_id":"file_write","effect_classes":["workspace_write"],"arguments":{"path":"a"}});
    let approval_token = manager
        .issue_approval("agent-a", "file_write", &approval_payload, 1)
        .await
        .ok_or_else(|| "approval token issued".to_string())?;
    require(
        manager
            .consume_approval(&approval_token, "agent-a", "file_write", &approval_payload)
            .await
            == Some(1)
            && manager
                .consume_approval(&approval_token, "agent-a", "file_write", &approval_payload)
                .await
                .is_none(),
        "hook approval is one-use and scoped to the exact invocation",
    )?;

    let chain = HookConfig {
        repository_identity: identity.clone(),
        handlers: vec![
            HookHandler {
                event: HookEvent::PreToolUse,
                command: vec!["perl".into(), "-e".into(), "exit 11".into()],
                class: HookClass::Security,
                tool: Some("terminal_exec".into()),
                effect_class: None,
                timeout_ms: 1_000,
            },
            HookHandler {
                event: HookEvent::PreToolUse,
                command: vec!["perl".into(), "-e".into(), "exit 10".into()],
                class: HookClass::Security,
                tool: Some("terminal_exec".into()),
                effect_class: None,
                timeout_ms: 1_000,
            },
        ],
    };
    fs::write(
        root.join(".agents/hooks.json"),
        serde_json::to_vec(&chain).unwrap(),
    )
    .map_err(io_error)?;
    let chain_manager = HookManager::load(Arc::new(enabled.clone())).map_err(|e| e.to_string())?;
    let chain_payload = json!({"tool_id":"terminal_exec","effect_classes":["process_exec"],"arguments":{"command":"true","args":[]}});
    let requested = chain_manager
        .invoke(HookEvent::PreToolUse, chain_payload.clone())
        .await;
    require(
        requested.decision == HookDecision::RequestApproval
            && requested.approval_checkpoint == Some(1),
        "approval records an explicit resume checkpoint",
    )?;
    let resumed = chain_manager
        .invoke_from(
            HookEvent::PreToolUse,
            chain_payload.clone(),
            requested.approval_checkpoint.unwrap(),
        )
        .await;
    require(
        resumed.decision == HookDecision::Block,
        "later blocking hook wins after approval",
    )?;

    let chain_continue = HookConfig {
        repository_identity: identity.clone(),
        handlers: vec![
            HookHandler {
                event: HookEvent::PreToolUse,
                command: vec!["perl".into(), "-e".into(), "exit 0".into()],
                class: HookClass::Security,
                tool: Some("terminal_exec".into()),
                effect_class: None,
                timeout_ms: 1_000,
            },
            HookHandler {
                event: HookEvent::PreToolUse,
                command: vec!["perl".into(), "-e".into(), "exit 11".into()],
                class: HookClass::Security,
                tool: Some("terminal_exec".into()),
                effect_class: None,
                timeout_ms: 1_000,
            },
            HookHandler {
                event: HookEvent::PreToolUse,
                command: vec!["perl".into(), "-e".into(), "exit 0".into()],
                class: HookClass::Security,
                tool: Some("terminal_exec".into()),
                effect_class: None,
                timeout_ms: 1_000,
            },
        ],
    };
    fs::write(
        root.join(".agents/hooks.json"),
        serde_json::to_vec(&chain_continue).unwrap(),
    )
    .map_err(io_error)?;
    let chain_continue_manager =
        HookManager::load(Arc::new(enabled.clone())).map_err(|e| e.to_string())?;
    let first = chain_continue_manager
        .invoke(HookEvent::PreToolUse, chain_payload.clone())
        .await;
    require(
        first.decision == HookDecision::RequestApproval && first.approval_checkpoint == Some(2),
        "approval checkpoint excludes completed hooks",
    )?;
    let final_result = chain_continue_manager
        .invoke_from(
            HookEvent::PreToolUse,
            chain_payload.clone(),
            first.approval_checkpoint.unwrap(),
        )
        .await;
    require(
        final_result.decision == HookDecision::Continue,
        "approved continuation executes remaining hooks exactly once",
    )?;

    for (session, tool, payload) in [
        ("agent-b", "terminal_exec", chain_payload.clone()),
        ("agent-a", "file_write", chain_payload.clone()),
        (
            "agent-a",
            "terminal_exec",
            json!({"tool_id":"terminal_exec","arguments":{"command":"touch","args":["changed"]}}),
        ),
    ] {
        let token = chain_continue_manager
            .issue_approval(session, tool, &payload, 1)
            .await
            .ok_or_else(|| "approval capacity unexpectedly exhausted".to_string())?;
        require(
            chain_continue_manager
                .consume_approval(&token, "agent-a", "terminal_exec", &chain_payload)
                .await
                .is_none(),
            "approval rejects wrong identity and invocation",
        )?;
    }

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

    for index in 0..256 {
        let _ = session_manager
            .start_session(&format!("capacity-{index}"), &identity)
            .await;
    }
    require(
        matches!(
            session_manager
                .start_session("capacity-overflow", &identity)
                .await,
            ai_tools::application::hooks::SessionStartOutcome::CapacityExhausted
        ),
        "session capacity fails closed instead of skipping lifecycle",
    )?;
    require(
        matches!(
            session_manager.start_session("capacity-0", &identity).await,
            ai_tools::application::hooks::SessionStartOutcome::AlreadyStarted
        ),
        "active stable session remains exactly once",
    )?;

    let security_start = HookConfig {
        repository_identity: identity.clone(),
        handlers: vec![HookHandler {
            event: HookEvent::SessionStart,
            command: vec!["false".into()],
            class: HookClass::Security,
            tool: None,
            effect_class: None,
            timeout_ms: 100,
        }],
    };
    fs::write(
        root.join(".agents/hooks.json"),
        serde_json::to_vec(&security_start).unwrap(),
    )
    .map_err(io_error)?;
    let security_start_manager =
        HookManager::load(Arc::new(config(&root, true))).map_err(|e| e.to_string())?;
    require(
        matches!(
            security_start_manager
                .start_session("failed-start", &identity)
                .await,
            ai_tools::application::hooks::SessionStartOutcome::SecurityFailure
        ),
        "security session-start failure fails closed",
    )?;

    let slow_failure = HookConfig {
        repository_identity: identity.clone(),
        handlers: vec![HookHandler {
            event: HookEvent::SessionStart,
            command: vec![
                "perl".into(),
                "-e".into(),
                "select undef, undef, undef, 0.2; exit 1".into(),
            ],
            class: HookClass::Security,
            tool: None,
            effect_class: None,
            timeout_ms: 1_000,
        }],
    };
    fs::write(
        root.join(".agents/hooks.json"),
        serde_json::to_vec(&slow_failure).unwrap(),
    )
    .map_err(io_error)?;
    let slow_failure_manager =
        HookManager::load(Arc::new(config(&root, true))).map_err(|e| e.to_string())?;
    let first_started = Instant::now();
    let (first_failure, second_failure) = tokio::join!(
        slow_failure_manager.start_session("race-failure", &identity),
        slow_failure_manager.start_session("race-failure", &identity),
    );
    require(
        first_failure == ai_tools::application::hooks::SessionStartOutcome::SecurityFailure
            && second_failure == ai_tools::application::hooks::SessionStartOutcome::SecurityFailure
            && slow_failure_manager.started_session_count().await == 0
            && slow_failure_manager.session_start_invocation_count() == 1
            && first_started.elapsed() >= Duration::from_millis(150),
        "concurrent slow security failure is shared and fails closed",
    )?;
    require(
        slow_failure_manager
            .start_session("race-failure", &identity)
            .await
            == ai_tools::application::hooks::SessionStartOutcome::SecurityFailure
            && slow_failure_manager.session_start_invocation_count() == 2,
        "failed session initialization is cleaned up for retry",
    )?;

    let slow_success = HookConfig {
        repository_identity: identity.clone(),
        handlers: vec![HookHandler {
            event: HookEvent::SessionStart,
            command: vec![
                "perl".into(),
                "-e".into(),
                "select undef, undef, undef, 0.2; exit 0".into(),
            ],
            class: HookClass::Security,
            tool: None,
            effect_class: None,
            timeout_ms: 1_000,
        }],
    };
    fs::write(
        root.join(".agents/hooks.json"),
        serde_json::to_vec(&slow_success).unwrap(),
    )
    .map_err(io_error)?;
    let slow_success_manager =
        HookManager::load(Arc::new(config(&root, true))).map_err(|e| e.to_string())?;
    let first_started = Instant::now();
    let (first_success, second_success) = tokio::join!(
        slow_success_manager.start_session("race-success", &identity),
        slow_success_manager.start_session("race-success", &identity),
    );
    require(
        matches!(
            first_success,
            ai_tools::application::hooks::SessionStartOutcome::Started { .. }
        ) && matches!(
            second_success,
            ai_tools::application::hooks::SessionStartOutcome::Started { .. }
        ) && slow_success_manager.started_session_count().await == 1
            && slow_success_manager.session_start_invocation_count() == 1
            && first_started.elapsed() >= Duration::from_millis(150),
        "concurrent slow security success is shared and completes after initialization",
    )?;
    require(
        slow_success_manager
            .start_session("race-success", &identity)
            .await
            == ai_tools::application::hooks::SessionStartOutcome::AlreadyStarted,
        "successful shared initialization remains started exactly once",
    )?;
    let cosmetic_start = HookConfig {
        repository_identity: identity.clone(),
        handlers: vec![HookHandler {
            event: HookEvent::SessionStart,
            command: vec!["false".into()],
            class: HookClass::Cosmetic,
            tool: None,
            effect_class: None,
            timeout_ms: 100,
        }],
    };
    fs::write(
        root.join(".agents/hooks.json"),
        serde_json::to_vec(&cosmetic_start).unwrap(),
    )
    .map_err(io_error)?;
    let cosmetic_start_manager =
        HookManager::load(Arc::new(config(&root, true))).map_err(|e| e.to_string())?;
    require(
        matches!(
            cosmetic_start_manager
                .start_session("cosmetic-start", &identity)
                .await,
            ai_tools::application::hooks::SessionStartOutcome::Started { context: None }
        ),
        "cosmetic session-start failure follows fail-open classification",
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
    let context = match context_manager
        .start_session("agent-context", &identity)
        .await
    {
        ai_tools::application::hooks::SessionStartOutcome::Started { context } => context,
        _ => None,
    };
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
    require(
        !stop_manager.pre_agent_stop("agent-stop").await
            && stop_manager.pre_agent_stop("agent-stop").await,
        "a later completion cycle receives a fresh remediation budget",
    )?;

    require(
        !stop_manager.pre_agent_stop("victim").await,
        "victim first stop boundary requires remediation",
    )?;
    for index in 0..256 {
        let _ = stop_manager
            .pre_agent_stop(&format!("unrelated-{index}"))
            .await;
    }
    require(
        stop_manager.pre_agent_stop("victim").await,
        "stop-state capacity churn cannot reset the victim budget",
    )?;
    for cycle in 0..4 {
        let _ = stop_manager
            .pre_agent_stop(&format!("unrelated-followup-{cycle}"))
            .await;
        require(
            stop_manager.pre_agent_stop("victim").await,
            "a saturated stop-state map forces untracked sessions through bounded completion",
        )?;
        for index in 0..256 {
            let _ = stop_manager
                .pre_agent_stop(&format!("unrelated-repeat-{cycle}-{index}"))
                .await;
        }
        require(
            stop_manager.pre_agent_stop("victim").await,
            "repeated unrelated churn cannot reset the victim into an unbounded loop",
        )?;
    }

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
