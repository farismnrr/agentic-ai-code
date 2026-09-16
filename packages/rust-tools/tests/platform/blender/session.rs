#![cfg(unix)]

use ai_tools::application::{blender, creative};
use ai_tools::core::config::ServerConfig;
use ai_tools::core::error::McpError;
use ai_tools::interfaces::mcp::ToolCallResult;
use serde_json::{json, Value};
use std::fs;
use std::io::Write as _;
use std::net::TcpListener as StdTcpListener;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use uuid::Uuid;

struct TempWorkspace(PathBuf);

impl TempWorkspace {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "blender-session-test-{}-{}",
            std::process::id(),
            Uuid::new_v4()
        ));
        fs::create_dir_all(&root).expect("Blender session test workspace");
        Self(root)
    }

    fn config(&self, port: u16) -> ServerConfig {
        ServerConfig {
            dir: Some(self.0.to_string_lossy().into_owned()),
            execution_root: Some(self.0.to_string_lossy().into_owned()),
            enable_creative: true,
            enable_blender: true,
            blender_bridge_port: port,
            blender_bridge_timeout_ms: 500,
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

#[derive(Clone, Copy)]
enum BridgeMode {
    Ready,
    Incompatible,
    Malformed,
    Oversized,
    Hang,
}

async fn start_bridge(mode: BridgeMode, connections: usize) -> (u16, JoinHandle<()>) {
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind fake Blender bridge");
    let port = listener.local_addr().unwrap().port();
    let handle = tokio::spawn(serve_bridge(listener, mode, connections));
    (port, handle)
}

fn reserve_port() -> u16 {
    let listener = StdTcpListener::bind(("127.0.0.1", 0)).expect("reserve loopback port");
    listener.local_addr().unwrap().port()
}

fn delayed_bridge(
    port: u16,
    delay_ms: u64,
    mode: BridgeMode,
    connections: usize,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        sleep(Duration::from_millis(delay_ms)).await;
        let listener = TcpListener::bind(("127.0.0.1", port))
            .await
            .expect("bind delayed fake Blender bridge");
        serve_bridge(listener, mode, connections).await;
    })
}

async fn serve_bridge(listener: TcpListener, mode: BridgeMode, connections: usize) {
    for _ in 0..connections {
        let (mut stream, _) = listener.accept().await.expect("accept fake bridge request");
        let mut request = Vec::new();
        let mut chunk = [0u8; 4096];
        loop {
            let read = stream.read(&mut chunk).await.expect("read bridge request");
            if read == 0 {
                break;
            }
            if let Some(index) = chunk[..read].iter().position(|byte| *byte == 0) {
                request.extend_from_slice(&chunk[..index]);
                break;
            }
            request.extend_from_slice(&chunk[..read]);
            assert!(request.len() <= blender::MAX_BLENDER_REQUEST_BYTES);
        }
        let request: Value = serde_json::from_slice(&request).expect("valid bridge request JSON");
        assert_eq!(request["type"], "execute");
        assert_eq!(request["strict_json"], true);
        assert!(request["code"]
            .as_str()
            .is_some_and(|code| code.contains("bpy")));

        match mode {
            BridgeMode::Ready => {
                let mut bytes = serde_json::to_vec(&json!({
                    "status":"ok",
                    "result":{"ready":true,"version":"4.3.2","scene":"Scene"},
                    "stdout":""
                }))
                .unwrap();
                bytes.push(0);
                stream.write_all(&bytes).await.unwrap();
            }
            BridgeMode::Incompatible => {
                let mut bytes = serde_json::to_vec(&json!({
                    "status":"ok",
                    "result":{"ready":false,"version":""}
                }))
                .unwrap();
                bytes.push(0);
                stream.write_all(&bytes).await.unwrap();
            }
            BridgeMode::Malformed => {
                stream.write_all(b"{not-json\0").await.unwrap();
            }
            BridgeMode::Oversized => {
                let bytes = vec![b'x'; blender::MAX_BLENDER_RESPONSE_BYTES + 1];
                stream.write_all(&bytes).await.unwrap();
                stream.write_all(&[0]).await.unwrap();
            }
            BridgeMode::Hang => sleep(Duration::from_millis(500)).await,
        }
    }
}

