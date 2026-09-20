use ai_tools::application::workspace::workspace_bootstrap;
use ai_tools::core::config::ServerConfig;
use serde_json::{json, to_value};
use std::fs;
use uuid::Uuid;

fn bootstrap_fixture() -> (std::path::PathBuf, ServerConfig) {
    let root = std::env::temp_dir().join(format!("ai-tools-bootstrap-{}", Uuid::new_v4()));
    fs::create_dir_all(&root).unwrap();
    let status = std::process::Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(&root)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .status()
        .unwrap();
    assert!(status.success());
    let config = ServerConfig {
        dir: Some(root.to_string_lossy().into_owned()),
        execution_root: Some(root.to_string_lossy().into_owned()),
        ..ServerConfig::default()
    };
    (root, config)
}

#[test]
fn workspace_bootstrap_inspect_is_read_only_and_stack_aware() {
    let (root, config) = bootstrap_fixture();
    fs::write(
        root.join("package.json"),
        r#"{"scripts":{"lint":"eslint .","typecheck":"tsc --noEmit","test":"vitest","build":"vite build"}}"#,
    )
    .unwrap();
    fs::write(root.join("pnpm-lock.yaml"), "lockfileVersion: '9.0'\n").unwrap();

    let result = workspace_bootstrap(
        &json!({"action":"inspect","cwd":root.to_string_lossy()}),
        &config,
    )
    .unwrap();
    let value = to_value(result).unwrap();

    assert_eq!(value["initialized"], false);
    assert_eq!(value["stacks"], json!(["node"]));
    assert_eq!(value["package_manager"], "pnpm");
    assert!(value["created"].as_array().unwrap().is_empty());
    assert!(value["would_create"]
        .as_array()
        .unwrap()
        .iter()
        .any(|path| path == "ai-self/registry.yaml"));
    assert!(value["guardrail_commands"]
        .as_array()
        .unwrap()
        .iter()
        .any(|command| command == "pnpm run lint"));
    assert!(!root.join("AGENTS.md").exists());
    assert!(!root.join(".agents").exists());

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn workspace_bootstrap_reconcile_updates_managed_stack_without_overwriting_user_state() {
    let (root, config) = bootstrap_fixture();
    fs::write(
        root.join("package.json"),
        r#"{"scripts":{"lint":"eslint .","test":"vitest"}}"#,
    )
    .unwrap();
    fs::write(root.join("pnpm-lock.yaml"), "lockfileVersion: '9.0'\n").unwrap();
    fs::write(root.join("AGENTS.md"), "# Existing project guidance\n").unwrap();

    let first = workspace_bootstrap(
        &json!({"action":"reconcile","cwd":root.to_string_lossy()}),
        &config,
    )
    .unwrap();
    let first = to_value(first).unwrap();

    assert_eq!(first["initialized"], true);
    assert_eq!(first["stacks"], json!(["node"]));
    assert!(first["preserved"]
        .as_array()
        .unwrap()
        .iter()
        .any(|path| path == "AGENTS.md"));
    assert!(first["created"]
        .as_array()
        .unwrap()
        .iter()
        .any(|path| path == "ai-self/registry.yaml"));
    assert!(first["created"]
        .as_array()
        .unwrap()
        .iter()
        .any(|path| path == "scripts/guardrail.sh"));
    assert!(root.join(".agents/plans/.gitkeep").is_file());
    assert!(root.join(".agents/skills/.gitkeep").is_file());
    assert_eq!(
        fs::read_to_string(root.join("AGENTS.md")).unwrap(),
        "# Existing project guidance\n"
    );

    let memory_path = root.join(".agents/memories/README.md");
    fs::write(
        &memory_path,
        format!(
            "{}\n- Durable user decision stays here.\n",
            fs::read_to_string(&memory_path).unwrap()
        ),
    )
    .unwrap();

    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname='backend'\nversion='0.1.0'\n",
    )
    .unwrap();

    let second = workspace_bootstrap(
        &json!({"action":"reconcile","cwd":root.to_string_lossy()}),
        &config,
    )
    .unwrap();
    let second = to_value(second).unwrap();

    assert_eq!(second["stacks"], json!(["node", "rust"]));
    assert!(second["created"].as_array().unwrap().is_empty());
    for path in [
        ".agents/knowledge/project.md",
        ".agents/knowledge/tooling.md",
        "scripts/guardrail.sh",
    ] {
        assert!(
            second["updated"]
                .as_array()
                .unwrap()
                .iter()
                .any(|updated| updated == path),
            "{path} should be refreshed after stack growth"
        );
    }
    assert!(fs::read_to_string(&memory_path)
        .unwrap()
        .contains("Durable user decision stays here."));
    assert_eq!(
        fs::read_to_string(root.join("AGENTS.md")).unwrap(),
        "# Existing project guidance\n"
    );

    let guardrail = fs::read_to_string(root.join("scripts/guardrail.sh")).unwrap();
    assert!(guardrail.contains("pnpm run lint"));
    assert!(guardrail.contains("pnpm run test"));
    assert!(guardrail.contains("cargo fmt --all -- --check"));
    assert!(guardrail.contains("cargo test --workspace"));
    assert!(guardrail.contains("maintainability baseline"));

    fs::remove_file(root.join("Cargo.toml")).unwrap();
    let third = workspace_bootstrap(
        &json!({"action":"reconcile","cwd":root.to_string_lossy()}),
        &config,
    )
    .unwrap();
    let third = to_value(third).unwrap();
    assert_eq!(third["stacks"], json!(["node"]));
    assert!(third["updated"]
        .as_array()
        .unwrap()
        .iter()
        .any(|path| path == "scripts/guardrail.sh"));
    let shrunk_guardrail = fs::read_to_string(root.join("scripts/guardrail.sh")).unwrap();
    assert!(shrunk_guardrail.contains("pnpm run lint"));
    assert!(!shrunk_guardrail.contains("cargo fmt --all -- --check"));
    assert!(!shrunk_guardrail.contains("cargo test --workspace"));

    let fourth = workspace_bootstrap(
        &json!({"action":"reconcile","cwd":root.to_string_lossy()}),
        &config,
    )
    .unwrap();
    let fourth = to_value(fourth).unwrap();
    assert!(fourth["created"].as_array().unwrap().is_empty());
    assert!(fourth["updated"].as_array().unwrap().is_empty());

    fs::remove_dir_all(root).unwrap();
}
