use ai_tools::application::lsp::{LspError, LspSessionManager};
use ai_tools::core::config::ServerConfig;
use serde_json::json;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

const MODES: &[&str] = &[
    "normal",
    "malformed-framing",
    "malformed-json",
    "oversized",
    "hang",
    "crash",
    "invalid-id",
    "invalid-envelope",
    "notify-broken-pipe",
    "notification-flood",
    "server-requests",
];

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("LSP_FOUNDATION_ACCEPTANCE_FAIL: {error}");
        std::process::exit(1);
    }
    println!("LSP_FOUNDATION_ACCEPTANCE_PASS");
}

async fn run() -> Result<(), String> {
    let root = fixture_root();
    if root.exists() {
        fs::remove_dir_all(&root).map_err(io_error)?;
    }
    fs::create_dir_all(root.join("bin")).map_err(io_error)?;
    fs::create_dir_all(root.join(".ssh")).map_err(io_error)?;
    fs::write(root.join(".ssh/secret"), "CREDENTIAL_CANARY_039C").map_err(io_error)?;
    let repo_a = create_repo(&root, "repo-a")?;
    let repo_b = create_repo(&root, "repo-b")?;
    install_fake_servers(&root)?;

    let mut config = ServerConfig {
        execution_root: Some(path_text(&root)?),
        dir: Some(path_text(&root)?),
        toolchain_paths: vec![path_text(&root.join("bin"))?],
        ..ServerConfig::default()
    };
    config.lsp_servers = MODES
        .iter()
        .map(|mode| format!("{mode}=fake-lsp-{mode}"))
        .collect();
    config.validate().map_err(|error| error.to_string())?;
    let manager = LspSessionManager::new(config).map_err(display_error)?;

    let session_a = manager
        .session_for(Some(&path_text(&repo_a)?), "normal")
        .await
        .map_err(display_error)?;
    require(session_a.capabilities().definition, "capability capture")?;
    require(session_a.capabilities().hover, "hover capability capture")?;
    require(
        session_a
            .latest_notification("fixture/unknownNotification")
            .await
            .is_none(),
        "unretained notification method not cached",
    )?;
    let echoed = session_a
        .request("fixture/echo", json!({"value":"correlated"}))
        .await
        .map_err(display_error)?;
    require(
        echoed == json!({"value":"correlated"}),
        "request correlation",
    )?;
    require(
        session_a
            .sync_document("src.txt")
            .await
            .map_err(display_error)?
            == 1,
        "document open",
    )?;
    fs::write(repo_a.join("src.txt"), "changed\n").map_err(io_error)?;
    require(
        session_a
            .sync_document("src.txt")
            .await
            .map_err(display_error)?
            == 2,
        "document refresh",
    )?;

    let malicious_marker = root.join("malicious-executable-ran");
    let malicious = repo_a.join("fake-lsp-normal");
    fs::write(
        &malicious,
        format!("#!/bin/sh\necho bad > {}\n", malicious_marker.display()),
    )
    .map_err(io_error)?;
    make_executable(&malicious)?;
    require(
        !malicious_marker.exists(),
        "repository executable not selected",
    )?;

    let security = session_a
        .request(
            "fixture/security",
            json!({
                "protected": path_text(&root.join(".ssh/secret"))?,
                "sibling": path_text(&repo_b.join("src.txt"))?
            }),
        )
        .await
        .map_err(display_error)?;
    require(
        security["protectedReadable"] == false,
        "protected path hidden",
    )?;
    require(
        security["siblingReadable"] == false,
        "sibling workspace hidden",
    )?;
    require(security["networkReachable"] == false, "network isolated")?;
    require(
        security["environment"]
            == json!([
                "CARGO_HOME",
                "CARGO_TARGET_DIR",
                "HOME",
                "LANG",
                "PATH",
                "PWD",
                "TMPDIR"
            ]),
        "bounded environment",
    )?;

    let repo_a_text = path_text(&repo_a)?;
    let (same_one, same_two) = tokio::join!(
        manager.session_for(Some(&repo_a_text), "normal"),
        manager.session_for(Some(&repo_a_text), "normal")
    );
    let same_one = same_one.map_err(display_error)?;
    let same_two = same_two.map_err(display_error)?;
    require(
        Arc::ptr_eq(&same_one, &same_two),
        "concurrent same-key session reuse",
    )?;

    let session_b = manager
        .session_for(Some(&path_text(&repo_b)?), "normal")
        .await
        .map_err(display_error)?;
    require(
        !Arc::ptr_eq(&session_a, &session_b),
        "workspace session isolation",
    )?;
    require(
        session_a.identity().root != session_b.identity().root,
        "workspace identity isolation",
    )?;

    assert_mode_error(
        &manager,
        &repo_a,
        "malformed-framing",
        LspError::MalformedResponse,
    )
    .await?;
    assert_mode_error(
        &manager,
        &repo_a,
        "malformed-json",
        LspError::MalformedResponse,
    )
    .await?;
    assert_mode_error(&manager, &repo_a, "oversized", LspError::OversizedResponse).await?;
    assert_mode_error(&manager, &repo_a, "hang", LspError::Timeout).await?;
    assert_mode_error(&manager, &repo_a, "crash", LspError::Crashed).await?;
    assert_mode_error(&manager, &repo_a, "invalid-id", LspError::MalformedResponse).await?;
    assert_mode_error(
        &manager,
        &repo_a,
        "invalid-envelope",
        LspError::MalformedResponse,
    )
    .await?;
    assert_notification_write_failure(&manager, &repo_a).await?;
    assert_notification_retention_bound(&manager, &repo_a).await?;
    assert_server_request_handling(&manager, &repo_a).await?;

    require(
        session_a.retained_stderr_bytes().await <= 64 * 1024,
        "stderr bounded",
    )?;
    require(
        LspError::StartupFailed.to_string() == "Language server could not be started",
        "public-safe errors",
    )?;
    manager.shutdown_all().await;
    fs::remove_dir_all(&root).map_err(io_error)?;
    Ok(())
}

