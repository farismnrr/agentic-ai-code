use relay_application::git::dispatch_git_tool;
use relay_core::config::ServerConfig;
use serde_json::json;
use std::{fs, os::unix::fs::PermissionsExt, path::Path, process::Command};
fn run(dir: &Path, args: &[&str]) {
    assert!(Command::new("git")
        .current_dir(dir)
        .args(args)
        .status()
        .unwrap()
        .success())
}
fn response(base: &Path, payload: &str) {
    fs::write(base.join("response.json"), payload).unwrap()
}
fn argv(base: &Path) -> Vec<String> {
    fs::read_to_string(base.join("argv.log"))
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect()
}
#[tokio::main]
async fn main() {
    let base = std::env::temp_dir().join(format!("relay-044b-actions-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    let repo = base.join("repo");
    fs::create_dir_all(&repo).unwrap();
    run(&repo, &["init", "-q", "-b", "main"]);
    run(
        &repo,
        &[
            "remote",
            "add",
            "origin",
            "https://github.com/farismnrr/ai-code.git",
        ],
    );
    let fake = base.join("fake-gh");
    fs::write(
        &fake,
        format!(
            "#!/usr/bin/env bash\nprintf '%s\\n' \"$@\" > '{}/argv.log'\ncat '{}/response.json'\n",
            base.display(),
            base.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).unwrap();
    std::env::set_var("RELAY_TEST_GH_PATH", &fake);
    let config = ServerConfig {
        execution_root: Some(base.to_string_lossy().into()),
        dir: Some(repo.to_string_lossy().into()),
        ..ServerConfig::default()
    };
    response(
        &base,
        r#"[{"id":1,"name":"CI","path":".github/workflows/ci.yml","state":"active"}]"#,
    );
    let out = dispatch_git_tool("workflow_list", &json!({"cwd":repo}), &config)
        .await
        .unwrap()
        .unwrap();
    assert!(out.content[0].text.contains("CI"));
    assert_eq!(
        argv(&base)[0..4],
        ["workflow", "list", "--repo", "farismnrr/ai-code"]
    );
    response(
        &base,
        r#"[{"databaseId":42,"name":"CI","workflowName":"CI","displayTitle":"test","event":"push","headBranch":"main","headSha":"aaaaaaaa","status":"completed","conclusion":"success","createdAt":"x","startedAt":"x","updatedAt":"x","url":"https://github.com/farismnrr/ai-code/actions/runs/42","attempt":1,"number":7}]"#,
    );
    let out = dispatch_git_tool(
        "workflow_run_list",
        &json!({"cwd":repo,"branch":"main"}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    assert!(out.content[0].text.contains("42"));
    assert!(argv(&base).contains(&"--branch".into()));
    response(
        &base,
        r#"{"databaseId":42,"name":"CI","workflowName":"CI","displayTitle":"test","event":"push","headBranch":"main","headSha":"aaaaaaaa","status":"completed","conclusion":"failure","createdAt":"x","startedAt":"x","updatedAt":"x","url":"https://github.com/farismnrr/ai-code/actions/runs/42","attempt":1,"number":7,"jobs":[{"databaseId":99,"name":"test","status":"completed","conclusion":"failure","startedAt":"x","completedAt":"x","url":"https://github.com/farismnrr/ai-code/actions/runs/42/job/99"}]}"#,
    );
    let out = dispatch_git_tool(
        "workflow_run_get",
        &json!({"cwd":repo,"number":42}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    assert!(out.content[0].text.contains("99"));
    let out = dispatch_git_tool(
        "workflow_job_get",
        &json!({"cwd":repo,"number":99}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    assert!(out.content[0].text.contains("test"));
    response(
        &base,
        "failure\tstep\tAuthorization: Bearer ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZ123456\nnormal line\n",
    );
    let out = dispatch_git_tool(
        "workflow_run_job_log",
        &json!({"cwd":repo,"number":42,"job_id":99,"max_lines":10}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    let text = &out.content[0].text;
    assert!(!text.contains("ghp_"));
    assert!(text.contains("[REDACTED]"));
    assert!(argv(&base).contains(&"--log-failed".into()));
    response(
        &base,
        r#"[{"databaseId":1,"name":"CI","url":"https://github.com/evil/repo/actions/runs/1"}]"#,
    );
    assert!(
        dispatch_git_tool("workflow_run_list", &json!({"cwd":repo}), &config)
            .await
            .is_err()
    );
    let _ = fs::remove_dir_all(&base);
    println!("044B actions acceptance: PASS");
}
