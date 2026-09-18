use ai_tools::application::workspace::{
    apply_patch, directory_list, dispatch_native_tool, file_edit, file_read, file_read_multiple,
    file_search, file_write,
};
use ai_tools::core::config::ServerConfig;
use ai_tools::interfaces::mcp::{
    find_tool, output_schema_for_tool, retained_tool_catalog, validate_tool_output,
    PRIMARY_TOOL_NAMES,
};
use serde::Serialize;
use serde_json::{json, to_value, Value};
use std::fs;
use uuid::Uuid;

fn public_output<T: Serialize>(tool_name: &str, result: T) -> Value {
    let mut value = to_value(result).unwrap();
    if let Some(object) = value.as_object_mut() {
        object.remove("_activity");
    }
    validate_tool_output(&find_tool(tool_name).unwrap(), &value).unwrap();
    value
}

fn fixture() -> (std::path::PathBuf, ServerConfig) {
    let root = std::env::temp_dir().join(format!("ai-tools-workspace-tools-{}", Uuid::new_v4()));
    fs::create_dir_all(&root).unwrap();
    let config = ServerConfig {
        dir: Some(root.to_string_lossy().into_owned()),
        execution_root: Some(root.to_string_lossy().into_owned()),
        ..ServerConfig::default()
    };
    (root, config)
}

#[test]
fn file_edit_schema_stays_flat_and_model_typed() {
    let tool = retained_tool_catalog()
        .into_iter()
        .find(|tool| tool.name == "file_edit")
        .expect("file_edit tool");
    assert!(tool.input_schema.get("oneOf").is_none());
    let properties = tool.input_schema["properties"].as_object().unwrap();
    for key in [
        "path",
        "cwd",
        "old_text",
        "new_text",
        "replace_all",
        "edits",
        "dry_run",
        "expected_sha256",
    ] {
        assert!(properties.contains_key(key), "missing typed field: {key}");
    }
    assert!(PRIMARY_TOOL_NAMES.contains(&"file_read_multiple"));

    for tool_name in [
        "directory_list",
        "file_search",
        "file_write",
        "file_edit",
        "file_read",
        "file_read_multiple",
        "text_search",
        "apply_patch",
    ] {
        let schema = output_schema_for_tool(tool_name)
            .unwrap_or_else(|| panic!("missing output schema: {tool_name}"));
        assert_eq!(
            schema["$schema"],
            "https://json-schema.org/draft/2020-12/schema"
        );
    }

    for tool_name in [
        "directory_list",
        "file_search",
        "file_write",
        "file_edit",
        "file_read",
        "file_read_multiple",
        "text_search",
    ] {
        let tool = retained_tool_catalog()
            .into_iter()
            .find(|tool| tool.name == tool_name)
            .unwrap_or_else(|| panic!("missing tool: {tool_name}"));
        let properties = tool.input_schema["properties"]
            .as_object()
            .unwrap_or_else(|| panic!("missing properties: {tool_name}"));
        for (name, schema) in properties {
            assert!(
                schema.get("description").and_then(Value::as_str).is_some(),
                "{tool_name}.{name} must describe its agent-facing semantics"
            );
        }
    }
}

