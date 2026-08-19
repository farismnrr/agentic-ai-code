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

fn set_fixture_response(base: &Path, payload: &str, exit_code: i32) {
    fs::write(base.join("response.json"), payload).unwrap();
    fs::write(base.join("exit_code"), exit_code.to_string()).unwrap();
}

fn read_argv(base: &Path) -> Vec<String> {
    let content = fs::read_to_string(base.join("argv.log")).unwrap_or_default();
    content.lines().map(str::to_owned).collect()
}

#[tokio::main]
async fn main() {
    let base = std::env::temp_dir().join(format!("relay-044a-issues-{}", std::process::id()));
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

    // Create fake gh provider executable
    let fake_gh = base.join("fake-gh");
    let script = format!(
        "#!/usr/bin/env bash\n\
         printf '%s\\n' \"$@\" > \"{}/argv.log\"\n\
         if [ -f \"{}/response.json\" ]; then cat \"{}/response.json\"; fi\n\
         if [ -f \"{}/exit_code\" ]; then exit $(cat \"{}/exit_code\"); fi\n\
         exit 0\n",
        base.display(),
        base.display(),
        base.display(),
        base.display(),
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

    // 1. Client Input Validation Tests (Fails before running gh)
    for st in ["evil", "merged"] {
        let err = dispatch_git_tool("issue_list", &json!({"cwd": repo, "state": st}), &config)
            .await
            .unwrap_err();
        assert!(format!("{err:?}").contains("issue state is invalid"));
    }

    let too_many_labels: Vec<String> = (0..11).map(|i| format!("label-{i}")).collect();
    let err = dispatch_git_tool(
        "issue_list",
        &json!({"cwd": repo, "labels": too_many_labels}),
        &config,
    )
    .await
    .unwrap_err();
    assert!(format!("{err:?}").contains("issue labels exceed maximum"));

    for l in ["", "has\nnewline", "has\0nul", &"a".repeat(129)] {
        let err = dispatch_git_tool("issue_list", &json!({"cwd": repo, "labels": [l]}), &config)
            .await
            .unwrap_err();
        assert!(format!("{err:?}").contains("issue label is invalid"));
    }

    let err = dispatch_git_tool("issue_get", &json!({"cwd": repo}), &config)
        .await
        .unwrap_err();
    assert!(format!("{err:?}").contains("issue number is required"));
    let err = dispatch_git_tool("issue_get", &json!({"cwd": repo, "number": 0}), &config)
        .await
        .unwrap_err();
    assert!(format!("{err:?}").contains("issue number is required"));

    // 2. Real Production issue_list Execution & Argv Verification
    let valid_list = json!([
        {
            "number": 10,
            "title": "Fix bug",
            "url": "https://github.com/farismnrr/ai-code/issues/10",
            "state": "OPEN",
            "stateReason": "",
            "author": { "login": "alice" },
            "labels": [{ "name": "bug" }, { "name": "forge" }],
            "createdAt": "2026-08-19T10:00:00Z",
            "updatedAt": "2026-08-19T11:00:00Z",
            "closedAt": null
        },
        {
            "number": 11,
            "title": "Closed issue",
            "url": "https://github.com/farismnrr/ai-code/issues/11",
            "state": "CLOSED",
            "stateReason": "COMPLETED",
            "author": null,
            "labels": [],
            "createdAt": "2026-08-18T10:00:00Z",
            "updatedAt": "2026-08-19T10:00:00Z",
            "closedAt": "2026-08-19T10:00:00Z"
        }
    ]);
    set_fixture_response(&base, &valid_list.to_string(), 0);

    let res = dispatch_git_tool(
        "issue_list",
        &json!({"cwd": repo, "state": "open", "labels": ["bug", "forge"]}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();

    let list_data: Value = serde_json::from_str(&res.content[0].text).unwrap();
    let issues = list_data["issues"].as_array().unwrap();
    assert_eq!(issues.len(), 2);
    assert_eq!(list_data["truncated"], false);
    assert_eq!(list_data["forge"]["owner"], "farismnrr");
    assert_eq!(list_data["forge"]["repository"], "ai-code");
    assert_eq!(issues[0]["number"], 10);
    assert_eq!(issues[0]["author"], "alice");
    assert_eq!(issues[0]["labels"], json!(["bug", "forge"]));
    assert_eq!(issues[1]["number"], 11);
    assert_eq!(issues[1]["stateReason"], "COMPLETED");

    // Verify exact real issue_list argv
    let argv = read_argv(&base);
    assert_eq!(argv[0], "issue");
    assert_eq!(argv[1], "list");
    assert_eq!(argv[2], "--repo");
    assert_eq!(argv[3], "farismnrr/ai-code");
    assert_eq!(argv[4], "--state");
    assert_eq!(argv[5], "open");
    assert_eq!(argv[6], "--limit");
    assert_eq!(argv[7], "51");
    assert_eq!(argv[8], "--json");
    assert_eq!(
        argv[9],
        "number,title,url,state,stateReason,author,labels,createdAt,updatedAt,closedAt"
    );
    assert_eq!(argv[10], "--label");
    assert_eq!(argv[11], "bug");
    assert_eq!(argv[12], "--label");
    assert_eq!(argv[13], "forge");

    assert!(
        !argv.iter().any(|a| a.contains("isPullRequest")),
        "issue_list must not contain isPullRequest"
    );
    assert!(
        !argv.iter().any(|a| a.contains("comments")),
        "issue_list must not overfetch comments"
    );

    // 3. Real Production issue_get Execution & Argv Verification
    let valid_detail = json!({
        "number": 10,
        "title": "Fix bug",
        "url": "https://github.com/farismnrr/ai-code/issues/10",
        "state": "OPEN",
        "stateReason": "",
        "author": { "login": "alice" },
        "labels": [{ "name": "bug" }],
        "createdAt": "2026-08-19T10:00:00Z",
        "updatedAt": "2026-08-19T11:00:00Z",
        "closedAt": null,
        "body": "## Description\nReal issue body content."
    });
    set_fixture_response(&base, &valid_detail.to_string(), 0);

    let res = dispatch_git_tool("issue_get", &json!({"cwd": repo, "number": 10}), &config)
        .await
        .unwrap()
        .unwrap();
    let get_data: Value = serde_json::from_str(&res.content[0].text).unwrap();
    assert_eq!(get_data["issue"]["number"], 10);
    assert_eq!(
        get_data["issue"]["body"],
        "## Description\nReal issue body content."
    );

    // Verify exact real issue_get argv
    let argv = read_argv(&base);
    assert_eq!(argv[0], "issue");
    assert_eq!(argv[1], "view");
    assert_eq!(argv[2], "10");
    assert_eq!(argv[3], "--repo");
    assert_eq!(argv[4], "farismnrr/ai-code");
    assert_eq!(argv[5], "--json");
    assert_eq!(
        argv[6],
        "number,title,url,state,stateReason,author,labels,createdAt,updatedAt,closedAt,body"
    );

    assert!(
        !argv.iter().any(|a| a.contains("isPullRequest")),
        "issue_get must not contain isPullRequest"
    );
    assert!(
        !argv.iter().any(|a| a.contains("comments")),
        "issue_get must not overfetch comments"
    );

    // 4. Real Truncation Handling (> 50 items)
    let many_items: Vec<Value> = (1..=51)
        .map(|i| {
            json!({
                "number": i, "title": format!("Issue {i}"),
                "url": format!("https://github.com/farismnrr/ai-code/issues/{i}"),
                "state": "OPEN", "labels": []
            })
        })
        .collect();
    set_fixture_response(&base, &json!(many_items).to_string(), 0);
    let res = dispatch_git_tool("issue_list", &json!({"cwd": repo}), &config)
        .await
        .unwrap()
        .unwrap();
    let list_data: Value = serde_json::from_str(&res.content[0].text).unwrap();
    assert_eq!(list_data["truncated"], true);
    assert_eq!(list_data["issues"].as_array().unwrap().len(), 50);

    // 5. PR URL Rejection (/pull/ path)
    let pr_list = json!([{
        "number": 1, "title": "PR", "url": "https://github.com/farismnrr/ai-code/pull/1", "state": "MERGED", "labels": []
    }]);
    set_fixture_response(&base, &pr_list.to_string(), 0);
    let err = dispatch_git_tool("issue_list", &json!({"cwd": repo}), &config)
        .await
        .unwrap_err();
    assert!(format!("{err:?}").contains("pull request cannot be accessed as an issue"));

    let pr_detail = json!({
        "number": 1, "title": "PR", "url": "https://github.com/farismnrr/ai-code/pull/1", "state": "MERGED", "labels": []
    });
    set_fixture_response(&base, &pr_detail.to_string(), 0);
    let err = dispatch_git_tool("issue_get", &json!({"cwd": repo, "number": 1}), &config)
        .await
        .unwrap_err();
    assert!(format!("{err:?}").contains("pull request cannot be accessed as an issue"));

    // 6. Identity Mismatches (Foreign owner / foreign repo / number mismatch)
    let foreign_owner = json!([{
        "number": 10, "title": "x", "url": "https://github.com/evil/ai-code/issues/10", "state": "OPEN", "labels": []
    }]);
    set_fixture_response(&base, &foreign_owner.to_string(), 0);
    let err = dispatch_git_tool("issue_list", &json!({"cwd": repo}), &config)
        .await
        .unwrap_err();
    assert!(format!("{err:?}").contains("issue repository identity mismatch"));

    let num_mismatch = json!({
        "number": 11, "title": "x", "url": "https://github.com/farismnrr/ai-code/issues/11", "state": "OPEN", "labels": []
    });
    set_fixture_response(&base, &num_mismatch.to_string(), 0);
    let err = dispatch_git_tool("issue_get", &json!({"cwd": repo, "number": 10}), &config)
        .await
        .unwrap_err();
    assert!(format!("{err:?}").contains("issue repository identity mismatch"));

    // 7. Bounds Violations (Oversized title, body, label name, excess labels, NUL bytes)
    let bad_title = json!([{
        "number": 10, "title": "a".repeat(257), "url": "https://github.com/farismnrr/ai-code/issues/10", "state": "OPEN", "labels": []
    }]);
    set_fixture_response(&base, &bad_title.to_string(), 0);
    let err = dispatch_git_tool("issue_list", &json!({"cwd": repo}), &config)
        .await
        .unwrap_err();
    assert!(format!("{err:?}").contains("git output is invalid"));

    let bad_body = json!({
        "number": 10, "title": "x", "url": "https://github.com/farismnrr/ai-code/issues/10", "state": "OPEN", "labels": [],
        "body": "a".repeat(64 * 1024 + 1)
    });
    set_fixture_response(&base, &bad_body.to_string(), 0);
    let err = dispatch_git_tool("issue_get", &json!({"cwd": repo, "number": 10}), &config)
        .await
        .unwrap_err();
    assert!(format!("{err:?}").contains("git output is invalid"));

    let bad_label = json!([{
        "number": 10, "title": "x", "url": "https://github.com/farismnrr/ai-code/issues/10", "state": "OPEN",
        "labels": [{ "name": "a".repeat(129) }]
    }]);
    set_fixture_response(&base, &bad_label.to_string(), 0);
    let err = dispatch_git_tool("issue_list", &json!({"cwd": repo}), &config)
        .await
        .unwrap_err();
    assert!(format!("{err:?}").contains("git output is invalid"));

    let excess_labels: Vec<Value> = (0..51)
        .map(|i| json!({ "name": format!("l{i}") }))
        .collect();
    let bad_labels = json!([{
        "number": 10, "title": "x", "url": "https://github.com/farismnrr/ai-code/issues/10", "state": "OPEN",
        "labels": excess_labels
    }]);
    set_fixture_response(&base, &bad_labels.to_string(), 0);
    let err = dispatch_git_tool("issue_list", &json!({"cwd": repo}), &config)
        .await
        .unwrap_err();
    assert!(format!("{err:?}").contains("git output is invalid"));

    // 8. Malformed Provider JSON & Error Exit Codes
    set_fixture_response(&base, "not valid json", 0);
    let err = dispatch_git_tool("issue_list", &json!({"cwd": repo}), &config)
        .await
        .unwrap_err();
    assert!(format!("{err:?}").contains("forge output is invalid"));

    set_fixture_response(&base, "{}", 1);
    let err = dispatch_git_tool("issue_list", &json!({"cwd": repo}), &config)
        .await
        .unwrap_err();
    assert!(format!("{err:?}").contains("forge operation failed"));

    let _ = fs::remove_dir_all(&base);
    println!("plan044a issue reads acceptance: PASS");
}
