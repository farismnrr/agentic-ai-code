#![cfg(unix)]

use ai_tools::application::{blender, creative};
use ai_tools::core::config::ServerConfig;
use ai_tools::core::error::McpError;
use ai_tools::interfaces::mcp::ToolCallResult;
use image::{ImageBuffer, Rgba};
use serde_json::{json, Value};
use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use uuid::Uuid;

struct TempWorkspace(PathBuf);

impl TempWorkspace {
    fn new() -> Self {
        let base = std::env::temp_dir().join(format!(
            "blender-artifacts-test-{}-{}",
            std::process::id(),
            Uuid::new_v4()
        ));
        let root = base.join("Blender").join("artifact-fixture");
        fs::create_dir_all(&root).expect("Blender artifacts test workspace");
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

    fn path(&self, relative: &str) -> PathBuf {
        self.0.join(relative)
    }
}

impl Drop for TempWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

async fn create_project_and_source(config: &ServerConfig, workspace: &TempWorkspace) -> String {
    let result = creative::dispatch_tool(
        "creative_project",
        &json!({
            "action":"create",
            "cwd":workspace.cwd(),
            "project_id":"project_artifacts",
            "title":"Blender artifacts",
            "intent":"contained Blender artifact fixture",
            "tracks":["anime"]
        }),
        config,
        "owner_a",
    )
    .await
    .expect("create project")
    .expect("project result");
    assert!(!result.is_error);

    let source_path = workspace.path("source.png");
    write_png(&source_path);
    let registered = creative::dispatch_tool(
        "creative_asset",
        &json!({
            "action":"register",
            "cwd":workspace.cwd(),
            "project_id":"project_artifacts",
            "path":"source.png",
            "media_type":"image/png",
            "role":"reference_image"
        }),
        config,
        "owner_a",
    )
    .await
    .expect("register source")
    .expect("source result");
    result_json(registered)["asset_id"]
        .as_str()
        .expect("source asset id")
        .to_owned()
}

async fn call(
    config: &ServerConfig,
    cwd: &str,
    owner: &str,
    tool: &str,
    extra: Value,
) -> Result<Value, McpError> {
    let mut arguments = json!({
        "cwd":cwd,
        "project_id":"project_artifacts"
    });
    for (key, value) in extra.as_object().expect("object arguments") {
        arguments[key] = value.clone();
    }
    let result = blender::dispatch_tool(tool, &arguments, config, owner)
        .await?
        .expect("Blender result");
    if result.is_error {
        return Err(McpError::InvalidRequest(result.content[0].text.clone()));
    }
    Ok(result_json(result))
}

fn result_json(result: ToolCallResult) -> Value {
    assert!(!result.is_error);
    serde_json::from_str(&result.content[0].text).expect("Blender JSON result")
}

async fn start_fixture_bridge(connections: usize) -> (u16, JoinHandle<()>) {
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind Blender artifact fixture");
    let port = listener.local_addr().unwrap().port();
    let handle = tokio::spawn(async move {
        for _ in 0..connections {
            let (mut stream, _) = listener.accept().await.expect("accept Blender request");
            let request = read_request(&mut stream).await;
            let code = request["code"].as_str().expect("code string");
            let result = if code.contains("# MASIHAWAM_ASSET_IMPORT_REFERENCE") {
                json!({"imported":["Reference_fixture"],"kind":"reference_image"})
            } else if code.contains("# MASIHAWAM_RENDER_STILL") {
                let path = extract_assignment_string(code, "_path = ").expect("still path");
                write_png(Path::new(&path));
                json!({"written":true})
            } else if code.contains("# MASIHAWAM_RENDER_ANIMATION") {
                let path = extract_assignment_string(code, "_path = ").expect("animation path");
                let frame = extract_assignment(code, "_frame = ")
                    .expect("animation frame")
                    .as_i64()
                    .expect("animation frame integer");
                write_png(Path::new(&path));
                json!({"written_frame":frame})
            } else if code.contains("# MASIHAWAM_ASSET_EXPORT") {
                let path = extract_assignment_string(code, "_path = ")
                    .map(PathBuf::from)
                    .or_else(|| extract_filepath(code))
                    .expect("export path");
                fs::write(&path, b"fixture-export").expect("write export fixture");
                json!({"exported":["Hero"],"fixture":true})
            } else if code.contains("# MASIHAWAM_CHECKPOINT_CREATE") {
                let path = extract_assignment_string(code, "_path = ").expect("checkpoint path");
                fs::write(&path, b"BLENDER-fixture-v1").expect("write checkpoint scene fixture");
                json!({"saved":true})
            } else if code.contains("# MASIHAWAM_CHECKPOINT_RESTORE") {
                json!({"restored":true})
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
        assert!(read > 0, "bridge closed before NUL terminator");
        if let Some(index) = chunk[..read].iter().position(|byte| *byte == 0) {
            bytes.extend_from_slice(&chunk[..index]);
            break;
        }
        bytes.extend_from_slice(&chunk[..read]);
        assert!(bytes.len() <= blender::MAX_BLENDER_REQUEST_BYTES);
    }
    serde_json::from_slice(&bytes).expect("valid Blender request")
}

fn extract_assignment(code: &str, prefix: &str) -> Option<Value> {
    let text = code.lines().find_map(|line| line.strip_prefix(prefix))?;
    serde_json::from_str(text).ok()
}

fn extract_assignment_string(code: &str, prefix: &str) -> Option<String> {
    extract_assignment(code, prefix)?
        .as_str()
        .map(str::to_owned)
}

fn extract_filepath(code: &str) -> Option<PathBuf> {
    for line in code.lines() {
        let marker = "filepath=";
        let start = line.find(marker)? + marker.len();
        let tail = &line[start..];
        if !tail.starts_with('"') {
            continue;
        }
        let bytes = tail.as_bytes();
        let mut escaped = false;
        for index in 1..bytes.len() {
            let byte = bytes[index];
            if escaped {
                escaped = false;
                continue;
            }
            if byte == b'\\' {
                escaped = true;
                continue;
            }
            if byte == b'"' {
                let literal = &tail[..=index];
                let value: String = serde_json::from_str(literal).ok()?;
                return Some(PathBuf::from(value));
            }
        }
    }
    None
}

fn write_png(path: &Path) {
    let image = ImageBuffer::<Rgba<u8>, Vec<u8>>::from_pixel(8, 8, Rgba([9, 8, 7, 255]));
    image.save(path).expect("write fixture PNG");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn blender_assets_renders_exports_and_checkpoints_stay_contained_and_traceable() {
    let workspace = TempWorkspace::new();
    let (port, bridge) = start_fixture_bridge(10).await;
    let config = workspace.config(port);
    let source_asset_id = create_project_and_source(&config, &workspace).await;

    let imported = call(
        &config,
        workspace.cwd(),
        "owner_a",
        "blender_asset_import",
        json!({
            "asset_id":source_asset_id,
            "purpose":"reference",
            "target_name":"front.png",
            "collection":"References"
        }),
    )
    .await
    .expect("import asset");
    assert_eq!(imported["relative_path"], "blender/references/front.png");
    assert!(workspace.path("blender/references/front.png").is_file());
    assert_ne!(
        imported["materialized_asset_id"],
        imported["source_asset_id"]
    );

    let repeated_import = call(
        &config,
        workspace.cwd(),
        "owner_a",
        "blender_asset_import",
        json!({
            "asset_id":source_asset_id,
            "purpose":"reference",
            "target_name":"front.png",
            "collection":"References"
        }),
    )
    .await
    .expect("repeat identical import");
    assert_eq!(
        repeated_import["materialized_asset_id"],
        imported["materialized_asset_id"]
    );
    assert_eq!(repeated_import["relative_path"], imported["relative_path"]);

    let still = call(
        &config,
        workspace.cwd(),
        "owner_a",
        "blender_render",
        json!({"mode":"still","output_scope":"final","file_name":"hero.png"}),
    )
    .await
    .expect("still render");
    assert_eq!(still["relative_path"], "blender/renders/final/hero.png");
    assert!(workspace.path("blender/renders/final/hero.png").is_file());

    let animation = call(
        &config,
        workspace.cwd(),
        "owner_a",
        "blender_render",
        json!({
            "mode":"animation",
            "output_scope":"preview",
            "file_name":"walk.png",
            "start_frame":1,
            "end_frame":3
        }),
    )
    .await
    .expect("animation render");
    assert_eq!(animation["outputs"].as_array().unwrap().len(), 3);
    assert!(workspace
        .path("blender/renders/preview/walk-000001.png")
        .is_file());

    let exported = call(
        &config,
        workspace.cwd(),
        "owner_a",
        "blender_asset_export",
        json!({
            "selection":["Hero"],
            "format":"glb",
            "output_scope":"export",
            "file_name":"hero.glb"
        }),
    )
    .await
    .expect("export asset");
    assert_eq!(exported["relative_path"], "blender/exports/hero.glb");
    assert!(workspace.path("blender/exports/hero.glb").is_file());

    let animation_export = call(
        &config,
        workspace.cwd(),
        "owner_a",
        "blender_asset_export",
        json!({
            "selection":["Hero"],
            "format":"glb",
            "output_scope":"animation",
            "file_name":"walk.glb"
        }),
    )
    .await
    .expect("animation export");
    assert_eq!(
        animation_export["relative_path"],
        "blender/animations/walk.glb"
    );
    assert!(workspace.path("blender/animations/walk.glb").is_file());

    let checkpoint = call(
        &config,
        workspace.cwd(),
        "owner_a",
        "blender_checkpoint_create",
        json!({"label":"before rig edit"}),
    )
    .await
    .expect("checkpoint create");
    let checkpoint_id = checkpoint["checkpoint_id"]
        .as_str()
        .expect("checkpoint id")
        .to_owned();
    let checkpoint_path = checkpoint["relative_path"]
        .as_str()
        .expect("checkpoint path")
        .to_owned();
    assert!(checkpoint_path.starts_with("blender/checkpoints/"));
    assert!(workspace.path(&checkpoint_path).is_file());
    assert!(checkpoint["scene_relative_path"]
        .as_str()
        .is_some_and(|value| value.starts_with("blender/scenes/")));
    assert!(!workspace
        .path("blender/checkpoints")
        .read_dir()
        .unwrap()
        .any(|entry| {
            entry
                .ok()
                .and_then(|value| value.file_name().to_str().map(str::to_owned))
                .is_some_and(|name| name.ends_with(".checkpoint.json"))
        }));

    let wrong_owner = call(
        &config,
        workspace.cwd(),
        "owner_b",
        "blender_checkpoint_restore",
        json!({"checkpoint_id":checkpoint_id}),
    )
    .await;
    assert!(wrong_owner.is_err());

    let restored = call(
        &config,
        workspace.cwd(),
        "owner_a",
        "blender_checkpoint_restore",
        json!({"checkpoint_id":checkpoint_id}),
    )
    .await
    .expect("checkpoint restore");
    assert_eq!(restored["checkpoint_id"], checkpoint_id);

    fs::write(workspace.path(&checkpoint_path), b"forged").expect("tamper checkpoint");
    let tampered = call(
        &config,
        workspace.cwd(),
        "owner_a",
        "blender_checkpoint_restore",
        json!({"checkpoint_id":checkpoint_id}),
    )
    .await;
    assert!(tampered.is_err());

    let escaped = call(
        &config,
        workspace.cwd(),
        "owner_a",
        "blender_asset_import",
        json!({
            "asset_id":source_asset_id,
            "purpose":"reference",
            "target_name":"../escape.png"
        }),
    )
    .await;
    assert!(escaped.is_err());

    let unsupported = call(
        &config,
        workspace.cwd(),
        "owner_a",
        "blender_asset_export",
        json!({
            "selection":["Hero"],
            "format":"stl",
            "file_name":"hero.stl"
        }),
    )
    .await;
    assert!(unsupported.is_err());

    fs::create_dir_all(workspace.path("blender/exports")).unwrap();
    let outside = workspace.path("outside.glb");
    fs::write(&outside, b"outside").unwrap();
    let symlink_path = workspace.path("blender/exports/link.glb");
    symlink(&outside, &symlink_path).unwrap();
    let symlink_result = call(
        &config,
        workspace.cwd(),
        "owner_a",
        "blender_asset_export",
        json!({
            "selection":["Hero"],
            "format":"glb",
            "file_name":"link.glb"
        }),
    )
    .await;
    assert!(symlink_result.is_err());

    bridge.await.expect("fixture bridge completes");
}