#[test]
fn workspace_output_schemas_match_public_runtime_results() {
    let (root, config) = fixture();
    fs::write(root.join("seed.txt"), "one\ntwo\n").unwrap();

    public_output(
        "directory_list",
        directory_list(&json!({"path":".","depth":1}), &config).unwrap(),
    );
    public_output(
        "file_search",
        file_search(&json!({"pattern":"**/*.txt","max_results":10}), &config).unwrap(),
    );
    public_output(
        "file_read",
        file_read(&json!({"path":"seed.txt","limit_lines":10}), &config).unwrap(),
    );
    public_output(
        "file_read_multiple",
        file_read_multiple(
            &json!({"paths":["seed.txt","missing.txt"],"limit_lines":10}),
            &config,
        )
        .unwrap(),
    );

    public_output(
        "file_write",
        file_write(
            &json!({"path":"write.txt","content":"alpha\nbeta\n"}),
            &config,
        )
        .unwrap(),
    );
    public_output(
        "file_edit",
        file_edit(
            &json!({
                "path":"write.txt",
                "old_text":"beta",
                "new_text":"gamma",
                "dry_run":true
            }),
            &config,
        )
        .unwrap(),
    );
    public_output(
        "apply_patch",
        apply_patch(
            &json!({
                "patch":"--- seed.txt\n+++ seed.txt\n@@ -1,2 +1,2 @@\n one\n-two\n+three\n",
                "dry_run":true
            }),
            &config,
        )
        .unwrap(),
    );

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn workspace_read_write_edit_search_lifecycle_is_concurrency_safe() {
    let (root, config) = fixture();

    file_write(
        &json!({
            "path": "a.txt",
            "content": "alpha\nbeta target\ngamma\nbeta target\n"
        }),
        &config,
    )
    .unwrap();
    file_write(
        &json!({
            "path": "skip.tmp",
            "content": "beta target\n"
        }),
        &config,
    )
    .unwrap();

    let dispatched = dispatch_native_tool(
        "file_read",
        &json!({"path":"a.txt","offset_line":1,"limit_lines":2}),
        &config,
    )
    .unwrap()
    .expect("file_read dispatch");
    let structured = dispatched
        .structured_content
        .as_ref()
        .expect("structured file_read result");
    assert!(dispatched.content.is_empty());
    validate_tool_output(&find_tool("file_read").unwrap(), structured).unwrap();

    let first = to_value(
        file_read(
            &json!({"path":"a.txt","offset_line":1,"limit_lines":2}),
            &config,
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(first["truncated"], true);
    assert_eq!(first["next_offset_line"], 3);
    let hash = first["sha256"].as_str().expect("complete-file sha256");
    assert_eq!(hash.len(), 64);

    let preview = to_value(
        file_edit(
            &json!({
                "path":"a.txt",
                "old_text":"gamma",
                "new_text":"delta",
                "expected_sha256":hash,
                "dry_run":true
            }),
            &config,
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(preview["dry_run"], true);
    assert_eq!(preview["changed"], true);
    assert_eq!(
        to_value(file_read(&json!({"path":"a.txt"}), &config).unwrap()).unwrap()["content"],
        Value::String("alpha\nbeta target\ngamma\nbeta target\n".into())
    );

    fs::write(root.join("a.txt"), "external change\n").unwrap();
    let stale = file_edit(
        &json!({
            "path":"a.txt",
            "old_text":"external",
            "new_text":"internal",
            "expected_sha256":hash
        }),
        &config,
    )
    .unwrap_err()
    .to_string();
    assert!(stale.contains("expected_sha256"));

    let current = to_value(file_read(&json!({"path":"a.txt"}), &config).unwrap()).unwrap();
    let current_hash = current["sha256"].as_str().unwrap();
    file_edit(
        &json!({
            "path":"a.txt",
            "old_text":"external",
            "new_text":"internal",
            "expected_sha256":current_hash
        }),
        &config,
    )
    .unwrap();

    let multiple = to_value(
        file_read_multiple(
            &json!({"paths":["a.txt","missing.txt","skip.tmp"],"limit_lines":10}),
            &config,
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(multiple["count"], 3);
    assert_eq!(multiple["failed"], 1);
    assert_eq!(multiple["files"][0]["ok"], true);
    assert_eq!(multiple["files"][1]["ok"], false);
    assert!(multiple["files"][1]["error"].as_str().is_some());
    assert_eq!(multiple["files"][2]["ok"], true);

    let searched = to_value(
        file_search(
            &json!({
                "pattern":"**/*",
                "exclude":["*.tmp"],
                "max_results":10
            }),
            &config,
        )
        .unwrap(),
    )
    .unwrap();
    let matches = searched["matches"].as_array().unwrap();
    assert!(matches.iter().any(|value| value == "a.txt"));
    assert!(!matches.iter().any(|value| value == "skip.tmp"));

    let write_hash = to_value(file_read(&json!({"path":"a.txt"}), &config).unwrap()).unwrap();
    let write_hash = write_hash["sha256"].as_str().unwrap();
    file_write(
        &json!({
            "path":"a.txt",
            "content":"final\n",
            "overwrite":true,
            "expected_sha256":write_hash
        }),
        &config,
    )
    .unwrap();
    assert!(file_write(
        &json!({
            "path":"a.txt",
            "content":"stale\n",
            "overwrite":true,
            "expected_sha256":"0000000000000000000000000000000000000000000000000000000000000000"
        }),
        &config,
    )
    .is_err());

    fs::remove_dir_all(root).unwrap();
}
