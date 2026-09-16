#![cfg(unix)]

use ai_tools::application::{blender, creative};
use ai_tools::core::config::ServerConfig;
use ai_tools::interfaces::mcp::ToolCallResult;
use image::{ImageBuffer, Rgba};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use uuid::Uuid;

struct TempWorkspace(PathBuf);

impl TempWorkspace {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "blender-reads-test-{}-{}",
            std::process::id(),
            Uuid::new_v4()
        ));
        fs::create_dir_all(&root).expect("Blender reads test workspace");
        Self(root)
    }

    fn config(&self, port: u16) -> ServerConfig {
        ServerConfig {
            dir: Some(self.0.to_string_lossy().into_owned()),
            execution_root: Some(self.0.to_string_lossy().into_owned()),
            enable_creative: true,
            blender_bridge_port: port,
            blender_bridge_timeout_ms: 1_000,
            ..ServerConfig::default()
        }
    }

    fn cwd(&self) -> &str {
        self.0.to_str().expect("UTF-8 temp path")
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
            "project_id":"project_reads",
            "title":"Blender reads",
            "intent":"structured Blender read fixture",
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

async fn call(config: &ServerConfig, cwd: &str, tool: &str, extra: Value) -> Value {
    let mut arguments = json!({
        "cwd":cwd,
        "project_id":"project_reads"
    });
    for (key, value) in extra.as_object().expect("object arguments") {
        arguments[key] = value.clone();
    }
    let result = blender::dispatch_tool(tool, &arguments, config, "owner_a")
        .await
        .expect("Blender dispatch")
        .expect("Blender result");
    result_json(result)
}

fn result_json(result: ToolCallResult) -> Value {
    assert!(!result.is_error);
    serde_json::from_str(&result.content[0].text).expect("Blender JSON result")
}

async fn start_fixture_bridge(connections: usize) -> (u16, JoinHandle<()>) {
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind Blender reads fixture");
    let port = listener.local_addr().unwrap().port();
    let handle = tokio::spawn(async move {
        for _ in 0..connections {
            let (mut stream, _) = listener.accept().await.expect("accept Blender request");
            let request = read_request(&mut stream).await;
            let code = request["code"].as_str().expect("code string");
            let result = if let Some(scope) = inspect_scope(code) {
                json!({"scope":scope,"fixture":true})
            } else if code.contains("# MASIHAWAM_DOCS") {
                json!({
                    "blender_version":"4.3.2",
                    "api_version":[4,3,2],
                    "docs_base_url":"https://docs.blender.org/api/4.3/",
                    "module":"bpy.ops",
                    "query":"render",
                    "matches":[{"name":"bpy.ops.render","kind":"fixture","doc":"fixture docs"}]
                })
            } else if code.contains("# MASIHAWAM_SCREENSHOT") {
                let path_value =
                    extract_json_assignment(code, "_path = ").expect("screenshot path value");
                let path = path_value.as_str().expect("screenshot path").to_owned();
                write_png(Path::new(&path));
                json!({"written":true})
            } else if code.contains("# MASIHAWAM_ANIMATION_PREVIEW") {
                let paths_value =
                    extract_json_assignment(code, "_paths = ").expect("animation path values");
                let paths = paths_value.as_array().expect("animation paths");
                for path in paths {
                    write_png(Path::new(path.as_str().expect("animation path")));
                }
                json!({"written":paths.len()})
            } else {
                panic!("unexpected Blender fixture code: {code}");
            };
            let mut response = serde_json::to_vec(&json!({
                "status":"ok",
                "result":result,
                "stdout":""
            }))
            .unwrap();
            response.push(0);
            stream.write_all(&response).await.unwrap();
        }
    });
    (port, handle)
}

async fn read_request(stream: &mut tokio::net::TcpStream) -> Value {
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let read = stream.read(&mut chunk).await.expect("read Blender request");
        assert!(read > 0, "bridge closed before NUL frame terminator");
        if let Some(index) = chunk[..read].iter().position(|byte| *byte == 0) {
            bytes.extend_from_slice(&chunk[..index]);
            break;
        }
        bytes.extend_from_slice(&chunk[..read]);
        assert!(bytes.len() <= blender::MAX_BLENDER_REQUEST_BYTES);
    }
    let request: Value = serde_json::from_slice(&bytes).expect("valid Blender request JSON");
    assert_eq!(request["type"], "execute");
    assert_eq!(request["strict_json"], true);
    request
}