async fn create_project(config: &ServerConfig, cwd: &str, project_id: &str) {
    let result = creative::dispatch_tool(
        "creative_project",
        &json!({
            "action":"create",
            "cwd":cwd,
            "project_id":project_id,
            "title":project_id,
            "intent":"Blender session lifecycle fixture",
            "tracks":["anime"]
        }),
        config,
        "owner_a",
    )
    .await
    .expect("create project")
    .expect("creative project result");
    assert!(!result.is_error);
}

async fn blender_call(
    config: &ServerConfig,
    cwd: &str,
    owner: &str,
    project_id: &str,
    action: &str,
) -> Result<Value, McpError> {
    let result = blender::dispatch_tool(
        "blender_session",
        &json!({
            "action":action,
            "cwd":cwd,
            "project_id":project_id
        }),
        config,
        owner,
    )
    .await?
    .expect("Blender session dispatch result");
    Ok(result_json(result))
}

fn result_json(result: ToolCallResult) -> Value {
    assert!(!result.is_error);
    serde_json::from_str(&result.content[0].text).expect("Blender session JSON result")
}

fn write_executable(path: &Path, body: &str) {
    let mut file = fs::File::create(path).expect("create fake Blender executable");
    file.write_all(body.as_bytes()).unwrap();
    let mut permissions = file.metadata().unwrap().permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(path, permissions).unwrap();
}

async fn wait_for_file(path: &Path) {
    for _ in 0..40 {
        if path.is_file() {
            return;
        }
        sleep(Duration::from_millis(25)).await;
    }
    panic!("timed out waiting for {}", path.display());
}

