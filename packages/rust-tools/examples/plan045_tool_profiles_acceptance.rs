use ai_tools::core::config::{ServerConfig, ToolProfile};
use ai_tools::interfaces::mcp::{
    find_tool_for_profile, retained_tool_catalog, runtime_tool_catalog, PRIMARY_TOOL_NAMES,
};

fn main() {
    let full = retained_tool_catalog();
    let primary = runtime_tool_catalog(ToolProfile::Primary, false);
    assert_eq!(primary.len(), 33);
    assert_eq!(PRIMARY_TOOL_NAMES.len(), 33);
    assert!(full.len() >= primary.len());
    for name in PRIMARY_TOOL_NAMES {
        assert!(find_tool_for_profile(name, ToolProfile::Primary).is_some());
    }
    assert!(find_tool_for_profile("agent_delegate", ToolProfile::Primary).is_none());
    assert!(find_tool_for_profile("issue_create", ToolProfile::Primary).is_none());
    assert!(find_tool_for_profile("issue_create", ToolProfile::Full).is_some());
    for name in [
        "workspace_add",
        "workspace_list",
        "workspace_get",
        "workspace_remove",
    ] {
        assert!(find_tool_for_profile(name, ToolProfile::Full).is_some());
        assert!(find_tool_for_profile(name, ToolProfile::Primary).is_none());
    }
    assert!(primary
        .iter()
        .all(|t| full.iter().any(|f| f.name == t.name)));
    let config = ServerConfig {
        tool_profile: ToolProfile::Primary,
        ..ServerConfig::default()
    };
    assert_eq!(config.tool_profile, ToolProfile::Primary);
    println!(
        "plan045 tool profiles acceptance: PASS full={} primary={}",
        full.len(),
        primary.len()
    );
}