fn inspect_scope(code: &str) -> Option<&str> {
    code.lines()
        .find_map(|line| line.strip_prefix("# MASIHAWAM_INSPECT:"))
}

fn extract_json_assignment(code: &str, prefix: &str) -> Option<Value> {
    let text = code.lines().find_map(|line| line.strip_prefix(prefix))?;
    serde_json::from_str(text).ok()
}

fn write_png(path: &Path) {
    let image = ImageBuffer::<Rgba<u8>, Vec<u8>>::from_pixel(8, 8, Rgba([1, 2, 3, 255]));
    image.save(path).expect("write fixture PNG");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn structured_blender_reads_docs_and_previews_are_bounded_and_contained() {
    const SCOPES: &[&str] = &[
        "scene",
        "object",
        "mesh",
        "uv",
        "rig",
        "animation",
        "material",
        "nodes",
        "physics",
        "asset",
        "render",
        "character",
    ];
    let workspace = TempWorkspace::new();
    let (port, bridge) = start_fixture_bridge(SCOPES.len() + 3).await;
    let config = workspace.config(port);
    create_project(&config, workspace.cwd()).await;

    for scope in SCOPES {
        let result = call(
            &config,
            workspace.cwd(),
            "blender_inspect",
            json!({"scope":scope,"detail":"standard"}),
        )
        .await;
        assert_eq!(result["inspection"]["scope"], *scope);
        assert_eq!(result["inspection"]["fixture"], true);
    }

    let docs = call(
        &config,
        workspace.cwd(),
        "blender_python_api_docs",
        json!({"query":"render","module":"bpy.ops","limit":4}),
    )
    .await;
    assert_eq!(docs["docs"]["blender_version"], "4.3.2");
    assert_eq!(
        docs["docs"]["docs_base_url"],
        "https://docs.blender.org/api/4.3/"
    );

    let screenshot = call(
        &config,
        workspace.cwd(),
        "blender_screenshot",
        json!({
            "source":"viewport",
            "width":128,
            "height":96,
            "save_name":"viewport.png"
        }),
    )
    .await;
    assert_eq!(screenshot["kind"], "screenshot");
    assert_eq!(
        screenshot["preview"]["relative_path"],
        "blender/renders/preview/viewport.png"
    );
    assert_eq!(screenshot["preview"]["width"], 8);
    assert!(screenshot["preview"]["checksum_sha256"]
        .as_str()
        .is_some_and(|value| value.len() == 64));

    let animation = call(
        &config,
        workspace.cwd(),
        "blender_animation_preview",
        json!({
            "start_frame":1,
            "end_frame":5,
            "step":2,
            "max_frames":3,
            "width":160,
            "height":90,
            "save_name":"walk"
        }),
    )
    .await;
    assert_eq!(animation["kind"], "animation_preview");
    assert_eq!(animation["sampled_frames"], json!([1, 3, 5]));
    assert_eq!(animation["previews"].as_array().unwrap().len(), 3);
    for preview in animation["previews"].as_array().unwrap() {
        assert!(preview["relative_path"]
            .as_str()
            .is_some_and(|path| path.starts_with("blender/renders/preview/walk-")));
    }

    bridge.await.expect("fixture bridge task");

    let invalid_docs = blender::dispatch_tool(
        "blender_python_api_docs",
        &json!({
            "cwd":workspace.cwd(),
            "project_id":"project_reads",
            "query":"render",
            "module":"os.system"
        }),
        &config,
        "owner_a",
    )
    .await;
    assert!(invalid_docs.is_err());

    let existing_target = blender::dispatch_tool(
        "blender_screenshot",
        &json!({
            "cwd":workspace.cwd(),
            "project_id":"project_reads",
            "source":"viewport",
            "width":128,
            "height":96,
            "save_name":"viewport.png"
        }),
        &config,
        "owner_a",
    )
    .await;
    assert!(existing_target.is_err());
}