async fn assert_notification_write_failure(
    manager: &Arc<LspSessionManager>,
    repo: &Path,
) -> Result<(), String> {
    let session = manager
        .session_for(Some(&path_text(repo)?), "notify-broken-pipe")
        .await
        .map_err(display_error)?;
    let mut result = Ok(());
    for _ in 0..1_000 {
        result = session.notify("fixture/notification", json!({})).await;
        if result.is_err() {
            break;
        }
        tokio::task::yield_now().await;
    }
    match result {
        Err(LspError::Crashed) => {
            require(
                session.is_faulted(),
                "notification write failure faults session",
            )?;
        }
        other => {
            return Err(format!(
                "notify-broken-pipe: expected Crashed, got {other:?}"
            ));
        }
    }
    let replacement = manager
        .session_for(Some(&path_text(repo)?), "notify-broken-pipe")
        .await;
    match replacement {
        Ok(replacement) => require(
            !Arc::ptr_eq(&session, &replacement),
            "faulted notification session replaced",
        )?,
        Err(error) => require(
            error == LspError::StartupFailed,
            "faulted notification session removed",
        )?,
    }
    manager.shutdown_all().await;
    require(
        manager.active_session_count().await == 0,
        "faulted notification session cleaned up",
    )
}

async fn assert_notification_retention_bound(
    manager: &Arc<LspSessionManager>,
    repo: &Path,
) -> Result<(), String> {
    let session = manager
        .session_for(Some(&path_text(repo)?), "notification-flood")
        .await
        .map_err(display_error)?;
    session
        .request("fixture/test", json!({}))
        .await
        .map_err(display_error)?;
    for index in 0..40 {
        require(
            session
                .latest_notification(&format!("flood/method-{index}"))
                .await
                .is_none(),
            "arbitrary notification method not retained under flood",
        )?;
    }
    require(!session.is_faulted(), "session healthy after flood")?;
    manager.shutdown_all().await;
    Ok(())
}

async fn assert_server_request_handling(
    manager: &Arc<LspSessionManager>,
    repo: &Path,
) -> Result<(), String> {
    let session = manager
        .session_for(Some(&path_text(repo)?), "server-requests")
        .await
        .map_err(display_error)?;
    let result = session
        .request("fixture/test", json!({}))
        .await
        .map_err(display_error)?;
    let expected_uri = url::Url::from_directory_path(session.identity().root.clone())
        .map_err(|_| "expected uri".to_string())?
        .to_string();
    require(
        result["folders"] == json!([{"uri": expected_uri, "name": "workspace"}]),
        "workspace/workspaceFolders returns canonical root only",
    )?;
    require(
        result["unknown"]["error"]["code"] == json!(-32601),
        "unknown server request receives Method not found",
    )?;
    require(
        !session.is_faulted(),
        "session healthy after protocol exchange",
    )?;
    manager.shutdown_all().await;
    Ok(())
}

async fn assert_mode_error(
    manager: &Arc<LspSessionManager>,
    repo: &Path,
    mode: &str,
    expected: LspError,
) -> Result<(), String> {
    let session = manager
        .session_for(Some(&path_text(repo)?), mode)
        .await
        .map_err(display_error)?;
    let result = session
        .request_with_timeout_ms("fixture/test", json!({}), 200)
        .await;
    match result {
        Err(actual) if actual == expected => {
            require(
                session.is_faulted(),
                &format!("{mode}: session faulted after error"),
            )?;
            Ok(())
        }
        other => Err(format!("{mode}: expected {expected:?}, got {other:?}")),
    }
}

fn create_repo(root: &Path, name: &str) -> Result<PathBuf, String> {
    let repo = root.join(name);
    fs::create_dir_all(&repo).map_err(io_error)?;
    fs::write(repo.join("src.txt"), "initial\n").map_err(io_error)?;
    let status = std::process::Command::new("git")
        .args(["init", "-q"])
        .current_dir(&repo)
        .status()
        .map_err(io_error)?;
    require(status.success(), "fixture git init")?;
    Ok(repo)
}

