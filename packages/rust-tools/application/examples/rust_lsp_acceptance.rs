use relay_application::lsp::{LspSessionManager, RustLanguageServer};
use relay_core::config::ServerConfig;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("RUST_LSP_ACCEPTANCE_FAIL: {error}");
        std::process::exit(1);
    }
    println!("RUST_LSP_ACCEPTANCE_PASS");
}

async fn run() -> Result<(), String> {
    let root = fixture_root();
    fs::create_dir_all(root.join("src")).map_err(io_error)?;
    let source = r#"pub trait Render { fn render(&self) -> String; }
pub struct Café { pub value: i32 }
impl Render for Café { fn render(&self) -> String { self.value.to_string() } }
/* café */ pub fn make(value: i32) -> Café { Café { value } }
pub fn call() -> String { make(7).render() }
pub fn broken() { let _: i32 = "not an integer"; }
"#;
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"lsp-proof\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .map_err(io_error)?;
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
    let manager = LspSessionManager::new(config).map_err(|error| error.to_string())?;
    let session = manager
        .session_for(Some(&path_text(&root)?), "rust")
        .await
        .map_err(|error| error.to_string())?;
    let rust = RustLanguageServer::new(session).map_err(|error| error.to_string())?;

    let symbols = rust
        .symbols("src/lib.rs")
        .await
        .map_err(|error| format!("symbols: {error}"))?;
    for name in ["Render", "Café", "make", "call", "broken"] {
        require(
            symbols.iter().any(|symbol| symbol.name == name),
            &format!("symbol {name}"),
        )?;
    }
    rust.wait_ready()
        .await
        .map_err(|error| format!("readiness: {error}"))?;
    let diagnostics = rust
        .diagnostics("src/lib.rs")
        .await
        .map_err(|error| format!("diagnostics: {error}"))?;
    let make_col = source_line(source, 3)
        .find("make")
        .ok_or("fixture make call")?;
    let definitions = rust
        .definition("src/lib.rs", 3, make_col)
        .await
        .map_err(|error| format!("definition: {error}"))?;
    require(
        definitions.iter().any(|location| {
            location.path.ends_with("src/lib.rs") && location.range.start.line == 3
        }),
        "make definition",
    )?;
    let references = rust
        .references(
            "src/lib.rs",
            3,
            source_line(source, 3)
                .find("make")
                .ok_or("fixture make definition")?,
        )
        .await
        .map_err(|error| format!("references: {error}"))?;
    require(
        references
            .iter()
            .any(|location| location.range.start.line == 4),
        "make reference",
    )?;
    let implementations = rust
        .implementations(
            "src/lib.rs",
            0,
            source_line(source, 0)
                .find("Render")
                .ok_or("fixture trait")?,
        )
        .await
        .map_err(|error| format!("implementations: {error}"))?;
    require(!implementations.is_empty(), "Render implementation")?;
    let cafe_col = source_line(source, 1)
        .find("Café")
        .ok_or("fixture unicode")?;
    let hover = rust
        .hover("src/lib.rs", 1, cafe_col)
        .await
        .map_err(|error| format!("hover: {error}"))?
        .ok_or("Café hover")?;
    require(
        hover.text.contains("Café") || hover.text.contains("struct"),
        "Café hover text",
    )?;
    require(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.message.contains("mismatched") || diagnostic.message.contains("expected")
        }),
        "deterministic diagnostic",
    )?;
    require(
        rust.session().capabilities().text_sync
            != relay_application::lsp::TextDocumentSyncKind::None,
        "document sync capability",
    )?;

    require(
        !root.join("Cargo.lock").exists(),
        "no workspace mutation (Cargo.lock not written)",
    )?;
    require(
        !root.join("target").exists(),
        "no workspace mutation (target dir not written)",
    )?;

    manager.shutdown_all().await;
    require(
        matches!(
            rust.wait_ready().await,
            Err(error) if error.kind() == "language_server_crashed"
        ),
        "readiness fails cleanly (bounded, not indefinite) once the session is gone",
    )?;

    fs::remove_dir_all(&root).map_err(io_error)?;
    Ok(())
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
    PathBuf::from(format!("/home/farismnrr/.cache/ai-code-rust-lsp-{nonce}"))
}

fn source_line(source: &str, line: usize) -> &str {
    source.lines().nth(line).unwrap_or("")
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
fn require(condition: bool, message: &str) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(message.into())
    }
}
fn io_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}
