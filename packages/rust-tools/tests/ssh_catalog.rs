use ai_tools::core::config::ToolProfile;
use ai_tools::interfaces::mcp::{find_tool_for_profile, tool_catalog, validate_tool_arguments};
use serde_json::json;

#[test]
fn dedicated_ssh_tool_is_portable_and_present_in_both_profiles() {
    for profile in [ToolProfile::Full, ToolProfile::Primary] {
        let tool = find_tool_for_profile("ssh_readonly_exec", profile)
            .expect("dedicated read-only SSH tool must be discoverable");
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
    }
}

#[test]
fn catalog_v14_snapshot_matches_current_static_surface() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(".agents/contracts/065-tool-catalog-v14.json");
    let expected = std::fs::read_to_string(root).expect("catalog v14 snapshot");
    let expected: serde_json::Value = serde_json::from_str(&expected).expect("valid catalog v14");
    let actual = serde_json::to_value(tool_catalog()).expect("serialize current catalog");
    assert_eq!(actual, expected);
}

#[test]
fn historical_catalog_is_immutable_and_only_terminal_descriptions_change() {
    use ring::digest::{digest, SHA256};
    let bytes = include_bytes!("../../../.agents/contracts/063-tool-catalog-v13.json");
    assert_eq!(
        digest(&SHA256, bytes)
            .as_ref()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>(),
        "606f16cab046283c77b7c5bf773c2dbfa51cf62d6488b63855705392e25a479e"
    );
    let mut historical: serde_json::Value = serde_json::from_slice(bytes).unwrap();
    let actual = serde_json::to_value(tool_catalog()).unwrap();
    for name in ["terminal_exec", "terminal_job_start"] {
        let new = actual
            .as_array()
            .unwrap()
            .iter()
            .find(|tool| tool["name"] == name)
            .unwrap();
        assert!(new["description"]
            .as_str()
            .unwrap()
            .contains("dedicated MCP tool"));
        let old = historical
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|tool| tool["name"] == name)
            .unwrap();
        old["description"] = new["description"].clone();
    }
    assert_eq!(historical, actual);
}

#[test]
fn dedicated_ssh_schema_accepts_structured_diagnostics_and_rejects_raw_options() {
    let tool = find_tool_for_profile("ssh_readonly_exec", ToolProfile::Primary)
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