fn install_fake_servers(root: &Path) -> Result<(), String> {
    let source = root.join("bin/fake-lsp-source");
    fs::write(&source, FAKE_SERVER).map_err(io_error)?;
    make_executable(&source)?;
    for mode in MODES {
        let destination = root.join("bin").join(format!("fake-lsp-{mode}"));
        fs::copy(&source, &destination).map_err(io_error)?;
        make_executable(&destination)?;
    }
    Ok(())
}

fn make_executable(path: &Path) -> Result<(), String> {
    let mut permissions = fs::metadata(path).map_err(io_error)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).map_err(io_error)
}

fn fixture_root() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("target")
        .join(format!("lsp-foundation-{nonce}"))
}

fn path_text(path: &Path) -> Result<String, String> {
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| "fixture path is not UTF-8".to_string())
}
fn display_error(error: LspError) -> String {
    format!("{} ({})", error, error.kind())
}
fn io_error(error: std::io::Error) -> String {
    error.to_string()
}
fn require(value: bool, label: &str) -> Result<(), String> {
    if value {
        Ok(())
    } else {
        Err(format!("assertion failed: {label}"))
    }
}

const FAKE_SERVER: &str = r#"#!/usr/bin/python3
import json, os, socket, sys, time
mode = os.path.basename(sys.argv[0]).replace("fake-lsp-", "")
def read_message():
    headers = {}
    while True:
        line = sys.stdin.buffer.readline()
        if not line: return None
        if line == b"\r\n": break
        name, value = line.decode("ascii").split(":", 1)
        headers[name.lower()] = value.strip()
    length = int(headers["content-length"])
    return json.loads(sys.stdin.buffer.read(length).decode("utf-8"))
def send(value):
    payload = json.dumps(value, separators=(",", ":")).encode()
    sys.stdout.buffer.write(f"Content-Length: {len(payload)}\r\n\r\n".encode() + payload)
    sys.stdout.buffer.flush()
def readable(path):
    try:
        with open(path, "rb") as handle: handle.read(1)
        return True
    except Exception: return False
while True:
    message = read_message()
    if message is None: sys.exit(0)
    method = message.get("method")
    if method == "initialize":
        send({"jsonrpc":"2.0","id":message["id"],"result":{"capabilities":{"definitionProvider":True,"hoverProvider":True,"textDocumentSync":{"openClose":True,"change":2}}}})
        send({"jsonrpc":"2.0","method":"fixture/unknownNotification","params":{}})
        if mode == "notification-flood":
            for i in range(40):
                send({"jsonrpc":"2.0","method":f"flood/method-{i}","params":{"blob":"x"*4096}})
    elif method == "shutdown": send({"jsonrpc":"2.0","id":message["id"],"result":None})
    elif method == "exit": sys.exit(0)
    elif method == "initialized":
        if mode == "notify-broken-pipe":
            os.close(0)
            sys.exit(0)
    elif method in ("textDocument/didOpen", "textDocument/didChange"): pass
    elif method == "fixture/echo": send({"jsonrpc":"2.0","id":message["id"],"result":message.get("params")})
    elif method == "fixture/security":
        params = message.get("params", {})
        network = False
        try:
            sock = socket.socket(); sock.settimeout(0.1); sock.connect(("1.1.1.1", 53)); network = True
        except Exception: pass
        send({"jsonrpc":"2.0","id":message["id"],"result":{"protectedReadable":readable(params.get("protected", "")),"siblingReadable":readable(params.get("sibling", "")),"networkReachable":network,"environment":sorted(os.environ.keys())}})
    elif method == "fixture/test":
        if mode == "malformed-framing": sys.stdout.buffer.write(b"Content-Length: 2\r\nContent-Length: 2\r\n\r\n{}"); sys.stdout.buffer.flush()
        elif mode == "malformed-json": sys.stdout.buffer.write(b"Content-Length: 1\r\n\r\n{"); sys.stdout.buffer.flush()
        elif mode == "oversized": sys.stdout.buffer.write(b"Content-Length: 4194305\r\n\r\n"); sys.stdout.buffer.flush()
        elif mode == "hang": time.sleep(5)
        elif mode == "crash": sys.stderr.write("SECRET=stderr-canary /home/private/path\n"); sys.stderr.flush(); sys.exit(7)
        elif mode == "invalid-id": send({"jsonrpc":"2.0","id":message["id"] + 99,"result":{}})
        elif mode == "invalid-envelope": send({"jsonrpc":"1.0","id":message["id"],"result":{}})
        elif mode == "server-requests":
            send({"jsonrpc":"2.0","id":9001,"method":"workspace/workspaceFolders","params":None})
            folders_response = read_message()
            send({"jsonrpc":"2.0","id":9002,"method":"fixture/unknownRequest","params":None})
            unknown_response = read_message()
            send({"jsonrpc":"2.0","id":message["id"],"result":{"folders":folders_response.get("result"),"unknown":unknown_response}})
        else: send({"jsonrpc":"2.0","id":message["id"],"result":{}})
    elif "id" in message: send({"jsonrpc":"2.0","id":message["id"],"error":{"code":-32601,"message":"unknown"}})
"#;
