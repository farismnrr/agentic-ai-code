use relay_application::execution::agent::{
    classify_failure, fallback_allowed, provider_argv, AgentProvider, FailureClass,
};
use relay_application::execution::{start_tool_task, JobManager, JobSnapshot, JobState};
use relay_core::config::{ServerConfig, ToolProfile};
use relay_interfaces::mcp::{
    find_tool_for_profile, find_tool_for_profile_and_agent_providers, tool_catalog,
};
use serde_json::{json, Value};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

fn write_executable(path: &Path, content: &str) {
    fs::write(path, content).unwrap();
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
}

fn result_json(snapshot: &JobSnapshot) -> Value {
    let result = snapshot
        .result
        .as_ref()
        .expect("job must retain a tool result");
    let text = result
        .content
        .first()
        .expect("agent result must contain text")
        .text
        .as_str();
    serde_json::from_str(text).expect("agent result text must be JSON")
}

async fn run_agent(
    config: &ServerConfig,
    cwd: &Path,
    providers: &[&str],
    prompt: &str,
    timeout_ms: u64,
) -> JobSnapshot {
    let tool = find_tool_for_profile_and_agent_providers(
        "agent_delegate",
        ToolProfile::Primary,
        providers,
    )
    .expect("authenticated delegation must be available in Primary");
    let manager = JobManager::new(config.clone());
    let task_id = start_tool_task(
        &tool,
        &json!({
            "prompt": prompt,
            "providers": providers,
            "cwd": cwd,
            "timeout_ms": timeout_ms,
            "fallback": true
        }),
        config,
        &manager,
    )
    .await
    .expect("agent task must start");
    manager
        .wait(&task_id)
        .await
        .expect("agent task must finish")
}

fn runtime_fixture() -> (PathBuf, PathBuf, ServerConfig) {
    let repo = std::env::current_dir().unwrap();
    let root = repo
        .join("target")
        .join(format!("plan046-agent-{}", uuid::Uuid::new_v4()));
    let workspace = root.join("workspace");
    let sibling = root.join("sibling");
    let bin = workspace.join("bin");
    let codex_auth = workspace.join("auth-codex");
    let claude_auth = workspace.join("auth-claude");
    for path in [&workspace, &sibling, &bin, &codex_auth, &claude_auth] {
        fs::create_dir_all(path).unwrap();
    }
    fs::write(workspace.join(".gitignore"), "ignored.txt\n").unwrap();
    fs::write(sibling.join("secret.txt"), "sibling-only\n").unwrap();
    assert!(Command::new("git")
        .args(["init", "-q"])
        .current_dir(&workspace)
        .status()
        .unwrap()
        .success());

    let sibling_secret = sibling.join("secret.txt");
    write_executable(
        &bin.join("codex"),
        &format!(
            r#"#!/bin/sh
if [ "${{1:-}}" = "login" ] && [ "${{2:-}}" = "status" ]; then
  exit 0
fi
mode=""
for arg in "$@"; do
  case "$arg" in
    fallback-test|mutation-test|timeout-test|network-test|sibling-test) mode="$arg" ;;
  esac
done
case "$mode" in
  fallback-test)
    echo 'rate limit exceeded' >&2
    exit 1
    ;;
  mutation-test)
    printf 'changed\n' > ignored.txt
    echo 'rate limit exceeded' >&2
    exit 1
    ;;
  timeout-test)
    printf 'timeout-start\n'
    sleep 2
    exit 0
    ;;
  network-test)
    if awk -F: 'NR > 2 {{ gsub(/ /, "", $1); if ($1 != "lo") found=1 }} END {{ exit !found }}' /proc/net/dev; then
      : > network-visible
    fi
    printf 'network-checked\n'
    exit 0
    ;;
  sibling-test)
    if [ -e '{}' ]; then
      : > sibling-visible
    fi
    printf 'sibling-checked\n'
    exit 0
    ;;
  *)
    echo 'unsupported fake codex invocation' >&2
    exit 2
    ;;
esac
"#,
            sibling_secret.display()
        ),
    );
    write_executable(
        &bin.join("claude"),
        r#"#!/bin/sh
if [ "${1:-}" = "auth" ] && [ "${2:-}" = "status" ]; then
  exit 0
fi
mode=""
for arg in "$@"; do
  case "$arg" in
    fallback-test|mutation-test) mode="$arg" ;;
  esac
done
case "$mode" in
  fallback-test)
    : > fallback-ran
    printf 'fallback-complete\n'
    exit 0
    ;;
  mutation-test)
    : > mutation-fallback-ran
    printf 'unsafe-fallback-ran\n'
    exit 0
    ;;
  *)
    echo 'unsupported fake claude invocation' >&2
    exit 2
    ;;
esac
"#,
    );

    let config = ServerConfig {
        dir: Some(root.to_string_lossy().into_owned()),
        execution_root: Some(root.to_string_lossy().into_owned()),
        tool_profile: ToolProfile::Primary,
        toolchain_paths: vec![bin.to_string_lossy().into_owned()],
        agent_auth_roots: vec![
            format!("codex={}", codex_auth.display()),
            format!("claude={}", claude_auth.display()),
        ],
        ..ServerConfig::default()
    };
    (root, workspace, config)
}

