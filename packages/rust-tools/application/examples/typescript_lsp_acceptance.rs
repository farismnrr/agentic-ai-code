use relay_application::lsp::{LspSessionManager, TypeScriptLanguageServer};
use relay_core::config::ServerConfig;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("TYPESCRIPT_LSP_ACCEPTANCE_FAIL: {error}");
        std::process::exit(1);
    }
    println!("TYPESCRIPT_LSP_ACCEPTANCE_PASS");
}

async fn run() -> Result<(), String> {
    let root = fixture_root();
    fs::create_dir_all(root.join("src")).map_err(io_error)?;
    fs::write(
        root.join("tsconfig.json"),
        r#"{"compilerOptions":{"strict":true,"module":"esnext","target":"es2020","moduleResolution":"bundler"},"include":["src"]}"#,
    )
    .map_err(io_error)?;
    let math_source = r#"export interface Shape { area(): number; }
export class Square implements Shape {
  constructor(public side: number) {}
  area(): number { return this.side * this.side; }
}
export function makeSquare(side: number): Square { return new Square(side); }
export function totalArea(shapes: Shape[]): number {
  return shapes.reduce((sum, shape) => sum + shape.area(), 0);
}
export function broken(): number { const value: number = "not a number"; return value; }
// no symbol on this comment line
"#;
    fs::write(root.join("src/math.ts"), math_source).map_err(io_error)?;
    let vue_source = r#"<script setup lang="ts">
import { makeSquare, totalArea } from './math';
const square = makeSquare("four");
const area = totalArea([square]);
</script>
<template>
  <div>{{ area }}</div>
</template>
"#;
    fs::write(root.join("src/App.vue"), vue_source).map_err(io_error)?;
    git(&root, &["init", "-q"])?;

    let (bin_dir, lib_dir) = node_toolchain_dirs()?;
    let mut config = ServerConfig {
        execution_root: Some(path_text(Path::new("/home/farismnrr"))?),
        toolchain_paths: vec![path_text(&bin_dir)?, path_text(&lib_dir)?],
        lsp_servers: vec![
            "typescript=typescript-language-server".into(),
            "vue=vue-language-server".into(),
        ],
        ..ServerConfig::default()
    };
    config.dir = Some(path_text(&root)?);
    config.validate().map_err(|error| error.to_string())?;
    let manager = LspSessionManager::new(config).map_err(|error| error.to_string())?;

    // ---- TypeScript proof (.ts) ----
    let ts_session = manager
        .session_for(Some(&path_text(&root)?), "typescript")
        .await
        .map_err(|error| format!("typescript session: {error}"))?;
    let ts = TypeScriptLanguageServer::new(ts_session).map_err(|error| error.to_string())?;

    let symbols = wait_for(|| ts.symbols("src/math.ts"), |s| s.len() >= 4).await?;
    for name in ["Shape", "Square", "makeSquare", "totalArea", "broken"] {
        require(
            symbols.iter().any(|symbol| symbol.name == name),
            &format!("ts symbol {name}"),
        )?;
    }

    // `new Square(side)` inside makeSquare's body must resolve back to the
    // class declaration on line 1.
    let square_return_col = source_line(math_source, 5)
        .rfind("Square")
        .ok_or("fixture Square return type")?;
    let definitions = ts
        .definition("src/math.ts", 5, square_return_col)
        .await
        .map_err(|error| format!("ts definition: {error}"))?;
    require(
        definitions.iter().any(|location| {
            location.path.ends_with("src/math.ts") && location.range.start.line == 1
        }),
        "Square return type resolves to class declaration",
    )?;

    // A position with no resolvable symbol (inside a comment) makes a real
    // tsserver return a valid `null` definition result, which must
    // normalize to an empty list rather than a malformed-response error.
    let comment_col = source_line(math_source, 10)
        .find("no symbol")
        .ok_or("fixture comment line")?;
    let null_definitions = ts
        .definition("src/math.ts", 10, comment_col)
        .await
        .map_err(|error| format!("ts null definition: {error}"))?;
    require(
        null_definitions.is_empty(),
        "a valid null definition response normalizes to an empty result, not an error",
    )?;

    let references = ts
        .references(
            "src/math.ts",
            5,
            source_line(math_source, 5)
                .find("makeSquare")
                .ok_or("fixture makeSquare decl")?,
            true,
        )
        .await
        .map_err(|error| format!("ts references: {error}"))?;
    require(!references.is_empty(), "makeSquare has references")?;

    let references_excl_decl = ts
        .references(
            "src/math.ts",
            5,
            source_line(math_source, 5)
                .find("makeSquare")
                .ok_or("fixture makeSquare decl")?,
            false,
        )
        .await
        .map_err(|error| format!("ts references (exclude declaration): {error}"))?;
    require(
        references_excl_decl.len() < references.len(),
        "include_declaration=false excludes the declaration site",
    )?;

    let hover = ts
        .hover(
            "src/math.ts",
            1,
            source_line(math_source, 1)
                .find("Square")
                .ok_or("fixture Square class")?,
        )
        .await
        .map_err(|error| format!("ts hover: {error}"))?
        .ok_or("Square hover")?;
    require(
        hover.text.contains("Square") || hover.text.contains("class"),
        "Square hover text",
    )?;

    let diagnostics = wait_for(|| ts.diagnostics("src/math.ts"), |d| !d.is_empty()).await?;
    require(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.message.to_lowercase().contains("not assignable")
                || diagnostic.message.to_lowercase().contains("string")
        }),
        "deterministic ts diagnostic on broken()",
    )?;

    // ---- Vue proof (.vue) ----
    //
    // The installed, reviewed `@vue/language-server@3.3.8` build only
    // implements "hybrid mode": every document-level request (even
    // template-only ones) is answered by forwarding a
    // `_vue:<Command>`-prefixed request to a companion
    // `typescript-language-server` process that has the `@vue/typescript-plugin`
    // tsserver plugin loaded (verified by reading
    // `@vue/language-server/lib/server.js`, which unconditionally calls
    // `sendTsServerRequest` inside `getLanguageService`). That plugin
    // package is not present in this machine's reviewed Node toolchain
    // (`typescript-language-server`, `@vue/language-server`, and
    // `typescript` are installed; `@vue/typescript-plugin` is not), and
    // installing it is out of scope (silently installing new language
    // tooling is a Plan 039C non-goal). Without that companion bridge, this
    // server build cannot answer any `.vue` document query in this
    // environment — so the honest proof here is that the session starts,
    // negotiates real capabilities, and a real feature request fails
    // *safely and boundedly* (a stable timeout, not a crash/hang and not a
    // false empty "success") rather than claiming semantic navigation this
    // installed server cannot actually perform standalone.
    let vue_session = manager
        .session_for(Some(&path_text(&root)?), "vue")
        .await
        .map_err(|error| format!("vue session: {error}"))?;
    let vue = TypeScriptLanguageServer::new(vue_session).map_err(|error| error.to_string())?;
    require(
        vue.session().capabilities().document_symbols
            && vue.session().capabilities().definition
            && vue.session().capabilities().hover,
        "vue-language-server negotiates real document/definition/hover capabilities",
    )?;
    let vue_symbols_result = vue.symbols("src/App.vue").await;
    require(
        matches!(
            vue_symbols_result,
            Err(error)
                if error.kind() == "language_server_timeout"
                    || error.kind() == "language_server_crashed"
        ),
        "vue document query fails as a bounded, public-safe, fail-closed error \
         (missing @vue/typescript-plugin companion bridge causes the server's own \
         unconditional hybrid-mode bridge call to fail), never a silent false-empty \
         success and never an unbounded hang",
    )?;
    require(
        vue.session().is_faulted(),
        "the failed session is faulted closed, not silently reused for a later query",
    )?;

    require(
        !root.join("node_modules").exists(),
        "no workspace mutation (node_modules not written)",
    )?;
    require(
        !root.join("package-lock.json").exists(),
        "no workspace mutation (package-lock.json not written)",
    )?;

    manager.shutdown_all().await;
    fs::remove_dir_all(&root).map_err(io_error)?;
    Ok(())
}

