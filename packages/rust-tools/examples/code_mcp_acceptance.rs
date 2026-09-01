//! Plan 039C PHASE-05 acceptance: proves the public `code_*` MCP tools
//! (catalog entries + `dispatch_code_tool`) work end-to-end against a real
//! `rust-analyzer` session, not just the underlying `RustLanguageServer`
//! adapter exercised by `rust_lsp_acceptance`.

use ai_tools::application::code::dispatch_code_tool;
use ai_tools::application::lsp::LspSessionManager;
use ai_tools::core::config::ServerConfig;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("CODE_MCP_ACCEPTANCE_FAIL: {error}");
        std::process::exit(1);
    }
    println!("CODE_MCP_ACCEPTANCE_PASS");
}

async fn run() -> Result<(), String> {
    for name in [
        "code_symbols",
        "code_definition",
        "code_references",
        "code_implementations",
        "code_hover",
        "code_diagnostics",
        "code_rename_preview",
    ] {
        require(
            ai_tools::interfaces::mcp::find_tool(name).is_some(),
            &format!("{name} is present in the MCP catalog"),
        )?;
    }
    let rename_tool = ai_tools::interfaces::mcp::find_tool("code_rename_preview").unwrap();
    require(
        rename_tool
            .annotations
            .as_ref()
            .is_some_and(|a| a.read_only_hint && !a.destructive_hint),
        "code_rename_preview is annotated read-only/non-destructive",
    )?;

    let root = fixture_root();
    fs::create_dir_all(root.join("src")).map_err(io_error)?;
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"code-mcp-proof\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .map_err(io_error)?;
    let source = "pub fn add(a: i32, b: i32) -> i32 { a + b }\n\
pub fn call_add() -> i32 { add(1, 2) + add(3, 4) }\n\
pub fn broken() -> i32 { \"not an integer\" }\n";
    fs::write(root.join("src/lib.rs"), source).map_err(io_error)?;
    git(&root, &["init", "-q"])?;

    let toolchain = rust_toolchain_bin()?;
    let mut config = ServerConfig {
        execution_root: Some(path_text(Path::new("/home/farismnrr"))?),
        toolchain_paths: vec![path_text(&toolchain)?],
        lsp_servers: vec!["rust=rust-analyzer".into()],
        ..ServerConfig::default()
    };
    config.dir = Some(path_text(&root)?);
    config.validate().map_err(|error| error.to_string())?;
    let manager = LspSessionManager::new(config.clone()).map_err(|error| error.to_string())?;
    let cwd = path_text(&root)?;

    // ---- code_symbols (path) ----
    let symbols = call(
        "code_symbols",
        json!({"cwd": cwd, "path": "src/lib.rs"}),
        &config,
        &manager,
    )
    .await?;
    let symbols = wait_ready(&symbols, "code_symbols", "src/lib.rs", &config, &manager).await?;
    require(
        symbols["symbols"]
            .as_array()
            .is_some_and(|list| list.iter().any(|s| s["name"] == "add")),
        "code_symbols returns add()",
    )?;

    // ---- code_symbols (workspace query / rust-analyzer) ----
    let workspace_symbols = call_until(
        "code_symbols",
        json!({"cwd": cwd, "query": "add", "max_results": 8}),
        &config,
        &manager,
        |value| {
            value["symbols"]
                .as_array()
                .is_some_and(|list| list.iter().any(|s| s["name"] == "add"))
        },
    )
    .await?;
    require(
        workspace_symbols["symbols"]
            .as_array()
            .is_some_and(|list| list.iter().any(|s| s["name"] == "add")),
        "Rust workspace-symbol search is backed by rust-analyzer and returns add()",
    )?;
    require(
        workspace_symbols.to_string().find("/home/").is_none(),
        "Rust workspace-symbol results do not expose absolute host paths",
    )?;

    // ---- code_definition ----
    let add_call_col = source
        .lines()
        .nth(1)
        .and_then(|line| line.find("add(1"))
        .ok_or("fixture call_add body")?;
    let definition_arguments =
        json!({"cwd": cwd, "path": "src/lib.rs", "line": 1, "column": add_call_col});
    let definition = call_until(
        "code_definition",
        definition_arguments,
        &config,
        &manager,
        |value| {
            value["locations"]
                .as_array()
                .is_some_and(|list| !list.is_empty())
        },
    )
    .await?;
    require(
        definition["locations"]
            .as_array()
            .is_some_and(|list| list.iter().any(|l| l["path"] == "src/lib.rs")),
        "code_definition resolves add() and returns a workspace-relative path (no absolute host path)",
    )?;

    // ---- code_references (pagination) ----
    let references_p1 = call(
        "code_references",
        json!({"cwd": cwd, "path": "src/lib.rs", "line": 0, "column": 7, "max_results": 1}),
        &config,
        &manager,
    )
    .await?;
    let page1 = references_p1["locations"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    require(page1.len() <= 1, "code_references honors max_results")?;
    if let Some(continuation) = references_p1.get("continuation").and_then(Value::as_str) {
        let references_p2 = call(
            "code_references",
            json!({"cwd": cwd, "path": "src/lib.rs", "line": 0, "column": 7, "max_results": 1, "continuation": continuation}),
            &config,
            &manager,
        )
        .await?;
        require(
            references_p2["locations"].as_array().is_some(),
            "code_references continuation token resumes the result list",
        )?;
    }

    // ---- code_hover ----
    let hover = call(
        "code_hover",
        json!({"cwd": cwd, "path": "src/lib.rs", "line": 0, "column": 7}),
        &config,
        &manager,
    )
    .await?;
    require(
        !hover.is_null(),
        "code_hover returns type information for add()",
    )?;

    // ---- code_diagnostics ----
    let diagnostics = call(
        "code_diagnostics",
        json!({"cwd": cwd, "path": "src/lib.rs"}),
        &config,
        &manager,
    )
    .await?;
    let diagnostics = diagnostics["diagnostics"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    require(
        diagnostics.iter().any(|d| {
            d["message"]
                .as_str()
                .unwrap_or("")
                .to_lowercase()
                .contains("mismatched")
                || d["message"]
                    .as_str()
                    .unwrap_or("")
                    .to_lowercase()
                    .contains("expected")
        }),
        "code_diagnostics surfaces the deterministic broken() type error",
    )?;

    // ---- code_implementations: capability-gated, distinct from failure ----
    // rust-analyzer does not advertise implementation search meaningfully for
    // a free function; assert the call either returns a bounded (possibly
    // empty) location list or a distinct unsupported-capability error, never
    // a generic/internal failure.
    let implementations_arguments =
        json!({"cwd": cwd, "path": "src/lib.rs", "line": 0, "column": 7});
    match dispatch_code_tool(
        "code_implementations",
        &implementations_arguments,
        &config,
        &manager,
    )
    .await
    {
        Ok(Some(result)) => require(!result.is_error, "code_implementations call did not error")?,
        Ok(None) => return Err("code_implementations must be handled".into()),
        Err(error) => {
            require(
                error.to_string().contains("unsupported_lsp_capability")
                    || error.to_string().contains("language_server"),
                &format!(
                    "code_implementations fails with a classified LSP error, not internal: {error}"
                ),
            )?;
        }
    }

    // ---- invalid request handling ----
    let missing_path = dispatch_code_tool(
        "code_definition",
        &json!({"line": 0, "column": 0}),
        &config,
        &manager,
    )
    .await;
    require(
        missing_path.is_err(),
        "code_definition without path is rejected",
    )?;

    let unsupported_extension = dispatch_code_tool(
        "code_definition",
        &json!({"cwd": cwd, "path": "README.md", "line": 0, "column": 0}),
        &config,
        &manager,
    )
    .await;
    require(
        unsupported_extension.is_err(),
        "code_definition on an unsupported file extension is rejected cleanly",
    )?;

    // ---- code_rename_preview: preview only, never mutates ----
    let before = fs::read_to_string(root.join("src/lib.rs")).map_err(io_error)?;
    let rename = call(
        "code_rename_preview",
        json!({"cwd": cwd, "path": "src/lib.rs", "line": 0, "column": 7, "new_name": "add_numbers"}),
        &config,
        &manager,
    )
    .await?;
    require(
        rename["applied"] == json!(false),
        "code_rename_preview reports applied=false",
    )?;
    let files = rename["files"].as_array().cloned().unwrap_or_default();
    require(
        !files.is_empty(),
        "code_rename_preview returns at least one file edit for add()",
    )?;
    for file in &files {
        require(
            file["path"].as_str().is_some_and(|p| !p.starts_with('/')),
            "code_rename_preview paths are workspace-relative, not absolute host paths",
        )?;
    }
    let after = fs::read_to_string(root.join("src/lib.rs")).map_err(io_error)?;
    require(
        before == after,
        "code_rename_preview performed no mutation on disk",
    )?;

    manager.shutdown_all().await;
    fs::remove_dir_all(&root).map_err(io_error)?;
    Ok(())
}

async fn call(
    name: &str,
    arguments: Value,
    config: &ServerConfig,
    manager: &std::sync::Arc<LspSessionManager>,
) -> Result<Value, String> {
    let result = dispatch_code_tool(name, &arguments, config, manager)
        .await
        .map_err(|error| format!("{name}: {error}"))?
        .ok_or_else(|| format!("{name}: not handled"))?;
    if result.is_error {
        return Err(format!("{name} unexpectedly failed"));
    }
    serde_json::from_str(&result.content[0].text).map_err(|error| error.to_string())
}

/// rust-analyzer's project loading is asynchronous; results may briefly be
/// empty right after the first session is created. Retries the exact same
/// public `code_*` MCP call (not a private readiness hook) until `ready`
/// accepts the result or a bounded deadline passes.
async fn call_until(
    name: &str,
    arguments: Value,
    config: &ServerConfig,
    manager: &std::sync::Arc<LspSessionManager>,
    ready: impl Fn(&Value) -> bool,
) -> Result<Value, String> {
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(30);
    let mut latest = call(name, arguments.clone(), config, manager)
        .await
        .unwrap_or(Value::Null);
    while !ready(&latest) && tokio::time::Instant::now() < deadline {
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
        latest = call(name, arguments.clone(), config, manager)
            .await
            .unwrap_or(Value::Null);
    }
    if latest.is_null() {
        return call(name, arguments, config, manager).await;
    }
    Ok(latest)
}

async fn wait_ready(
    first: &Value,
    name: &str,
    path: &str,
    config: &ServerConfig,
    manager: &std::sync::Arc<LspSessionManager>,
) -> Result<Value, String> {
    if first["symbols"]
        .as_array()
        .is_some_and(|list| !list.is_empty())
    {
        return Ok(first.clone());
    }
    call_until(
        name,
        json!({"path": path, "cwd": config.dir.clone().unwrap_or_default()}),
        config,
        manager,
        |value| {
            value["symbols"]
                .as_array()
                .is_some_and(|list| !list.is_empty())
        },
    )
    .await
}

fn rust_toolchain_bin() -> Result<PathBuf, String> {
    let candidates = [
        "/home/farismnrr/.rustup/toolchains/1.95.0-x86_64-unknown-linux-gnu/bin",
        "/home/farismnrr/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin",
    ];
    candidates
        .iter()
        .map(PathBuf::from)
        .find(|path| path.join("rust-analyzer").is_file())
        .ok_or_else(|| "standalone rust-analyzer toolchain bin not found".into())
}

fn fixture_root() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    PathBuf::from(format!("/home/farismnrr/.cache/ai-code-mcp-lsp-{nonce}"))
}

fn path_text(path: &Path) -> Result<String, String> {
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| "non-utf8 fixture path".into())
}

fn git(cwd: &Path, args: &[&str]) -> Result<(), String> {
    let status = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .status()
        .map_err(io_error)?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("git {:?} failed", args))
    }
}

fn io_error(error: std::io::Error) -> String {
    error.to_string()
}

fn require(condition: bool, message: &str) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(message.into())
    }
}
