#![cfg(unix)]

use ai_tools::application::{
    activity::event_for_tool, blender, creative, hooks::effect_classes_for_call,
};
use ai_tools::core::config::ServerConfig;
use ai_tools::interfaces::mcp::{blender_tool_catalog, ToolCallResult};
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use uuid::Uuid;

struct TempWorkspace(PathBuf);

impl TempWorkspace {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "blender-authoring-test-{}-{}",
            std::process::id(),
            Uuid::new_v4()
        ));
        fs::create_dir_all(&root).expect("Blender authoring test workspace");
        Self(root)
    }

    fn config(&self, port: u16) -> ServerConfig {
        ServerConfig {
            dir: Some(self.0.to_string_lossy().into_owned()),
            execution_root: Some(self.0.to_string_lossy().into_owned()),
            enable_creative: true,
            enable_blender: true,
            blender_bridge_port: port,
            blender_bridge_timeout_ms: 1_000,
            ..ServerConfig::default()
        }
    }

    fn cwd(&self) -> &str {
        self.0.to_str().expect("UTF-8 temp workspace")
    }
}

impl Drop for TempWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

async fn create_project(config: &ServerConfig, cwd: &str) {
    let result = creative::dispatch_tool(
        "creative_project",
        &json!({
            "action":"create",
            "cwd":cwd,
            "project_id":"project_authoring",
            "title":"Blender authoring",
            "intent":"privileged Python fixture",
            "tracks":["anime"]
        }),
        config,
        "owner_a",
    )
    .await
    .expect("create project")
    .expect("project result");
    assert!(!result.is_error);
}

async fn start_bridge(expected_codes: Vec<&'static str>) -> (u16, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind fake Blender authoring bridge");
    let port = listener.local_addr().unwrap().port();
    let handle = tokio::spawn(async move {
        for (index, expected_code) in expected_codes.into_iter().enumerate() {
            let (mut stream, _) = listener.accept().await.expect("accept Blender request");
            let mut bytes = Vec::new();
            let mut chunk = [0u8; 4096];
            loop {
                let read = stream.read(&mut chunk).await.expect("read Blender request");
                assert!(read > 0);
                if let Some(position) = chunk[..read].iter().position(|byte| *byte == 0) {
                    bytes.extend_from_slice(&chunk[..position]);
                    break;
                }
                bytes.extend_from_slice(&chunk[..read]);
            }
            let request: Value = serde_json::from_slice(&bytes).expect("valid bridge request");
            assert_eq!(request["type"], "execute");
            assert_eq!(request["code"], expected_code);
            if index == 0 {
                assert_eq!(request["strict_json"], true);
            } else {
                assert_eq!(request["strict_json"], false);
            }
            let result = if index == 0 {
                json!({"objects_created":1,"name":"Cube"})
            } else {
                json!("authoring complete")
            };
            let mut response = serde_json::to_vec(&json!({
                "status":"ok",
                "result":result,
                "stdout":"fixture stdout"
            }))
            .unwrap();
            response.push(0);
            stream.write_all(&response).await.unwrap();
        }
    });
    (port, handle)
}

async fn call(config: &ServerConfig, cwd: &str, code: &str, result_mode: &str) -> Value {
    let result = blender::dispatch_tool(
        "blender_execute_python",
        &json!({
            "cwd":cwd,
            "project_id":"project_authoring",
            "code":code,
            "result_mode":result_mode
        }),
        config,
        "owner_a",
    )
    .await
    .expect("authoring dispatch")
    .expect("authoring result");
    result_json(result)
}

fn result_json(result: ToolCallResult) -> Value {
    assert!(!result.is_error);
    serde_json::from_str(&result.content[0].text).expect("Blender authoring JSON result")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn privileged_python_is_bounded_unsandboxed_and_activity_safe() {
    let workspace = TempWorkspace::new();
    let json_code = "import bpy\nbpy.ops.mesh.primitive_cube_add()\nresult = {'objects_created': 1, 'name': bpy.context.object.name}";
    let text_code = "import bpy\nresult = 'authoring complete'";
    let (port, bridge) = start_bridge(vec![json_code, text_code]).await;
    let config = workspace.config(port);
    create_project(&config, workspace.cwd()).await;

    let json_result = call(&config, workspace.cwd(), json_code, "json").await;
    assert_eq!(json_result["result_mode"], "json");
    assert_eq!(json_result["result"]["objects_created"], 1);
    assert_eq!(json_result["stdout"], "fixture stdout");
    assert_eq!(json_result["sandboxed"], false);
    assert_eq!(json_result["authority"], "blender_host_user");

    let text_result = call(&config, workspace.cwd(), text_code, "text").await;
    assert_eq!(text_result["result_mode"], "text");
    assert_eq!(text_result["result"], "authoring complete");
    assert_eq!(text_result["sandboxed"], false);

    bridge.await.expect("fake bridge completes");

    let effects = effect_classes_for_call(
        "blender_execute_python",
        false,
        true,
        &json!({"code":json_code}),
    );
    assert!(effects.contains(&"external_mutation"));
    assert!(effects.contains(&"privileged_bridge"));
    assert!(effects.contains(&"process_exec"));

    let activity = event_for_tool(
        &config,
        "blender_execute_python",
        &effects,
        &json!({
            "cwd":workspace.cwd(),
            "project_id":"project_authoring",
            "code":"private_api_key = 'activity-secret'",
            "result_mode":"json"
        }),
        None,
    );
    let action = activity.presentation.action.expect("activity action");
    assert!(action.contains("blender execute python"));
    assert!(!action.contains("private_api_key"));
    assert!(!action.contains("activity-secret"));

    let tool = blender_tool_catalog()
        .into_iter()
        .find(|tool| tool.name == "blender_execute_python")
        .expect("Blender Python tool");
    let annotations = tool.annotations.expect("tool annotations");
    assert!(!annotations.read_only_hint);
    assert!(!annotations.idempotent_hint);
    assert!(annotations.open_world_hint);

    let oversized = "x".repeat(blender::MAX_BLENDER_PYTHON_BYTES + 1);
    let error = blender::dispatch_tool(
        "blender_execute_python",
        &json!({
            "cwd":workspace.cwd(),
            "project_id":"project_authoring",
            "code":oversized,
            "result_mode":"json"
        }),
        &config,
        "owner_a",
    )
    .await
    .expect_err("oversized Python must fail before bridge use");
    assert!(format!("{error:?}").contains("bounds"));
}