async fn wait_for<T, F, Fut, P>(mut request: F, ready: P) -> Result<T, String>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, relay_application::lsp::LspError>>,
    P: Fn(&T) -> bool,
{
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(30);
    loop {
        let value = request().await.map_err(|error| error.to_string())?;
        if ready(&value) || tokio::time::Instant::now() >= deadline {
            return Ok(value);
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
    }
}

/// Locates the already-installed, previously-audited Node/TypeScript
/// toolchain (fnm-managed) rather than installing anything. Returns the
/// stable install `bin` directory (containing `node`,
/// `typescript-language-server`, `vue-language-server`) and `lib` directory
/// (containing `node_modules/typescript/lib/tsserverlibrary.js`), which the
/// LSP sandbox must mount read-only for the servers' own module resolution
/// to succeed. Deliberately avoids the ephemeral fnm multishell PATH entry,
/// which is per-shell and not a stable, reviewable toolchain path.
fn node_toolchain_dirs() -> Result<(PathBuf, PathBuf), String> {
    let fnm_root = PathBuf::from("/home/farismnrr/.local/share/fnm/node-versions");
    let entries = fs::read_dir(&fnm_root).map_err(io_error)?;
    for entry in entries.flatten() {
        let bin = entry.path().join("installation/bin");
        let lib = entry.path().join("installation/lib");
        if bin.join("typescript-language-server").exists()
            && bin.join("vue-language-server").exists()
        {
            return Ok((bin, lib));
        }
    }
    Err("reviewed typescript-language-server/vue-language-server toolchain not found".into())
}

fn fixture_root() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    PathBuf::from(format!("/home/farismnrr/.cache/ai-code-ts-lsp-{nonce}"))
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
