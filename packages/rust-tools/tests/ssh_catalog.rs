use ai_tools::core::config::ToolProfile;
use ai_tools::interfaces::mcp::{
    find_tool_for_profile, runtime_tool_catalog, validate_tool_arguments,
};
use serde_json::json;

#[test]
fn dedicated_ssh_tool_is_portable_and_full_profile_only() {
    let tool = find_tool_for_profile("ssh_readonly_exec", ToolProfile::Full)
        .expect("dedicated read-only SSH tool must be discoverable in Full");
    let annotations = tool.annotations.expect("SSH annotations");
    assert!(annotations.read_only_hint);
    assert!(!annotations.destructive_hint);
    assert!(annotations.idempotent_hint);
    assert!(annotations.open_world_hint);
    assert_eq!(
        tool.execution
            .as_ref()
            .and_then(|value| value.get("taskSupport"))
            .and_then(serde_json::Value::as_str),
        Some("optional")
    );
    assert!(find_tool_for_profile("ssh_readonly_exec", ToolProfile::Primary).is_none());
}

#[test]
fn historical_catalog_snapshots_are_immutable() {
    use ring::digest::{digest, SHA256};
    for (path, expected_hash) in [
        (
            "../../../.agents/contracts/063-tool-catalog-v13.json",
            "606f16cab046283c77b7c5bf773c2dbfa51cf62d6488b63855705392e25a479e",
        ),
        (
            "../../../.agents/contracts/065-tool-catalog-v14.json",
            "6db382b1dd0cbce72190d87794470323caa29b9981a225f4dcba95b9457942b8",
        ),
        (
            "../../../.agents/contracts/067-tool-catalog-v15.json",
            "38e4d27f101fc94679c8f0e4f4f8bc8225c69bc60204c728d97a8c0c12d1af84",
        ),
    ] {
        let bytes = std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join(path.trim_start_matches("../../../")),
        )
        .expect("historical catalog");
        let actual_hash = digest(&SHA256, &bytes)
            .as_ref()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        assert_eq!(
            actual_hash, expected_hash,
            "historical snapshot changed: {path}"
        );
    }
}

#[test]
fn primary_profile_contains_only_the_fifteen_core_tools() {
    let tools = runtime_tool_catalog(ToolProfile::Primary, false);
    assert_eq!(tools.len(), 15);
    assert!(tools
        .iter()
        .all(|tool| ai_tools::interfaces::mcp::PRIMARY_TOOL_NAMES.contains(&tool.name)));
    assert!(!tools.iter().any(|tool| tool.name.starts_with("git_")));
    assert!(!tools.iter().any(|tool| tool.name.starts_with("code_")));
}

#[test]
fn dedicated_ssh_schema_accepts_structured_diagnostics_and_rejects_raw_options() {
    let tool = find_tool_for_profile("ssh_readonly_exec", ToolProfile::Full)
        .expect("dedicated read-only SSH tool");
    validate_tool_arguments(
        &tool,
        &json!({
            "alias": "smart-meeting",
            "command": "docker",
            "args": ["ps"],
            "timeout_ms": 30_000,
            "execution_mode": "auto"
        }),
    )
    .expect("valid structured SSH diagnostic input");

    for forbidden in [
        json!({"alias":"smart-meeting","command":"docker","ssh_options":["-F","/dev/null"]}),
        json!({"alias":"smart-meeting","command":"docker","identity_file":"/tmp/key"}),
        json!({"alias":"smart-meeting","command":"docker","config":"/tmp/ssh_config"}),
        json!({"alias":"smart-meeting","command":"docker","port":22}),
    ] {
        assert!(validate_tool_arguments(&tool, &forbidden).is_err());
    }
}
