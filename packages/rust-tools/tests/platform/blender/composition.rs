use ai_tools::application::creative;
use ai_tools::core::config::{ServerConfig, ToolProfile};
use ai_tools::interfaces::mcp::{blender_tool_catalog, runtime_tool_catalog};
use serde_json::{json, Value};

fn tool_names(profile: ToolProfile, creative: bool) -> Vec<&'static str> {
    runtime_tool_catalog(profile, creative)
        .into_iter()
        .map(|tool| tool.name)
        .collect()
}

fn status_json(result: ai_tools::interfaces::mcp::ToolCallResult) -> Value {
    assert!(!result.is_error);
    serde_json::from_str(&result.content[0].text).expect("Creative status JSON")
}

#[tokio::test]
async fn blender_catalog_and_status_follow_the_single_creative_master_flag() {
    let blender_names = blender_tool_catalog()
        .into_iter()
        .map(|tool| tool.name)
        .collect::<Vec<_>>();
    assert_eq!(blender_names.len(), 5);

    for names in [
        tool_names(ToolProfile::Full, false),
        tool_names(ToolProfile::Primary, true),
    ] {
        assert!(names.contains(&"creative_status"));
        assert!(blender_names.iter().all(|name| !names.contains(name)));
    }

    let enabled = tool_names(ToolProfile::Full, true);
    for name in &blender_names {
        assert_eq!(
            enabled
                .iter()
                .filter(|candidate| *candidate == name)
                .count(),
            1
        );
    }

    let disabled = ServerConfig::default();
    let disabled_status = status_json(
        creative::dispatch_tool("creative_status", &json!({}), &disabled, "owner_a")
            .await
            .expect("disabled status dispatch")
            .expect("disabled status result"),
    );
    assert_eq!(disabled_status["blender"]["enabled"], false);
    assert_eq!(disabled_status["blender"]["tool_count"], 0);
    assert!(disabled_status["blender"]["activation"]
        .as_str()
        .unwrap()
        .contains("--enable-creative"));

    let fully_enabled = ServerConfig {
        enable_creative: true,
        ..ServerConfig::default()
    };
    let enabled_status = status_json(
        creative::dispatch_tool("creative_status", &json!({}), &fully_enabled, "owner_a")
            .await
            .expect("enabled status dispatch")
            .expect("enabled status result"),
    );
    assert_eq!(enabled_status["blender"]["enabled"], true);
    assert_eq!(enabled_status["blender"]["tool_count"], 5);
    assert!(enabled_status["blender"]["activation"].is_null());
}
