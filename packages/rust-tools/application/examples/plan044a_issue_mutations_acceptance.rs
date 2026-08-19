use relay_application::git::dispatch_git_tool;
use relay_core::config::ServerConfig;
use serde_json::{json, Value};
use std::{fs, os::unix::fs::PermissionsExt, path::Path, process::Command};

fn run(dir: &Path, args: &[&str]) {
    assert!(Command::new("git")
        .current_dir(dir)
        .args(args)
        .status()
        .unwrap()
        .success());
}

fn set_exit_code(base: &Path, exit_code: i32) {
    fs::write(base.join("exit_code"), exit_code.to_string()).unwrap();
}

fn set_override_response(base: &Path, payload: &str) {
    fs::write(base.join("override_response.json"), payload).unwrap();
}

fn set_view_response(base: &Path, payload: &str) {
    fs::write(base.join("view_response.json"), payload).unwrap();
}

fn read_last_argv(base: &Path) -> Vec<String> {
    let content = fs::read_to_string(base.join("argv.log")).unwrap_or_default();
    content
        .lines()
        .filter_map(|l| l.strip_prefix("ARG:").map(str::to_owned))
        .collect()
}

fn read_all_argv_blocks(base: &Path) -> Vec<Vec<String>> {
    let content = fs::read_to_string(base.join("all_argv.log")).unwrap_or_default();
    content
        .split("---BLOCK---\n")
        .map(|block| {
            block
                .lines()
                .filter_map(|l| l.strip_prefix("ARG:").map(str::to_owned))
                .collect()
        })
        .filter(|v: &Vec<String>| !v.is_empty())
        .collect()
}

fn clear_argv_logs(base: &Path) {
    let _ = fs::remove_file(base.join("argv.log"));
    let _ = fs::remove_file(base.join("all_argv.log"));
    let _ = fs::remove_file(base.join("exit_code"));
    let _ = fs::remove_file(base.join("override_response.json"));
}

async fn assert_err(tool: &str, args: Value, config: &ServerConfig, expected_err: &str) {
    let err = dispatch_git_tool(tool, &args, config).await.unwrap_err();
    assert!(
        format!("{err:?}").contains(expected_err),
        "tool {tool} with args {args:?} expected error containing {expected_err:?}, got {err:?}"
    );
}

