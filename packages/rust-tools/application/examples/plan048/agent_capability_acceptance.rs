use relay_application::execution::agent::{provider_argv, AgentProvider};
use relay_core::config::ToolProfile;
use relay_interfaces::mcp::{
    find_tool_for_profile_and_agent_providers, tool_catalog,
    tool_catalog_for_profile_and_agent_providers,
};

fn main() {
    let static_catalog = tool_catalog();
    let all = ["external-mcp", "agy", "external-mcp"];
    let none = tool_catalog_for_profile_and_agent_providers(ToolProfile::Full, &[]);
    assert!(
        find_tool_for_profile_and_agent_providers("agent_delegate", ToolProfile::Full, &[])
            .is_none()
    );
    assert_eq!(none.len() + 1, static_catalog.len());

    let external-mcp = tool_catalog_for_profile_and_agent_providers(ToolProfile::Full, &["external-mcp"]);
    let delegated = external-mcp
        .iter()
        .find(|tool| tool.name == "agent_delegate")
        .expect("authenticated provider must be advertised");
    assert_eq!(
        delegated.input_schema["properties"]["providers"]["items"]["enum"],
        serde_json::json!(["external-mcp"])
    );
    assert_eq!(
        delegated.input_schema["properties"]["providers"]["default"],
        serde_json::json!(["external-mcp"])
    );

    let all_tools = tool_catalog_for_profile_and_agent_providers(ToolProfile::Full, &all);
    assert_eq!(
        serde_json::to_value(&all_tools).unwrap(),
        serde_json::to_value(&static_catalog).unwrap()
    );
    let primary = tool_catalog_for_profile_and_agent_providers(ToolProfile::Primary, &["external-mcp"]);
    let primary_delegate = primary
        .iter()
        .find(|tool| tool.name == "agent_delegate")
        .expect("primary profile must expose authenticated delegation");
    assert_eq!(
        primary_delegate.input_schema["properties"]["providers"]["items"]["enum"],
        serde_json::json!(["external-mcp"])
    );
    assert!(
        find_tool_for_profile_and_agent_providers("agent_delegate", ToolProfile::Primary, &[])
            .is_none()
    );

    assert_eq!(
        AgentProvider::external MCP client.auth_probe_argv(),
        Some(["login", "status"].as_slice())
    );
    assert_eq!(
        AgentProvider::external MCP client.auth_probe_argv(),
        Some(["auth", "status"].as_slice())
    );
    assert_eq!(AgentProvider::Antigravity.auth_probe_argv(), None);
    assert_eq!(
        provider_argv(AgentProvider::Antigravity, "fix it", 3),
        vec!["--print", "--mode=accept-edits", "fix it"]
    );
    let external-mcp = provider_argv(AgentProvider::external MCP client, "fix it", 3);
    assert!(external-mcp
        .windows(2)
        .any(|pair| { pair[0] == "--sandbox" && pair[1] == "workspace-write" }));
    assert!(external-mcp.iter().any(|arg| arg == "--approve-for-me"));
    for provider in [
        AgentProvider::external MCP client,
        AgentProvider::Antigravity,
        AgentProvider::external MCP client,
    ] {
        assert!(!provider_argv(provider, "fix it", 3).iter().any(|arg| {
            matches!(
                arg.as_str(),
                "--yolo"
                    | "--dangerously-skip-permissions"
                    | "--dangerously-bypass-approvals-and-sandbox"
                    | "--no-sandbox"
                    | "--api-key"
                    | "--with-api-key"
            )
        }));
    }

    println!("plan048 authenticated CLI capability acceptance: PASS");
}
