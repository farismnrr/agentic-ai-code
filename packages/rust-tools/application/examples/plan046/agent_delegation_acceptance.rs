use relay_application::execution::agent::{
    classify_failure, fallback_allowed, provider_argv, AgentProvider, FailureClass,
};
use relay_core::config::{ServerConfig, ToolProfile};
use relay_interfaces::mcp::{find_tool_for_profile, tool_catalog};

fn main() {
    let full = tool_catalog();
    assert!(find_tool_for_profile("agent_delegate", ToolProfile::Full).is_some());
    assert!(find_tool_for_profile("agent_delegate", ToolProfile::Primary).is_none());
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
        vec!["exec", "--full-auto", "fix it"]
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
                "--yolo" | "--dangerously-skip-permissions" | "--no-sandbox"
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
    assert!(!ServerConfig::default().allow_agent_network);
    assert_eq!(ServerConfig::default().tool_profile, ToolProfile::Full);
    println!("plan046 agent delegation acceptance: PASS");
}