async fn runtime_acceptance() {
    let (root, workspace, config) = runtime_fixture();

    let fallback = run_agent(
        &config,
        &workspace,
        &["codex", "claude"],
        "fallback-test",
        5_000,
    )
    .await;
    assert_eq!(fallback.state, JobState::Completed);
    assert_eq!(fallback.exit_code, Some(0));
    assert!(workspace.join("fallback-ran").is_file());
    assert_eq!(result_json(&fallback)["fallback_used"], true);

    let mutation = run_agent(
        &config,
        &workspace,
        &["codex", "claude"],
        "mutation-test",
        5_000,
    )
    .await;
    assert_eq!(mutation.state, JobState::Completed);
    assert_eq!(mutation.exit_code, Some(1));
    assert!(workspace.join("ignored.txt").is_file());
    assert!(!workspace.join("mutation-fallback-ran").exists());
    assert_eq!(result_json(&mutation)["workspace_changed"], true);

    let timeout = run_agent(&config, &workspace, &["codex"], "timeout-test", 100).await;
    assert_eq!(timeout.state, JobState::TimedOut);
    assert!(timeout.stdout.contains("timeout-start"));
    assert!(timeout
        .result
        .as_ref()
        .is_some_and(|result| result.is_error));

    let mut network_config = config.clone();
    network_config.allow_terminal_network = true;
    network_config.allow_agent_network = false;
    let network = run_agent(
        &network_config,
        &workspace,
        &["codex"],
        "network-test",
        5_000,
    )
    .await;
    assert_eq!(network.state, JobState::Completed);
    assert!(!workspace.join("network-visible").exists());

    let sibling = run_agent(&config, &workspace, &["codex"], "sibling-test", 5_000).await;
    assert_eq!(sibling.state, JobState::Completed);
    assert!(!workspace.join("sibling-visible").exists());

    fs::remove_dir_all(root).unwrap();
}

#[tokio::main]
async fn main() {
    let full = tool_catalog();
    assert!(find_tool_for_profile("agent_delegate", ToolProfile::Full).is_some());
    assert!(find_tool_for_profile("agent_delegate", ToolProfile::Primary).is_some());
    let tool = full
        .iter()
        .find(|tool| tool.name == "agent_delegate")
        .unwrap();
    assert_eq!(tool.execution.as_ref().unwrap()["taskSupport"], "optional");
    assert!(
        tool.input_schema["properties"]["providers"]["items"]["enum"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value == "codex")
    );
    assert!(serde_json::to_string(&tool.input_schema)
        .unwrap()
        .contains("fallback"));

    assert_eq!(
        provider_argv(AgentProvider::Codex, "fix it", 3),
        vec![
            "exec",
            "--approve-for-me",
            "--sandbox",
            "workspace-write",
            "--ephemeral",
            "fix it",
        ]
    );
    assert_eq!(
        provider_argv(AgentProvider::Antigravity, "fix it", 3),
        vec!["--print", "--mode=accept-edits", "fix it"]
    );
    let claude = provider_argv(AgentProvider::Claude, "fix it", 3);
    assert!(claude
        .windows(2)
        .any(|pair| pair[0] == "--permission-mode" && pair[1] == "acceptEdits"));
    for provider in [
        AgentProvider::Codex,
        AgentProvider::Antigravity,
        AgentProvider::Claude,
    ] {
        assert!(!provider_argv(provider, "fix it", 3).iter().any(|arg| {
            matches!(
                arg.as_str(),
                "--yolo"
                    | "--dangerously-skip-permissions"
                    | "--dangerously-bypass-approvals-and-sandbox"
                    | "--no-sandbox"
            )
        }));
    }

    assert_eq!(
        classify_failure(1, "", "rate limit exceeded"),
        Some(FailureClass::Quota)
    );
    assert_eq!(
        classify_failure(1, "", "login required"),
        Some(FailureClass::Auth)
    );
    assert_eq!(
        classify_failure(1, "", "command not found"),
        Some(FailureClass::Unavailable)
    );
    assert_eq!(
        classify_failure(1, "", "syntax error"),
        Some(FailureClass::Failed)
    );
    assert_eq!(classify_failure(0, "quota mentioned", ""), None);
    assert!(fallback_allowed(FailureClass::Quota, false));
    assert!(fallback_allowed(FailureClass::Auth, false));
    assert!(fallback_allowed(FailureClass::Unavailable, false));
    assert!(!fallback_allowed(FailureClass::Failed, false));
    assert!(!fallback_allowed(FailureClass::Quota, true));
    let defaults = ServerConfig::default();
    assert!(!defaults.allow_agent_network);
    assert!(!defaults.allow_terminal_network);
    assert_eq!(defaults.tool_profile, ToolProfile::Full);

    runtime_acceptance().await;
    println!("plan046 agent delegation acceptance: PASS");
}
