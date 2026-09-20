use crate::application::workspace::file_write;
use crate::core::config::ServerConfig;
use crate::core::error::McpError;
use serde::Serialize;
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;

const MANAGED_MARKER: &str = "masih-awam-managed:v1";

#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceBootstrapResult {
    action: String,
    workspace: String,
    initialized: bool,
    stacks: Vec<String>,
    package_manager: Option<String>,
    existing: Vec<String>,
    created: Vec<String>,
    updated: Vec<String>,
    preserved: Vec<String>,
    would_create: Vec<String>,
    would_update: Vec<String>,
    guardrail_commands: Vec<String>,
}

#[derive(Debug)]
struct BootstrapModel {
    root: PathBuf,
    stacks: Vec<String>,
    package_manager: Option<String>,
    node_fast: Vec<String>,
    node_full: Vec<String>,
    other_fast: Vec<String>,
    other_full: Vec<String>,
    source_globs: Vec<&'static str>,
}

pub fn workspace_bootstrap(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<WorkspaceBootstrapResult, McpError> {
    let action = arguments
        .get("action")
        .and_then(Value::as_str)
        .unwrap_or("inspect");
    if !matches!(action, "inspect" | "reconcile") {
        return Err(McpError::InvalidRequest(
            "workspace bootstrap action must be inspect or reconcile".into(),
        ));
    }

    let root = resolve_repository_root(config, arguments.get("cwd").and_then(Value::as_str))?;
    let model = detect_model(root)?;
    let guardrail_commands = guardrail_commands(&model);
    let templates = templates(&model, &guardrail_commands);

    let mut existing = Vec::new();
    let mut preserved = Vec::new();
    let mut would_create = Vec::new();
    let mut would_update = Vec::new();

    for (path, desired) in &templates {
        let target = model.root.join(path);
        if !target.exists() {
            would_create.push((*path).to_owned());
            continue;
        }
        existing.push((*path).to_owned());
        let current = fs::read_to_string(&target)
            .map_err(|_| McpError::InvalidRequest(format!("{path} is inaccessible")))?;
        if current.contains(MANAGED_MARKER) {
            if current != *desired {
                would_update.push((*path).to_owned());
            }
        } else {
            preserved.push((*path).to_owned());
        }
    }

    let initialized = model.root.join("ai-self/registry.yaml").is_file()
        && model.root.join(".agents/memories/README.md").is_file()
        && model
            .root
            .join(".agents/knowledge/self-improvement.md")
            .is_file();

    if action == "inspect" {
        return Ok(WorkspaceBootstrapResult {
            action: action.into(),
            workspace: model.root.to_string_lossy().into_owned(),
            initialized,
            stacks: model.stacks,
            package_manager: model.package_manager,
            existing,
            created: Vec::new(),
            updated: Vec::new(),
            preserved,
            would_create,
            would_update,
            guardrail_commands,
        });
    }

    let mut created = Vec::new();
    let mut updated = Vec::new();
    for (path, desired) in &templates {
        let target = model.root.join(path);
        if !target.exists() {
            file_write(
                &json!({
                    "cwd": model.root.to_string_lossy(),
                    "path": *path,
                    "content": desired,
                    "create_parents": true,
                    "overwrite": false
                }),
                config,
            )?;
            created.push((*path).to_owned());
            continue;
        }

        let current = fs::read_to_string(&target)
            .map_err(|_| McpError::InvalidRequest(format!("{path} is inaccessible")))?;
        if current.contains(MANAGED_MARKER) && current != *desired {
            file_write(
                &json!({
                    "cwd": model.root.to_string_lossy(),
                    "path": *path,
                    "content": desired,
                    "create_parents": true,
                    "overwrite": true
                }),
                config,
            )?;
            updated.push((*path).to_owned());
        }
    }

    Ok(WorkspaceBootstrapResult {
        action: action.into(),
        workspace: model.root.to_string_lossy().into_owned(),
        initialized: true,
        stacks: model.stacks,
        package_manager: model.package_manager,
        existing,
        created,
        updated,
        preserved,
        would_create: Vec::new(),
        would_update: Vec::new(),
        guardrail_commands,
    })
}

fn resolve_repository_root(config: &ServerConfig, cwd: Option<&str>) -> Result<PathBuf, McpError> {
    config
        .ensure_workspaces_initialized()
        .map_err(|error| McpError::Internal(error.to_string()))?;
    let guard = config
        .workspaces
        .read()
        .map_err(|_| McpError::Internal("workspace lock poisoned".into()))?;
    let resolved = crate::core::workspace_path::resolve_contained_cwd_in_allowlist(&guard, cwd)?;
    let boundary = guard
        .containing_root(&resolved)
        .unwrap_or_else(|| guard.primary_root())
        .to_path_buf();

    let output = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(&resolved)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .output()
        .map_err(|_| McpError::InvalidRequest("Git repository verification failed".into()))?;
    if !output.status.success() {
        return Err(McpError::InvalidRequest(
            "workspace bootstrap requires a Git repository".into(),
        ));
    }
    let root = String::from_utf8(output.stdout)
        .map_err(|_| McpError::InvalidRequest("Git repository root is invalid".into()))?;
    let canonical = fs::canonicalize(root.trim())
        .map_err(|_| McpError::InvalidRequest("repository root is inaccessible".into()))?;
    if !canonical.starts_with(&boundary) {
        return Err(McpError::InvalidRequest(
            "repository root escapes authorized workspace".into(),
        ));
    }
    Ok(canonical)
}

fn detect_model(root: PathBuf) -> Result<BootstrapModel, McpError> {
    let mut stacks = Vec::new();
    let mut package_manager = None;
    let mut node_fast = Vec::new();
    let mut node_full = Vec::new();
    let mut other_fast = Vec::new();
    let mut other_full = Vec::new();
    let mut source_globs = Vec::new();

    if root.join("package.json").is_file() {
        stacks.push("node".into());
        source_globs.extend(["*.js", "*.jsx", "*.ts", "*.tsx", "*.vue", "*.mjs", "*.cjs"]);
        package_manager = Some(
            if root.join("pnpm-lock.yaml").exists() {
                "pnpm"
            } else if root.join("bun.lock").exists() || root.join("bun.lockb").exists() {
                "bun"
            } else if root.join("yarn.lock").exists() {
                "yarn"
            } else {
                "npm"
            }
            .into(),
        );

        let raw = fs::read_to_string(root.join("package.json"))
            .map_err(|_| McpError::InvalidRequest("package.json is inaccessible".into()))?;
        let package: Value = serde_json::from_str(&raw)
            .map_err(|_| McpError::InvalidRequest("package.json is invalid JSON".into()))?;
        let scripts = package
            .get("scripts")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let pm = package_manager.as_deref().unwrap_or("npm");

        for name in ["lint", "typecheck", "type-check", "check"] {
            if scripts.contains_key(name) {
                push_unique(&mut node_fast, format!("{pm} run {name}"));
            }
        }
        node_full.extend(node_fast.iter().cloned());
        for name in ["test", "build"] {
            if scripts.contains_key(name) {
                push_unique(&mut node_full, format!("{pm} run {name}"));
            }
        }
    }

    if root.join("Cargo.toml").is_file() {
        stacks.push("rust".into());
        source_globs.push("*.rs");
        other_fast.extend([
            "cargo fmt --all -- --check".into(),
            "cargo clippy --workspace --all-targets -- -D warnings".into(),
            "cargo check --workspace".into(),
        ]);
        other_full.push("cargo test --workspace".into());
    }

    if root.join("pyproject.toml").is_file()
        || root.join("requirements.txt").is_file()
        || root.join("setup.py").is_file()
    {
        stacks.push("python".into());
        source_globs.push("*.py");
        let pyproject = fs::read_to_string(root.join("pyproject.toml")).unwrap_or_default();
        let prefix = if root.join("uv.lock").exists() {
            "uv run "
        } else {
            ""
        };

        if root.join("ruff.toml").exists()
            || root.join(".ruff.toml").exists()
            || pyproject.contains("[tool.ruff")
        {
            other_fast.push(format!("{prefix}ruff check ."));
        }
        if root.join("pyrightconfig.json").exists() {
            other_fast.push(format!("{prefix}pyright"));
        } else if pyproject.contains("[tool.mypy") {
            other_fast.push(format!("{prefix}mypy ."));
        }
        if root.join("pytest.ini").exists()
            || root.join("conftest.py").exists()
            || pyproject.contains("[tool.pytest")
        {
            other_full.push(format!("{prefix}pytest"));
        }
    }

    if root.join("go.mod").is_file() {
        stacks.push("go".into());
        source_globs.push("*.go");
        other_fast.extend(["test -z \"$(gofmt -l .)\"".into(), "go vet ./...".into()]);
        other_full.push("go test ./...".into());
    }

    if stacks.is_empty() {
        stacks.push("generic".into());
    }

    Ok(BootstrapModel {
        root,
        stacks,
        package_manager,
        node_fast,
        node_full,
        other_fast,
        other_full,
        source_globs,
    })
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.contains(&value) {
        values.push(value);
    }
}

fn guardrail_commands(model: &BootstrapModel) -> Vec<String> {
    let mut commands = model.node_full.clone();
    for command in model.other_fast.iter().chain(model.other_full.iter()) {
        push_unique(&mut commands, command.clone());
    }
    commands
}

fn templates(model: &BootstrapModel, commands: &[String]) -> Vec<(&'static str, String)> {
    let stacks = model.stacks.join(", ");
    let package_manager = model.package_manager.as_deref().unwrap_or("none detected");
    let tooling_commands = if commands.is_empty() {
        "- No existing stack-native lint/typecheck/test/build command was safely detected. Add project-native checks before claiming those gates.".into()
    } else {
        commands
            .iter()
            .map(|command| format!("- {command}"))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let mut templates = vec![
        (
            "AGENTS.md",
            "# AGENTS.md\n\nProject-owned agent guidance lives in .agents/. Start at .agents/README.md, then read the knowledge files relevant to the task, the single canonical memory, and any current numbered plan.\n\nBefore mutation, resolve and verify this repository fresh. Existing project conventions remain authoritative. Before completion, follow .agents/knowledge/self-improvement.md and run the applicable local guardrail.\n".into(),
        ),
        (
            "ai-self/BOOTSTRAP.md",
            "# Masih Awam Workspace Bootstrap\n\nThis repository is initialized for portable agent work. Resolve the Git root fresh before mutation, keep writes inside that verified root, read ai-self/CONSTITUTION.md, ai-self/registry.yaml, and applicable .agents/ guidance, and never infer delivery stages such as commit/push/PR/merge/deploy from earlier authorization.\n\nFor milestones, blockers, handoffs, and completion reports use: Workspace, Issue(s), Work Completed, Verification, Next Steps, and Restart / Operator Action. Report only checks and actions that actually occurred.\n".into(),
        ),
        (
            "ai-self/CONSTITUTION.md",
            "# Workspace Constitution\n\n1. Verify the current repository root before mutation.\n2. Preserve existing project conventions and security boundaries.\n3. Prefer existing stack-native tooling; do not install or replace dependencies without authorization.\n4. Keep durable context under .agents/ concise and current.\n5. Treat inspect, implement, verify, install, restart, deploy, commit, push, PR, and merge as distinct authorization stages.\n6. Never claim unrun verification or unperformed delivery.\n7. Guardrails are local evidence, not proof of deployed/live behavior.\n".into(),
        ),
        (
            "ai-self/registry.yaml",
            "schema_version: 1\ngovernance_root: .agents\ncanonical_memory: .agents/memories/README.md\nplans_root: .agents/plans\nskills_root: .agents/skills\nself_improvement: .agents/knowledge/self-improvement.md\nguardrail: scripts/guardrail.sh\n".into(),
        ),
        (
            ".agents/README.md",
            "# Agent Guidance\n\nUse this directory for durable, vendor-neutral project context.\n\n- memories/README.md: the single canonical durable memory.\n- knowledge/project.md: current project/stack orientation.\n- knowledge/tooling.md: detected validation and guardrail commands.\n- knowledge/self-improvement.md: closeout/update lifecycle.\n- knowledge/resources.md: discoverability index for skills and resources.\n- plans/<NNN>-*.md: create only when multi-step work needs a durable plan.\n- skills/<name>/SKILL.md: add project-local reusable skills only when repeated project-specific guidance justifies them.\n".into(),
        ),
        (
            ".agents/plans/.gitkeep",
            String::new(),
        ),
        (
            ".agents/skills/.gitkeep",
            String::new(),
        ),
        (
            ".agents/memories/README.md",
            format!("# Canonical Project Memory\n\n## Workspace baseline\n\n- Masih Awam governance initialized for this repository.\n- Detected stack(s): {stacks}.\n- Detected package manager: {package_manager}.\n\nKeep this file concise. Record durable decisions, recurring traps, and non-obvious invariants; do not turn it into a session transcript.\n"),
        ),
        (
            ".agents/knowledge/project.md",
            format!("# Project Orientation\n\nDetected stack(s): {stacks}.\n\nThis file is a starting point, not a substitute for source inspection. Update it when architecture, ownership, stack, or repository layout materially changes. Existing project documentation and configuration remain authoritative.\n"),
        ),
        (
            ".agents/knowledge/tooling.md",
            format!("# Tooling and Verification\n\nDetected package manager: {package_manager}.\n\n## Adopted stack-native commands\n\n{tooling_commands}\n\n## Guardrail\n\n- Fast/checkpoint: sh scripts/guardrail.sh fast\n- Full/closure: sh scripts/guardrail.sh full\n\nThe generated guardrail uses only commands inferred from existing repository markers/configuration. It does not install dependencies. If the project later adopts different canonical commands, update this file and the guardrail together.\n"),
        ),
        (
            ".agents/knowledge/self-improvement.md",
            "# Self-improvement and closeout\n\nBefore declaring substantial workspace work complete:\n\n1. Review the task diff/findings for durable decisions, traps, or constraints.\n2. Update the single canonical .agents/memories/README.md only when something durable changed.\n3. If work belongs to a numbered plan, update that plan honestly; create a new plan only for genuinely multi-step durable work.\n4. Remove or amend guidance that became false.\n5. Re-check ownership and maintainability after structural changes.\n6. Run focused verification plus sh scripts/guardrail.sh full before closure when the generated guardrail applies.\n7. Keep source/local verification/deployed runtime/live acceptance as separate evidence layers.\n\nDo not invent memory or plan updates merely to satisfy process.\n".into(),
        ),
        (
            ".agents/knowledge/resources.md",
            "# Agent Resources\n\nRepository-local reusable skills belong under .agents/skills/<name>/SKILL.md when justified. Plans belong under .agents/plans/ only for durable multi-step work. The file layout is the source of truth; update this index when durable resources are added, removed, or moved.\n\nMasih Awam MCP may be used when available, but tool availability must be inspected rather than assumed.\n".into(),
        ),
        ("scripts/guardrail.sh", render_guardrail(model)),
    ];

    for (path, body) in &mut templates {
        if *path == ".agents/memories/README.md" {
            continue;
        }
        let marker = if path.ends_with(".md") {
            format!("<!-- {MANAGED_MARKER} -->\n")
        } else {
            format!("# {MANAGED_MARKER}\n")
        };
        body.insert_str(0, &marker);
    }

    templates
}

fn render_guardrail(model: &BootstrapModel) -> String {
    let mut fast = model.node_fast.clone();
    fast.extend(model.other_fast.iter().cloned());
    let mut full = model.node_full.clone();
    for command in model.other_fast.iter().chain(model.other_full.iter()) {
        push_unique(&mut full, command.clone());
    }

    let fast_body = render_commands(&fast);
    let full_body = render_commands(&full);
    let maintainability = render_maintainability(&model.source_globs);

    format!(
        "#!/bin/sh\nset -eu\n\nif [ \"$#\" -eq 0 ]; then mode=fast; else mode=\"$1\"; fi\ncase \"$mode\" in\n  fast)\n{fast_body}    ;;\n  full)\n{full_body}{maintainability}    ;;\n  *) echo \"usage: scripts/guardrail.sh [fast|full]\" >&2; exit 2 ;;\nesac\n"
    )
}

fn render_commands(commands: &[String]) -> String {
    if commands.is_empty() {
        return "    echo \"guardrail: no stack-native command detected; governance checks only\"\n".into();
    }
    commands
        .iter()
        .map(|command| format!("    echo '+ {command}'\n    {command}\n"))
        .collect()
}

fn render_maintainability(globs: &[&str]) -> String {
    if globs.is_empty() {
        return String::new();
    }
    let predicates = globs
        .iter()
        .map(|glob| format!("-name '{glob}'"))
        .collect::<Vec<_>>()
        .join(" -o ");
    format!(
        "    echo '+ maintainability baseline (warn >400 lines, fail >500 lines)'\n    find . -type f \\( {predicates} \\) \\\n      ! -path './.git/*' ! -path './node_modules/*' ! -path './target/*' \\\n      ! -path './vendor/*' ! -path './dist/*' ! -path './build/*' ! -path './.nuxt/*' \\\n      -exec sh -c 'failed=0; for file do lines=$(wc -l < \"$file\"); if [ \"$lines\" -gt 500 ]; then echo \"maintainability: FAIL $file ($lines lines)\" >&2; failed=1; elif [ \"$lines\" -gt 400 ]; then echo \"maintainability: REVIEW $file ($lines lines)\" >&2; fi; done; exit $failed' sh {{}} +\n"
    )
}