fn process_exists(pid: i32) -> bool {
    let outcome = unsafe { libc::kill(pid, 0) };
    outcome == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

async fn wait_process_gone(pid: i32) {
    for _ in 0..40 {
        if !process_exists(pid) {
            return;
        }
        sleep(Duration::from_millis(25)).await;
    }
    panic!("process {pid} survived relay-owned Blender stop");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn blender_session_lifecycle_is_loopback_bounded_and_owner_safe() {
    let workspace = TempWorkspace::new();

    let (external_port, external_bridge) = start_bridge(BridgeMode::Ready, 3).await;
    let external_config = workspace.config(external_port);
    create_project(&external_config, workspace.cwd(), "project_a").await;
    create_project(&external_config, workspace.cwd(), "project_b").await;

    let status = blender_call(
        &external_config,
        workspace.cwd(),
        "owner_a",
        "project_a",
        "status",
    )
    .await
    .unwrap();
    assert_eq!(status["session"]["state"], "ready");
    assert_eq!(status["session"]["ownership"], "external");

    let started = blender_call(
        &external_config,
        workspace.cwd(),
        "owner_a",
        "project_a",
        "start",
    )
    .await
    .unwrap();
    assert_eq!(started["session"]["ownership"], "external");
    let external_stop = blender_call(
        &external_config,
        workspace.cwd(),
        "owner_a",
        "project_a",
        "stop",
    )
    .await;
    assert!(external_stop.is_err());
    external_bridge.await.unwrap();

    for mode in [
        BridgeMode::Incompatible,
        BridgeMode::Malformed,
        BridgeMode::Oversized,
        BridgeMode::Hang,
    ] {
        let (port, bridge) = start_bridge(mode, 1).await;
        let mut config = workspace.config(port);
        config.blender_bridge_timeout_ms = 100;
        let status = blender_call(&config, workspace.cwd(), "owner_a", "project_a", "status")
            .await
            .unwrap();
        assert_eq!(status["session"]["state"], "incompatible");
        if matches!(mode, BridgeMode::Hang) {
            bridge.abort();
        } else {
            bridge.await.unwrap();
        }
    }

    let cold_port = reserve_port();
    let runtime_home = workspace.0.join(".masihawam/blender-runtime-home");
    fs::create_dir_all(&runtime_home).unwrap();
    let fake_blender = workspace.0.join("fake-blender");
    write_executable(
        &fake_blender,
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$TMPDIR/fake-argv.txt\"\nprintf '%s\\n' \"$HOME\" > \"$TMPDIR/fake-home.txt\"\nsleep 1000 &\necho \"$!\" > \"$TMPDIR/fake-child.pid\"\nwait\n",
    );
    let mut cold_config = workspace.config(cold_port);
    cold_config.blender_executable = Some(fake_blender.to_string_lossy().into_owned());
    cold_config.blender_bridge_timeout_ms = 1_500;
    let cold_bridge = delayed_bridge(cold_port, 250, BridgeMode::Ready, 2);

    let started = blender_call(
        &cold_config,
        workspace.cwd(),
        "owner_a",
        "project_a",
        "start",
    )
    .await
    .unwrap();
    assert_eq!(started["session"]["state"], "ready");
    assert_eq!(started["session"]["ownership"], "relay_owned");

    let argv = fs::read_to_string(workspace.0.join("blender/tmp/fake-argv.txt")).unwrap();
    assert_eq!(
        argv.lines().map(str::to_owned).collect::<Vec<_>>(),
        vec![
            "--background".to_owned(),
            "--online-mode".to_owned(),
            "--command".to_owned(),
            "blender_mcp".to_owned(),
            "--".to_owned(),
            "--host".to_owned(),
            "127.0.0.1".to_owned(),
            "--port".to_owned(),
            cold_port.to_string(),
        ]
    );

    let launched_home = fs::read_to_string(workspace.0.join("blender/tmp/fake-home.txt")).unwrap();
    assert_eq!(launched_home.trim(), runtime_home.to_string_lossy());

    let child_pid_path = workspace.0.join("blender/tmp/fake-child.pid");
    wait_for_file(&child_pid_path).await;
    let child_pid: i32 = fs::read_to_string(&child_pid_path)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    assert!(process_exists(child_pid));

    let owner_mismatch = blender_call(
        &cold_config,
        workspace.cwd(),
        "owner_b",
        "project_a",
        "stop",
    )
    .await;
    assert!(owner_mismatch.is_err());
    let project_mismatch = blender_call(
        &cold_config,
        workspace.cwd(),
        "owner_a",
        "project_b",
        "stop",
    )
    .await;
    assert!(project_mismatch.is_err());

    let owned_status = blender_call(
        &cold_config,
        workspace.cwd(),
        "owner_a",
        "project_a",
        "status",
    )
    .await
    .unwrap();
    assert_eq!(owned_status["session"]["ownership"], "relay_owned");

    let stopped = blender_call(
        &cold_config,
        workspace.cwd(),
        "owner_a",
        "project_a",
        "stop",
    )
    .await
    .unwrap();
    assert_eq!(stopped["session"]["state"], "closed");
    wait_process_gone(child_pid).await;
    cold_bridge.await.unwrap();

    for relative in [
        "blender/scenes/.relay-layout",
        "blender/assets/.relay-layout",
        "blender/references/.relay-layout",
        "blender/renders/preview/.relay-layout",
        "blender/renders/final/.relay-layout",
        "blender/animations/.relay-layout",
        "blender/exports/.relay-layout",
        "blender/checkpoints/.relay-layout",
        "blender/tmp/.relay-layout",
    ] {
        assert!(workspace.0.join(relative).is_file(), "missing {relative}");
    }

    let fail_port = reserve_port();
    let fail_executable = workspace.0.join("fake-blender-fail");
    write_executable(&fail_executable, "#!/bin/sh\nexit 7\n");
    let mut fail_config = workspace.config(fail_port);
    fail_config.blender_executable = Some(fail_executable.to_string_lossy().into_owned());
    let startup_failure = blender_call(
        &fail_config,
        workspace.cwd(),
        "owner_a",
        "project_a",
        "start",
    )
    .await;
    assert!(startup_failure.is_err());

    let timeout_port = reserve_port();
    let timeout_executable = workspace.0.join("fake-blender-timeout");
    write_executable(&timeout_executable, "#!/bin/sh\nsleep 1000\n");
    let mut timeout_config = workspace.config(timeout_port);
    timeout_config.blender_executable = Some(timeout_executable.to_string_lossy().into_owned());
    timeout_config.blender_bridge_timeout_ms = 250;
    let readiness_timeout = blender_call(
        &timeout_config,
        workspace.cwd(),
        "owner_a",
        "project_a",
        "start",
    )
    .await;
    assert!(readiness_timeout.is_err());
}