#[tokio::main]
async fn main() {
    let base = std::env::temp_dir().join(format!("relay-044a-mut-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    let repo = base.join("repo");
    fs::create_dir_all(&repo).unwrap();

    run(&repo, &["init", "-q", "-b", "main"]);
    run(&repo, &["config", "user.email", "fixture@example.test"]);
    run(&repo, &["config", "user.name", "fixture"]);
    fs::write(repo.join("README.md"), "test\n").unwrap();
    run(&repo, &["add", "README.md"]);
    run(&repo, &["commit", "-qm", "init"]);
    run(
        &repo,
        &[
            "remote",
            "add",
            "origin",
            "https://github.com/farismnrr/ai-code.git",
        ],
    );

    let fake_gh = base.join("fake-gh");
    let script = format!(
        r#"#!/usr/bin/env bash
base="{}"
rm -f "$base/argv.log"
for arg in "$@"; do
    printf 'ARG:%s\n' "$arg" >> "$base/argv.log"
    printf 'ARG:%s\n' "$arg" >> "$base/all_argv.log"
done
echo '---BLOCK---' >> "$base/all_argv.log"
cmd="$1"
subcmd="$2"
if [ -f "$base/exit_code" ]; then
    code=$(cat "$base/exit_code"); rm -f "$base/exit_code"; exit "$code"
fi
if [ -f "$base/override_response.json" ]; then
    cat "$base/override_response.json"; rm -f "$base/override_response.json"; exit 0
fi
if [ "$cmd" = "issue" ] && [ "$subcmd" = "create" ]; then
    echo "https://github.com/farismnrr/ai-code/issues/42"; exit 0
fi
if [ "$cmd" = "issue" ] && [ "$subcmd" = "edit" ]; then exit 0; fi
if [ "$cmd" = "issue" ] && [ "$subcmd" = "comment" ]; then
    num="$3"; echo "https://github.com/farismnrr/ai-code/issues/$num#issuecomment-987654321"; exit 0
fi
if [ "$cmd" = "issue" ] && [ "$subcmd" = "view" ]; then
    if [ -f "$base/view_response.json" ]; then cat "$base/view_response.json"
    else echo '{{"number":42,"title":"Default","url":"https://github.com/farismnrr/ai-code/issues/42","state":"OPEN","stateReason":"","author":{{"login":"alice"}},"labels":[],"createdAt":"2026-08-19T10:00:00Z","updatedAt":"2026-08-19T10:00:00Z","closedAt":null,"body":"Default body"}}'
    fi
    exit 0
fi
exit 0
"#,
        base.display()
    );
    fs::write(&fake_gh, script).unwrap();
    fs::set_permissions(&fake_gh, fs::Permissions::from_mode(0o755)).unwrap();
    std::env::set_var("RELAY_TEST_GH_PATH", &fake_gh);

    let config = ServerConfig {
        execution_root: Some(base.to_string_lossy().into()),
        dir: Some(repo.to_string_lossy().into()),
        ..ServerConfig::default()
    };

    // 1. issue_create Tests
    assert_err(
        "issue_create",
        json!({"cwd": repo}),
        &config,
        "title is required",
    )
    .await;
    for t in ["", "   ", "a\0b", &"a".repeat(257)] {
        assert_err(
            "issue_create",
            json!({"cwd": repo, "title": t}),
            &config,
            "title is invalid",
        )
        .await;
    }
    for b in ["a\0b", &"a".repeat(64 * 1024 + 1)] {
        assert_err(
            "issue_create",
            json!({"cwd": repo, "title": "t", "body": b}),
            &config,
            "body is invalid",
        )
        .await;
    }
    let too_many: Vec<String> = (0..51).map(|i| format!("l{i}")).collect();
    assert_err(
        "issue_create",
        json!({"cwd": repo, "title": "t", "labels": too_many}),
        &config,
        "issue labels exceed maximum",
    )
    .await;
    for l in ["", "   ", "has\nnewline", "has\0nul", &"a".repeat(129)] {
        assert_err(
            "issue_create",
            json!({"cwd": repo, "title": "t", "labels": [l]}),
            &config,
            "issue label is invalid",
        )
        .await;
    }

    clear_argv_logs(&base);
    let view_created = json!({
        "number": 42, "title": "Bug in parser", "url": "https://github.com/farismnrr/ai-code/issues/42",
        "state": "OPEN", "stateReason": "", "author": { "login": "alice" },
        "labels": [{ "name": "bug" }, { "name": "forge" }],
        "createdAt": "2026-08-19T12:00:00Z", "updatedAt": "2026-08-19T12:00:00Z", "closedAt": null,
        "body": "Detailed steps to reproduce."
    });
    set_view_response(&base, &view_created.to_string());
    let res = dispatch_git_tool(
        "issue_create",
        &json!({"cwd": repo, "title": "Bug in parser", "body": "Detailed steps to reproduce.", "labels": ["bug", "forge"]}),
        &config,
    ).await.unwrap().unwrap();
    let data: Value = serde_json::from_str(&res.content[0].text).unwrap();
    assert_eq!(data["issue"]["number"], 42);
    assert_eq!(data["issue"]["title"], "Bug in parser");
    assert_eq!(data["issue"]["labels"], json!(["bug", "forge"]));
    assert_eq!(data["forge"]["owner"], "farismnrr");

    let blocks = read_all_argv_blocks(&base);
    assert_eq!(blocks.len(), 2);
    assert_eq!(
        blocks[0],
        vec![
            "issue",
            "create",
            "--repo",
            "farismnrr/ai-code",
            "--title",
            "Bug in parser",
            "--body",
            "Detailed steps to reproduce.",
            "--label",
            "bug",
            "--label",
            "forge"
        ]
    );
    assert_eq!(
        blocks[1],
        vec![
            "issue",
            "view",
            "42",
            "--repo",
            "farismnrr/ai-code",
            "--json",
            "number,title,url,state,stateReason,author,labels,createdAt,updatedAt,closedAt,body"
        ]
    );

    clear_argv_logs(&base);
    let _ = dispatch_git_tool(
        "issue_create",
        &json!({"cwd": repo, "title": "Min"}),
        &config,
    )
    .await
    .unwrap();
    let blocks = read_all_argv_blocks(&base);
    assert_eq!(
        blocks[0],
        vec![
            "issue",
            "create",
            "--repo",
            "farismnrr/ai-code",
            "--title",
            "Min",
            "--body",
            ""
        ]
    );

    set_override_response(&base, "https://github.com/evil-org/ai-code/issues/42\n");
    assert_err(
        "issue_create",
        json!({"cwd": repo, "title": "Evil"}),
        &config,
        "created issue identity is invalid",
    )
    .await;
    set_override_response(&base, "garbage output\n");
    assert_err(
        "issue_create",
        json!({"cwd": repo, "title": "Garbage"}),
        &config,
        "created issue identity is invalid",
    )
    .await;
    set_exit_code(&base, 1);
    assert_err(
        "issue_create",
        json!({"cwd": repo, "title": "Fail"}),
        &config,
        "forge operation failed",
    )
    .await;

    // 2. issue_update Tests
    assert_err(
        "issue_update",
        json!({"cwd": repo}),
        &config,
        "issue number is required",
    )
    .await;
    assert_err(
        "issue_update",
        json!({"cwd": repo, "number": 0}),
        &config,
        "issue number is required",
    )
    .await;
    assert_err(
        "issue_update",
        json!({"cwd": repo, "number": 42}),
        &config,
        "no issue update was supplied",
    )
    .await;
    assert_err(
        "issue_update",
        json!({"cwd": repo, "number": 42, "add_labels": [], "remove_labels": []}),
        &config,
        "no issue update was supplied",
    )
    .await;

    clear_argv_logs(&base);
    let view_updated = json!({
        "number": 42, "title": "Updated Title", "url": "https://github.com/farismnrr/ai-code/issues/42",
        "state": "OPEN", "stateReason": "", "author": { "login": "alice" },
        "labels": [{ "name": "enhancement" }],
        "createdAt": "2026-08-19T12:00:00Z", "updatedAt": "2026-08-19T13:00:00Z", "closedAt": null,
        "body": "New updated body."
    });
    set_view_response(&base, &view_updated.to_string());
    let res = dispatch_git_tool(
        "issue_update",
        &json!({"cwd": repo, "number": 42, "title": "Updated Title", "body": "New updated body.", "add_labels": ["enhancement"], "remove_labels": ["bug"]}),
        &config,
    ).await.unwrap().unwrap();
    let data: Value = serde_json::from_str(&res.content[0].text).unwrap();
    assert_eq!(data["issue"]["title"], "Updated Title");
    assert_eq!(data["issue"]["labels"], json!(["enhancement"]));

    let blocks = read_all_argv_blocks(&base);
    assert_eq!(blocks.len(), 2);
    assert_eq!(
        blocks[0],
        vec![
            "issue",
            "edit",
            "42",
            "--repo",
            "farismnrr/ai-code",
            "--title",
            "Updated Title",
            "--body",
            "New updated body.",
            "--add-label",
            "enhancement",
            "--remove-label",
            "bug"
        ]
    );
    assert_eq!(
        blocks[1],
        vec![
            "issue",
            "view",
            "42",
            "--repo",
            "farismnrr/ai-code",
            "--json",
            "number,title,url,state,stateReason,author,labels,createdAt,updatedAt,closedAt,body"
        ]
    );

    set_exit_code(&base, 1);
    assert_err(
        "issue_update",
        json!({"cwd": repo, "number": 42, "title": "New"}),
        &config,
        "forge operation failed",
    )
    .await;

    // 3. issue_comment Tests
    assert_err(
        "issue_comment",
        json!({"cwd": repo}),
        &config,
        "issue number is required",
    )
    .await;
    assert_err(
        "issue_comment",
        json!({"cwd": repo, "number": 0, "body": "c"}),
        &config,
        "issue number is required",
    )
    .await;
    assert_err(
        "issue_comment",
        json!({"cwd": repo, "number": 42}),
        &config,
        "body is required",
    )
    .await;
    for b in ["", "   ", "a\0b", &"a".repeat(64 * 1024 + 1)] {
        assert_err(
            "issue_comment",
            json!({"cwd": repo, "number": 42, "body": b}),
            &config,
            "body is invalid",
        )
        .await;
    }

    clear_argv_logs(&base);
    let res = dispatch_git_tool(
        "issue_comment",
        &json!({"cwd": repo, "number": 42, "body": "Fixed in commit 123456"}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    let data: Value = serde_json::from_str(&res.content[0].text).unwrap();
    assert_eq!(data["issueNumber"], 42);
    assert_eq!(
        data["commentUrl"],
        "https://github.com/farismnrr/ai-code/issues/42#issuecomment-987654321"
    );
    assert_eq!(data["forge"]["owner"], "farismnrr");

    let argv = read_last_argv(&base);
    assert_eq!(
        argv,
        vec![
            "issue",
            "comment",
            "42",
            "--repo",
            "farismnrr/ai-code",
            "--body",
            "Fixed in commit 123456"
        ]
    );

    set_override_response(
        &base,
        "https://github.com/farismnrr/ai-code/issues/99#issuecomment-123\n",
    );
    assert_err(
        "issue_comment",
        json!({"cwd": repo, "number": 42, "body": "c"}),
        &config,
        "comment identity is invalid",
    )
    .await;
    set_override_response(&base, "https://github.com/farismnrr/ai-code/issues/42\n");
    assert_err(
        "issue_comment",
        json!({"cwd": repo, "number": 42, "body": "c"}),
        &config,
        "comment identity is invalid",
    )
    .await;
    set_override_response(&base, "not a url\n");
    assert_err(
        "issue_comment",
        json!({"cwd": repo, "number": 42, "body": "c"}),
        &config,
        "comment identity is invalid",
    )
    .await;
    set_exit_code(&base, 1);
    assert_err(
        "issue_comment",
        json!({"cwd": repo, "number": 42, "body": "c"}),
        &config,
        "forge operation failed",
    )
    .await;

    let _ = fs::remove_dir_all(&base);
    println!("plan044a issue mutations acceptance: PASS");
}
